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

pub(crate) struct Delegate;

#[derive(Debug, Clone)]
pub(crate) struct ScreenData {
    pub(crate) resolution: (i32, i32),
    pub(crate) logical_resolution: (i32, i32),
    pub(crate) output: WlOutput,
}

#[derive(Debug, Clone)]
pub(crate) struct ScreenshotData {
    pub(crate) screen_data: ScreenData,
    buffer: WlBuffer,
    surface: Option<WlSurface>,

    pub(crate) offset: usize,
    pub(crate) span: usize,
}

impl ScreenshotData {
    pub(crate) fn new(
        qh: &QueueHandle<Delegate>,
        screen_data: ScreenData,
        shm_pool: &WlShmPool,
        offset: usize,
        span: usize,
    ) -> ScreenshotData {
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

        ScreenshotData {
            screen_data,
            buffer,
            offset,
            span,
            surface: None,
        }
    }

    pub(crate) fn attach_surface(&mut self, surface: WlSurface) {
        self.surface = Some(surface);
    }
    pub(crate) fn attach_buffer(&mut self, buffer: WlBuffer) {
        self.buffer = buffer;
    }

    pub(crate) fn screencopy(
        &self,
        screencopying_counter: &Arc<Mutex<u8>>,
        qh: &QueueHandle<Delegate>,
        screencopy_manager: &ZwlrScreencopyManagerV1,
    ) {
        *screencopying_counter.lock().unwrap() += 1;

        let a = screencopy_manager.capture_output(
            0,
            &self.screen_data.output,
            qh,
            screencopying_counter.clone(),
        );
        a.copy(&self.buffer);
    }
}

pub(crate) struct Screenshots {
    pub(crate) file: File,
    pub(crate) file_len: usize,

    pub(crate) screenshots_data: Vec<ScreenshotData>,
}

impl Screenshots {
    pub(crate) fn render(&self, qh: &QueueHandle<Delegate>) {
        self.screenshots_data.iter().for_each(|a| match &a.surface {
            Some(surface) => {
                // surface.damage_buffer(0, 0, a.screen_data.resolution.0, a.screen_data.resolution.1);
                surface.frame(qh, ());
                surface.attach(Some(&a.buffer), 0, 0);
                surface.commit();
            }
            None => println!("EVERYTHING IS UNDER CONTROL PROB"),
        });
    }
}
