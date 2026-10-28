use std::{rc::Rc, cell::RefCell, time::Instant};
use slint::Model;
use slint::{ComponentHandle, Rgb8Pixel, platform::{Platform, software_renderer::{MinimalSoftwareWindow, RepaintBufferType}}};
slint::include_modules!();

struct OffscreenPlatform { window: Rc<MinimalSoftwareWindow>, started: Instant }
impl Platform for OffscreenPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> { Ok(self.window.clone()) }
    fn duration_since_start(&self) -> std::time::Duration { self.started.elapsed() }
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
    slint::platform::set_platform(Box::new(OffscreenPlatform { window: window.clone(), started: Instant::now() }))?;
    let ui = ConsolePreview::new()?;
    let weak = ui.as_weak();
    ui.global::<AppBackend>().on_get_console_presets(move || weak.unwrap().get_demo_presets());
    let weak=ui.as_weak();
    ui.global::<AppBackend>().on_get_console_presets_for_brand(move|brand|slint::ModelRc::new(slint::VecModel::from(weak.unwrap().get_demo_presets().iter().filter(|p|p.brand.as_str().eq_ignore_ascii_case(&brand)).collect::<Vec<_>>())));
    let weak=ui.as_weak();
    ui.global::<AppBackend>().on_get_console_brand(move|code|weak.unwrap().get_demo_presets().iter().find(|p|p.code==code).map(|p|p.brand).unwrap_or_else(||"Generic".into()));
    ui.global::<AppBackend>().on_parse_console_color(|code|slint::Color::from_argb_encoded(0xff000000|code.get(..6).and_then(|s|u32::from_str_radix(s,16).ok()).unwrap_or(0x555555)));
    ui.global::<AppBackend>().on_get_console_colors(|code|{
        let rows=include_str!("../../core/data/bambu-color-names.csv").lines().filter_map(|line|{
            let f:Vec<_>=line.split(',').collect();if f.len()<3||f[0]!=code.as_str(){return None;}
            Some(UiConsoleColor{name:f[2].into(),code:f[1].into(),tint:slint::Color::from_argb_encoded(0xff000000|u32::from_str_radix(&f[1][..6],16).unwrap())})
        }).collect::<Vec<_>>();slint::ModelRc::new(slint::VecModel::from(rows))
    });
    let weak = ui.as_weak();
    ui.global::<AppBackend>().on_get_slot_display(move |_| weak.unwrap().get_demo_slot());
    ui.global::<AppBackend>().on_get_console_calibrations(|_, _| Default::default());
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
    click(ui.window(), 300., 107.);
    render(&ui,&window,&output,"brands.png")?;
    click(ui.window(),200.,75.);
    render(&ui, &window, &output, "materials.png")?;
    click(ui.window(),200.,75.);
    render(&ui, &window, &output, "palette.png")?;
    click(ui.window(),200.,115.);
    click(ui.window(), 420., 289.);
    assert_eq!(submissions.borrow().as_slice(), &[(0, "GFA00".into(), "PLA".into(), "FF6A13FF".into(), -1)]);
    click(ui.window(), 420., 289.);
    assert_eq!(submissions.borrow().len(), 1, "Repeated confirmation must not submit twice");
    println!("PASS: material picker, palette selection, outgoing metadata, duplicate-submit guard");
    ui.set_preview_mode(2);
    render(&ui, &window, &output, "registration.png")?;
    ui.set_preview_mode(3);
    render(&ui, &window, &output, "settings.png")?;
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
    click(ui.window(),300.,179.);click(ui.window(),200.,73.);
    render(&ui,&window,&output,"custom-rgb.png")?;
    for (x,y) in [(182.,102.),(298.,102.),(414.,102.),(65.,143.),(182.,143.),(298.,143.)]{click(ui.window(),x,y);}
    click(ui.window(),370.,275.);click(ui.window(),420.,289.);
    assert_eq!(submissions.borrow().last().unwrap().3,"123456FF");
    println!("PASS: custom RGB entered with on-screen hex keypad");
    Ok(())
}
