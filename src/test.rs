use crate::wayland_data::{ScreenShotViewer, WindowState};
use crate::wayland_fractional_scale::FractionalScale;
use crate::wayland_screencopy::ScreenCopyManager;
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::globals::GlobalData;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::xdg::window::WindowDecorations;
use smithay_client_toolkit::shell::xdg::XdgShell;
use smithay_client_toolkit::shm::slot::SlotPool;
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::{
    delegate_compositor, delegate_output, delegate_registry, delegate_shm, delegate_xdg_shell,
    delegate_xdg_window,
};
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::wl_shm::Format;
use wayland_client::{Connection, QueueHandle};

pub fn test() {
    delegate_compositor!(WindowState);
    delegate_output!(WindowState);
    delegate_shm!(WindowState);

    delegate_xdg_shell!(WindowState);
    delegate_xdg_window!(WindowState);

    delegate_registry!(WindowState);
    // Init. wayland
    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh: QueueHandle<WindowState> = event_queue.handle();

    // Init. window_state, and bind globals.
    let shm = Shm::bind(&globals, &qh).expect("wl shm is not available.");
    let pool = SlotPool::new(256 * 256, &shm).expect("Failed to create pool");
    let xdg_shell = XdgShell::bind(&globals, &qh).expect("xdg shell is not available");
    let compositor_state =
        CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
    let zwlr_screencopy_manager =
        ScreenCopyManager::bind(&globals, &qh).expect("zwlr_screencopy_manager not available");

    let wp_fractional_scale_manager_v1 =
        FractionalScale::bind(&globals, &qh).expect("wp_fractional_scale_manager_v1 not available");

    let mut window_state = WindowState {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        xdg_shell,
        compositor_state,
        shm,
        pool,
        zwlr_screencopy_manager,
        windows: vec![],
    };

    event_queue.roundtrip(&mut window_state).unwrap();

    // Init a window per monitor
    for (i, screen) in window_state.output_state.outputs().enumerate() {
        let (width, height) = match window_state.output_state.info(&screen) {
            None => {
                continue;
            }
            Some(info) => info.modes[0].dimensions,
        };

        let (buffer, _) = window_state
            .pool
            .create_buffer(width, height, width * 4, Format::Xbgr8888)
            .unwrap();

        let screenshot = window_state
            .zwlr_screencopy_manager
            .capture_output(0, &screen, &qh, GlobalData);

        screenshot.copy(buffer.wl_buffer());

        let surface = window_state.compositor_state.create_surface(&qh);
        let window =
            window_state
                .xdg_shell
                .create_window(surface, WindowDecorations::RequestServer, &qh);
        window_state.windows.push(ScreenShotViewer {
            window,
            screen: i,
            screenshot_buffer: buffer,
            width: width as u32,
            height: height as u32,
            format: Format::Xbgr8888,
            first_configure: true,
            damaged: false,
        });
    }

    loop {
        event_queue.blocking_dispatch(&mut window_state).unwrap();

        if window_state.windows.is_empty() {
            println!("Exiting.");
            break;
        }
    }
}
