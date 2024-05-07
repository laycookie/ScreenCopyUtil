use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    os::fd::AsFd,
};

use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalList, GlobalListContents},
    protocol::{
        wl_buffer::WlBuffer,
        wl_callback::WlCallback,
        wl_compositor::WlCompositor,
        wl_output::WlOutput,
        wl_registry::{self, WlRegistry},
        wl_shell_surface::WlShellSurface,
        wl_shm::{Format, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
    Connection, Dispatch, EventQueue, QueueHandle,
};
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;
use wayland_protocols_wlr::{
    layer_shell::v1::client::{
        zwlr_layer_shell_v1::{Layer, ZwlrLayerShellV1},
        zwlr_layer_surface_v1::{self, KeyboardInteractivity, ZwlrLayerSurfaceV1},
    },
    screencopy::v1::client::{
        zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1,
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
    conn: Connection,
    pub(crate) globals: GlobalList,
    pub(crate) event_queue: EventQueue<Delegate>,
    pub(crate) qh: QueueHandle<Delegate>,
}

pub(crate) fn create() -> WaylandVars {
    let conn = Connection::connect_to_env().unwrap();
    let (globals, event_queue) = registry_queue_init::<Delegate>(&conn).unwrap();
    let qh = event_queue.handle();

    WaylandVars {
        conn,
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
impl Dispatch<ZwlrScreencopyFrameV1, ()> for Delegate {
    fn event(
        _: &mut Self,
        _: &ZwlrScreencopyFrameV1,
        event: <ZwlrScreencopyFrameV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        println!("{:#?}", event);
    }
}

struct WaylandWindowData {
    ressolution: (i32, i32),
    offset: i32,
    buffer: WlBuffer,
    surface: WlSurface,
    output: WlOutput,
}
struct WindowDataManager<'a> {
    file: File,
    qh: &'a QueueHandle<Delegate>,
    wayland_window_data: Vec<WaylandWindowData>,
}

impl WindowDataManager<'_> {
    fn atach(&self) {
        self.wayland_window_data.iter().for_each(|win_data| {
            win_data.surface.attach(Some(&win_data.buffer), 0, 0);
            win_data.surface.commit();
        })
    }

    fn draw(&mut self, monitor: usize) {
        let monitor_data = &self.wayland_window_data[monitor];
        let (buf_x, buf_y) = (monitor_data.ressolution.0, monitor_data.ressolution.1);
        let tmp = &mut self.file;

        use std::{cmp::min, io::Write};
        let mut buf = std::io::BufWriter::new(tmp);

        buf.seek(SeekFrom::Start(monitor_data.offset as u64))
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

    fn screencopy(&mut self, monitor: usize, screencopy_manager: &ZwlrScreencopyManagerV1) {
        let monitor_data = &self.wayland_window_data[monitor];
        let _tmp = &mut self.file;

        let a = screencopy_manager.capture_output(0, &monitor_data.output, self.qh, ());
        a.copy(&monitor_data.buffer);
    }
}

pub(crate) fn screen_shot_overlay(wayland: &mut WaylandVars, screens: Vec<ScreenData>) {
    delegate_noop!(Delegate: ignore WlCompositor);
    delegate_noop!(Delegate: ignore WlBuffer);
    delegate_noop!(Delegate: ignore WlSurface);
    delegate_noop!(Delegate: ignore WlCallback);
    delegate_noop!(Delegate: ignore XdgWmBase);
    delegate_noop!(Delegate: ignore WlShm);
    delegate_noop!(Delegate: ignore WlShmPool);
    delegate_noop!(Delegate: ignore WlShellSurface);
    delegate_noop!(Delegate: ignore ZwlrLayerShellV1);
    delegate_noop!(Delegate: ignore ZwlrScreencopyManagerV1);

    let qh = &wayland.qh;

    // Globals
    let compositor: WlCompositor = wayland.globals.bind(qh, 1..=4, ()).unwrap();
    let shm: WlShm = wayland.globals.bind(qh, 1..=1, ()).unwrap();
    let layer_shell: ZwlrLayerShellV1 = wayland.globals.bind(qh, 1..=1, ()).unwrap();
    let screencopy_manager: ZwlrScreencopyManagerV1 = wayland.globals.bind(qh, 1..=1, ()).unwrap();

    // Config window data store
    let screens_pixel_count = screens
        .iter()
        .map(|screen_data| screen_data.resolution.0 * screen_data.resolution.1 * 4)
        .sum::<i32>();

    let file = tempfile::tempfile().unwrap();
    file.set_len(screens_pixel_count as u64).unwrap();

    let mut window_data_manager = WindowDataManager {
        file,
        wayland_window_data: vec![],
        qh,
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

        window_data_manager
            .wayland_window_data
            .push(WaylandWindowData {
                buffer: shm_pool.create_buffer(
                    pixels_passed,
                    width,
                    height,
                    width * 4,
                    Format::Xrgb8888,
                    qh,
                    (),
                ),
                surface: compositor.create_surface(qh, ()),

                ressolution: (width, height),
                offset: pixels_passed,
                output: screen.wl_output,
            });

        let window_data = window_data_manager.wayland_window_data.last_mut().unwrap();

        let layer_surface = layer_shell.get_layer_surface(
            &window_data.surface,
            Some(&window_data.output),
            Layer::Overlay,
            "ScreenshotUtil".to_string(),
            qh,
            (),
        );

        // Configure surface
        layer_surface.set_size(width as u32, height as u32);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
        window_data.surface.commit();

        wayland
            .event_queue
            .blocking_dispatch(&mut Delegate)
            .unwrap();

        pixels_passed += width * height * 4;
    }
    //window_data_manager.draw(0);
    //window_data_manager.draw(1);

    let mut buf = [0; 100];
    window_data_manager.file.read_exact(&mut buf).unwrap();
    println!("{:?}", buf);

    window_data_manager.screencopy(0, &screencopy_manager);
    window_data_manager.atach();

    window_data_manager.file.read_exact(&mut buf).unwrap();
    println!("{:?}", buf);
    //window_data_manager.screencopy(1, &screencopy_manager);

    loop {
        wayland
            .event_queue
            .blocking_dispatch(&mut Delegate)
            .unwrap();
    }
}
