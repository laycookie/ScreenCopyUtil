use std::{
    fs::File,
    sync::{Arc, Mutex},
};

use wayland_client::{
    protocol::{
        wl_buffer::WlBuffer, wl_output::WlOutput, wl_shm::Format, wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
    EventQueue, QueueHandle,
};
use wayland_protocols_wlr::{
    layer_shell::v1::client::zwlr_layer_surface_v1::{
        Anchor, KeyboardInteractivity, ZwlrLayerSurfaceV1,
    },
    screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

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
    pub(crate) layer_surface: ZwlrLayerSurfaceV1,
}

impl Popup {
    pub(crate) fn config_layer_surface(&self, event_queue: &mut EventQueue<Delegate>) {
        let (l_width, l_height) = self.screen_data.screen_data.logical_resolution;

        self.layer_surface.set_size(l_width as u32, l_height as u32);
        self.layer_surface.set_anchor(Anchor::Bottom);
        self.layer_surface.set_margin(0, 0, 0, 0);
        self.layer_surface
            .set_keyboard_interactivity(KeyboardInteractivity::None);
        self.layer_surface.set_exclusive_zone(-1);
        self.surface.commit();

        println!("prep for config");
        event_queue.dispatch_pending(&mut Delegate).unwrap();
    }
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
