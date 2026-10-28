use alloc::{format, rc::Rc, string::{String, ToString}, vec::Vec};
use core::{cell::{Cell, RefCell}, net::Ipv4Addr};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};
use framework::prelude::*;
use slint::ComponentHandle;
use crate::{app::{AppWindow, HubUi}, hub_protocol::{Brand, Config, Filament, Slot}};

pub const CONFIG_KEY: &str = "ams_hub_v2";
enum Request { Read, Write { box_id:i64,index:u8,filament:Filament } }
type Requests = Channel<NoopRawMutex,Request,1>;

pub fn validate_config(c:&Config) -> Result<(), &'static str> {
    c.host.parse::<Ipv4Addr>().map_err(|_|"打印机地址必须为 IPv4")?;
    if c.api_key.contains(['\r','\n']) || c.api_key.len()>256 || c.ca_pem.len()>16384 || c.tls_name.len()>128 { return Err("连接配置无效"); }
    Ok(())
}
pub fn save_config(framework:&Framework, state:&RefCell<Config>, config:Config) -> Result<(), &'static str> {
    validate_config(&config)?;
    let text=serde_json::to_string(&config).map_err(|_|"配置序列化失败")?;
    framework.store(CONFIG_KEY.into(),text).map_err(|_|"保存连接配置失败")?;
    *state.borrow_mut()=config; Ok(())
}

pub fn init(framework:Rc<RefCell<Framework>>,ui:slint::Weak<AppWindow>) -> Rc<RefCell<Config>> {
    let config=framework.borrow().fetch(CONFIG_KEY.into()).ok().flatten().and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default();
    let config=Rc::new(RefCell::new(config));
    let channel=Rc::new(Requests::new());
    let busy=Rc::new(Cell::new(false));
    let state=ui.unwrap(); let state=state.global::<HubUi>();
    state.set_status("在连接设置网页配置 U1 / Anycubic 地址，然后点击读取".into());
    let tx=channel.clone(); let b=busy.clone();
    state.on_refresh(move || { if !b.replace(true) { if tx.try_send(Request::Read).is_err(){b.set(false);} } });
    let tx=channel.clone(); let b=busy.clone();
    state.on_apply(move |box_id,index,material,color,vendor,subtype| {
        if b.get() { return "正在等待机器读回".into(); }
        if !(0..4).contains(&index) || box_id<0 { return "请选择有效槽位".into(); }
        if let Err(e)=crate::hub_protocol::rgba(&color) {return e.into();}
        let filament=Filament {vendor:vendor.to_string(), material:material.to_string(),subtype:subtype.to_string(),rgba:color.to_string()};
        let slot=Slot {box_id:box_id as i64,index:index as u8,filament:filament.clone(),locked:false};
        // Validate material tokens and color before queueing, for both brands.
        if let Err(e)=crate::hub_protocol::ac_request(Some(&slot),0,"validation") {return e.into();}
        if tx.try_send(Request::Write{box_id:box_id as i64,index:index as u8,filament}).is_err() {return "设备忙".into();}
        b.set(true); String::new().into()
    });
    framework.borrow().spawner.spawn_heap(worker(framework.clone(),ui,config.clone(),channel,busy)).ok();
    config
}

async fn worker(framework:Rc<RefCell<Framework>>,ui:slint::Weak<AppWindow>,config:Rc<RefCell<Config>>,requests:Rc<Requests>,busy:Rc<Cell<bool>>) {
    loop {
        let request=requests.receive().await;
        let Some(window)=ui.upgrade() else {return;};
        let state=window.global::<HubUi>();
        state.set_busy(true); state.set_status("连接打印机，等待真实状态…".into());
        let config=config.borrow().clone();
        state.set_brand_name(if config.brand==Brand::U1{"Snapmaker U1"}else{"Anycubic ACE"}.into());
        state.set_host(config.host.clone().into());
        let requested=match request { Request::Read=>None,Request::Write{box_id,index,filament}=>Some(Slot{box_id,index,filament,locked:false}) };
        let result = if let Err(e)=validate_config(&config) { Err(e) }
        else {
            let stack=framework.borrow().stack;
            if !stack.is_link_up() { Err("Wi-Fi 尚未连接") }
            else if config.brand==Brand::U1 {
                if let Some(slot)=&requested {crate::hub_lan::u1_write(stack,&config,slot).await}
                else {crate::hub_lan::u1_read(stack,&config).await}
            } else {crate::hub_lan::ac_exchange(framework.clone(),&config,requested.as_ref()).await}
        };
        match result {
            Ok(slots)=>{
                let rows:Vec<_>=slots.iter().map(|s|crate::app::HubSlot{box_id:s.box_id as i32,index:s.index as i32,
                    label:format!("{}{} · {}",if config.brand==Brand::U1{"T"}else{"槽 "},s.index+1,s.filament.material).into(),
                    rgba:s.filament.rgba.clone().into(),locked:s.locked}).collect();
                state.set_slots(slint::ModelRc::new(slint::VecModel::from(rows)));
                state.set_status(if requested.is_some(){"机器已读回确认。请在切片软件设备页核对同步"}else{"已读取机器槽位状态；选择槽位后可写入扫描耗材"}.into());
            }
            Err(e)=>state.set_status(e.into()),
        }
        state.set_busy(false);busy.set(false);
    }
}
