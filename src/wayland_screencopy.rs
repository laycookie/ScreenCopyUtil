use wayland_client::{
    globals::{BindError, GlobalList},
    protocol::wl_output::WlOutput,
    Dispatch, QueueHandle,
};

use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1, zxdg_output_v1::ZxdgOutputV1,
};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use smithay_client_toolkit::globals::GlobalData;

#[derive(Debug)]
pub(crate) struct ScreenCopyManager {
    zwlr_screencopy_manager: ZwlrScreencopyManagerV1,
}

#[derive(Debug)]
pub(crate) struct OutputManager {
    zxdg_output_manager_v1: ZxdgOutputManagerV1,
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
        Ok(ScreenCopyManager {
            zwlr_screencopy_manager,
        })
    }

    pub(crate) fn zwlr_screencopy_manager(&self) -> &ZwlrScreencopyManagerV1 {
        &self.zwlr_screencopy_manager
    }

    pub(crate) fn capture_output<
        U: Send + Sync + 'static,
        D: Dispatch<ZwlrScreencopyFrameV1, U> + 'static,
    >(
        &self,
        overlay_cursor: i32,
        output: &WlOutput,
        qh: &QueueHandle<D>,
        udata: U,
    ) -> ZwlrScreencopyFrameV1 {
        self.zwlr_screencopy_manager
            .capture_output(overlay_cursor, output, &qh, udata)
    }
}

impl OutputManager {
    pub(crate) fn bind<State>(
        globals: &GlobalList,
        qh: &QueueHandle<State>,
    ) -> Result<OutputManager, BindError>
    where
        State: Dispatch<ZxdgOutputManagerV1, GlobalData, State> + 'static,
    {
        let zxdg_output_manager_v1 = globals.bind(qh, 1..=1, GlobalData)?;
        Ok(OutputManager {
            zxdg_output_manager_v1,
        })
    }
    pub(crate) fn get_xdg_output<
        U: Send + Sync + 'static,
        D: Dispatch<ZxdgOutputV1, U> + 'static,
    >(
        &self,
        wl_output: &WlOutput,
        qh: &QueueHandle<D>,
        udata: U,
    ) -> ZxdgOutputV1 {
        self.zxdg_output_manager_v1
            .get_xdg_output(wl_output, qh, udata)
    }
}
