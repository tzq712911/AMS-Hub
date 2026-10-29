//! Read legacy snapshots and the version-1 Bambu driver envelope.
use alloc::{format, string::{String, ToString}};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Also bounds the serial before using the legacy FAT 8.3 directory layout.
pub fn validate_serial(serial:&str)->Result<(), &'static str>{
    if !(11..=64).contains(&serial.len())||!serial.bytes().all(|b|b.is_ascii_alphanumeric()){
        return Err("打印机序列号需为 11–64 位字母或数字，请填写完整 SN，不能使用设备名称或占位值");
    }
    Ok(())
}
pub fn validate_connection(serial:Option<&str>,access:Option<&str>)->Result<(), &'static str>{
    validate_serial(serial.unwrap_or_default())?;
    let access=access.unwrap_or_default();
    if access.is_empty()||access.len()>64||access.chars().any(char::is_control){return Err("请填写有效的局域网访问码");}
    Ok(())
}
pub fn file_path(serial:&str,file:&str)->Result<String,String>{
    validate_serial(serial).map_err(String::from)?;
    if file.is_empty()||!file.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'.')||file=="."||file==".."{return Err("打印机状态文件名无效".into());}
    let len=serial.len();
    Ok(format!("/state/{}.{}/{}",&serial[len-11..len-3],&serial[len-3..],file))
}

pub fn decode<T: DeserializeOwned>(text: &str, serial: &str) -> Result<(T, bool), String> {
    let mut value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let wrapped = value.get("driver_private").is_some();
    if wrapped {
        if value["version"].as_u64() != Some(1) || value["driver_kind"].as_str() != Some("Bambu") {
            return Err("Unsupported printer state version or driver".into());
        }
        if value["printer_id"].as_str() != Some(format!("bambu_printer_{serial}").as_str()) {
            return Err("Printer state belongs to another device".into());
        }
        value = value["driver_private"].take();
    }
    serde_json::from_value(value).map(|state| (state, wrapped)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    #[derive(Debug, Deserialize, PartialEq)]
    struct State { ams_trays: alloc::vec::Vec<Value> }
    #[test] fn malformed_serials_never_panic_or_make_state_paths(){
        for serial in ["","0","1234567890","12345678迁","0941234567/89","0941234567\0AB"]{
            assert!(validate_serial(serial).is_err());assert!(file_path(serial,"startup.jsn").is_err());
        }
        assert!(validate_serial(&"A".repeat(65)).is_err());
        assert_eq!(file_path("094123456789ABC","startup.jsn").unwrap(),"/state/23456789.ABC/startup.jsn");
    }
    #[test] fn valid_printers_keep_legacy_paths_and_reject_unsafe_filenames(){
        let serial="094123456789ABC";let len=serial.len();
        assert_eq!(file_path(serial,"print.ci0").unwrap(),format!("/state/{}.{}/print.ci0",&serial[len-11..len-3],&serial[len-3..]));
        for file in ["../startup.jsn","/startup.jsn","..",""]{assert!(file_path(serial,file).is_err());}
        assert!(validate_connection(Some(serial),Some("12345678")).is_ok());
        assert!(validate_connection(Some("0"),Some("12345678")).is_err());
        assert!(validate_connection(Some(serial),None).is_err());
    }
    #[test] fn reads_legacy_state() {
        assert_eq!(decode::<State>(r#"{"ams_trays":[]}"#, "test").unwrap(), (State{ams_trays:alloc::vec![]}, false));
    }
    #[test] fn reads_native_state_from_versioned_envelope() {
        let text=r#"{"version":1,"printer_id":"bambu_printer_test","driver_kind":"Bambu","generic":{},"driver_private":{"ams_trays":[]}}"#;
        assert_eq!(decode::<State>(text,"test").unwrap(),(State{ams_trays:alloc::vec![]},true));
        assert!(decode::<State>(text,"other").is_err());
        assert!(decode::<State>(&text.replace("\"Bambu\"","\"U1\""),"test").is_err());
        assert!(decode::<State>(&text.replace("\"version\":1","\"version\":2"),"test").is_err());
    }
    #[test] fn rejects_corrupt_or_missing_driver_state() {
        for text in ["", "{", r#"{"generic":{"ams_trays":[]}}"#, r#"{"ams_trays":null}"#] {
            assert!(decode::<State>(text,"test").is_err());
        }
    }
}
