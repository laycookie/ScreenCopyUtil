use std::fs::File;

#[cfg(target_os = "linux")]
use crate::wayland::types::ScreenData;

pub(crate) struct BuffersStore<B> {
    pub(crate) buffer_file: File,
    pub(crate) file_len: usize,

    pub(crate) buffers_metadata: Vec<B>,
}
#[derive(Clone)]
pub(crate) struct Screenshot {
    #[cfg(target_os = "linux")]
    pub(crate) screen_data: ScreenData,

    pub(crate) offset: usize,
    pub(crate) span: usize,
}
