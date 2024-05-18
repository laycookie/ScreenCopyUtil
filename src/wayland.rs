use std::{
    fs::File,
    os::fd::AsFd,
    sync::{Arc, Mutex},
};

use tempfile::tempfile;
use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalList, GlobalListContents},
    protocol::{
        wl_buffer::WlBuffer,
        wl_callback::WlCallback,
        wl_compositor::WlCompositor,
        wl_output::{self, WlOutput},
        wl_registry::{self, WlRegistry},
        wl_shm::{Format, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
    Connection, Dispatch, EventQueue, QueueHandle,
};
use wayland_protocols::{
    wp::viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
    xdg::xdg_output::zv1::client::{
        zxdg_output_manager_v1::ZxdgOutputManagerV1,
        zxdg_output_v1::{self, ZxdgOutputV1},
    },
};
use wayland_protocols_wlr::{
    layer_shell::v1::client::{
        zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
        zwlr_layer_surface_v1::{self, Anchor, KeyboardInteractivity, ZwlrLayerSurfaceV1},
    },
    screencopy::v1::client::{
        zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
        zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
    },
};

use crate::types::{BuffersStore, Screenshot};

use self::types::{Delegate, Popup, ScreenData, Screenshots};

pub mod get_screencopy;
pub mod init;
pub mod types;

delegate_noop!(Delegate: ignore WlShm);
delegate_noop!(Delegate: ignore WlShmPool);
delegate_noop!(Delegate: ignore WlBuffer);
delegate_noop!(Delegate: ignore WlCompositor);
delegate_noop!(Delegate: ignore WlSurface);
delegate_noop!(Delegate: ignore WlCallback);
delegate_noop!(Delegate: ignore WpViewporter);
delegate_noop!(Delegate: ignore WpViewport);
delegate_noop!(Delegate: ignore ZwlrScreencopyManagerV1);
delegate_noop!(Delegate: ignore ZxdgOutputManagerV1);
delegate_noop!(Delegate: ignore ZwlrLayerShellV1);

impl Dispatch<WlRegistry, GlobalListContents> for Delegate {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _event: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlOutput, Arc<Mutex<Option<(i32, i32)>>>> for Delegate {
    fn event(
        _: &mut Self,
        _: &WlOutput,
        event: <WlOutput as wayland_client::Proxy>::Event,
        resolution: &Arc<Mutex<Option<(i32, i32)>>>,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Mode { width, height, .. } = event {
            let mut resolution = resolution.lock().unwrap();
            *resolution = Some((width, height));
        }
    }
}
impl Dispatch<ZwlrScreencopyFrameV1, Arc<Mutex<u8>>> for Delegate {
    fn event(
        _: &mut Self,
        _: &ZwlrScreencopyFrameV1,
        event: <ZwlrScreencopyFrameV1 as wayland_client::Proxy>::Event,
        screencopying_counter: &Arc<Mutex<u8>>,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwlr_screencopy_frame_v1::Event::Ready { .. } = event {
            *screencopying_counter.lock().unwrap() -= 1;
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
impl Dispatch<ZwlrLayerSurfaceV1, ()> for Delegate {
    fn event(
        _: &mut Self,
        layer_surface: &ZwlrLayerSurfaceV1,
        event: <ZwlrLayerSurfaceV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwlr_layer_surface_v1::Event::Configure { serial, .. } = event {
            layer_surface.ack_configure(serial);
        }
    }
}

pub(crate) struct WaylandVarsNew {
    pub(crate) event_queue: EventQueue<Delegate>,
    pub(crate) qh: QueueHandle<Delegate>,
    pub(crate) globals: GlobalList,
}

pub(crate) fn init() -> WaylandVarsNew {
    let conn = Connection::connect_to_env().unwrap();
    let (globals, event_queue) = registry_queue_init::<Delegate>(&conn).unwrap();
    let qh = event_queue.handle();

    WaylandVarsNew {
        globals,
        event_queue,
        qh,
    }
}

// ===

pub(crate) fn screenshot(vars: &mut WaylandVarsNew, file: File) -> BuffersStore<Screenshot> {
    let screensdata = get_screen_data(vars);
    let (qh, screencopying_counter) = (&vars.qh, Arc::new(Mutex::new(0)));

    // Globals
    let shm: WlShm = vars.globals.bind(qh, 1..=1, ()).unwrap();
    let screencopy_manager: ZwlrScreencopyManagerV1 = vars.globals.bind(qh, 1..=1, ()).unwrap();

    // Logic
    let format_bytes = 4;
    let pixel_count = screensdata
        .iter()
        .map(|screen_data| screen_data.resolution.0 * screen_data.resolution.1)
        .sum::<i32>();

    file.set_len((pixel_count * format_bytes) as u64).unwrap();

    let shm_pool = shm.create_pool(file.as_fd(), pixel_count * format_bytes, qh, ());

    let (mut screenshots, mut pixels_passed) = (vec![], 0usize);
    for screen in screensdata {
        let (width, height) = (screen.resolution.0, screen.resolution.1);
        let screen_byte_span = pixels_passed + (width * height * format_bytes) as usize;

        let buffer = shm_pool.create_buffer(
            pixels_passed as i32,
            width,
            height,
            width * format_bytes,
            Format::Xrgb8888,
            qh,
            (),
        );

        *screencopying_counter.lock().unwrap() += 1;
        let frame =
            screencopy_manager.capture_output(0, &screen.output, qh, screencopying_counter.clone());
        frame.copy(&buffer);

        screenshots.push(Screenshot {
            screen_data: screen,
            offset: pixels_passed,
            span: screen_byte_span,
        });

        vars.event_queue.blocking_dispatch(&mut Delegate).unwrap();
        pixels_passed += screen_byte_span;
    }

    // A wait until finished copying screenshot to buffer
    loop {
        vars.event_queue.blocking_dispatch(&mut Delegate).unwrap();
        let val = screencopying_counter.lock().unwrap();
        if *val == 0 {
            break;
        };
    }

    BuffersStore {
        buffer_file: file,
        file_len: pixels_passed,
        buffers_metadata: screenshots,
    }
}

fn get_screen_data(vars: &mut WaylandVarsNew) -> Vec<ScreenData> {
    let (globals, event_queue, qh) = (&vars.globals, &mut vars.event_queue, &vars.qh);

    let output_manager: ZxdgOutputManagerV1 = globals.bind(qh, 1..=3, ()).unwrap();

    let mut screens_data: Vec<ScreenData> = vec![];
    for global in globals.contents().clone_list() {
        if let "wl_output" = &global.interface[..] {
            let resolution = Arc::new(Mutex::new(None));
            let logical_resolution = Arc::new(Mutex::new(None));

            let output: WlOutput =
                globals
                    .registry()
                    .bind(global.name, global.version, qh, resolution.clone());
            output_manager.get_xdg_output(&output, qh, logical_resolution.clone());

            event_queue.blocking_dispatch(&mut Delegate).unwrap();

            screens_data.push(ScreenData {
                resolution: resolution.lock().unwrap().expect("res not found"),
                logical_resolution: logical_resolution.lock().unwrap().expect("l_res not found"),
                output,
            });
        }
    }

    screens_data
}
pub(crate) fn create_popup(
    vars: &mut WaylandVarsNew,
    screenshots_data: &BuffersStore<Screenshot>,
) -> BuffersStore<Popup> {
    let qh = &vars.qh;

    let compositor: WlCompositor = vars.globals.bind(qh, 1..=4, ()).unwrap();
    let viewporter: WpViewporter = vars.globals.bind(qh, 1..=1, ()).unwrap();
    let layer_shell: ZwlrLayerShellV1 = vars.globals.bind(qh, 1..=1, ()).unwrap();
    let shm: WlShm = vars.globals.bind(qh, 1..=1, ()).unwrap();

    let backing_memory = tempfile().unwrap();
    backing_memory
        .set_len(screenshots_data.file_len as u64)
        .unwrap();

    let shm_pool = shm.create_pool(
        backing_memory.as_fd(),
        screenshots_data.file_len as i32,
        qh,
        (),
    );

    // ===
    let mut screens = vec![];
    for screen in screenshots_data.buffers_metadata.iter() {
        let (l_width, l_height) = (
            screen.screen_data.logical_resolution.0,
            screen.screen_data.logical_resolution.1,
        );

        let (width, height) = (
            screen.screen_data.resolution.0,
            screen.screen_data.resolution.1,
        );
        let buffer = shm_pool.create_buffer(
            screen.offset as i32,
            width,
            height,
            width * 4,
            Format::Abgr8888,
            qh,
            (),
        );

        let surface = compositor.create_surface(qh, ());
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            Some(&screen.screen_data.output),
            Layer::Overlay,
            "ScreenshotUtil".to_string(),
            qh,
            (),
        );
        let viewport = viewporter.get_viewport(&surface, qh, ());
        viewport.set_destination(l_width, l_height);

        layer_surface.set_size(l_width as u32, l_height as u32);
        layer_surface.set_anchor(Anchor::Bottom);
        layer_surface.set_margin(0, 0, 0, 0);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
        surface.commit();

        vars.event_queue.blocking_dispatch(&mut Delegate).unwrap();
        // change to new buffer

        let mut screen_clone = screen.clone();
        screens.push(Popup {
            screen_data: screen.clone(),
            buffer,
            surface,
        });
    }

    BuffersStore {
        buffer_file: backing_memory,
        file_len: screenshots_data.file_len,
        buffers_metadata: screens,
    }
}
