//! Pure validation for bounded NTP fallback and browser supplied time.
pub fn valid_epoch_ms(ms:u64)->bool{(1577836800000..4102444800000).contains(&ms)}
#[derive(Clone,Copy)]
pub struct Anchor{pub epoch_ms:u64,pub uptime_ms:u64}
impl Anchor{
    pub fn now(self,uptime_ms:u64)->Option<u64>{self.epoch_ms.checked_add(uptime_ms.checked_sub(self.uptime_ms)?)}
}
pub fn ntp_reply(packet:&[u8],nonce:&[u8;8])->Option<u64>{
    if packet.len()<48||packet[0]>>6==3||packet[0]&7!=4||!(1..=15).contains(&packet[1])||packet[24..32]!=*nonce{return None;}
    let seconds=u32::from_be_bytes(packet[40..44].try_into().ok()?) as u64;
    // NTP's 32 bit seconds wrap in 2036; both current eras are supported.
    let seconds=if seconds<2208988800{seconds+(1u64<<32)}else{seconds};
    let fraction=u32::from_be_bytes(packet[44..48].try_into().ok()?) as u64;
    let ms=seconds.checked_sub(2208988800)?*1000+((fraction*1000)>>32);
    valid_epoch_ms(ms).then_some(ms)
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn elapsed_time_does_not_use_another_boots_uptime(){let a=Anchor{epoch_ms:1800000000000,uptime_ms:5000};assert_eq!(a.now(6000),Some(1800000001000));assert_eq!(a.now(1),None);}
    #[test]fn rejects_unset_clocks_and_validates_ntp_origin(){
        assert!(!valid_epoch_ms(1000000));let nonce=[5;8];let mut p=[0u8;48];p[0]=0x24;p[1]=2;p[24..32].copy_from_slice(&nonce);p[40..44].copy_from_slice(&(1800000000u32+2208988800).to_be_bytes());
        assert_eq!(ntp_reply(&p,&nonce),Some(1800000000000));assert_eq!(ntp_reply(&p,&[6;8]),None);p[0]=0xe4;assert_eq!(ntp_reply(&p,&nonce),None);
    }
    #[test]fn supports_ntp_era_after_2036(){let mut p=[0u8;48];p[0]=0x24;p[1]=1;p[40..44].copy_from_slice(&((2300000000u64+2208988800) as u32).to_be_bytes());assert_eq!(ntp_reply(&p,&[0;8]),Some(2300000000000));}
}
