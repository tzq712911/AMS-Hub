//! AC needs accurate UTC even when the framework's first NTP server is blocked.
use alloc::rc::Rc;
use core::cell::RefCell;
use embassy_net::{udp::{PacketMetadata,UdpSocket}};
use embassy_time::{Duration,Instant,Timer,with_timeout};
use framework::prelude::*;
use smoltcp::wire::DnsQueryType;
use crate::time_sync::{Anchor,valid_epoch_ms,ntp_reply};
static FALLBACK:critical_section::Mutex<RefCell<Option<Anchor>>>=critical_section::Mutex::new(RefCell::new(None));
pub fn now_ms()->Option<u64>{
    if let Some(seconds)=crate::store::store_safe_time_now().filter(|s|*s>0){let ms=seconds as u64*1000;if valid_epoch_ms(ms){return Some(ms);}}
    critical_section::with(|cs|FALLBACK.borrow_ref(cs).as_ref().and_then(|a|a.now(Instant::now().as_millis())))
}
pub fn accept(ms:u64)->Result<(), &'static str>{
    if !valid_epoch_ms(ms){return Err("校时时间无效，请检查电脑的日期和时间");}
    critical_section::with(|cs|*FALLBACK.borrow_ref_mut(cs)=Some(Anchor{epoch_ms:ms,uptime_ms:Instant::now().as_millis()}));Ok(())
}
pub async fn task(framework:Rc<RefCell<Framework>>){
    Framework::wait_for_wifi(&framework).await;let stack=framework.borrow().stack;
    loop{
        for server in ["ntp.aliyun.com","cn.pool.ntp.org","time.windows.com","time.cloudflare.com"]{
            if now_ms().is_some(){return;}
            let Ok(Ok(addresses))=with_timeout(Duration::from_secs(2),stack.dns_query(server,DnsQueryType::A)).await else{continue;};
            for ip in addresses.iter().take(2){
                let mut rx_meta=[PacketMetadata::EMPTY;1];let mut tx_meta=[PacketMetadata::EMPTY;1];let mut rx=[0;128];let mut tx=[0;64];
                let mut socket=UdpSocket::new(stack,&mut rx_meta,&mut rx,&mut tx_meta,&mut tx);if socket.bind(0).is_err(){continue;}
                let mut request=[0u8;48];request[0]=0x23;let mut nonce=[0u8;8];nonce[..4].copy_from_slice(&framework::utils::random_u32().to_be_bytes());nonce[4..].copy_from_slice(&framework::utils::random_u32().to_be_bytes());request[40..48].copy_from_slice(&nonce);
                let exchange=async{
                    socket.send_to(&request,(*ip,123)).await.ok()?;let mut packet=[0u8;128];
                    let (len,meta)=socket.recv_from(&mut packet).await.ok()?;
                    if meta.endpoint.addr!=*ip||meta.endpoint.port!=123{return None;}
                    ntp_reply(&packet[..len],&nonce)
                };
                if let Ok(Some(ms))=with_timeout(Duration::from_secs(2),exchange).await{if accept(ms).is_ok(){term_info!("备用网络校时成功，AC 可继续连接");return;}}
            }
        }
        Timer::after_secs(5).await;
    }
}
