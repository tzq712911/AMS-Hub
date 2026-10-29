//! Private material identity and per-printer projections. Never mutate RFID truth.
use alloc::{format, string::{String, ToString}, vec::Vec};
use serde::{Deserialize, Serialize};

static REVISION:core::sync::atomic::AtomicU32=core::sync::atomic::AtomicU32::new(0);
pub fn revision()->u32{REVISION.load(core::sync::atomic::Ordering::Relaxed)}
pub fn invalidate(){REVISION.fetch_add(1,core::sync::atomic::Ordering::Relaxed);}
pub const KEY: &str = "filament_library_v1";
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Entry {
    pub code: String,
    pub brand: String,
    pub name: String,
    pub material: String,
    #[serde(skip_serializing_if="String::is_empty")]
    pub color: String,
    #[serde(skip_serializing_if="String::is_empty")]
    pub color_name: String,
    pub temp_min: i32,
    pub temp_max: i32,
    pub bambu_code: String,
    pub bambu_setting: String,
    pub u1_vendor: String,
    pub u1_subtype: String,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Library { pub entries: Vec<Entry> }

pub fn generic(material: &str) -> Option<Entry> {
    include_str!("../data/materials.csv").lines().skip(1).find_map(|line| {
        let f: Vec<_> = line.split(',').collect();
        if f.len() < 4 || f[0] != material { return None; }
        Some(Entry { code:f[1].into(), bambu_code:f[1].into(), brand:"Generic".into(),
            name:format!("Generic {}", material), material:material.into(),
            temp_min:f[2].parse().ok()?, temp_max:f[3].parse().ok()?, ..Default::default() })
    })
}
pub fn types() -> Vec<String> {
    include_str!("../data/materials.csv").lines().skip(1).filter_map(|l|l.split(',').next()).map(ToString::to_string).collect()
}
fn token(s: &str) -> bool {s.len() <= 64 && s.bytes().all(|b|b.is_ascii_alphanumeric() || b"_-+.".contains(&b))}
fn u1_token(s:&str)->bool{s.len()<=64&&s.bytes().all(|b|b.is_ascii_alphanumeric()||b" _-+./".contains(&b))}
pub fn validate(lib:&Library) -> Result<(), &'static str> {
    if lib.entries.len()>256 { return Err("私有目录最多 256 项，请分批整理"); }
    for (i,e) in lib.entries.iter().enumerate() {
        if e.code.is_empty() || !token(&e.code) || e.brand.trim().is_empty() || e.name.trim().is_empty()
            || e.brand.len()>128 || e.name.len()>192 || e.brand.chars().chain(e.name.chars()).any(char::is_control)
            || generic(&e.material).is_none() || e.temp_min<=0 || e.temp_min>e.temp_max || e.temp_max>500
            || !token(&e.bambu_code) || !token(&e.bambu_setting) || !u1_token(&e.u1_vendor) || !u1_token(&e.u1_subtype)
            || lib.entries[..i].iter().any(|p|p.code==e.code) { return Err("私有目录含无效或重复配置，请检查品牌、类型和映射 ID"); }
        if (!e.color.is_empty() && (e.color.len()!=8 || !e.color.bytes().all(|b|b.is_ascii_hexdigit()))) || e.color_name.len()>96 || e.color_name.chars().any(char::is_control){return Err("耗材颜色需为八位 RGBA，颜色名称不能含控制字符");}
        if !e.bambu_code.is_empty() {
            if !e.bambu_code.starts_with(['G','P']) {return Err("拓竹映射必须使用已有 G/P 预设 ID");}
            let known=include_str!("../data/base-filaments-index.csv").lines().find_map(|l|{
                let f:Vec<_>=l.split(',').collect();if f.len()>=5 && f[0]==e.bambu_code {Some(f[4])}else{None}
            }).or_else(||include_str!("../data/materials.csv").lines().skip(1).find_map(|l|{
                let f:Vec<_>=l.split(',').collect();if f.len()>=2 && f[1]==e.bambu_code {Some(f[0])}else{None}
            }));
            if known.is_some_and(|m|m!=e.material) {return Err("机器预设与真实基础类型不一致，不能跨 PLA/PETG 等类型映射");}
        }
    }
    Ok(())
}
impl Library {
    /// Register an exact physical variant without changing existing projections.
    /// Validate a candidate copy before committing, including flash storage size.
    pub fn ensure_rfid_profile(&mut self,brand:&str,name:&str,material:&str,color:&str)->Result<(String,bool), &'static str>{
        let (brand,name)=(brand.trim(),name.trim());
        let color=color.to_ascii_uppercase();
        if color.len()!=8||!color.bytes().all(|b|b.is_ascii_hexdigit()){return Err("耗材颜色无效");}
        if let Some(e)=self.entries.iter().find(|e|e.brand==brand&&e.name==name&&e.material==material&&e.color.eq_ignore_ascii_case(&color)){
            return Ok((e.code.clone(),false));
        }
        let mut entry=generic(material).ok_or("基础材料类型无效")?;
        let mut hash=0xcbf29ce484222325u64;
        for part in [brand,name,material,&color] {for byte in part.bytes().chain(core::iter::once(0)){hash^=byte as u64;hash=hash.wrapping_mul(0x100000001b3);}}
        let base=format!("L{hash:016X}");let mut code=base.clone();let mut suffix=0;
        while self.entries.iter().any(|e|e.code==code){suffix+=1;code=format!("{base}_{suffix}");}
        entry.code=code.clone();entry.brand=brand.into();entry.name=name.into();entry.color=color;
        entry.bambu_code.clear();entry.u1_vendor="Generic".into();
        let mut candidate=self.clone();candidate.entries.push(entry);validate(&candidate)?;
        let json=serde_json::to_string(&candidate).map_err(|_|"私有库编码失败")?;
        if json.len()>49152{return Err("私有库过大，请先整理耗材档案");}
        *self=candidate;Ok((code,true))
    }
    pub fn identity(&self,code:&str,brand:&str,series:&str,material:&str,color:&str)->Option<&Entry>{
        let same=|e:&&Entry|e.brand==brand&&e.name==series&&e.material==material;
        self.entries.iter().find(|e|e.code==code&&e.material==material)
            .or_else(||self.entries.iter().filter(same).find(|e|!color.is_empty()&&!e.color.is_empty()&&e.color.eq_ignore_ascii_case(color)))
            .or_else(||self.entries.iter().filter(same).find(|e|e.color.is_empty()))
    }
    pub fn bambu_identity(&self,code:&str,brand:&str,series:&str,material:&str)->Option<(String,String)>{self.bambu_identity_color(code,brand,series,material,"")}
    pub fn bambu_identity_color(&self,code:&str,brand:&str,series:&str,material:&str,color:&str)->Option<(String,String)>{
        let entry=self.identity(code,brand,series,material,color);
        self.bambu(entry.map(|e|e.code.as_str()).unwrap_or(code),material)
    }
    pub fn bambu(&self,code:&str,material:&str) -> Option<(String,String)> {
        if let Some(e)=self.entries.iter().find(|e|(e.code==code || (!e.bambu_code.is_empty() && e.bambu_code==code)) && e.material==material) {
            if !e.bambu_code.is_empty() { return Some((e.bambu_code.clone(),e.bambu_setting.clone())); }
            return generic(material).map(|g|(g.code,String::new()));
        }
        // Recognised system IDs retain their exact preset; foreign/private IDs
        // must use a matching generic type instead of leaking to another brand.
        let known=include_str!("../data/base-filaments-index.csv").lines().any(|l|{
            let f:Vec<_>=l.split(',').collect();f.len()>=5 && f[0]==code && f[4]==material
        });
        if known {Some((code.into(),String::new()))} else {generic(material).map(|g|(g.code,String::new()))}
    }
    pub fn u1(&self,actual:&crate::hub_protocol::Filament) -> crate::hub_protocol::Filament {
        let mut wire=actual.clone();wire.vendor="Generic".into();wire.subtype.clear();
        if let Some(e)=self.identity("",&actual.vendor,&actual.subtype,&actual.material,&actual.rgba) {
            if !e.u1_vendor.is_empty() {wire.vendor=e.u1_vendor.clone();wire.subtype=e.u1_subtype.clone();}
        }
        wire
    }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn rfid_profiles_reuse_exact_variants_and_keep_existing_mapping(){
        let mut lib=Library::default();
        let (red,changed)=lib.ensure_rfid_profile(" Local ","PLA Plus","PLA","ff0000ff").unwrap();assert!(changed);
        assert_eq!(lib.entries[0].u1_vendor,"Generic");assert!(lib.entries[0].bambu_code.is_empty());
        assert_eq!(lib.bambu(&red,"PLA").unwrap().0,"GFL99");
        lib.entries[0].bambu_code="Pcustom".into();
        assert_eq!(lib.ensure_rfid_profile("Local","PLA Plus","PLA","FF0000FF").unwrap(),(red.clone(),false));
        let (blue,changed)=lib.ensure_rfid_profile("Local","PLA Plus","PLA","0000FFFF").unwrap();assert!(changed);assert_ne!(red,blue);
        assert_eq!(lib.entries[0].bambu_code,"Pcustom");assert_eq!(lib.entries[0].color,"FF0000FF");assert_eq!(lib.entries.len(),2);
    }
    #[test] fn failed_rfid_catalog_update_leaves_library_untouched(){
        let mut lib=Library::default();assert!(lib.ensure_rfid_profile("Local","PLA","PLA","invalid").is_err());assert!(lib.entries.is_empty());
        for i in 0..256{let mut e=generic("PLA").unwrap();e.code=format!("L{i}");lib.entries.push(e);}
        assert!(lib.ensure_rfid_profile("New","PLA","PLA","123456FF").is_err());assert_eq!(lib.entries.len(),256);
    }
    #[test] fn unknown_brand_maps_by_exact_type() {
        let lib=Library{entries:alloc::vec![Entry{code:"Ltest".into(),brand:"Unknown".into(),name:"PLA Plus".into(),material:"PLA".into(),temp_min:190,temp_max:240,..Default::default()}]};
        assert!(validate(&lib).is_ok());assert_eq!(lib.bambu("Ltest","PLA").unwrap().0,"GFL99");
        assert_eq!(lib.entries[0].brand,"Unknown");assert_eq!(lib.entries[0].name,"PLA Plus");
        assert!(lib.bambu("Ltest","not-a-type").is_none());
    }
    #[test] fn explicit_printer_preset_and_u1_projection_keep_truth() {
        let lib=Library{entries:alloc::vec![Entry{code:"Ltest".into(),brand:"Local brand".into(),name:"Speed PLA".into(),material:"PLA".into(),temp_min:190,temp_max:240,bambu_code:"P1234567".into(),bambu_setting:"PFUS123".into(),u1_vendor:"Snapmaker".into(),u1_subtype:"Speed".into(),..Default::default()}]};
        assert_eq!(lib.bambu("Ltest","PLA").unwrap(),("P1234567".into(),"PFUS123".into()));
        assert_eq!(lib.bambu_identity("GFL99","Local brand","Speed PLA","PLA").unwrap().0,"P1234567");
        let actual=crate::hub_protocol::Filament{vendor:"Local brand".into(),subtype:"Speed PLA".into(),material:"PLA".into(),rgba:"123456FF".into()};
        let wire=lib.u1(&actual);assert_eq!(wire.vendor,"Snapmaker");assert_eq!(actual.vendor,"Local brand");assert_eq!(wire.rgba,actual.rgba);
    }
    #[test] fn rejects_cross_type_and_command_injection() {
        let mut lib=Library{entries:alloc::vec![generic("PLA").unwrap()]};
        lib.entries[0].u1_vendor="Generic\nG28".into();assert!(validate(&lib).is_err());
        lib.entries[0].u1_vendor.clear();lib.entries[0].material="Unknown".into();assert!(validate(&lib).is_err());
        lib.entries[0].material="PLA".into();lib.entries[0].bambu_code="GFG99".into();assert!(validate(&lib).is_err());
    }
    #[test] fn old_catalogs_keep_unset_colors_and_invalid_colors_are_rejected(){
        let old=r#"{"code":"Lold","brand":"Local","name":"PLA Plus","material":"PLA","temp_min":190,"temp_max":240}"#;
        let entry:Entry=serde_json::from_str(old).unwrap();assert!(entry.color.is_empty());assert!(!serde_json::to_string(&entry).unwrap().contains("color"));
        let mut lib=Library{entries:alloc::vec![entry]};assert!(validate(&lib).is_ok());
        lib.entries[0].color="12GG56FF".into();assert!(validate(&lib).is_err());
        lib.entries[0].color="123456FF".into();lib.entries[0].color_name="翡翠绿".into();assert!(validate(&lib).is_ok());
    }
    #[test] fn legacy_rfid_identity_uses_matching_color_without_repainting_truth(){
        let mut red=generic("PLA").unwrap();red.code="Lred".into();red.brand="Local".into();red.name="PLA Plus".into();red.color="FF0000FF".into();red.bambu_code="PRED".into();red.u1_vendor="Snapmaker".into();red.u1_subtype="Speed PLA".into();
        let mut blue=red.clone();blue.code="Lblue".into();blue.color="0000FFFF".into();blue.bambu_code="PBLUE".into();blue.u1_subtype="PLA".into();
        let lib=Library{entries:alloc::vec![red,blue]};assert!(validate(&lib).is_ok());
        assert_eq!(lib.bambu_identity_color("old","Local","PLA Plus","PLA","0000ffff").unwrap().0,"PBLUE");
        assert_eq!(lib.bambu_identity_color("old","Local","PLA Plus","PLA","00FF00FF").unwrap().0,"GFL99");
        let actual=crate::hub_protocol::Filament{vendor:"Local".into(),subtype:"PLA Plus".into(),material:"PLA".into(),rgba:"0000FFFF".into()};
        let wire=lib.u1(&actual);assert_eq!(wire.subtype,"PLA");assert_eq!(wire.rgba,actual.rgba);assert_eq!(actual.vendor,"Local");
    }

}
