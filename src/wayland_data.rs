use smithay_client_toolkit::compositor::CompositorHandler;
use smithay_client_toolkit::globals::GlobalData;
use smithay_client_toolkit::output::OutputHandler;
use smithay_client_toolkit::registry::ProvidesRegistryState;
use smithay_client_toolkit::shell::xdg::window::{WindowConfigure, WindowHandler};
use smithay_client_toolkit::shell::xdg::XdgShell;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::ShmHandler;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    shell::xdg::window::Window,
    shm::{
        slot::{Buffer, SlotPool},
        Shm,
    },
};
use wayland_client::protocol::wl_output::{Transform, WlOutput};
use wayland_client::protocol::wl_shm::Format;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1;
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1;
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

use crate::wayland_screencopy::ScreenCopyManager;

pub(crate) struct WindowState {
    pub(crate) registry_state: RegistryState,
    pub(crate) output_state: OutputState,
    pub(crate) compositor_state: CompositorState,
    pub(crate) shm: Shm,
    pub(crate) zwlr_screencopy_manager: ScreenCopyManager,
    pub(crate) xdg_shell: XdgShell,

    pub(crate) pool: SlotPool,
    pub(crate) windows: Vec<ScreenShotViewer>,
}

#[derive(Debug)]
pub(crate) struct ScreenShotViewer {
    pub(crate) window: Window,
    pub(crate) screenshot_buffer: Buffer,
    pub(crate) screen: usize,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) format: Format,
    pub(crate) first_configure: bool,
    pub(crate) damaged: bool,
}

impl WindowState {
    pub fn draw(&mut self, _conn: &Connection, qh: &QueueHandle<Self>) {
        for viewer in &mut self.windows {
            if viewer.first_configure || !viewer.damaged {
                continue;
            }

            let window = &viewer.window;

            // Damage the entire window
            window
                .wl_surface()
                .damage_buffer(0, 0, viewer.width as i32, viewer.height as i32);
            viewer.damaged = false;

            // Request our next frame
            window.wl_surface().frame(qh, window.wl_surface().clone());

            // Attach and commit to present.
            window.wl_surface().set_buffer_scale(2);
            viewer
                .screenshot_buffer
                .attach_to(window.wl_surface())
                .unwrap();
            window.wl_surface().commit();
        }
    }
}

impl WindowHandler for WindowState {
    fn request_close(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, window: &Window) {
        self.windows.retain(|v| v.window != *window);
    }

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        //println!("{:#?}", configure);
        for view in &mut self.windows {
            if view.window != *window {
                continue;
            }

            view.width = configure.new_size.0.map(|v| v.get()).unwrap_or(256);
            view.height = configure.new_size.1.map(|v| v.get()).unwrap_or(256);
            view.first_configure = false;
            view.damaged = true;
        }
        self.draw(&conn, &qh);
    }
}

impl ShmHandler for WindowState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl OutputHandler for WindowState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {}

    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {}

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
    }
}

impl CompositorHandler for WindowState {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_transform: Transform,
    ) {
    }

    fn frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _time: u32,
    ) {
        self.draw(conn, qh);
    }
}

impl ProvidesRegistryState for WindowState {
    fn registry(&mut self) -> &mut RegistryState {
        todo!()
    }

    fn runtime_add_global(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _name: u32,
        _interface: &str,
        _version: u32,
    ) {
        todo!()
    }

    fn runtime_remove_global(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _name: u32,
        _interface: &str,
    ) {
        todo!()
    }
}

impl Dispatch<ZwlrScreencopyManagerV1, GlobalData> for WindowState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrScreencopyManagerV1,
        _event: <ZwlrScreencopyManagerV1 as Proxy>::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        println!("ScreenCopy");
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, GlobalData> for WindowState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrScreencopyFrameV1,
        event: <ZwlrScreencopyFrameV1 as Proxy>::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        println!("{:?}", event);
    }
}

impl Dispatch<WpFractionalScaleV1, GlobalData> for WindowState {
    fn event(
        _state: &mut Self,
        _proxy: &WpFractionalScaleV1,
        event: <WpFractionalScaleV1 as Proxy>::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        println!("{:?}", event);
    }
}

impl Dispatch<WpFractionalScaleManagerV1, GlobalData> for WindowState {
    fn event(
        _state: &mut Self,
        _proxy: &WpFractionalScaleManagerV1,
        _event: <WpFractionalScaleManagerV1 as Proxy>::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        println!("Stufffaaa");
    }
}

