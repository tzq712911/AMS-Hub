extern crate alloc;
#[path="../../core/src/hub_protocol.rs"]
pub mod protocol;
#[path="../../core/src/load_transition.rs"]
pub mod load_transition;
#[path="../../core/src/console_validation.rs"]
pub mod console_validation;
#[path="../../core/src/printer_state.rs"]
pub mod printer_state;
#[path="../../core/src/filament_library.rs"]
pub mod filament_library;
pub use protocol as hub_protocol;
#[cfg(test)] mod tests {
    use super::protocol::*;
    fn actual()->Filament{Filament{vendor:"Bambu Lab".into(),material:"PLA".into(),subtype:"Bambu PLA Lite".into(),rgba:"4DAFDAFF".into()}}
    fn slot(filament:Filament)->Slot{Slot{box_id:0,index:1,filament,locked:false,present:Some(true)}}
    #[test] fn native_u1_series_with_spaces_is_quoted_without_injection(){
        let mut requested=slot(actual());requested.filament.vendor="Snapmaker".into();requested.filament.subtype="Speed PLA".into();
        assert!(u1_command(&requested).unwrap().contains("FILAMENT_SUBTYPE=\"Speed PLA\""));
        requested.filament.subtype="Speed\nG28".into();assert!(u1_command(&requested).is_err());
    }
    #[test] fn local_identity_is_not_modified(){let truth=actual();let wire=machine_profile(Brand::U1,&truth).unwrap();assert_eq!(truth.vendor,"Bambu Lab");assert_eq!(truth.subtype,"Bambu PLA Lite");assert_eq!(wire.vendor,"Generic");assert_eq!(wire.material,"PLA");assert_eq!(wire.rgba,truth.rgba);assert!(wire.subtype.is_empty());}
    #[test] fn u1_command_contains_generic_and_exact_colour(){let wire=slot(machine_profile(Brand::U1,&actual()).unwrap());let cmd=u1_command(&wire).unwrap();assert!(cmd.contains("VENDOR=Generic"));assert!(cmd.contains("FILAMENT_COLOR_RGBA=4DAFDAFF"));assert!(!cmd.contains("Bambu"));}
    #[test] fn ace_packet_does_not_send_brand_or_series(){let wire=slot(machine_profile(Brand::Anycubic,&actual()).unwrap());let p=ac_request(Some(&wire),1,"test").unwrap();let s=&p["data"]["multi_color_box"][0]["slots"][0];assert_eq!(s["type"],"PLA");assert_eq!(s["color"],serde_json::json!([77,175,218]));assert!(s.get("vendor").is_none());assert!(s.get("subtype").is_none());}
    #[test] fn ace_black_is_approximated_only_on_machine(){let mut truth=actual();truth.rgba="000000FF".into();let wire=machine_profile(Brand::Anycubic,&truth).unwrap();assert_eq!(wire.rgba,"010101FF");assert_eq!(truth.rgba,"000000FF");}
    #[test] fn matching_type_with_unchanged_colour_is_not_success(){for brand in [Brand::U1,Brand::Anycubic]{let expected=slot(machine_profile(brand,&actual()).unwrap());let mut read=expected.clone();read.filament.rgba="FFFFFFFF".into();assert!(!confirmed(brand,&expected,&[read]));}}
    #[test] fn association_keeps_both_true_and_effective_profiles(){let p=AppliedProfile{actual:actual(),wire:machine_profile(Brand::U1,&actual()).unwrap()};let saved=serde_json::to_string(&p).unwrap();let restored:AppliedProfile=serde_json::from_str(&saved).unwrap();assert_eq!(restored.actual.vendor,"Bambu Lab");assert_eq!(restored.wire.vendor,"Generic");}
    #[test] fn partial_application_never_rewrites_rfid_truth(){let p=AppliedProfile{actual:actual(),wire:machine_profile(Brand::U1,&actual()).unwrap()};let mut reported=slot(p.wire.clone());reported.filament.rgba="FFFFFFFF".into();assert!(!synchronized(Brand::U1,&p,&reported));assert_eq!(p.actual.rgba,"4DAFDAFF");assert_eq!(p.actual.subtype,"Bambu PLA Lite");assert_eq!(p.actual.vendor,"Bambu Lab");}
    #[test] fn pla_lite_has_its_own_brand_and_colour_list(){let rows=include_str!("../../core/data/base-filaments-index.csv");let lite=rows.lines().find(|r|r.starts_with("GFA18,")).unwrap();assert!(lite.contains("Bambu PLA Lite"));assert!(lite.ends_with(",Bambu Lab"));assert!(include_str!("../../core/data/bambu-color-names.csv").lines().filter(|r|r.starts_with("GFA18,")).count()>1);}
}

#[cfg(test)] mod device_config_tests {
    #[test] fn old_connections_gain_optional_names_without_losing_authentication(){
        let old=r#"{"brand":"anycubic","host":"192.0.2.12","api_key":"legacy","ca_pem":"old-ca","tls_name":"printer.local","ac_empty_status":0}"#;
        let mut config:super::protocol::Config=serde_json::from_str(old).unwrap();
        assert!(config.name.is_empty());assert!(config.id.is_empty());
        config.id="device-12".into();config.name="ACE 工作室".into();
        let saved=serde_json::to_string(&config).unwrap();let restored:super::protocol::Config=serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.name,"ACE 工作室");assert_eq!(restored.id,"device-12");assert_eq!(restored.api_key,"legacy");assert_eq!(restored.ca_pem,"old-ca");assert_eq!(restored.tls_name,"printer.local");assert_eq!(restored.ac_empty_status,Some(0));
    }
}

#[path="../../core/src/ui_text.rs"]
pub mod ui_text;

#[path="../../core/src/color_choices.rs"]
pub mod color_choices;

#[path="../../core/src/time_sync.rs"]
pub mod time_sync;

#[path="../../vendor/esp-hal-app-framework/src/touch_events.rs"]
pub mod touch_events;
