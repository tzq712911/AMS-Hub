use alloc::{format,rc::Rc,string::{String,ToString},vec::Vec};
use core::{cell::{Cell,RefCell},net::Ipv4Addr};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex,channel::Channel};
use embassy_time::{with_timeout,Duration,Instant};
use framework::prelude::*;
use slint::ComponentHandle;
use crate::{app::{AppWindow,HubUi},hub_protocol::{Brand,Config,Filament,Slot,AppliedProfile,machine_profile},load_transition::Detector};
pub const CONFIG_KEY:&str="ams_hub_v2";
enum Request {Read,Arm(Filament),Write(Slot)}
type Requests=Channel<NoopRawMutex,Request,2>;
pub fn validate_config(c:&Config)->Result<(),&'static str>{
    c.host.parse::<Ipv4Addr>().map_err(|_|"打印机地址必须为 IPv4")?;
    if c.api_key.contains(['\r','\n'])||c.api_key.len()>256||c.ca_pem.len()>16384||c.tls_name.len()>128||c.ac_empty_status==Some(5){return Err("连接配置无效");} Ok(())
}
fn profile_key(brand:Brand)->&'static str {if brand==Brand::U1{"ams_hub_u1"}else{"ams_hub_ace"}}
pub fn save_config(f:&Framework,state:&RefCell<Config>,config:Config)->Result<(),&'static str>{
    validate_config(&config)?;let text=serde_json::to_string(&config).map_err(|_|"配置序列化失败")?;
    f.store(profile_key(config.brand).into(),text.clone()).map_err(|_|"保存连接配置失败")?;
    f.store(CONFIG_KEY.into(),text).map_err(|_|"保存连接配置失败")?;
    if state.borrow().brand==config.brand{*state.borrow_mut()=config;}Ok(())
}
pub fn init(framework:Rc<RefCell<Framework>>,ui:slint::Weak<AppWindow>)->Rc<RefCell<Config>>{
    let config:Config=framework.borrow().fetch(CONFIG_KEY.into()).ok().flatten().and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default();
    let config=Rc::new(RefCell::new(config));let channel=Rc::new(Requests::new());let busy=Rc::new(Cell::new(false));
    let window=ui.unwrap();let state=window.global::<HubUi>();
    let selected=framework.borrow().fetch("hub_brand".into()).ok().flatten().and_then(|s|s.parse::<i32>().ok()).filter(|v|(0..=2).contains(v)).unwrap_or(if config.borrow().host.is_empty(){0}else if config.borrow().brand==Brand::U1{1}else{2});
    if selected!=0 {
        let brand=if selected==1{Brand::U1}else{Brand::Anycubic};
        if config.borrow().brand!=brand {
            let restored=framework.borrow().fetch(profile_key(brand).into()).ok().flatten().and_then(|s|serde_json::from_str::<Config>(&s).ok()).unwrap_or(Config{brand,..Default::default()});
            *config.borrow_mut()=restored;
        }
    }
    state.set_active_brand(selected);state.set_visible(selected!=0);
    state.set_brand_name(if selected==1{"Snapmaker U1"}else{"Anycubic ACE"}.into());state.set_host(config.borrow().host.clone().into());
    state.set_status("选择品牌并配置对应机器地址".into());
    let tx=channel.clone();let b=busy.clone();state.on_refresh(move||{if !b.get(){let _=tx.try_send(Request::Read);}});
    let tx=channel.clone();let cfg=config.clone();let fw=framework.clone();let weak=ui.clone();
    state.on_select_brand(move|index|{
        let w=weak.unwrap();let s=w.global::<HubUi>();s.set_slots(Default::default());s.set_active_brand(index);s.set_visible(index!=0);
        let _=fw.borrow().store("hub_brand".into(),index.to_string());
        if index==0{return;}
        let brand=if index==1{Brand::U1}else{Brand::Anycubic};
        let current=cfg.borrow().clone();
        if current.brand!=brand {
            if !current.host.is_empty(){let _=fw.borrow().store(profile_key(current.brand).into(),serde_json::to_string(&current).unwrap());}
            let next=fw.borrow().fetch(profile_key(brand).into()).ok().flatten().and_then(|t|serde_json::from_str::<Config>(&t).ok()).unwrap_or(Config{brand,..Default::default()});
            *cfg.borrow_mut()=next;
        }
        s.set_brand_name(if index==1{"Snapmaker U1"}else{"Anycubic ACE"}.into());s.set_host(cfg.borrow().host.clone().into());
        s.set_status("读取对应品牌的机器状态；未配置时请到连接设置网页填写".into());let _=tx.try_send(Request::Read);
    });
    let tx=channel.clone();let weak=ui.clone();
    state.on_scan(move|material,color,vendor,series|{
        if weak.unwrap().global::<HubUi>().get_active_brand()==0{return;}
        weak.unwrap().global::<HubUi>().set_status("正在读取空槽基线，请稍候再装料…".into());
        let vendor=if vendor.is_empty(){"Generic".into()}else{vendor.to_string()};
        let _=tx.try_send(Request::Arm(Filament{vendor,material:material.to_string(),rgba:color.to_string(),subtype:series.to_string()}));
    });
    let tx=channel.clone();let b=busy.clone();
    state.on_apply(move|box_id,index,material,color,vendor,subtype|{
        if b.get(){return "正在等待机器读回".into();}
        if !(0..4).contains(&index)||box_id<0{return "请选择有效槽位".into();}
        let slot=Slot{box_id:box_id as i64,index:index as u8,locked:false,present:None,filament:Filament{vendor:vendor.to_string(),material:material.to_string(),rgba:color.to_string(),subtype:subtype.to_string()}};
        if let Err(e)=crate::hub_protocol::ac_request(Some(&slot),0,"validation"){return e.into();}
        if tx.try_send(Request::Write(slot)).is_err(){return "设备忙".into();}String::new().into()
    });
    if selected!=0{let _=channel.try_send(Request::Read);}
    framework.borrow().spawner.spawn_heap(worker(framework.clone(),ui,config.clone(),channel,busy)).ok();config
}
async fn read(f:Rc<RefCell<Framework>>,c:&Config)->Result<Vec<Slot>,&'static str>{
    validate_config(c)?;let stack=f.borrow().stack;if !stack.is_link_up(){return Err("Wi-Fi 尚未连接");}
    if c.brand==Brand::U1{crate::hub_lan::u1_read(stack,c).await}else{crate::hub_lan::ac_exchange(f,c,None).await}
}
async fn write(f:Rc<RefCell<Framework>>,c:&Config,slot:&Slot)->Result<Vec<Slot>,&'static str>{
    let mut wire=slot.clone();wire.filament=machine_profile(c.brand,&slot.filament)?;
    let before=read(f.clone(),c).await?;
    let target=before.iter().find(|v|v.box_id==slot.box_id&&v.index==slot.index).ok_or("机器未报告目标槽位")?;
    if target.locked{return Err("该槽位不可覆盖");}
    // An explicit assignment records physical identity even if the printer later drops a field.
    let profile=AppliedProfile{actual:slot.filament.clone(),wire:wire.filament.clone()};
    f.borrow().store(format!("hs_{}_{}_{}",c.host,slot.box_id,slot.index),serde_json::to_string(&profile).unwrap()).map_err(|_|"本地真实耗材资料保存失败，未发送机器命令")?;
    if c.brand==Brand::U1 {let stack=f.borrow().stack;crate::hub_lan::u1_write(stack,c,&wire).await}
    else{crate::hub_lan::ac_exchange(f,c,Some(&wire)).await}
}
async fn worker(f:Rc<RefCell<Framework>>,ui:slint::Weak<AppWindow>,config:Rc<RefCell<Config>>,requests:Rc<Requests>,busy:Rc<Cell<bool>>){
    let mut armed:Option<(Filament,Instant,String,Brand)>=None;let mut detector=Detector::default();
    loop{
        let request=with_timeout(Duration::from_secs(if armed.is_some(){1}else{10}),requests.receive()).await.ok();
        let Some(w)=ui.upgrade()else{return;};let s=w.global::<HubUi>();let c=config.borrow().clone();
        if s.get_active_brand()==0{armed=None;continue;}
        if armed.as_ref().is_some_and(|(_,until,host,brand)|Instant::now()>*until||host!=&c.host||*brand!=c.brand||w.global::<crate::app::AppState>().get_spool_staging_state()==crate::app::SpoolStagingState::Empty){armed=None;s.set_status("等待已取消，请重新扫描耗材".into());}
        busy.set(true);s.set_busy(true);
        let mut requested=None;let mut baseline=None;
        match request{Some(Request::Arm(filament))=>{armed=None;baseline=Some(filament);},Some(Request::Write(slot))=>{armed=None;requested=Some(slot);},_=>{}}
        let result=if let Some(slot)=&requested{write(f.clone(),&c,slot).await}else{read(f.clone(),&c).await};
        match result {
            Ok(mut slots)=>{
                let app=w.global::<crate::app::AppState>();
                let current=app.get_spool_staging_info().spool_record;
                let matching=|filament:&Filament| app.get_spool_staging_state()!=crate::app::SpoolStagingState::Empty && app.get_control_state()!=crate::app::ControlState::NewTagScan && current.material_type.as_str()==filament.material && current.color_code.as_str()==filament.rgba && current.material_subtype.as_str()==filament.subtype && (if current.brand.is_empty(){"Generic"}else{current.brand.as_str()})==filament.vendor;
                if armed.as_ref().is_some_and(|(filament,_,_,_)|!matching(filament)){armed=None;}
                if baseline.as_ref().is_some_and(|filament|!matching(filament)){baseline=None;}
                if let Some(filament)=baseline {
                    detector.baseline(slots.iter().map(|v|(v.box_id,v.index,v.present)).collect());
                    if slots.iter().any(|v|!v.locked&&v.present==Some(false)){
                        armed=Some((filament,Instant::now()+Duration::from_secs(120),c.host.clone(),c.brand));s.set_status("已识别 · 请装入一个空槽，等待自动写入".into());
                    }else{s.set_status("未获得可靠空槽传感器状态，请手选槽位；勿以颜色推断装料".into());}
                }else if requested.is_none()&&armed.is_some(){
                    match detector.observe(slots.iter().map(|v|(v.box_id,v.index,v.present)).collect()){
                        Ok(Some((b,i)))=>{
                            let (filament,_,_,_)=armed.take().unwrap();
                            if let Some(mut slot)=slots.iter().find(|v|v.box_id==b&&v.index==i&&!v.locked).cloned(){
                                slot.filament=filament;match write(f.clone(),&c,&slot).await{Ok(actual)=>{slots=actual;requested=Some(slot);},Err(e)=>s.set_status(e.into())}
                            }
                        },Err(_)=>{armed=None;s.set_status("多个槽位变化或设备拓扑改变，请重新扫描或手选".into());},_=>{}
                    }
                }
                if let Some(slot)=&requested {
                    let association=AppliedProfile{actual:slot.filament.clone(),wire:machine_profile(c.brand,&slot.filament).unwrap()};
                    let _=f.borrow().store(format!("hs_{}_{}_{}",c.host,slot.box_id,slot.index),serde_json::to_string(&association).unwrap());
                    s.set_status("类型和颜色已读回确认 · 机器使用兼容配置，本地保留实际信息".into());
                    w.global::<crate::app::AppState>().invoke_empty_spool_staging();
                }
                let rows:Vec<_>=slots.iter().map(|v|{
                    let key=format!("hs_{}_{}_{}",c.host,v.box_id,v.index);
                    if v.present==Some(false){let _=f.borrow().remove(key.clone());}
                    let local=f.borrow().fetch(key).ok().flatten().and_then(|text|serde_json::from_str::<AppliedProfile>(&text).ok());
                    let synced=local.as_ref().map(|p|crate::hub_protocol::synchronized(c.brand,p,v)).unwrap_or(true);
                    let actual=local.as_ref().map(|p|&p.actual).unwrap_or(&v.filament);
                    let rgb=actual.rgba.get(..6).and_then(|c|u32::from_str_radix(c,16).ok()).unwrap_or(0x555555);
                    let series=if actual.subtype.is_empty(){actual.material.as_str()}else{actual.subtype.as_str()};
                    let display=series.strip_prefix("Bambu ").or_else(||series.strip_prefix(&format!("{} ",actual.vendor))).unwrap_or(series);
                    let catalog:Vec<Vec<&str>>=crate::app_config::BASE_FILAMENTS.lines().map(|r|r.split(',').collect()).collect();
                    let profile=catalog.iter().find(|r|r.len()>=6&&r[1]==actual.subtype).or_else(||catalog.iter().find(|r|r.len()>=6&&r[5]=="Generic"&&r[4]==actual.material));
                    crate::app::HubSlot{box_id:v.box_id as i32,index:v.index as i32,label:format!("槽 {}",v.index+1).into(),rgba:actual.rgba.clone().into(),locked:v.locked,brand:actual.vendor.clone().into(),material:display.into(),tint:slint::Color::from_argb_encoded(0xff000000|rgb),machine_detail:format!("{} · 机器 {} {} #{}",if synced{"已同步"}else{"未完全同步"},v.filament.vendor,v.filament.material,v.filament.rgba).into(),series_name:series.into(),base_material:actual.material.clone().into(),preset_code:profile.map(|r|r[0]).unwrap_or("").into(),temp_min:profile.and_then(|r|r[2].parse().ok()).unwrap_or(0),temp_max:profile.and_then(|r|r[3].parse().ok()).unwrap_or(0)}
                }).collect();s.set_slots(slint::ModelRc::new(slint::VecModel::from(rows)));
            },Err(e)=>{armed=None;s.set_status(e.into());}
        }
        s.set_busy(false);busy.set(false);
    }
}
