use wayland_client::{
    Dispatch,
    QueueHandle,
    globals::{BindError, GlobalList},
    protocol::wl_output::WlOutput,
};


use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1
};

use smithay_client_toolkit::globals::GlobalData;

#[derive(Debug)]
pub(crate) struct ScreenCopyManager {
    zwlr_screencopy_manager: ZwlrScreencopyManagerV1,
}

impl ScreenCopyManager {
    pub(crate) fn bind<State>(
        globals: &GlobalList,
        qh: &QueueHandle<State>,
    ) -> Result<ScreenCopyManager, BindError>
        where
            State: Dispatch<ZwlrScreencopyManagerV1, GlobalData, State> + 'static,
    {
        let zwlr_screencopy_manager = globals.bind(qh, 1..=1, GlobalData)?;
        Ok(ScreenCopyManager { zwlr_screencopy_manager })
    }

    pub(crate) fn zwlr_screencopy_manager(&self) -> &ZwlrScreencopyManagerV1 {
        &self.zwlr_screencopy_manager
    }

    pub(crate) fn capture_output<U: Send + Sync + 'static, D: Dispatch<ZwlrScreencopyFrameV1, U> + 'static>(
        &self,
        overlay_cursor: i32,
        output: &WlOutput,
        qh: &QueueHandle<D>,
        udata: U,
    ) -> ZwlrScreencopyFrameV1 {
        self.zwlr_screencopy_manager.capture_output(overlay_cursor, output, &qh, udata)
    }
}
