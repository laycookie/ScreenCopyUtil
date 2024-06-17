use std::{
    fs::File,
    sync::{Arc, Mutex},
};

use wayland_client::{
    protocol::{
        wl_buffer::WlBuffer, wl_output::WlOutput, wl_shm::Format, wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
    QueueHandle,
};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

use crate::types::Screenshot;

pub(crate) struct Delegate;

#[derive(Debug, Clone)]
pub(crate) struct ScreenData {
    pub(crate) resolution: (i32, i32),
    pub(crate) logical_resolution: (i32, i32),
}

#[derive(Debug, Clone)]
pub(crate) struct Popup {
    pub(crate) screen_data: Screenshot,
    pub(crate) buffer: WlBuffer,
    pub(crate) surface: WlSurface,
}

#[derive(Debug, Clone)]
pub(crate) struct ScreenshotWayland {
    pub(crate) output: WlOutput,
    pub(crate) buffer: WlBuffer,
}

impl ScreenshotWayland {
    pub(crate) fn new(
        qh: &QueueHandle<Delegate>,
        screen_data: ScreenData,
        output: WlOutput,
        shm_pool: &WlShmPool,
        offset: usize,
    ) -> ScreenshotWayland {
        let (width, height) = (screen_data.resolution.0, screen_data.resolution.1);

        let buffer = shm_pool.create_buffer(
            offset as i32,
            width,
            height,
            width * 4,
            Format::Xrgb8888,
            qh,
            (),
        );

        ScreenshotWayland { buffer, output }
    }

    pub(crate) fn attach_buffer(&mut self, buffer: WlBuffer) {
        self.buffer = buffer;
    }

    pub(crate) fn screencopy(
        &self,
        screencopying_counter: Arc<Mutex<u8>>,
        qh: &QueueHandle<Delegate>,
        screencopy_manager: &ZwlrScreencopyManagerV1,
    ) {
        *screencopying_counter.lock().unwrap() += 1;

        let a =
            screencopy_manager.capture_output(0, &self.output, qh, screencopying_counter.clone());
        a.copy(&self.buffer);
    }
}

pub(crate) struct Screenshots {
    pub(crate) file: File,
    pub(crate) file_len: usize,

    pub(crate) screenshots_data: Vec<ScreenshotWayland>,
}
