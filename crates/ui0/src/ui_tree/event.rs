use crate::app::EventCx;
use crate::ui_tree::NodeId;

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
        const POINTER_ENTER = 1 << 9;
        const POINTER_LEAVE = 1 << 10;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPhase {
    Target,
    Bubble,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerEvent {
    pub pointer_id: u64,
    pub position: Point,
    pub button: Option<PointerButton>,
    pub buttons: PointerButtons,
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Auxiliary,
    Back,
    Forward,
    Other(u16),
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct PointerButtons: u16 {
        const PRIMARY   = 1 << 0;
        const SECONDARY = 1 << 1;
        const AUXILIARY = 1 << 2;
        const BACK      = 1 << 3;
        const FORWARD   = 1 << 4;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct KeyModifiers: u16 {
        const SHIFT = 1 << 0;
        const CTRL  = 1 << 1;
        const ALT   = 1 << 2;
        const SUPER = 1 << 3;
    }
}

pub type ClickHandler = Box<dyn for<'a> FnMut(&mut EventCx<'a>)>;
pub type PointerHandler = Box<dyn for<'a> FnMut(&mut EventCx<'a>, &PointerEvent)>;

#[derive(Default)]
pub struct EventHandlers {
    pub click: Option<ClickHandler>,
    pub pointer_down: Option<PointerHandler>,
    pub pointer_up: Option<PointerHandler>,
    pub pointer_move: Option<PointerHandler>,
    pub pointer_enter: Option<PointerHandler>,
    pub pointer_leave: Option<PointerHandler>,
}

impl EventHandlers {
    pub(crate) fn flags(&self) -> EventFlags {
        let mut flags = EventFlags::empty();
        if self.click.is_some() {
            flags.insert(EventFlags::CLICK);
        }
        if self.pointer_down.is_some() {
            flags.insert(EventFlags::POINTER_DOWN);
        }
        if self.pointer_up.is_some() {
            flags.insert(EventFlags::POINTER_UP);
        }
        if self.pointer_move.is_some() {
            flags.insert(EventFlags::POINTER_MOVE);
        }
        if self.pointer_enter.is_some() {
            flags.insert(EventFlags::POINTER_ENTER);
        }
        if self.pointer_leave.is_some() {
            flags.insert(EventFlags::POINTER_LEAVE);
        }
        flags
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HitTestResult {
    pub target: Option<NodeId>,
    pub path: Vec<NodeId>,
}
