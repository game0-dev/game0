use winit::dpi::LogicalSize;
use winit::window::WindowAttributes;

use crate::app::{AppHandle, Application, WindowCx};

slotmap::new_key_type! {
    pub struct WindowId;
}

#[derive(Debug, Clone)]
pub struct WindowDesc {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub transparent: bool,
}

impl WindowDesc {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 1024,
            height: 768,
            resizable: true,
            transparent: false,
        }
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    pub(crate) fn to_attributes(&self) -> WindowAttributes {
        WindowAttributes::default()
            .with_title(self.title.clone())
            .with_inner_size(LogicalSize::new(self.width, self.height))
            .with_resizable(self.resizable)
            .with_transparent(self.transparent)
    }
}

impl Default for WindowDesc {
    fn default() -> Self {
        Self::new("ui0")
    }
}

pub struct WindowHandle<A: Application> {
    pub(crate) id: WindowId,
    pub(crate) app: AppHandle<A>,
}

impl<A: Application> Clone for WindowHandle<A> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            app: self.app.clone(),
        }
    }
}

impl<A: Application> WindowHandle<A> {
    pub fn id(&self) -> WindowId {
        self.id
    }

    pub fn request_redraw(&self) {
        self.app.request_redraw(self.id);
    }

    pub fn close(&self) {
        self.app.close_window(self.id);
    }

    pub fn run_on_ui<F>(&self, f: F)
    where
        F: FnOnce(&mut A, &mut WindowCx<A>) + Send + 'static,
    {
        self.app.send_window(self.id, f);
    }
}
