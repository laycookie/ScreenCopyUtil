use std::{
    fs::File,
    io::Read,
    os::fd::AsFd,
    rc::Rc,
    sync::{Arc, Mutex},
};

use memmap::Mmap;
use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalList, GlobalListContents},
    protocol::{
        wl_buffer::WlBuffer,
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
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

pub mod get_screencopy;
pub mod init;

struct Delegate;
delegate_noop!(Delegate: ignore WlShm);
delegate_noop!(Delegate: ignore WlShmPool);
delegate_noop!(Delegate: ignore WlBuffer);
delegate_noop!(Delegate: ignore WlCompositor);
delegate_noop!(Delegate: ignore WlSurface);
delegate_noop!(Delegate: ignore WpViewporter);
delegate_noop!(Delegate: ignore WpViewport);
delegate_noop!(Delegate: ignore ZwlrScreencopyManagerV1);
delegate_noop!(Delegate: ignore ZxdgOutputManagerV1);

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

#[derive(Debug)]
pub(crate) struct ScreenData {
    pub(crate) resolution: (i32, i32),
    logical_resolution: (i32, i32),
    output: WlOutput,
}

#[derive(Debug)]
pub(crate) struct PopupData {
    pub screen_data: ScreenData,
    buffer: WlBuffer,
    surface: WlSurface,

    pub(crate) offset: usize,
    pub(crate) span: usize,
}

impl PopupData {
    fn screencopy(
        &self,
        screencopying_counter: &Arc<Mutex<u8>>,
        qh: &QueueHandle<Delegate>,
        screencopy_manager: &ZwlrScreencopyManagerV1,
    ) {
        *screencopying_counter.lock().unwrap() += 1;

        let a = screencopy_manager.capture_output(
            0,
            &self.screen_data.output,
            qh,
            screencopying_counter.clone(),
        );
        a.copy(&self.buffer);
    }
}

pub(crate) fn screenshot(mut vars: WaylandVarsNew, file: &mut File) -> Vec<PopupData> {
    let screensdata = get_screen_data(&mut vars);
    let (globals, qh, screencopying_counter) = (vars.globals, &vars.qh, Arc::new(Mutex::new(0)));

    // Globals
    let shm: WlShm = globals.bind(qh, 1..=1, ()).unwrap();
    let compositor: WlCompositor = globals.bind(qh, 1..=4, ()).unwrap();
    let viewporter: WpViewporter = globals.bind(qh, 1..=1, ()).unwrap();
    let screencopy_manager: ZwlrScreencopyManagerV1 = globals.bind(qh, 1..=1, ()).unwrap();

    // Logic
    let format_bytes = 4;
    let pixel_count = screensdata
        .iter()
        .map(|screen_data| screen_data.resolution.0 * screen_data.resolution.1)
        .sum::<i32>();

    //let mut file = tempfile::tempfile().unwrap();
    file.set_len((pixel_count * format_bytes) as u64).unwrap();

    let shm_pool = shm.create_pool(file.as_fd(), pixel_count * format_bytes, qh, ());
    let (mut popups, mut pixels_passed) = (vec![], 0usize);
    for screen in screensdata {
        let (width, height) = (screen.resolution.0, screen.resolution.1);
        let (l_width, l_height) = (screen.logical_resolution.0, screen.logical_resolution.1);
        let screen_byte_span = pixels_passed + (width * height * format_bytes) as usize;

        let buffer = shm_pool.create_buffer(
            pixels_passed as i32,
            width,
            height,
            width * 4,
            Format::Xrgb8888,
            qh,
            (),
        );
        let surface = compositor.create_surface(qh, ());

        let viewport = viewporter.get_viewport(&surface, qh, ());
        viewport.set_destination(l_width, l_height);

        let popup_data = PopupData {
            screen_data: screen,
            buffer,
            surface,

            offset: pixels_passed,
            span: screen_byte_span,
        };

        popup_data.screencopy(&screencopying_counter, qh, &screencopy_manager);
        popups.push(popup_data);

        pixels_passed += screen_byte_span;
    }

    // A wait until finished copying screenshot to buffer
    loop {
        vars.event_queue.blocking_dispatch(&mut Delegate).unwrap();
        let val = screencopying_counter.lock().unwrap();
        println!("{}", val);
        if *val == 0 {
            break;
        };
    }

    popups
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
