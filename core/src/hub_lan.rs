//! ESP32 transports for the independent AMS-Hub page.
use alloc::{ffi::CString, format, rc::Rc, string::{String, ToString}, vec, vec::Vec};
use core::{cell::RefCell, str::FromStr};
use aes::cipher::{BlockDecrypt, KeyInit};
use base64::Engine;
use embassy_net::{tcp::TcpSocket, Ipv4Address, Stack};
use embassy_time::{with_timeout, Duration, Instant, Timer};
use embedded_io_async::{Read, Write};
use framework::prelude::*;
use mqttrust::{Packet, Publish, QoS, SubscribeTopic};
use serde_json::{json, Value};
use crate::{hub_protocol::{self as p, Brand, Config, Slot}, my_mqtt::MyMqtt};

type Result<T> = p::Result<T>;
const TIMEOUT: Duration = Duration::from_secs(12);

// HTTP/1.1 parser is bounded and accepts content-length, chunked and close-delimited responses.
pub async fn http(stack: Stack<'static>, config: &Config, port: u16, method: &str, path: &str, body: &str) -> Result<Value> {
    with_timeout(TIMEOUT, http_inner(stack, config, port, method, path, body)).await.map_err(|_|"HTTP 连接超时")?
}
async fn http_inner(stack: Stack<'static>, config: &Config, port: u16, method: &str, path: &str, body: &str) -> Result<Value> {
    if config.api_key.contains(['\r','\n']) || !path.starts_with('/') || path.contains(['\r','\n']) { return Err("请求配置无效"); }
    let ip = Ipv4Address::from_str(&config.host).map_err(|_|"请设置打印机 IPv4 地址")?;
    let mut rx = vec![0;4096]; let mut tx = vec![0;2048];
    let mut socket = TcpSocket::new(stack, &mut rx, &mut tx);
    socket.set_timeout(Some(TIMEOUT));
    socket.connect((ip,port)).await.map_err(|_|"无法连接打印机 HTTP 接口")?;
    let auth = if config.api_key.is_empty() { String::new() } else { format!("X-Api-Key: {}\r\n", config.api_key) };
    let request = format!("{method} {path} HTTP/1.1\r\nHost: {}:{port}\r\nConnection: close\r\n{auth}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",config.host,body.len());
    socket.write_all(request.as_bytes()).await.map_err(|_|"HTTP 发送失败")?;
    let mut response = Vec::new(); let mut chunk = [0u8;2048];
    loop {
        let n = socket.read(&mut chunk).await.map_err(|_|"HTTP 读取失败")?;
        if n == 0 { break; }
        if response.len()+n > 65536 { return Err("HTTP 回复超过容量限制"); }
        response.extend_from_slice(&chunk[..n]);
        if let Some(body) = decode_http(&response)? { return serde_json::from_slice(&body).map_err(|_|"HTTP JSON 无效"); }
    }
    let body = decode_http(&response)?.ok_or("HTTP 回复不完整")?;
    serde_json::from_slice(&body).map_err(|_|"HTTP JSON 无效")
}

pub fn decode_http(raw: &[u8]) -> Result<Option<Vec<u8>>> {
    let Some(end) = raw.windows(4).position(|w| w == b"\r\n\r\n") else { return Ok(None); };
    let head = core::str::from_utf8(&raw[..end]).map_err(|_|"HTTP 头无效")?;
    let code = head.split_whitespace().nth(1).and_then(|s|s.parse::<u16>().ok()).ok_or("HTTP 状态无效")?;
    if !(200..300).contains(&code) { return Err("打印机拒绝 HTTP 请求，请检查权限及接口"); }
    let body = &raw[end+4..];
    let mut length = None; let mut chunked = false;
    for line in head.lines().skip(1) {
        if let Some((key,value)) = line.split_once(':') {
            if key.eq_ignore_ascii_case("content-length") { length = Some(value.trim().parse::<usize>().map_err(|_|"HTTP 长度无效")?); }
            if key.eq_ignore_ascii_case("transfer-encoding") { chunked = value.trim().eq_ignore_ascii_case("chunked"); }
        }
    }
    if chunked {
        let mut pos=0; let mut out=Vec::new();
        loop {
            let Some(n) = body[pos..].windows(2).position(|w|w==b"\r\n") else { return Ok(None); };
            let size = core::str::from_utf8(&body[pos..pos+n]).map_err(|_|"HTTP 块无效")?;
            let size = usize::from_str_radix(size.split(';').next().unwrap(),16).map_err(|_|"HTTP 块长度无效")?;
            pos += n+2;
            if size == 0 { return Ok(Some(out)); }
            if size > 65536 || body.len().saturating_sub(pos) < size+2 { return Ok(None); }
            if &body[pos+size..pos+size+2] != b"\r\n" { return Err("HTTP 块结尾无效"); }
            out.extend_from_slice(&body[pos..pos+size]); pos+=size+2;
        }
    }
    if let Some(length) = length { return Ok((body.len()>=length).then(||body[..length].to_vec())); }
    // Close-delimited JSON can be recognised once a complete JSON value has arrived.
    Ok(serde_json::from_slice::<Value>(body).ok().map(|_|body.to_vec()))
}

pub async fn u1_read(stack: Stack<'static>, config: &Config) -> Result<Vec<Slot>> {
    p::u1_slots(&http(stack,config,7125,"GET","/printer/objects/query?print_task_config","").await?)
}
pub async fn u1_write(stack: Stack<'static>, config: &Config, requested: &Slot) -> Result<Vec<Slot>> {
    let before = u1_read(stack,config).await?;
    let old = before.iter().find(|s|s.index==requested.index).ok_or("U1 未报告目标工具头")?;
    if old.locked { return Err("该工具头使用官方 RFID 耗材，不支持覆盖"); }
    let script = p::u1_command(requested)?;
    let reply = http(stack,config,7125,"POST","/printer/gcode/script",&json!({"script":script}).to_string()).await?;
    if reply.get("error").is_some() { return Err("U1 拒绝耗材设置命令"); }
    for _ in 0..4 {
        Timer::after_millis(500).await;
        let slots=u1_read(stack,config).await?;
        if p::confirmed(Brand::U1,requested,&slots) { return Ok(slots); }
    }
    Err("U1 命令已发送，但耗材读回不一致")
}

struct Credentials { model: String, device: String, username: String, password: String, cert: String, key: String }
fn s<'a>(value: &'a Value, key: &str) -> Result<&'a str> { value[key].as_str().ok_or("Anycubic 认证数据不完整") }
fn epoch_ms() -> Result<u64> { crate::store::store_safe_time_now().filter(|t|*t>0).map(|t|t as u64*1000).ok_or("等待网络时间同步后重试") }

async fn ac_auth(stack: Stack<'static>, config: &Config) -> Result<Credentials> {
    let info=http(stack,config,18910,"GET","/info","").await?;
    if info["ctrlType"] == "cloud" { return Err("请先将 Anycubic 切换为局域网模式"); }
    let token=s(&info,"token")?;
    if !token.is_ascii() || token.len()<32 { return Err("此固件不支持已知的局域网认证协议"); }
    // Never follow a printer-supplied URL to another host or send its token off-LAN.
    let ctrl=s(&info,"ctrlInfoUrl")?;
    let prefix=format!("http://{}:18910",config.host);
    if ctrl != format!("{prefix}/ctrl") && ctrl != "/ctrl" { return Err("Anycubic 认证地址与打印机不一致"); }
    let ts=epoch_ms()?;
    let nonce=format!("{:08x}",framework::utils::random_u32());
    let first=format!("{:x}",md5::compute(&token.as_bytes()[..16]));
    let sign=format!("{:x}",md5::compute(format!("{first}{ts}{nonce}")));
    let did=format!("AMSHUB{:08X}{:08X}{:010X}",framework::utils::random_u32(),framework::utils::random_u32(),framework::utils::random_u32());
    let path=format!("/ctrl?ts={ts}&nonce={nonce}&sign={sign}&did={did}");
    let ctrl=http(stack,config,18910,"POST",&path,"").await?;
    if ctrl["code"] != 200 { return Err("Anycubic 认证被拒绝"); }
    let data=&ctrl["data"];
    let encrypted=base64::engine::general_purpose::STANDARD.decode(s(data,"info")?).map_err(|_|"认证数据编码无效")?;
    let iv=s(data,"token")?.as_bytes();
    let clear=decrypt_cbc(&encrypted,&token.as_bytes()[16..32],iv)?;
    let data:Value=serde_json::from_slice(&clear).map_err(|_|"认证数据 JSON 无效")?;
    // The printer must issue a broker on its own local address, TLS port 9883.
    if s(&data,"broker")? != format!("mqtts://{}:9883",config.host) { return Err("MQTT 地址与打印机不一致或不是 TLS 9883"); }
    let model = info["modelId"].as_str().map(ToString::to_string).or_else(||info["modelId"].as_u64().map(|v|v.to_string())).ok_or("缺少 Anycubic 型号编号")?;
    Ok(Credentials { model,device:s(&data,"deviceId")?.into(),username:s(&data,"username")?.into(),password:s(&data,"password")?.into(),
        cert:s(&data,"devicecrt")?.into(), key:s(&data,"devicepk")?.into() })
}

pub fn decrypt_cbc(input:&[u8],key:&[u8],iv:&[u8]) -> Result<Vec<u8>> {
    if input.is_empty() || input.len()%16!=0 || key.len()!=16 { return Err("认证密文长度无效"); }
    let cipher=aes::Aes128::new_from_slice(key).map_err(|_|"认证密钥无效")?;
    let mut previous=[0u8;16]; let n=iv.len().min(16); previous[..n].copy_from_slice(&iv[..n]);
    let mut out=Vec::with_capacity(input.len());
    for encrypted in input.chunks_exact(16) {
        let mut block=aes::cipher::Block::<aes::Aes128>::clone_from_slice(encrypted);
        cipher.decrypt_block(&mut block);
        out.extend(block.iter().zip(previous).map(|(a,b)|a^b)); previous.copy_from_slice(encrypted);
    }
    let pad=*out.last().unwrap() as usize;
    if pad==0 || pad>16 || out[out.len()-pad..].iter().any(|b|*b as usize!=pad) { return Err("认证数据填充无效"); }
    out.truncate(out.len()-pad); Ok(out)
}

pub async fn ac_exchange(framework: Rc<RefCell<Framework>>, config:&Config, requested:Option<&Slot>) -> Result<Vec<Slot>> {
    with_timeout(Duration::from_secs(35), ac_inner(framework,config,requested)).await.map_err(|_|"ACE 读回超时，不能确认写入结果")?
}
async fn ac_inner(framework: Rc<RefCell<Framework>>, config:&Config, requested:Option<&Slot>) -> Result<Vec<Slot>> {
    let (stack,tls)={let f=framework.borrow();(f.stack,f.tls)};
    let auth=ac_auth(stack,config).await?;
    let mut rx=vec![0u8;8192]; let mut tx=vec![0u8;4096];
    let mut socket=TcpSocket::new(stack,&mut rx,&mut tx);
    socket.set_timeout(Some(TIMEOUT));
    let ip=Ipv4Address::from_str(&config.host).map_err(|_|"打印机地址无效")?;
    socket.connect((ip,9883)).await.map_err(|_|"无法连接 Anycubic MQTT")?;
    let cert=CString::new(auth.cert).map_err(|_|"客户端证书无效")?;
    let key=CString::new(auth.key).map_err(|_|"客户端密钥无效")?;
    let ca=CString::new(config.ca_pem.clone()).map_err(|_|"CA 证书无效")?;
    let name=CString::new(if config.tls_name.is_empty(){config.host.clone()}else{config.tls_name.clone()}).map_err(|_|"TLS 名称无效")?;
    let certificates=esp_mbedtls::Certificates {
        certificate:Some(esp_mbedtls::X509::pem(cert.as_bytes_with_nul()).map_err(|_|"客户端证书无效")?),
        private_key:Some(esp_mbedtls::X509::pem(key.as_bytes_with_nul()).map_err(|_|"客户端密钥无效")?),
        ca_chain:if config.ca_pem.is_empty(){None}else{Some(esp_mbedtls::X509::pem(ca.as_bytes_with_nul()).map_err(|_|"CA 证书无效")?)},password:None };
    let mut session=esp_mbedtls::asynch::Session::new(socket,esp_mbedtls::Mode::Client{servername:name.as_c_str()},esp_mbedtls::TlsVersion::Tls1_2,certificates,tls).map_err(|_|"TLS 初始化失败")?;
    session.connect().await.map_err(|_|"Anycubic TLS 认证失败")?;
    let mut mqtt=MyMqtt::new(session,TIMEOUT);
    mqtt.connect(60,Some(&auth.username),Some(auth.password.as_bytes())).await.map_err(|_|"MQTT 登录失败")?;
    let report=p::ac_topic(&auth.model,&auth.device,true)?;
    mqtt.subscribe(None,&[SubscribeTopic{topic_path:&report,qos:QoS::AtMostOnce}]).await.map_err(|_|"ACE 状态订阅失败")?;
    let topic=p::ac_topic(&auth.model,&auth.device,false)?;
    // Read before writing: only a currently reported box and slot may be addressed.
    ac_publish(&mut mqtt,&topic,None).await?;
    let before=ac_receive(&mut mqtt,&report).await?;
    if let Some(expected)=requested {
        if !before.iter().any(|s|s.box_id==expected.box_id&&s.index==expected.index) { return Err("ACE 未报告该料盒或槽位"); }
        ac_publish(&mut mqtt,&topic,Some(expected)).await?;
        // ACE persistence lags the transport ACK. Discard earlier reports and query after 3 s.
        let until=Instant::now()+Duration::from_secs(3);
        while Instant::now()<until { let _=with_timeout(until-Instant::now(),mqtt.read()).await; }
        ac_publish(&mut mqtt,&topic,None).await?;
        loop {
            let actual=ac_receive(&mut mqtt,&report).await?;
            if p::confirmed(Brand::Anycubic,expected,&actual) { return Ok(actual); }
        }
    }
    Ok(before)
}
async fn ac_publish<T: Read+Write>(mqtt:&mut MyMqtt<'_,T>,topic:&str,slot:Option<&Slot>) -> Result<()> {
    let msgid=format!("ams-hub-{:08x}-{:08x}",framework::utils::random_u32(),framework::utils::random_u32());
    let body=p::ac_request(slot,epoch_ms()?,&msgid)?.to_string();
    mqtt.write(Packet::Publish(Publish{dup:false,qos:QoS::AtMostOnce,pid:None,retain:false,topic_name:topic,payload:body.as_bytes()})).await.map_err(|_|"ACE 请求发送失败")
}
async fn ac_receive<T: Read+Write>(mqtt:&mut MyMqtt<'_,T>,topic:&str) -> Result<Vec<Slot>> {
    loop {
        if let Some(Packet::Publish(publish))=mqtt.read().await.map_err(|_|"ACE 状态读取失败")? {
            if publish.topic_name==topic {
                if let Ok(body)=serde_json::from_slice::<Value>(publish.payload) {
                    if let Ok(slots)=p::ac_slots(&body) { return Ok(slots); }
                }
            }
        }
    }
}
