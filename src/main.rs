// mod wayland_app;
// mod wayland_data;
// use std::time::Duration;
// use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
// use wayland_client::globals::{BindError, GlobalList, registry_queue_init};
// use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1;
//
// use smithay_client_toolkit::activation::{ActivationState, RequestData};
// use smithay_client_toolkit::{delegate_output, delegate_registry, output::{OutputState}, registry::{RegistryState}, compositor::{CompositorState}, delegate_compositor, delegate_shm, delegate_keyboard, delegate_seat, delegate_xdg_shell, delegate_xdg_window, delegate_activation, delegate_pointer};
// use smithay_client_toolkit::globals::GlobalData;
// use smithay_client_toolkit::reexports::calloop::{EventLoop};
// use smithay_client_toolkit::reexports::csd_frame::{WindowManagerCapabilities, WindowState};
// use smithay_client_toolkit::reexports::protocols::xdg::activation::v1::client::xdg_activation_v1;
// use smithay_client_toolkit::seat::{SeatState};
// use smithay_client_toolkit::shell::WaylandSurface;
// use smithay_client_toolkit::shell::xdg::window::{DecorationMode, WindowConfigure, WindowDecorations, WindowHandler};
// use smithay_client_toolkit::shell::xdg::{XdgShell, XdgSurface};
// use smithay_client_toolkit::shm::{Shm};
// use smithay_client_toolkit::shm::slot::{SlotPool};
// use wayland_client::protocol::wl_buffer::WlBuffer;
// use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1;
// use std::num::NonZeroU32;
// use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1;
// use crate::wayland_app::{App, ScreenCopyManager};
mod test;
mod wayland_data;
mod wayland_screencopy;
mod wayland_fractional_scale;

use crate::test::test;



fn main() {
    test();
    /*
    delegate_compositor!(App);
    delegate_output!(App);
    delegate_shm!(App);

    delegate_seat!(App);
    delegate_keyboard!(App);
    delegate_pointer!(App);

    delegate_xdg_shell!(App);
    delegate_xdg_window!(App);
    delegate_activation!(App);

    delegate_registry!(App);

    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh: QueueHandle<App> = event_queue.handle();
    let mut event_loop: EventLoop<App> =
        EventLoop::try_new().expect("Failed to initialize the event loop!");

    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
    let xdg_shell = XdgShell::bind(&globals, &qh).expect("xdg shell is not available");
    let shm = Shm::bind(&globals, &qh).expect("wl shm is not available.");
    let xdg_activation = ActivationState::bind(&globals, &qh).ok();
    let zwlr_screencopy_manager = ScreenCopyManager::bind(&globals, &qh).unwrap();

    let surface = compositor.create_surface(&qh);
    let window = xdg_shell.create_window(surface, WindowDecorations::RequestServer, &qh);

    window.set_title("A wayland window");
    window.set_min_size(Some((256, 256)));
    window.set_app_id(String::from("screen_capture"));
    window.commit();

    if let Some(activation) = xdg_activation.as_ref() {
        activation.request_token(
            &qh,
            RequestData {
                seat_and_serial: None,
                surface: Some(window.wl_surface().clone()),
                app_id: Some(String::from("screen_capture")),
            },
        )
    }

    let pool = SlotPool::new(256 * 256, &shm).expect("Failed to create pool");


    let mut app = App {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),

        shm,
        xdg_activation,
        zwlr_screencopy_manager,

        window,
        pool,
        first_configure: false,
        keyboard_focus: false,
        exit: false,
        buffer: None,
        screen_shot_buffer: None,
        width: 256,
        height: 256,
        shift: None,
        keyboard: None,
        pointer: None,
        loop_handle: event_loop.handle(),
    };



    let out = app.output_state.outputs().next().unwrap();
    let screenshot = app.zwlr_screencopy_manager.capture_output(0, &out, &qh, GlobalData);
    event_queue.blocking_dispatch(&mut app).unwrap();
    match app.screen_shot_buffer {
        None => { panic!("temp"); }
        Some(ref temp) => {
            println!("{:?}", temp);
            screenshot.copy(temp.wl_buffer());
        }
    }

    event_queue.blocking_dispatch(&mut app).unwrap();
    app.draw(&conn, &qh);

    loop {
        event_queue.blocking_dispatch(&mut app).unwrap();

        if app.exit {
            println!("Exiting.");
            break;
        }
    }

     */
}
