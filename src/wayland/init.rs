use std::{
    fs::File,
    io::{Seek, SeekFrom},
    os::fd::AsFd,
    sync::{Arc, Mutex},
};

use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalList, GlobalListContents},
    protocol::{
        wl_buffer::WlBuffer,
        wl_callback::WlCallback,
        wl_compositor::WlCompositor,
        wl_registry::{self, WlRegistry},
        wl_shell_surface::WlShellSurface,
        wl_shm::{Format, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
    Connection, Dispatch, EventQueue, QueueHandle,
};
use wayland_protocols::{
    wp::viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
    xdg::shell::client::xdg_wm_base::XdgWmBase,
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

use super::get_screencopy::ScreenData;

pub(crate) struct Delegate;

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

pub(crate) struct WaylandVars {
    pub(crate) globals: GlobalList,
    pub(crate) event_queue: EventQueue<Delegate>,
    pub(crate) qh: QueueHandle<Delegate>,
}

pub(crate) fn create() -> WaylandVars {
    let conn = Connection::connect_to_env().unwrap();
    let (globals, event_queue) = registry_queue_init::<Delegate>(&conn).unwrap();
    let qh = event_queue.handle();

    WaylandVars {
        globals,
        event_queue,
        qh,
    }
}

// ===============

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

struct WindowDataManager<'a> {
    file: File,
    qh: &'a QueueHandle<Delegate>,
    wayland_window_data: Vec<PopupData>,
}

#[derive(Debug)]
struct PopupData {
    screen_data: ScreenData,
    _offset: i32,
    buffer: WlBuffer,
    surface: WlSurface,
}

impl WindowDataManager<'_> {
    fn atach(&self) {
        self.wayland_window_data.iter().for_each(|win_data| {
            win_data.surface.attach(Some(&win_data.buffer), 0, 0);
            win_data.surface.commit();
        })
    }

    fn _draw(&mut self, monitor: usize) {
        let monitor_data = &self.wayland_window_data[monitor];
        let (buf_x, buf_y) = (
            monitor_data.screen_data.resolution.0,
            monitor_data.screen_data.resolution.1,
        );
        let tmp = &mut self.file;

        use std::{cmp::min, io::Write};
        let mut buf = std::io::BufWriter::new(tmp);

        buf.seek(SeekFrom::Start(monitor_data._offset as u64))
            .unwrap();

        for y in 0..buf_y {
            for x in 0..buf_x {
                let a = 0xFF;
                let r = min(((buf_x - x) * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
                let g = min((x * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
                let b = min(((buf_x - x) * 0xFF) / buf_x, (y * 0xFF) / buf_y);
                buf.write_all(&[b as u8, g as u8, r as u8, a as u8])
                    .unwrap();
            }
        }
        buf.flush().unwrap();
    }

    /// Returns a bool when the screen has been copied into the buffer.
    fn screencopy(&self, screencopy_manager: &ZwlrScreencopyManagerV1) -> Arc<Mutex<u8>> {
        let screencopying_counter = Arc::new(Mutex::new(0u8));

        for win_data in &self.wayland_window_data {
            *screencopying_counter.lock().unwrap() += 1;

            let a = screencopy_manager.capture_output(
                0,
                &win_data.screen_data.wl_output,
                self.qh,
                screencopying_counter.clone(),
            );
            a.copy(&win_data.buffer);
        }

        screencopying_counter
    }
}

delegate_noop!(Delegate: ignore WlCompositor);
delegate_noop!(Delegate: ignore WlBuffer);
delegate_noop!(Delegate: ignore WlSurface);
delegate_noop!(Delegate: ignore WlShm);
delegate_noop!(Delegate: ignore WlShmPool);
delegate_noop!(Delegate: ignore WlShellSurface);
delegate_noop!(Delegate: ignore ZwlrLayerShellV1);
delegate_noop!(Delegate: ignore ZwlrScreencopyManagerV1);
delegate_noop!(Delegate: ignore WpViewporter);
delegate_noop!(Delegate: ignore WpViewport);
pub(crate) fn screen_shot_overlay(wayland: &mut WaylandVars, screens: Vec<ScreenData>) {
    let qh = &wayland.qh;

    // Globals
    let compositor: WlCompositor = wayland.globals.bind(qh, 1..=4, ()).unwrap();
    let shm: WlShm = wayland.globals.bind(qh, 1..=1, ()).unwrap();
    let layer_shell: ZwlrLayerShellV1 = wayland.globals.bind(qh, 1..=1, ()).unwrap();
    let screencopy_manager: ZwlrScreencopyManagerV1 = wayland.globals.bind(qh, 1..=1, ()).unwrap();
    let viewporter: WpViewporter = wayland.globals.bind(qh, 1..=1, ()).unwrap();

    // Config window data store
    let screens_pixel_count = screens
        .iter()
        .map(|screen_data| screen_data.resolution.0 * screen_data.resolution.1 * 4)
        .sum::<i32>();

    let file = tempfile::tempfile().unwrap();
    file.set_len(screens_pixel_count as u64).unwrap();

    let mut window_data_manager = WindowDataManager {
        file,
        qh,
        wayland_window_data: vec![],
    };
    let shm_pool = shm.create_pool(
        window_data_manager.file.as_fd(),
        screens_pixel_count,
        qh,
        (),
    );

    let mut pixels_passed = 0;
    for screen in screens {
        let (width, height) = (screen.resolution.0, screen.resolution.1);
        let (l_width, l_height) = (screen.logical_resolution.0, screen.logical_resolution.1);

        let surface = compositor.create_surface(qh, ());
        let viewport = viewporter.get_viewport(&surface, qh, ());

        viewport.set_destination(l_width, l_height);
        //surface.set_buffer_scale(2);
        window_data_manager.wayland_window_data.push(PopupData {
            buffer: shm_pool.create_buffer(
                pixels_passed,
                width,
                height,
                width * 4,
                Format::Xrgb8888,
                qh,
                (),
            ),
            surface,
            _offset: pixels_passed,
            screen_data: screen,
        });

        let window_data = window_data_manager.wayland_window_data.last_mut().unwrap();

        let layer_surface = layer_shell.get_layer_surface(
            &window_data.surface,
            Some(&window_data.screen_data.wl_output),
            Layer::Overlay,
            "ScreenshotUtil".to_string(),
            qh,
            (),
        );

        // Configure surface
        layer_surface.set_size(l_width as u32, l_height as u32);
        layer_surface.set_anchor(Anchor::Bottom);
        layer_surface.set_margin(0, 0, 0, 0);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
        window_data.surface.commit();

        wayland
            .event_queue
            .blocking_dispatch(&mut Delegate)
            .unwrap();

        pixels_passed += width * height * 4;
    }

    // Wait until screencopy finishes copying to the buffer and then attch the buffer to the
    // surface
    {
        let success = window_data_manager.screencopy(&screencopy_manager);
        loop {
            wayland
                .event_queue
                .blocking_dispatch(&mut Delegate)
                .unwrap();
            let val = success.lock().unwrap();
            if *val == 0 {
                break;
            };
        }
    }

    window_data_manager.atach();

    loop {
        wayland
            .event_queue
            .blocking_dispatch(&mut Delegate)
            .unwrap();
    }
}
