extern crate alloc;
#[path="../../core/src/ui_text.rs"]
mod ui_text;
#[path="../../core/src/color_choices.rs"]
mod color_choices;
use std::{rc::Rc, cell::RefCell, time::{Instant, Duration}};
use slint::Model;
use slint::{ComponentHandle, Rgb8Pixel, platform::{Platform, software_renderer::{MinimalSoftwareWindow, RepaintBufferType}}};
slint::include_modules!();

struct OffscreenPlatform { window: Rc<MinimalSoftwareWindow>, started: Instant, time_offset: Rc<core::cell::Cell<Duration>> }
impl Platform for OffscreenPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> { Ok(self.window.clone()) }
    fn duration_since_start(&self) -> std::time::Duration { self.started.elapsed()+self.time_offset.get() }
}

fn click(window: &slint::Window, x: f32, y: f32) {
    use slint::platform::{WindowEvent, PointerEventButton};
    let position = slint::LogicalPosition::new(x, y);
    window.dispatch_event(WindowEvent::PointerMoved { position });
    window.dispatch_event(WindowEvent::PointerPressed { position, button: PointerEventButton::Left });
    window.dispatch_event(WindowEvent::PointerReleased { position, button: PointerEventButton::Left });
    slint::platform::update_timers_and_animations();
}

fn render(ui: &ConsolePreview, window: &MinimalSoftwareWindow, output: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    slint::platform::update_timers_and_animations();
    ui.window().request_redraw();
    let mut pixels = vec![Rgb8Pixel::default(); 480 * 320];
    assert!(window.draw_if_needed(|renderer| { renderer.render(&mut pixels, 480); }));
    let bytes: Vec<u8> = pixels.into_iter().flat_map(|p| [p.r, p.g, p.b]).collect();
    image::save_buffer(std::path::Path::new(output).join(name), &bytes, 480, 320, image::ColorType::Rgb8)?;
    println!("Rendered {name}: 480x320, simulated data");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args().nth(1).unwrap_or_else(|| "screenshots".into());
    std::fs::create_dir_all(&output)?;
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    let time_offset=Rc::new(core::cell::Cell::new(Duration::ZERO));
    slint::platform::set_platform(Box::new(OffscreenPlatform { window: window.clone(), started: Instant::now(), time_offset:time_offset.clone() }))?;
    let ui = ConsolePreview::new()?;
    ui.global::<AppBackend>().on_console_text(|text|ui_text::display(&text).into());
    let weak = ui.as_weak();
    ui.global::<AppBackend>().on_get_console_presets(move || weak.unwrap().get_demo_presets());
    let weak=ui.as_weak();
    ui.global::<AppBackend>().on_get_console_presets_for_brand(move|brand|slint::ModelRc::new(slint::VecModel::from(weak.unwrap().get_demo_presets().iter().filter(|p|p.brand.as_str().eq_ignore_ascii_case(&brand)).collect::<Vec<_>>())));
    let weak=ui.as_weak();
    ui.global::<AppBackend>().on_get_console_brand(move|code|weak.unwrap().get_demo_presets().iter().find(|p|p.code==code).map(|p|p.brand).unwrap_or_else(||"Generic".into()));
    ui.global::<AppBackend>().on_parse_console_color(|code|slint::Color::from_argb_encoded(0xff000000|code.get(..6).and_then(|s|u32::from_str_radix(s,16).ok()).unwrap_or(0x555555)));
    ui.global::<AppBackend>().on_get_console_colors(|code|{
        let rows=color_choices::choices(Vec::new(),&code).into_iter().map(|(name,color)|UiConsoleColor{
            name:name.into(),tint:slint::Color::from_argb_encoded(0xff000000|u32::from_str_radix(&color[..6],16).unwrap()),code:color.into()
        }).collect::<Vec<_>>();slint::ModelRc::new(slint::VecModel::from(rows))
    });
    let weak = ui.as_weak();
    ui.global::<AppBackend>().on_get_slot_display(move |_| weak.unwrap().get_demo_slot());
    ui.global::<AppBackend>().on_get_console_calibrations(|_, _| Default::default());
    ui.global::<AppBackend>().on_get_console_mapping(|_,material,_,_,_|format!("机器：Generic {} · 演示",material).into());
    ui.global::<AppBackend>().on_get_console_materials(||slint::ModelRc::new(slint::VecModel::from(include_str!("../../core/data/materials.csv").lines().skip(1).filter_map(|l|l.split(',').next()).map(Into::into).collect::<Vec<slint::SharedString>>())));
    ui.global::<AppBackend>().on_get_console_generic(|material|{
        include_str!("../../core/data/materials.csv").lines().skip(1).find_map(|l|{
            let f:Vec<_>=l.split(',').collect();if f.len()<4||f[0]!=material.as_str(){return None;}
            Some(UiConsolePreset{brand:"Generic".into(),name:format!("Generic {}",material).into(),code:f[1].into(),material:material.clone(),temp_min:f[2].parse().unwrap(),temp_max:f[3].parse().unwrap()})
        }).unwrap_or_default()
    });
    ui.global::<AppBackend>().on_get_console_brands(|| slint::ModelRc::new(slint::VecModel::from(vec!["Bambu Lab".into(),"Generic".into(),"SUNLU".into(),"eSUN".into()])));
    let submissions = Rc::new(RefCell::new(Vec::new()));
    let captured = submissions.clone();
    ui.global::<AppBackend>().on_configure_console_slot(move |tray, preset, material, color, calibration| {
        captured.borrow_mut().push((tray, preset.to_string(), material.to_string(), color.to_string(), calibration));
        "".into()
    });
    let captured=submissions.clone();
    ui.global::<AppBackend>().on_configure_console_profile(move|tray,preset,material,color,calibration,_,_|{
        captured.borrow_mut().push((tray,preset.to_string(),material.to_string(),color.to_string(),calibration));"".into()
    });
    ui.window().set_size(slint::PhysicalSize::new(480, 320));
    ui.show()?;
    for (mode, name) in [(0, "home.png"), (1, "editor.png")] {
        ui.set_preview_mode(mode);
        render(&ui, &window, &output, name)?;
    }
    click(ui.window(), 300.,106.);
    render(&ui,&window,&output,"brands.png")?;
    click(ui.window(),250.,130.);
    render(&ui,&window,&output,"brand-selected.png")?;
    click(ui.window(),300.,150.);
    render(&ui, &window, &output, "materials.png")?;
    click(ui.window(),250.,130.);
    click(ui.window(),150.,194.);
    render(&ui, &window, &output, "palette.png")?;
    let palette_image=image::open(std::path::Path::new(&output).join("palette.png"))?.to_rgb8();
    let mut orange_pixels=Vec::new();
    for y in 99..141 {for x in 137..429 {if palette_image.get_pixel(x,y).0==[255,255,255]{orange_pixels.push((x,y));}}}
    assert!(orange_pixels.len()>1000,"Grid swatch must be fully visible");
    click(ui.window(),158.,120.);
    click(ui.window(), 420., 289.);
    assert_eq!(submissions.borrow().as_slice(), &[(0, "GFA00".into(), "PLA".into(), "FFFFFFFF".into(), -1)]);
    click(ui.window(), 420., 289.);
    assert_eq!(submissions.borrow().len(), 1, "Repeated confirmation must not submit twice");
    println!("PASS: material picker, palette selection, outgoing metadata, duplicate-submit guard");
    ui.set_preview_mode(2);
    render(&ui, &window, &output, "registration.png")?;
    ui.set_preview_mode(3);
    ui.global::<FrameworkState>().invoke_set_web_config_url("http://192.0.2.10/config".into(), "Demo Wi-Fi".into());
    ui.global::<FrameworkState>().invoke_web_config_started("DEMO-KEY".into(), WebConfigState::StartedSTADisplayed);
    render(&ui, &window, &output, "settings.png")?;
    let starts=Rc::new(core::cell::Cell::new(0));
    let count=starts.clone(); let weak=ui.as_weak();
    ui.global::<FrameworkBackend>().on_start_web_config(move||{
        count.set(count.get()+1);
        weak.unwrap().global::<FrameworkState>().invoke_web_config_started("DEMO-KEY".into(),WebConfigState::StartedSTADisplayed);
    });
    click(ui.window(),110.,290.);
    assert_eq!(starts.get(),1,"Settings must allow restarting web configuration");
    render(&ui,&window,&output,"settings-key.png")?;
    ui.set_preview_mode(4);
    ui.global::<FrameworkState>().set_term_text((0..35).map(|i|format!("Line {i}: runtime log / 配置打印机\n")).collect::<String>().into());
    render(&ui,&window,&output,"terminal.png")?;
    click(ui.window(),460.,20.);
    assert!(ui.get_terminal_scroll_offset()>0.,"Up button must scroll long logs");
    click(ui.window(),460.,260.);
    assert_eq!(ui.get_terminal_scroll_offset(),0.,"Down button must return to bottom");
    for _ in 0..3 {click(ui.window(),460.,260.);}
    assert_eq!(ui.get_terminal_scroll_offset(),0.,"Down button must not overscroll");
    click(ui.window(),380.,20.);
    assert_eq!(ui.get_preview_mode(),3,"Terminal must provide direct configuration access");
    ui.global::<FrameworkState>().invoke_web_config_started("DEMO-KEY".into(),WebConfigState::StartedAP);
    click(ui.window(),110.,290.);
    assert_eq!(starts.get(),1,"AP provisioning must not be replaced by a STA server");
    println!("PASS: web configuration entry, restart callback, AP preservation, terminal scroll bounds");
    let hub_submissions=Rc::new(RefCell::new(Vec::new()));
    let captured=hub_submissions.clone();
    ui.global::<HubUi>().on_apply(move |box_id,index,material,color,vendor,subtype| {
        captured.borrow_mut().push((box_id,index,material.to_string(),color.to_string(),vendor.to_string(),subtype.to_string()));
        "".into()
    });
    for (profile,name) in [(1,"u1.png"),(2,"anycubic.png")] {
        ui.invoke_home();
        ui.invoke_load_hub_demo(profile,false);
        ui.set_hub_mode(true);
        render(&ui,&window,&output,name)?;
        click(ui.window(),185.,104.);
        render(&ui,&window,&output,if profile==1{"u1-editor.png"}else{"ace-editor.png"})?;
        click(ui.window(),400.,290.);
        assert_eq!(hub_submissions.borrow().last(),Some(&(0,1,"PLA".into(),"00CE9BFF".into(),"Bambu Lab".into(),"Bambu PLA Basic".into())));
        let n=hub_submissions.borrow().len();
        ui.global::<HubUi>().set_busy(true);
        render(&ui,&window,&output,"busy.png")?;
        click(ui.window(),400.,290.);
        assert_eq!(hub_submissions.borrow().len(),n,"Busy button must reject pointer clicks");
        ui.global::<HubUi>().set_busy(false);
    }
    ui.global::<HubUi>().set_editing(false);ui.invoke_load_hub_demo(1,false);
    let long_slots=ui.global::<HubUi>().get_slots().iter().enumerate().map(|(i,mut row)|{row.brand=if i==0{"Extremely Long Filament Brand Name".into()}else if i==2{"A Very Long Chinese Brand 名称".into()}else{"A".into()};row}).collect::<Vec<_>>();
    ui.global::<HubUi>().set_slots(slint::ModelRc::new(slint::VecModel::from(long_slots)));
    render(&ui,&window,&output,"u1-long-brand-equal-slots.png")?;
    let equal=image::open(std::path::Path::new(&output).join("u1-long-brand-equal-slots.png"))?.to_rgb8();
    for x in [122,239,356]{assert_eq!(equal.get_pixel(x,110).0,[20,20,20],"Long brand names must not move slot gutters");}
    for x in [115,232,349,465]{assert_eq!(equal.get_pixel(x,110).0,[36,36,36],"Each fixed-width slot must retain its own background");}
    let device_targets=Rc::new(RefCell::new(Vec::new()));let targets=device_targets.clone();
    ui.global::<HubUi>().on_select_device(move|id|targets.borrow_mut().push(id.to_string()));
    click(ui.window(),325.,25.);assert_eq!(device_targets.borrow().as_slice(),&["device-b"],"Device tabs must submit stable IDs, not mutable list indices");
    ui.global::<HubUi>().set_devices(slint::ModelRc::new(slint::VecModel::from(vec!["迁变U1".into(),"A\u{10ffff}B".into()])));
    render(&ui,&window,&output,"chinese-device-name.png")?;
    assert_eq!(ui.global::<AppBackend>().invoke_console_text("迁变U1".into()),"迁变U1");
    assert_eq!(ui.global::<AppBackend>().invoke_console_text("A\u{10ffff}B".into()),"A[U+10FFFF]B");
    println!("PASS: Chinese device name and explicit missing-glyph fallback");
    println!("PASS: long brand labels do not change the four equal slot widths");
    println!("PASS: U1/ACE slot selection, outgoing callback metadata, busy click guard");
    ui.set_hub_mode(false);ui.global::<HubUi>().set_visible(false);ui.invoke_multi_ams_demo();
    render(&ui,&window,&output,"multi-ams.png")?;
    ui.global::<AppState>().set_curr_ams_id(4);
    render(&ui,&window,&output,"ams-ht.png")?;
    assert_eq!(ui.global::<AppState>().get_trays_state().row_data(18).unwrap().id,16);
    let selections=Rc::new(RefCell::new(Vec::new()));let chosen=selections.clone();
    ui.global::<HubUi>().on_select_brand(move|brand|chosen.borrow_mut().push(brand));
    click(ui.window(),95.,25.);
    assert_eq!(ui.global::<HubUi>().get_active_brand(),1);
    assert!(ui.global::<HubUi>().get_visible());
    render(&ui,&window,&output,"brand-tab-u1.png")?;
    click(ui.window(),150.,25.);
    assert_eq!(ui.global::<HubUi>().get_active_brand(),2);
    assert_eq!(selections.borrow().as_slice(),&[1,2]);
    println!("PASS: on-device U1/ACE Logo tabs and AMS-HT tray mapping");
    ui.global::<HubUi>().set_visible(false);ui.global::<HubUi>().set_editing(false);ui.global::<HubUi>().set_active_brand(0);
    ui.invoke_scan_record("04A1B2C3".into(),"Bambu Lab".into(),"PLA".into(),"4DAFDAFF".into(),slint::Color::from_argb_encoded(0xff4dafda),"GFA18".into(),190,240,"Bambu PLA Lite".into());
    ui.set_preview_mode(1);render(&ui,&window,&output,"pla-lite.png")?;
    click(ui.window(),300.,194.);click(ui.window(),250.,65.);
    render(&ui,&window,&output,"custom-rgb.png")?;
    for (x,y) in [(280.,110.),(355.,110.),(430.,110.),(205.,149.),(280.,149.),(355.,149.)]{click(ui.window(),x,y);}
    click(ui.window(),390.,285.);click(ui.window(),420.,289.);
    assert_eq!(submissions.borrow().last().unwrap().3,"123456FF");
    println!("PASS: custom RGB entered with on-screen hex keypad");
    ui.global::<AppState>().set_control_state(ControlState::Ready);
    ui.set_preview_mode(5);
    ui.set_list_test_values(slint::ModelRc::new(slint::VecModel::from((0..50).map(|i|format!("Choice {i}").into()).collect::<Vec<slint::SharedString>>())));
    render(&ui,&window,&output,"scroll-list.png")?;
    use slint::platform::{WindowEvent,PointerEventButton};
    ui.window().dispatch_event(WindowEvent::PointerPressed{position:slint::LogicalPosition::new(200.,260.),button:PointerEventButton::Left});
    for y in [240.,210.,170.,120.,70.] {
        std::thread::sleep(std::time::Duration::from_millis(20));
        ui.window().dispatch_event(WindowEvent::PointerMoved{position:slint::LogicalPosition::new(200.,y)});
        slint::platform::update_timers_and_animations();
    }
    ui.window().dispatch_event(WindowEvent::PointerReleased{position:slint::LogicalPosition::new(200.,70.),button:PointerEventButton::Left});
    assert!(ui.get_list_test_offset()<0.,"Finger drag must scroll the list");
    assert_eq!(ui.get_list_test_choice(),-1,"A drag must not select the touched row");
    for _ in 0..15 {click(ui.window(),460.,260.);}
    render(&ui,&window,&output,"scroll-list-end.png")?;
    click(ui.window(),200.,295.);
    assert_eq!(ui.get_list_test_choice(),49,"Last choice must be reachable");
    println!("PASS: finger scrolling cancels row click; arrows reach the final choice");
    let registrations=Rc::new(RefCell::new(Vec::new()));let captured=registrations.clone();
    ui.global::<AppBackend>().on_register_console_tag_brand(move|_,preset,material,color,brand,series|{
        captured.borrow_mut().push((preset.to_string(),material.to_string(),color.to_string(),brand.to_string(),series.to_string()));"".into()
    });
    ui.global::<AppState>().set_new_tag_scan_tag_id("04AABBCC".into());
    ui.set_preview_mode(2);render(&ui,&window,&output,"unknown-rfid.png")?;
    click(ui.window(),300.,99.);click(ui.window(),250.,65.);
    render(&ui,&window,&output,"custom-brand-keyboard.png")?;
    for _ in 0..3 {click(ui.window(),184.,243.);}
    click(ui.window(),390.,285.);
    render(&ui,&window,&output,"custom-brand-selected.png")?;
    assert!(registrations.borrow().is_empty(),"Selecting a brand must not bind an RFID");
    click(ui.window(),300.,153.);
    for _ in 0..5 {click(ui.window(),450.,260.);}
    render(&ui,&window,&output,"material-type-scrolled.png")?;
    click(ui.window(),250.,255.);
    click(ui.window(),300.,207.);click(ui.window(),250.,65.);
    for (x,y) in [(280.,110.),(355.,110.),(430.,110.),(205.,149.),(280.,149.),(355.,149.)]{click(ui.window(),x,y);}
    click(ui.window(),390.,285.);click(ui.window(),420.,289.);
    assert_eq!(registrations.borrow().as_slice(),&[("GFL99".into(),"PLA".into(),"123456FF".into(),"XXX".into(),"PLA".into())]);
    println!("PASS: custom brand + offscreen base type + RGB bind without changing actual identity");
    // Full home/editor flow: same UID enters reassignment, not a second inventory item.
    ui.set_hub_mode(false);ui.global::<HubUi>().set_editing(false);ui.global::<HubUi>().set_visible(false);
    ui.invoke_scan_record("04A1B2C3".into(),"Bambu Lab".into(),"PLA".into(),"4DAFDAFF".into(),slint::Color::from_argb_encoded(0xff4dafda),"GFA18".into(),190,240,"Bambu PLA Lite".into());
    let weak=ui.as_weak();
    ui.global::<AppBackend>().on_reassign_scanned_tag(move||{
        let w=weak.unwrap();let app=w.global::<AppState>();let original=app.get_spool_staging_info();
        app.invoke_new_tag_scanned(original.spool_record.tag_id.clone());app.set_reassign_info(original);app.set_reassigning_tag(true);app.invoke_empty_spool_staging();"".into()
    });
    click(ui.window(),420.,284.);
    assert_eq!(ui.global::<AppState>().get_control_state(),ControlState::NewTagScan);
    assert_eq!(ui.global::<AppState>().get_new_tag_scan_tag_id(),"04A1B2C3");
    render(&ui,&window,&output,"reassign-rfid.png")?;
    let saved_uid=Rc::new(RefCell::new(String::new()));let saved=saved_uid.clone();
    ui.global::<AppBackend>().on_register_console_tag_brand(move|tag,_,_,_,_,_|{*saved.borrow_mut()=tag.to_string();"".into()});
    click(ui.window(),420.,289.);assert_eq!(&*saved_uid.borrow(),"04A1B2C3");
    ui.invoke_home();render(&ui,&window,&output,"reassign-return.png")?;ui.set_preview_mode(1);ui.global::<AppState>().set_some_printer_connected(true);
    render(&ui,&window,&output,"reset-editor.png")?;
    let clears=Rc::new(RefCell::new(Vec::new()));let captured=clears.clone();
    ui.global::<AppBackend>().on_clear_console_slot(move|tray|{captured.borrow_mut().push(tray);"".into()});
    click(ui.window(),340.,289.);assert_eq!(clears.borrow().len(),1,"Reset must call the machine clear backend");
    ui.invoke_home();ui.global::<AppState>().set_reassigning_tag(false);
    ui.global::<AppState>().set_available_printers(slint::ModelRc::new(slint::VecModel::from(vec!["X2D-A".into(),"X2D-B".into()])));
    ui.global::<AppState>().set_curr_printer("X2D-A".into());ui.global::<AppState>().set_selected_printer_connected(true);
    ui.global::<AppState>().set_ams_exists(slint::ModelRc::new(slint::VecModel::from(vec![0,4])));ui.global::<AppState>().set_curr_ams_id(0);
    render(&ui,&window,&output,"ams-with-side-ht.png")?;
    click(ui.window(),420.,165.);assert_eq!(ui.global::<AppState>().get_tray_selected_tray_index(),18,"Fifth card must select HT, not external");
    ui.invoke_home();
    ui.invoke_show_message("写入完成".into());slint::platform::update_timers_and_animations();
    for _ in 0..19{time_offset.set(time_offset.get()+Duration::from_millis(1001));slint::platform::update_timers_and_animations();}
    assert_eq!(ui.global::<AppState>().get_control_state(),ControlState::PostAction,"Result must remain visible for 20 seconds");
    time_offset.set(time_offset.get()+Duration::from_millis(1001));slint::platform::update_timers_and_animations();
    assert_eq!(ui.global::<AppState>().get_control_state(),ControlState::Ready);
    assert_eq!(ui.global::<AppState>().get_spool_staging_state(),SpoolStagingState::Empty);
    println!("PASS: same-UID reassignment, reset backend, side HT target and 20-second auto return");
    let printer_targets=Rc::new(RefCell::new(Vec::new()));let targets=printer_targets.clone();
    ui.global::<AppBackend>().on_select_printer(move|name|targets.borrow_mut().push(name.to_string()));
    click(ui.window(),340.,25.);assert_eq!(printer_targets.borrow().as_slice(),&["X2D-B"]);
    ui.invoke_multi_ams_demo();render(&ui,&window,&output,"multi-ams-before-swipe.png")?;
    ui.window().dispatch_event(WindowEvent::PointerPressed{position:slint::LogicalPosition::new(340.,70.),button:PointerEventButton::Left});
    for x in [300.,250.,200.,150.,100.]{
        std::thread::sleep(Duration::from_millis(20));ui.window().dispatch_event(WindowEvent::PointerMoved{position:slint::LogicalPosition::new(x,70.)});
    }
    ui.window().dispatch_event(WindowEvent::PointerReleased{position:slint::LogicalPosition::new(100.,70.),button:PointerEventButton::Left});
    render(&ui,&window,&output,"multi-ams-after-swipe.png")?;
    click(ui.window(),330.,70.);assert_eq!(ui.global::<AppState>().get_curr_ams_id(),5,"Horizontal touch scroll must reach the final HT tab");
    println!("PASS: printer identity tabs target the correct machine; horizontal scrolling reaches last AMS/HT");
    Ok(())
}
