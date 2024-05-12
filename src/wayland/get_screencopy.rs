use std::sync::{Arc, Mutex};

use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::{delegate_noop, protocol, Connection, Dispatch, QueueHandle};
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_manager_v1::ZxdgOutputManagerV1;
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_v1::{self, ZxdgOutputV1};

use super::init::{Delegate, WaylandVars};

impl Dispatch<WlOutput, Arc<Mutex<Option<(i32, i32)>>>> for Delegate {
    fn event(
        _: &mut Self,
        _: &WlOutput,
        event: <WlOutput as wayland_client::Proxy>::Event,
        resolution: &Arc<Mutex<Option<(i32, i32)>>>,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let protocol::wl_output::Event::Mode { width, height, .. } = event {
            let mut resolution = resolution.lock().unwrap();
            *resolution = Some((width, height));
        }
    }
}

impl Dispatch<ZxdgOutputV1, Arc<Mutex<Option<(i32, i32)>>>> for Delegate {
    fn event(
        _: &mut Self,
        _: &ZxdgOutputV1,
        event: <ZxdgOutputV1 as wayland_client::Proxy>::Event,
        data: &Arc<Mutex<Option<(i32, i32)>>>,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zxdg_output_v1::Event::LogicalSize { width, height } = event {
            let mut logical_resolution = data.lock().unwrap();
            *logical_resolution = Some((width, height))
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ScreenData {
    pub(crate) logical_resolution: (i32, i32),
    pub(crate) resolution: (i32, i32),
    pub(crate) wl_output: WlOutput,
}

pub(crate) fn get_screen_data(wayland: &mut WaylandVars) -> Vec<ScreenData> {
    delegate_noop!(Delegate: ignore ZxdgOutputManagerV1);

    let qh = &wayland.qh;
    let globals = &mut wayland.globals;

    let xdg_output: ZxdgOutputManagerV1 = globals.bind(qh, 1..=3, ()).unwrap();

    // Get the data from the display
    let mut screens_data: Vec<ScreenData> = vec![];
    for global in globals.contents().clone_list() {
        if let "wl_output" = &global.interface[..] {
            let resolution = Arc::new(Mutex::new(None));
            let logical_resolution = Arc::new(Mutex::new(None));

            let wl_output: WlOutput =
                globals
                    .registry()
                    .bind(global.name, global.version, qh, resolution.clone());

            xdg_output.get_xdg_output(&wl_output, qh, logical_resolution.clone());
            wayland
                .event_queue
                .blocking_dispatch(&mut Delegate)
                .unwrap();

            let screen_data = ScreenData {
                wl_output,
                resolution: resolution.lock().unwrap().expect("res not found"),
                logical_resolution: logical_resolution.lock().unwrap().expect("l_res not found"),
            };

            screens_data.push(screen_data);
        }
    }

    screens_data
}
