use core::cell::RefCell;
use core::future::ready;
use core::net::Ipv4Addr;

use alloc::format;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use embedded_sdmmc::asynchronous::LfnBuffer;
use framework::framework_web_app::{encrypt, encrypt_bytes, FrameworkState};
use hashbrown::HashMap;
use picoserve::response::chunked::{ChunkWriter, ChunkedResponse, ChunksWritten};
use picoserve::response::StatusCode;
use picoserve::routing::{get, get_service};
use picoserve::{
    extract::{FromRequest, State},
    io::Read,
    request::{RequestBody, RequestParts},
    routing::post,
    AppWithStateBuilder,
};

use framework::{
    encrypted_input,
    framework_web_app::{
        decrypt, CustomNotFound, Encryptable, EncryptedRejection, Encryption, NestedAppWithWebAppStateBuilder, SetConfigResponseDTO, WebAppState,
    },
    prelude::*,
};
use framework_macros::include_bytes_gz;
use serde::{Deserialize, Serialize};
use shared::gcode_analysis_task::Fetch3mf;

use crate::app_config::{AppConfig, DefaultPrinterConfig, PrinterConfig, PrintersConfig, ScaleConfig, FILAMENT_BRAND_NAMES, SPOOLS_CATALOG};
use crate::bambu::KInfo;
use crate::spool_record::{SpoolRecord, SpoolRecordExt};
use crate::store::{BackupMeta, FileMeta, Store};
use crate::view_model::ViewModel;

#[derive(Clone)]
pub struct ConsoleAppState {
    pub app_config: Rc<RefCell<AppConfig>>,
    pub view_model: Rc<RefCell<ViewModel>>,
    pub store: Rc<Store>,
}

impl picoserve::extract::FromRef<WebAppState<ConsoleAppState>> for ConsoleAppState {
    fn from_ref(state: &WebAppState<ConsoleAppState>) -> Self {
        state.more_state.clone()
    }
}

pub struct NestedAppBuilder {
    pub framework: Rc<RefCell<Framework>>,
    pub app_config: Rc<RefCell<AppConfig>>,
}

impl NestedAppWithWebAppStateBuilder<ConsoleAppState> for NestedAppBuilder {
    fn path_description(&self) -> &'static str {
        "" // this nests it at the root.
    }
}

impl AppWithStateBuilder for NestedAppBuilder {
    type State = WebAppState<ConsoleAppState>;
    type PathRouter = impl picoserve::routing::PathRouter<WebAppState<ConsoleAppState>>;

    fn build_app(self) -> picoserve::Router<Self::PathRouter, Self::State> {
        let _app_config = self.app_config.clone();
        let _framework = self.framework.clone();

        let router = picoserve::Router::from_service(CustomNotFound {
            web_server_captive: self.framework.borrow().settings.web_server_captive,
        }); // Handler in case page is not found for captive portal support
            // let router = router.route("/", get(|| Redirect::to("/config"))); // Redirect root for now

        // Redirect root to the current active application - either config, or encode or whatever
        // For that, in order to preserve the hash (for sk=...), using a html/js redirect technique
        let router = router.route(
            "/",
            get(move |state: State<ConsoleAppState>| {
                ready({
                    let redirect_url = &state.0.app_config.borrow().root_redirect;
                    let redirect_html =
                        format!(r#"<!doctype html><script>location.href=location.hash?"{redirect_url}"+location.hash:"{redirect_url}"</script>"#);
                    HtmlStringResponse::new(redirect_html)
                })
            }),
        );

//        TODO: >>>>>> Move to framework with setting for the css
        let router=router.route("/config-app.js",get_service(picoserve::response::File::with_content_type_and_headers("application/javascript",include_bytes_gz!("static/config-app.js"),&[("Content-Encoding","gzip")])));
        let router=router.route("/filament-library.js",get_service(picoserve::response::File::with_content_type_and_headers(
            "application/javascript",include_bytes_gz!("static/filament-library.js"),&[("Content-Encoding","gzip")]
        )));
        let router = router.route(
            "/styles.css",
            get_service(picoserve::response::File::with_content_type_and_headers(
                "text/css",
                include_bytes_gz!("static/styles.css"),
                &[("Content-Encoding", "gzip")],
            )),
        );

        let router = router.route(
            "/favicon-48x48.png",
            get_service(picoserve::response::File::with_content_type(
                "image/png",
                include_bytes!("../static/favicon-48x48.png"),
            )),
        );

        let router = router.route(
            "/api/printer-config",
            post(
                move |State(Encryption(key)): State<Encryption>, state: State<ConsoleAppState>, printers_config_dto: PrintersConfigDTO| {
                    let default_printer_serial = printers_config_dto.default_printer_serial.clone();
                    ready(
                        match state.0.app_config.borrow_mut().set_printers_config(
                            printers_config_dto.into(),
                            DefaultPrinterConfig {
                                serial: default_printer_serial,
                            },
                        ) {
                            Ok(_) => SetConfigResponseDTO { error_text: None }.encrypt(&key.borrow()),
                            Err(e) => SetConfigResponseDTO {
                                error_text: Some(format!("{e:?}")),
                            }
                            .encrypt(&key.borrow()),
                        },
                    )
                },
            )
            .get(move |State(Encryption(key)): State<Encryption>, state: State<ConsoleAppState>| {
                ready({
                    let borrowed_app_config = state.0.app_config.borrow(); // notice the borrow, can't async here
                    let printers = &borrowed_app_config.configured_printers;
                    let default_printer = &borrowed_app_config.configured_default_printer;
                    let mut printers_config = PrintersConfigDTO::from(printers);
                    printers_config.default_printer_serial = default_printer.serial.clone();
                    printers_config.encrypt(&key.borrow())
                })
            }),
        );

        let router = router.route("/api/hub-config", post(
            move |State(Encryption(key)): State<Encryption>, state:State<ConsoleAppState>, dto:HubConfigDTO| {
                let vm=state.0.view_model.borrow();
                let result=crate::hub::save_config(&state.0.app_config.borrow().framework.borrow(), &vm.hub_config, dto.config);
                ready(SetConfigResponseDTO {error_text:result.err().map(ToString::to_string)}.encrypt(&key.borrow()))
            }
        ).get(move |State(Encryption(key)):State<Encryption>,state:State<ConsoleAppState>| {
            let config=state.0.view_model.borrow().hub_config.borrow().clone();
            ready(HubConfigDTO{config}.encrypt(&key.borrow()))
        }));
        let router=router.route("/api/hub-devices",get(move|State(Encryption(key)):State<Encryption>,state:State<ConsoleAppState>|{
            let list=crate::hub::all_devices(&state.0.app_config.borrow().framework.borrow());ready(HubDevicesDTO{devices:list}.encrypt(&key.borrow()))
        }));
        let router=router.route("/api/hub-device-delete",post(move|State(Encryption(key)):State<Encryption>,state:State<ConsoleAppState>,dto:HubDeleteDTO|{
            let vm=state.0.view_model.borrow();let result=crate::hub::delete_config(&state.0.app_config.borrow().framework.borrow(),&vm.hub_config,&dto.id);
            ready(SetConfigResponseDTO{error_text:result.err().map(ToString::to_string)}.encrypt(&key.borrow()))
        }));
        let router=router.route("/api/hub-test",post(move|State(Encryption(key)):State<Encryption>,state:State<ConsoleAppState>,dto:HubConfigDTO|async move{
            let encryption=key.borrow().clone();let f=state.0.app_config.borrow().framework.clone();
            let result=crate::hub::test_connection(f,&dto.config).await;
            HubTestDTO{slots:result.as_ref().ok().copied(),error_text:result.err().map(ToString::to_string)}.encrypt(&encryption)
        }));
        let router=router.route("/api/ui-glyphs",get(move|State(Encryption(key)):State<Encryption>|{
            ready(UiGlyphsDTO{characters:crate::ui_text::COVERAGE}.encrypt(&key.borrow()))
        }));
        let router=router.route("/api/rfid-bindings",get(move|State(Encryption(key)):State<Encryption>,state:State<ConsoleAppState>,picoserve::extract::Query(query):picoserve::extract::Query<BindingsQuery>|{
            let mut out=BindingsDTO{bindings:Vec::new(),total:0,next_offset:None,error_text:None};
            if let Some(db)=state.0.store.spools_db.get(){
                let records=db.records.borrow();
                let mut rows:Vec<_>=records.values().filter(|r|r.data.has_valid_tag_id()).map(|r|&r.data).collect();rows.sort_by(|a,b|a.id.cmp(&b.id));
                out.total=rows.len();let end=query.offset.saturating_add(50).min(rows.len());
                if query.offset<rows.len(){out.bindings=rows[query.offset..end].iter().map(|r|RfidBindingDTO{id:r.id.clone(),tag_id:r.tag_id.clone(),brand:r.brand.clone(),name:r.material_subtype.clone(),material:r.material_type.clone(),color:r.color_code.clone(),preset_code:r.slicer_filament.clone()}).collect();}
                if end<rows.len(){out.next_offset=Some(end);}
            }else{out.error_text=Some("RFID 库存尚未就绪，请稍后刷新".into());}
            ready(out.encrypt(&key.borrow()))
        }));
        let router=router.route("/api/filament-library",post(
            move |State(Encryption(key)):State<Encryption>,state:State<ConsoleAppState>,dto:LibraryDTO| {
                let result=crate::filament_library::validate(&dto.library).and_then(|_|{
                    let json=serde_json::to_string(&dto.library).map_err(|_|"目录编码失败")?;
                    if json.len()>49152{return Err("目录过大，请分批导入");}
                    state.0.app_config.borrow().framework.borrow().store(crate::filament_library::KEY.into(),json).map_err(|_|"保存私有目录失败")
                });
                if result.is_ok(){crate::filament_library::invalidate();}
                ready(SetConfigResponseDTO{error_text:result.err().map(ToString::to_string)}.encrypt(&key.borrow()))
            }
        ).get(move |State(Encryption(key)):State<Encryption>,state:State<ConsoleAppState>|{
            let library=state.0.app_config.borrow().framework.borrow().fetch(crate::filament_library::KEY.into()).ok().flatten().and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default();
            ready(LibraryDTO{library}.encrypt(&key.borrow()))
        }));

        let router=router.route("/api/time-sync",post(move|State(Encryption(key)):State<Encryption>,dto:TimeSyncDTO|{
            ready(SetConfigResponseDTO{error_text:crate::network_time::accept(dto.unix_ms).err().map(String::from)}.encrypt(&key.borrow()))
        }));

        let router = router.route(
            "/api/scale-config",
            post(
                move |State(Encryption(key)): State<Encryption>, state: State<ConsoleAppState>, scale_config_dto: ScaleConfigDTO| {
                    ready(match state.0.app_config.borrow_mut().set_scale_config(scale_config_dto.into()) {
                        Ok(_) => SetConfigResponseDTO { error_text: None }.encrypt(&key.borrow()),
                        Err(e) => SetConfigResponseDTO {
                            error_text: Some(format!("{e:?}")),
                        }
                        .encrypt(&key.borrow()),
                    })
                },
            )
            .get(move |State(Encryption(key)): State<Encryption>, state: State<ConsoleAppState>| {
                ready({
                    let borrowed_app_config = state.0.app_config.borrow(); // notice the borrow, can't async here
                    let default_scale_config = ScaleConfig::default();
                    let scale = borrowed_app_config.configured_scale.as_ref().unwrap_or(&default_scale_config);
                    let scale_config = ScaleConfigDTO::from(scale);
                    scale_config.encrypt(&key.borrow())
                })
            }),
        );

        let router = router.route(
            "/spools-catalog",
            get_service(picoserve::response::File::with_content_type(
                "text/plain; charset=utf-8",
                SPOOLS_CATALOG.as_bytes(),
            )),
        );

        let router = router.route(
            "/filament-brands",
            get_service(picoserve::response::File::with_content_type(
                "text/plain; charset=utf-8",
                FILAMENT_BRAND_NAMES.as_bytes(),
            )),
        );

        let router = router.route(
            "/api/spools-config",
            post(
                move |State(Encryption(key)): State<Encryption>, state: State<ConsoleAppState>, SpoolsConfigDTO { spools }| {
                    let spools = if let Some(spools) = spools {
                        if !spools.trim().is_empty() {
                            Some(spools.trim().replace("\r\n", "\n").replace("\n", "\r\n"))
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    ready(match state.0.app_config.borrow_mut().set_user_cores(spools) {
                        Ok(_) => SetConfigResponseDTO { error_text: None }.encrypt(&key.borrow()),
                        Err(e) => SetConfigResponseDTO {
                            error_text: Some(format!("{e:?}")),
                        }
                        .encrypt(&key.borrow()),
                    })
                },
            )
            .get(move |State(Encryption(key)): State<Encryption>, state: State<ConsoleAppState>| {
                ready({
                    let borrowed_app_config = state.0.app_config.borrow(); // notice the borrow, can't async here
                    let spools = &borrowed_app_config.user_cores;
                    let spools_config = SpoolsConfigDTO { spools: spools.clone() };
                    spools_config.encrypt(&key.borrow())
                })
            }),
        );

        let router = router.route(
            "/api/filaments-config",
            post(
                move |State(Encryption(key)): State<Encryption>, state: State<ConsoleAppState>, FilamentsConfigDTO { custom_filaments }| {
                    let custom_filaments = if let Some(custom_filaments) = custom_filaments {
                        if !custom_filaments.trim().is_empty() {
                            Some(custom_filaments.trim().replace("\r\n", "\n").replace("\n", "\r\n"))
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    ready(match state.0.app_config.borrow_mut().set_filaments(custom_filaments) {
                        Ok(_) => SetConfigResponseDTO { error_text: None }.encrypt(&key.borrow()),
                        Err(e) => SetConfigResponseDTO {
                            error_text: Some(format!("{e:?}")),
                        }
                        .encrypt(&key.borrow()),
                    })
                },
            )
            .get(move |State(Encryption(key)): State<Encryption>, state: State<ConsoleAppState>| {
                ready({
                    let borrowed_app_config = state.0.app_config.borrow(); // notice the borrow, can't async here
                    let custom_filaments = &borrowed_app_config.custom_filaments;
                    let filaments_config = FilamentsConfigDTO {
                        custom_filaments: custom_filaments.clone(),
                    };
                    filaments_config.encrypt(&key.borrow())
                })
            }),
        );

        let router = router.route(
            "/api/store-backup",
            get(move |State(Encryption(key)), State(FrameworkState(framework))| async move {
                ChunkedResponse::new(StoreBackupChunks {
                    framework: framework.clone(),
                    key,
                })
                .into_response()
            }),
        );

        #[derive(serde::Deserialize)]
        struct ScreenshotQueryParams {
            key: String,
            file: String,
        }
        let router = router.route(
            "/insecure/screenshot",
            get(
                move |picoserve::extract::Query(ScreenshotQueryParams { file, key }),
                      State(Encryption(_key)),
                      State(state): State<ConsoleAppState>,
                      State(FrameworkState(framework))| async move {
                    if key == framework.borrow().web_config_key {
                        let screenshot = state.view_model.borrow().taks_screenshot();
                        let resp = ChunkedResponse::new(ScreenshotChunks { screenshot }).into_response();
                        resp.with_header("Content-Disposition", format!("attachment; filename=\"{file}\""))
                            .with_status_code(StatusCode::OK)
                    } else {
                        let screenshot = Err(slint::PlatformError::Other("Security Key Error".to_string()));
                        let resp = ChunkedResponse::new(ScreenshotChunks { screenshot }).into_response();
                        resp.with_header("", String::new()).with_status_code(StatusCode::UNAUTHORIZED)
                    }
                },
            ),
        );

        // #[allow(clippy::let_and_return)]
        // let router = router.route(
        //     "/style.css",
        //     get_service(picoserve::response::File::with_content_type_and_headers(
        //         "text/css",
        //         include_bytes!("../static/inventory/style.css.gz"),
        //         &[("Content-Encoding", "gzip")],
        //     )),
        // );

        router
    }
}

struct ScreenshotChunks {
    screenshot: Result<slint::SharedPixelBuffer<slint::Rgba8Pixel>, slint::PlatformError>,
}

impl picoserve::response::chunked::Chunks for ScreenshotChunks {
    fn content_type(&self) -> &'static str {
        "application/octet-stream"
    }
    async fn write_chunks<W: picoserve::io::Write>(self, mut chunk_writer: ChunkWriter<W>) -> Result<ChunksWritten, W::Error> {
        if let Ok(screenshot) = self.screenshot {
            chunk_writer.write_chunk(screenshot.as_bytes()).await?;
        }
        chunk_writer.finalize().await
    }
}

struct StoreBackupChunks {
    framework: Rc<RefCell<Framework>>,
    key: &'static RefCell<Vec<u8>>,
}

impl picoserve::response::chunked::Chunks for StoreBackupChunks {
    fn content_type(&self) -> &'static str {
        "text/plain"
    }

    async fn write_chunks<W: picoserve::io::Write>(self, mut chunk_writer: ChunkWriter<W>) -> Result<ChunksWritten, W::Error> {
        info!("Backup Store Started");
        let file_store = self.framework.borrow().file_store();
        let mut files: Vec<String> = Vec::new();
        let mut dirs: Vec<String> = Vec::new();
        dirs.push("/store".to_string());
        let mut lfn_buffer_storage = alloc::vec![0u8;32];
        let mut lfn_buffer = LfnBuffer::new(lfn_buffer_storage.as_mut_slice());
        let backup_meta = BackupMeta {
            spoolease_console_ver: self.framework.borrow().settings.app_cargo_pkg_version.to_string(),
        };
        let mut backup_meta_str = serde_json::to_string(&backup_meta).unwrap();
        backup_meta_str += "\n";
        let encrypted = encrypt(&self.key.borrow(), &backup_meta_str);
        chunk_writer.write_chunk(encrypted.as_bytes()).await?;
        chunk_writer.write_chunk("|".as_bytes()).await?;
        while !dirs.is_empty() {
            let curr_dir_path = dirs.remove(0);
            {
                info!("Traversing directory: {curr_dir_path}");
                {
                    let mut file_store = file_store.lock().await;
                    match file_store.open_dir(&curr_dir_path, framework::sdcard_store::Mode::ReadOnly).await {
                        Ok(rawdir) => {
                            let dir = rawdir.to_directory(file_store.volume_mgr());
                            if let Err(e) = dir
                                .iterate_dir_lfn(&mut lfn_buffer, |dir_entry, long_name| {
                                    let dir_entry_name = if let Some(long_name) = long_name {
                                        long_name.to_string()
                                    } else {
                                        dir_entry.name.to_string()
                                    };
                                    if !dir_entry_name.starts_with(".") {
                                        let full_path = format!("{}/{}", curr_dir_path, dir_entry.name);
                                        if dir_entry.attributes.is_directory() {
                                            dirs.push(full_path);
                                        } else {
                                            files.push(full_path);
                                        }
                                    }
                                })
                                .await
                            {
                                error!("Error iterating directory {curr_dir_path} : {e:?}");
                            }
                            let rawdir = dir.to_raw_directory();
                            if let Err(e) = file_store.close_dir(rawdir).await {
                                error!("Error closing sdcard directory : {e:?}");
                            }
                        }
                        Err(_) => todo!(),
                    }
                }
                let mut buffer = Vec::<u8>::with_capacity(1024);

                for file_path in files.drain(..) {
                    info!("Backing up file {file_path}");
                    buffer.clear();
                    let file_content = {
                        let mut file_store = file_store.lock().await;
                        if let Ok(file_content) = file_store.read_file_str(&file_path).await {
                            file_content
                        } else {
                            error!("Error reading file {file_path}");
                            format!("Error reading file {file_path}")
                        }
                    };

                    let file_meta = FileMeta {
                        path: file_path,
                        length: file_content.len(),
                    };
                    let file_meta_str = serde_json::to_string(&file_meta).unwrap();
                    buffer.extend_from_slice(file_meta_str.as_bytes());
                    buffer.extend_from_slice("\n".as_bytes());
                    buffer.extend_from_slice(file_content.as_bytes());
                    buffer.extend_from_slice("\n".as_bytes());
                    let encrypted = encrypt_bytes(&self.key.borrow(), &buffer);
                    chunk_writer.write_chunk(encrypted.as_bytes()).await?;
                    chunk_writer.write_chunk("|".as_bytes()).await?;
                }
            }
        }
        let res = chunk_writer.finalize().await;
        info!("Backup Store Completed");
        res
    }
}
#[derive(serde::Deserialize, serde::Serialize)]
struct PrinterConfigDTO {
    ip: Option<String>,
    name: Option<String>,
    serial: Option<String>,
    access_code: Option<String>,
    log_filter: Option<log::LevelFilter>,
    auto_restore_k: bool,
    track_print_consume: bool,
    fetch_3mf: Option<String>,
}
encrypted_input!(PrinterConfigDTO);
impl From<PrinterConfigDTO> for PrinterConfig {
    fn from(v: PrinterConfigDTO) -> Self {
        Self {
            ip: v.ip.and_then(|s| s.parse::<Ipv4Addr>().ok()),
            name: v.name,
            serial: v.serial,
            access_code: v.access_code,
            log_filter: v.log_filter,
            auto_restore_k: v.auto_restore_k,
            track_print_consume: v.track_print_consume,
            fetch_3mf: if v.fetch_3mf.as_deref().unwrap_or("") == "printer-ftp" {
                Fetch3mf::PrinterFtp
            } else {
                Fetch3mf::CloudHttp
            },
        }
    }
}
impl From<&PrinterConfig> for PrinterConfigDTO {
    fn from(v: &PrinterConfig) -> Self {
        Self {
            ip: v.ip.map(|ip| ip.to_string()),
            name: v.name.clone(),
            serial: v.serial.clone(),
            access_code: v.access_code.clone(),
            log_filter: v.log_filter,
            auto_restore_k: v.auto_restore_k,
            track_print_consume: v.track_print_consume,
            fetch_3mf: match v.fetch_3mf {
                Fetch3mf::PrinterFtp => Some("printer-ftp".to_string()),
                Fetch3mf::CloudHttp => Some("cloud-http".to_string()),
            },
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct PrintersConfigDTO {
    printers: Vec<PrinterConfigDTO>,
    default_printer_serial: Option<String>,
}
encrypted_input!(PrintersConfigDTO);
impl From<PrintersConfigDTO> for PrintersConfig {
    fn from(v: PrintersConfigDTO) -> Self {
        Self {
            printers: v
                .printers
                .into_iter()
                .map(PrinterConfig::from) // Convert each Printer to PrinterDTO
                .collect(),
        }
    }
}
impl From<&PrintersConfig> for PrintersConfigDTO {
    fn from(v: &PrintersConfig) -> Self {
        Self {
            printers: v
                .printers
                .iter()
                .map(PrinterConfigDTO::from) // Convert each Printer to PrinterDTO
                .collect(),
            default_printer_serial: None,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct SpoolsConfigDTO {
    spools: Option<String>,
}
encrypted_input!(SpoolsConfigDTO);

#[derive(serde::Deserialize, serde::Serialize)]
struct FilamentsConfigDTO {
    custom_filaments: Option<String>,
}
encrypted_input!(FilamentsConfigDTO);

#[derive(serde::Deserialize, serde::Serialize)]
struct ScaleConfigDTO {
    available: bool,
    name: Option<String>,
    ip: Option<String>,
    key: Option<String>,
}
encrypted_input!(ScaleConfigDTO);

#[derive(serde::Deserialize, serde::Serialize)]
struct HubConfigDTO { config:crate::hub_protocol::Config }
encrypted_input!(HubConfigDTO);
#[derive(Serialize)]
struct UiGlyphsDTO{characters:&'static str}
#[derive(Deserialize,Default)]
#[serde(default)]
struct BindingsQuery{offset:usize}
#[derive(Serialize)]
struct RfidBindingDTO{id:String,tag_id:String,brand:String,name:String,material:String,color:String,preset_code:String}
#[derive(Serialize)]
struct BindingsDTO{bindings:Vec<RfidBindingDTO>,total:usize,next_offset:Option<usize>,error_text:Option<String>}
#[derive(Serialize)]
struct HubDevicesDTO{devices:Vec<crate::hub_protocol::Config>}
#[derive(Deserialize)]
struct HubDeleteDTO{id:String}
encrypted_input!(HubDeleteDTO);
#[derive(Serialize)]
struct HubTestDTO{slots:Option<usize>,error_text:Option<String>}
#[derive(Serialize,Deserialize)]
struct LibraryDTO { library:crate::filament_library::Library }
encrypted_input!(LibraryDTO);

impl From<ScaleConfigDTO> for ScaleConfig {
    fn from(v: ScaleConfigDTO) -> Self {
        Self {
            available: v.available,
            ip: v.ip.and_then(|s| s.parse::<Ipv4Addr>().ok()),
            name: v.name.filter(|s| !s.is_empty()),
            key: v.key.filter(|s| !s.is_empty()),
        }
    }
}
impl From<&ScaleConfig> for ScaleConfigDTO {
    fn from(v: &ScaleConfig) -> Self {
        Self {
            available: v.available,
            ip: v.ip.map(|ip| ip.to_string()),
            name: v.name.clone(),
            key: v.key.clone(),
        }
    }
}

// #[derive(serde::Deserialize, serde::Serialize, Default, Debug)]
// pub struct EncodeInfoDTO {
//     pub tray_id: i32,
//     pub id: String,
//     pub tag_id: String,
//     pub color_code: String,
//     pub color_name: String,
//     pub material: String,
//     pub filament_subtype: String,
//     pub slicer_filament: String,
//     pub brand: String,
//     pub weight_advertised: i32,
//     pub weight_core: i32,
//     pub note: String,
// }
// encrypted_input!(EncodeInfoDTO);

#[derive(serde::Deserialize, serde::Serialize)]
pub struct DeleteSpoolDTO {
    pub id: String,
}
encrypted_input!(DeleteSpoolDTO);

#[derive(serde::Deserialize, serde::Serialize)]
pub struct AddSpoolDTO {
    pub tag_id: String,
    pub id: String,
    pub rgba: String,
    pub color_name: String,
    pub material: String,
    pub subtype: String,
    pub brand: String,
    pub core_weight: i32,
    pub label_weight: i32,
    pub note: String,
    pub slicer_filament: String,
    pub full_unused: String,
    pub k_info: Option<KInfo>,
}
encrypted_input!(AddSpoolDTO);

#[derive(serde::Deserialize, serde::Serialize)]
pub struct AddSpoolDTOResponse {
    pub id: String,
    pub csv: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct GetPrintersFilamentPaDTO {
    slicer_filament_code: String,
}
encrypted_input!(GetPrintersFilamentPaDTO);

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct GetPrintersFilamentPaDTOResponse {
    pub printers: HashMap<String, PrinterEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrinterEntry {
    pub name: String,
    pub extruders: u32,
    pub pressure_advance: Vec<PressureAdvanceEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PressureAdvanceEntry {
    pub extruder: i32,
    pub diameter: String,
    pub nozzle_id: String,
    pub name: String,
    pub k_value: String,
    pub cali_idx: i32,
    pub setting_id: Option<String>,
}

//

#[derive(serde::Deserialize, serde::Serialize)]
pub struct GetSpoolKInfoDTO {
    id: String,
}
encrypted_input!(GetSpoolKInfoDTO);

#[derive(serde::Deserialize, serde::Serialize)]
pub struct GetSpoolKInfoDTOResponse {
    pub k_info: Option<KInfo>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct GetSpoolsInPrintersResponse {
    pub spools: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddPressureAdvanceDTO {
    printer_serial: String,
    filament_id: String,
    pressure_advance_entry: PressureAdvanceEntry,
}
encrypted_input!(AddPressureAdvanceDTO);

#[derive(Debug, Serialize, Deserialize)]
pub struct GenericResonse {
    text: String,
}
encrypted_input!(GenericResonse);

/////////////////////////////////////////////

struct HtmlStringResponse {
    html: String,
}

impl HtmlStringResponse {
    pub fn new(html: String) -> Self {
        Self { html }
    }
}

impl picoserve::response::Content for HtmlStringResponse {
    fn content_type(&self) -> &'static str {
        "text/html; charset=utf-8"
    }

    fn content_length(&self) -> usize {
        self.html.len()
    }

    async fn write_content<W: embedded_io_async::Write>(self, writer: W) -> Result<(), W::Error> {
        self.html.as_bytes().write_content(writer).await
    }
}

#[derive(Deserialize)]
struct TimeSyncDTO{unix_ms:u64}
encrypted_input!(TimeSyncDTO);
