//! LAN filament metadata protocols. No printer motion, heating or cloud commands.
use alloc::{format, string::{String, ToString}, vec::Vec};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub type Result<T> = core::result::Result<T, &'static str>;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Brand { #[default] U1, Anycubic }

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub brand: Brand,
    pub host: String,
    pub api_key: String,
    /// Optional trusted CA and expected server name for Anycubic TLS.
    pub ca_pem: String,
    pub tls_name: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Filament {
    pub vendor: String,
    pub material: String,
    pub subtype: String,
    pub rgba: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Slot { pub box_id: i64, pub index: u8, pub filament: Filament, pub locked: bool }

pub fn rgba(value: &str) -> Result<String> {
    if value.len() != 8 || !value.bytes().all(|b| b.is_ascii_hexdigit()) { return Err("颜色必须为八位 RGBA"); }
    Ok(value.to_ascii_uppercase())
}
fn token(value: &str) -> Result<&str> {
    if value.is_empty() || value.len() > 64 || !value.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-+.".contains(&b)) {
        return Err("耗材名称含不支持的字符");
    }
    Ok(value)
}

pub fn u1_command(slot: &Slot) -> Result<String> {
    if slot.index > 3 || slot.box_id != 0 { return Err("U1 工具头必须为 1–4"); }
    if slot.locked { return Err("官方 RFID 耗材不可覆盖"); }
    let f = &slot.filament;
    // Klipper requires SUBTYPE when TYPE is present. Empty quoted subtype is valid.
    let subtype = if f.subtype.is_empty() { "\"\"" } else { token(&f.subtype)? };
    Ok(format!("SET_PRINT_FILAMENT_CONFIG CONFIG_EXTRUDER={} VENDOR={} FILAMENT_TYPE={} FILAMENT_SUBTYPE={} FILAMENT_COLOR_RGBA={}",
        slot.index, token(&f.vendor)?, token(&f.material)?, subtype, rgba(&f.rgba)?))
}

pub fn u1_slots(body: &Value) -> Result<Vec<Slot>> {
    let state = &body["result"]["status"]["print_task_config"];
    let types = state["filament_type"].as_array().ok_or("U1 未返回耗材状态，请检查 Moonraker 权限与固件")?;
    if types.len() != 4 { return Err("U1 工具头状态不完整"); }
    (0..4).map(|i| Ok(Slot { box_id: 0, index: i as u8,
        locked: state["filament_official"][i].as_bool().ok_or("U1 未返回 RFID 锁定状态")?,
        filament: Filament {
            material: types[i].as_str().ok_or("U1 类型字段无效")?.into(),
            vendor: state["filament_vendor"][i].as_str().ok_or("U1 品牌字段无效")?.into(),
            subtype: state["filament_sub_type"][i].as_str().ok_or("U1 子类型字段无效")?.into(),
            rgba: rgba(state["filament_color_rgba"][i].as_str().ok_or("U1 颜色字段无效")?)?,
        }
    })).collect()
}

pub fn ac_topic(model: &str, device: &str, report: bool) -> Result<String> {
    token(model)?; token(device)?;
    Ok(if report { format!("anycubic/anycubicCloud/v1/printer/public/{model}/{device}/multiColorBox/report") }
       else { format!("anycubic/anycubicCloud/v1/web/printer/{model}/{device}/multiColorBox") })
}
pub fn ac_request(slot: Option<&Slot>, timestamp: u64, msgid: &str) -> Result<Value> {
    let data = if let Some(slot) = slot {
        if slot.index > 3 || slot.box_id < 0 { return Err("ACE 槽位无效"); }
        let f = &slot.filament;
        let color = rgba(&f.rgba)?;
        let rgb: Vec<u8> = (0..3).map(|i| u8::from_str_radix(&color[i*2..i*2+2],16).unwrap()).collect();
        token(&f.material)?;
        // Only these fields are effective on ACE. Do not send Bambu IDs, brand, temperature or K.
        json!({"multi_color_box":[{"id":slot.box_id,"slots":[{"index":slot.index,"type":f.material,"color":rgb}]}]})
    } else { json!({"multi_color_box":[]}) };
    Ok(json!({"type":"multiColorBox","action":if slot.is_some(){"setInfo"}else{"getInfo"},"timestamp":timestamp,"msgid":msgid,"data":data}))
}

pub fn ac_slots(body: &Value) -> Result<Vec<Slot>> {
    if body["type"] != "multiColorBox" { return Err("不是 ACE 状态消息"); }
    let boxes = body["data"]["multi_color_box"].as_array().ok_or("ACE 未返回槽位状态")?;
    let mut out = Vec::new();
    for b in boxes {
        let id = b["id"].as_i64().ok_or("ACE 料盒编号无效")?;
        for s in b["slots"].as_array().ok_or("ACE 槽位状态无效")? {
            let index = s["index"].as_u64().filter(|i| *i < 4).ok_or("ACE 槽位编号无效")? as u8;
            let c = s["color"].as_array().filter(|c| c.len() == 3).ok_or("ACE 颜色无效")?;
            let mut rgb = [0u8;3];
            for i in 0..3 { rgb[i] = c[i].as_u64().filter(|n| *n <= 255).ok_or("ACE 颜色无效")? as u8; }
            out.push(Slot { box_id:id,index,locked:false,filament:Filament {
                vendor: "".into(), subtype:"".into(), material:s["type"].as_str().ok_or("ACE 类型无效")?.to_string(),
                rgba:format!("{:02X}{:02X}{:02X}FF",rgb[0],rgb[1],rgb[2]),
            }});
        }
    }
    Ok(out)
}

pub fn confirmed(brand: Brand, expected: &Slot, actual: &[Slot]) -> bool {
    actual.iter().any(|s| s.box_id == expected.box_id && s.index == expected.index
        && s.filament.material.eq_ignore_ascii_case(&expected.filament.material)
        && if brand == Brand::Anycubic { s.filament.rgba[..6].eq_ignore_ascii_case(&expected.filament.rgba[..6]) }
           else { s.filament.rgba.eq_ignore_ascii_case(&expected.filament.rgba)
             && s.filament.vendor == expected.filament.vendor && s.filament.subtype == expected.filament.subtype })
}
