use std::{
    fs::File,
    sync::{Arc, Mutex},
};

use wayland_client::{protocol::wl_output::WlOutput, QueueHandle};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

use crate::wayland::types::ScreenData;
#[cfg(target_os = "linux")]
use crate::wayland::{init::Delegate, types::ScreenshotWayland};

#[derive(Debug)]
pub(crate) struct BuffersStore<B> {
    pub(crate) buffer_file: File,
    pub(crate) file_len: usize,

    pub(crate) buffers_metadata: Vec<B>,
}

#[derive(Debug, Clone)]
pub(crate) struct Screenshot {
    pub(crate) screen_data: ScreenData,
    #[cfg(target_os = "linux")]
    pub(crate) wayland_data: ScreenshotWayland,

    pub(crate) offset: usize,
    pub(crate) span: usize,
}

#[cfg(target_os = "linux")]
impl Screenshot {
    fn screencopy(
        &self,
        screencopy_manager: ZwlrScreencopyManagerV1,
        output: &WlOutput,
        qh: &QueueHandle<Delegate>,
        screencopying_counter: Arc<Mutex<u8>>,
    ) {
        *screencopying_counter.lock().unwrap() += 1;
        let frame = screencopy_manager.capture_output(1, output, qh, screencopying_counter.clone());
        frame.copy(&self.wayland_data.buffer);
    }
}
