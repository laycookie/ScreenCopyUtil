use smithay_client_toolkit::globals::GlobalData;
use wayland_client::globals::{BindError, GlobalList};
use wayland_client::{Dispatch, QueueHandle};
use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1
};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

pub(crate) struct FractionalScale {
    wp_fractional_scale_manager_v1: WpFractionalScaleManagerV1
}

impl FractionalScale {
    pub(crate) fn bind<State>(
        globals: &GlobalList,
        qh: &QueueHandle<State>,
    ) -> Result<FractionalScale, BindError>
        where
            State: Dispatch<WpFractionalScaleManagerV1, GlobalData, State> + 'static,
    {
        let wp_fractional_scale_manager_v1 = globals.bind(qh, 1..=1, GlobalData)?;
        Ok(FractionalScale { wp_fractional_scale_manager_v1 })
    }

    pub(crate) fn wp_fractional_scale_manager_v1(&self) -> &WpFractionalScaleManagerV1 {
        &self.wp_fractional_scale_manager_v1
    }
}
