//! Embedded bitmap fonts have finite coverage. Never silently drop a user name.
use alloc::{format,string::String};
pub const COVERAGE:&str=include_str!("../ui/fonts/coverage.txt");
pub fn display(text:&str)->String{
    let mut result=String::new();
    for c in text.chars(){
        if COVERAGE.contains(c){result.push(c);}else{result.push_str(&format!("[U+{:04X}]",c as u32));}
    }result
}
#[cfg(test)]mod tests{
    #[test]fn common_names_and_ascii_remain_unchanged(){assert_eq!(super::display("迁变 U1-01"),"迁变 U1-01");}
    #[test]fn missing_glyph_is_visible_without_losing_identity(){assert_eq!(super::display("A\u{10ffff}B"),"A[U+10FFFF]B");}
}
