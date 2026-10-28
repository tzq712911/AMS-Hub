//! One-shot insertion detector: no target inferred from material or colour.
extern crate alloc;
use alloc::vec::Vec;
pub type Sensor = (i64,u8,Option<bool>);
#[derive(Default)]
pub struct Detector { previous:Vec<Sensor>, candidate:Option<(i64,u8)> }
impl Detector {
    pub fn baseline(&mut self, values:Vec<Sensor>) {self.previous=values;self.candidate=None;}
    pub fn observe(&mut self, values:Vec<Sensor>) -> Result<Option<(i64,u8)>,()> {
        if values.len()!=self.previous.len() || values.iter().any(|(b,i,_)|!self.previous.iter().any(|(pb,pi,_)|pb==b&&pi==i)) {return Err(());}
        let edges:Vec<_>=values.iter().filter(|(b,i,p)|*p==Some(true)&&self.previous.iter().any(|(pb,pi,pp)|pb==b&&pi==i&&*pp==Some(false))).map(|(b,i,_)|(*b,*i)).collect();
        if edges.len()>1 || (self.candidate.is_some()&&!edges.is_empty()) {return Err(());}
        let confirmed=self.candidate.filter(|(b,i)|values.iter().any(|(pb,pi,p)|pb==b&&pi==i&&*p==Some(true)));
        self.candidate=edges.first().copied();self.previous=values;
        Ok(confirmed)
    }
}
#[cfg(test)] mod tests {
    use super::*;use alloc::vec;
    #[test] fn insertion_requires_baseline_and_stability(){let mut d=Detector::default();d.baseline(vec![(0,0,Some(false))]);assert_eq!(d.observe(vec![(0,0,Some(true))]),Ok(None));assert_eq!(d.observe(vec![(0,0,Some(true))]),Ok(Some((0,0))));}
    #[test] fn metadata_or_unknown_is_not_insertion(){let mut d=Detector::default();d.baseline(vec![(0,0,None)]);assert_eq!(d.observe(vec![(0,0,Some(true))]),Ok(None));assert_eq!(d.observe(vec![(0,0,Some(true))]),Ok(None));}
    #[test] fn multiple_insertions_are_ambiguous(){let mut d=Detector::default();d.baseline(vec![(0,0,Some(false)),(0,1,Some(false))]);assert!(d.observe(vec![(0,0,Some(true)),(0,1,Some(true))]).is_err());}
    #[test] fn reconnect_or_new_box_is_not_insertion(){let mut d=Detector::default();d.baseline(vec![]);assert!(d.observe(vec![(0,0,Some(true))]).is_err());}
}
