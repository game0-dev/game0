use std::sync::{Arc, Mutex};

use crate::gpu_runtime::GpuRuntime;
use crate::text_system::SharedTextSystem;

#[derive(Debug, Clone, Default)]
pub struct FrameStats {
    pub frame_count: u64,
    pub layout_ms: f32,
    pub build_ms: f32,
    pub upload_ms: f32,
    pub draw_ms: f32,
    pub glyph_upload_count: usize,
}

pub struct SharedRenderServices {
    pub runtime: Arc<GpuRuntime>,
    pub text_system: Arc<Mutex<SharedTextSystem>>,
    pub frame_stats: Arc<Mutex<FrameStats>>,
}

impl SharedRenderServices {
    pub fn from_runtime(runtime: Arc<GpuRuntime>) -> Self {
        Self {
            runtime,
            text_system: Arc::new(Mutex::new(SharedTextSystem::new(2048))),
            frame_stats: Arc::new(Mutex::new(FrameStats::default())),
        }
    }

    pub fn with_text_system(
        runtime: Arc<GpuRuntime>,
        text_system: Arc<Mutex<SharedTextSystem>>,
    ) -> Self {
        Self {
            runtime,
            text_system,
            frame_stats: Arc::new(Mutex::new(FrameStats::default())),
        }
    }
}
