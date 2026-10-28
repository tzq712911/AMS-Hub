//! Metadata-only editor input validation, also runnable as a standalone host test.
pub fn validate_metadata(tray: i32, color: &str, preset: &str, material: &str, low: u32, high: u32) -> Result<(), &'static str> {
    if !((0..=23).contains(&tray) || tray == 254 || tray == 255) {
        return Err("无效槽位");
    }
    if color.len() != 8 || !color.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("颜色格式应为 8 位 RGBA");
    }
    if preset.is_empty() || material.is_empty() || low == 0 || low > high {
        return Err("耗材信息不完整");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_metadata;
    #[test]
    fn accepts_ams_ht_and_both_external_holders() {
        for tray in [0, 15, 16, 23, 254, 255] {
            assert!(validate_metadata(tray, "00ce9bFF", "GFA00", "PLA", 190, 240).is_ok());
        }
    }
    #[test]
    fn rejects_indices_that_could_index_outside_tray_storage() {
        for tray in [-1, 24, 128, 253, 256, i32::MAX] {
            assert!(validate_metadata(tray, "00CE9BFF", "GFA00", "PLA", 190, 240).is_err());
        }
    }
    #[test]
    fn requires_rgba_not_rgb_or_non_hex() {
        for color in ["", "FFFFFF", "#FFFFFF", "GG0000FF", "白FFFFFF", "FFFFFFFFF"] {
            assert!(validate_metadata(0, color, "GFA00", "PLA", 190, 240).is_err());
        }
    }
    #[test]
    fn rejects_incomplete_or_inverted_temperature_metadata() {
        assert!(validate_metadata(0, "FFFFFFFF", "", "PLA", 190, 240).is_err());
        assert!(validate_metadata(0, "FFFFFFFF", "GFA00", "", 190, 240).is_err());
        assert!(validate_metadata(0, "FFFFFFFF", "GFA00", "PLA", 0, 240).is_err());
        assert!(validate_metadata(0, "FFFFFFFF", "GFA00", "PLA", 241, 240).is_err());
    }
}
