//! Shared terminal palette: preferred colours first, then 24 everyday colours.
use alloc::{format, string::String, vec::Vec};

pub fn choices(preferred: impl IntoIterator<Item=(String,String)>, _code:&str) -> Vec<(String,String)> {
    let mut rows=Vec::new();
    let mut push=|name:String,color:String| {
        let color=color.to_ascii_uppercase();
        if color.len()==8 && color.bytes().all(|b|b.is_ascii_hexdigit())
            && !rows.iter().any(|(_,c)|c==&color) { rows.push((name,color)); }
    };
    for (name,color) in preferred {push(name,color);}
    // 24 everyday colours, matching the reference swatches. Saved custom
    // colours remain available, without appending the full vendor catalogue.
    for rgb in [0xffffff,0xe0e0e0,0x8e9099,0x000000,0x704c2e,0xffe6a3,0xf7c42b,
        0xf4ff00,0xbfc9a5,0x2c9d58,0x35fa00,0x1b6124,0x88d4e8,0x46b5ae,
        0x248ae3,0x043977,0x7058af,0x9320a9,0xda1261,0xf4517e,0xef2b16,
        0xf0d4e1,0xfa9000,0xeb5400] {
        let c=format!("{rgb:06X}FF");push(c.clone(),c);
    }
    rows
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn private_color_keeps_common_palette_and_deduplicates(){
        let rows=choices(alloc::vec![("Saved".into(),"ff0000ff".into())],"Lprivate");
        assert_eq!(rows[0].1,"FF0000FF");assert_eq!(rows.len(),25);
        assert_eq!(rows.iter().filter(|(_,c)|c=="FF0000FF").count(),1);
        assert!(rows.iter().any(|(_,c)|c=="000000FF"));
    }
    #[test] fn palette_stays_compact_for_native_presets(){assert_eq!(choices(Vec::new(),"GFA00").len(),24);assert_eq!(choices(Vec::new(),"GFA00")[0].1,"FFFFFFFF");}
}
