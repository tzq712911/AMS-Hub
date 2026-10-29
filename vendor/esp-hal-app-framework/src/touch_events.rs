//! Hardware-independent contact state and bounded render-time input backlog.
use alloc::collections::VecDeque;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct TouchPosition{pub x:i32,pub y:i32}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum TouchEvent{TouchPressed(TouchPosition),TouchReleased(TouchPosition),TouchMoved(TouchPosition)}
impl TouchEvent{pub fn touch_position(&self)->TouchPosition{match *self{Self::TouchPressed(p)|Self::TouchReleased(p)|Self::TouchMoved(p)=>p}}}
#[derive(Default)]
pub struct ContactState{position:Option<TouchPosition>}
impl ContactState{
    pub fn is_down(&self)->bool{self.position.is_some()}
    pub fn contact(&mut self,p:TouchPosition)->Option<TouchEvent>{
        let previous=self.position.replace(p);
        match previous{None=>Some(TouchEvent::TouchPressed(p)),Some(old)if old!=p=>Some(TouchEvent::TouchMoved(p)),_=>None}
    }
    pub fn release(&mut self,p:Option<TouchPosition>)->Option<TouchEvent>{self.position.take().map(|old|TouchEvent::TouchReleased(p.unwrap_or(old)))}
}
#[derive(Default)]
pub struct TouchBacklog{events:VecDeque<TouchEvent>,overflowed:bool}
impl TouchBacklog{
    pub fn push(&mut self,event:TouchEvent){
        if self.overflowed{return;}
        // Never replay an incomplete sequence after overflow: cancel the gesture.
        if self.events.len()==64{self.events.clear();self.overflowed=true;return;}
        self.events.push_back(event);
    }
    pub fn pop(&mut self)->Option<TouchEvent>{self.events.pop_front()}
    pub fn is_empty(&self)->bool{self.events.is_empty()}
    pub fn len(&self)->usize{self.events.len()}
    pub fn take_overflowed(&mut self)->bool{core::mem::take(&mut self.overflowed)}
}
#[cfg(test)]mod tests{
    use super::*;
    fn p(x:i32)->TouchPosition{TouchPosition{x,y:20}}
    #[test]fn rapid_taps_at_same_position_keep_both_clicks(){let mut state=ContactState::default();let mut q=TouchBacklog::default();for _ in 0..2{q.push(state.contact(p(10)).unwrap());assert!(state.contact(p(10)).is_none());q.push(state.release(None).unwrap());assert!(state.release(None).is_none());}assert_eq!(q.len(),4);for _ in 0..2{assert_eq!(q.pop(),Some(TouchEvent::TouchPressed(p(10))));assert_eq!(q.pop(),Some(TouchEvent::TouchReleased(p(10))));}}
    #[test]fn preserves_drag_excursion_return_and_next_press(){let mut q=TouchBacklog::default();q.push(TouchEvent::TouchPressed(p(1)));q.push(TouchEvent::TouchMoved(p(100)));q.push(TouchEvent::TouchMoved(p(1)));q.push(TouchEvent::TouchReleased(p(1)));q.push(TouchEvent::TouchPressed(p(3)));assert_eq!(q.len(),5);assert_eq!(q.pop(),Some(TouchEvent::TouchPressed(p(1))));assert_eq!(q.pop(),Some(TouchEvent::TouchMoved(p(100))));assert_eq!(q.pop(),Some(TouchEvent::TouchMoved(p(1))));assert_eq!(q.pop(),Some(TouchEvent::TouchReleased(p(1))));assert_eq!(q.pop(),Some(TouchEvent::TouchPressed(p(3))));}
    #[test]fn overflow_discards_incomplete_gestures_instead_of_replaying_clicks(){let mut q=TouchBacklog::default();for _ in 0..65{q.push(TouchEvent::TouchPressed(p(1)));}assert!(q.is_empty());assert!(q.take_overflowed());assert!(!q.take_overflowed());q.push(TouchEvent::TouchReleased(p(1)));assert_eq!(q.len(),1);}
}
