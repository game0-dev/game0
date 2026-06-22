use crate::app::EventCx;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct EventFlags: u32 {
        const CLICK        = 1 << 0;
        const POINTER_DOWN = 1 << 1;
        const POINTER_UP   = 1 << 2;
        const POINTER_MOVE = 1 << 3;
        const WHEEL        = 1 << 4;
        const KEY_DOWN     = 1 << 5;
        const KEY_UP       = 1 << 6;
        const FOCUS        = 1 << 7;
        const BLUR         = 1 << 8;
    }
}

pub type ClickHandler = Box<dyn for<'a> FnMut(&mut EventCx<'a>)>;

#[derive(Default)]
pub struct EventHandlers {
    pub click: Option<ClickHandler>,
}
