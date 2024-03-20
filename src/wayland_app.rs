use std::fmt::Debug;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};
use wayland_client::protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface};

use smithay_client_toolkit::{
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
};
use smithay_client_toolkit::activation::{ActivationHandler, ActivationState, RequestData};
use smithay_client_toolkit::compositor::CompositorHandler;
use smithay_client_toolkit::globals::GlobalData;
use smithay_client_toolkit::reexports::calloop::LoopHandle;
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::seat::keyboard::{KeyboardHandler, KeyEvent, Keysym, Modifiers};
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerHandler, PointerEventKind};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::xdg::window::{Window, WindowConfigure, WindowHandler};
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};
use wayland_client::globals::{BindError, GlobalList};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_keyboard::WlKeyboard;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_shm::Format;
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::{Event, ZwlrScreencopyFrameV1};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1;
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

#[derive(Debug)]
pub(crate) struct App {
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) output_state: OutputState,
    pub(crate) shm: Shm,
    pub(crate) xdg_activation: Option<ActivationState>,
    pub(crate) zwlr_screencopy_manager: ScreenCopyManager,
    pub(crate) window: Window,

    pub(crate) screen_shot_buffer: Option<Buffer>,

    pub(crate) pool: SlotPool,
    pub(crate) first_configure: bool,
    pub(crate) keyboard_focus: bool,
    pub(crate) exit: bool,
    pub(crate) buffer: Option<Buffer>,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) shift: Option<i32>,
    pub(crate) keyboard: Option<WlKeyboard>,
    pub(crate) pointer: Option<wl_pointer::WlPointer>,
    pub(crate) loop_handle: LoopHandle<'static, App>,
}

impl App {
    pub fn draw(&mut self, _conn: &Connection, qh: &QueueHandle<Self>) {
        let width = self.width;
        let height = self.height;
        let stride = width * 4;

        let buffer = self.buffer.get_or_insert_with(|| {
            self.pool
                .create_buffer(width, height, stride, wl_shm::Format::Argb8888)
                .expect("create buffer")
                .0
        });

        // let canvas = match self.pool.canvas(buffer) {
        //     Some(canvas) => canvas,
        //     None => {
        //         // This should be rare, but if the compositor has not released the previous
        //         // buffer, we need double-buffering.
        //         let (second_buffer, canvas) = self
        //             .pool
        //             .create_buffer(
        //                 width,
        //                 height,
        //                 stride,
        //                 wl_shm::Format::Argb8888,
        //             )
        //             .expect("create buffer");
        //         *buffer = second_buffer;
        //         canvas
        //     }
        // };


        // Draw to the window:
        {
            //     let shift = self.shift.unwrap_or(0);
            //     canvas.chunks_exact_mut(4).enumerate().for_each(|(index, chunk)| {
            //         let x = ((index + shift as usize) % width as usize) as u32;
            //         let y = (index / width as usize) as u32;
            //
            //         let a = 0x00;
            //         let r = 0xff;
            //         let g = 0x00;
            //         let b = 0x00;
            //
            //
            //         let array: &mut [u8; 4] = chunk.try_into().unwrap();
            //         *array = [a, r, g, b];
            //     });
        }


        // Damage the entire window
        self.window.wl_surface().damage_buffer(0, 0, self.width, self.height);

        // Request our next frame
        self.window.wl_surface().frame(qh, self.window.wl_surface().clone());

        // Attach and commit to present.
        // let screen = &self.screen_shot_buffer.unwrap();
        // self.window.attach(Option::from(buffer.wl_buffer()), 0, 0);
        match &self.screen_shot_buffer {
            None => {
                panic!("Whatevea")
            }
            Some(screen) => {
                screen.attach_to(self.window.wl_surface()).expect("TODO: panic message");
                println!("{:#?}", screen);
            }
        }

        self.window.commit();
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {}

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {}

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {}
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers! { OutputState, }
}

impl WindowHandler for App {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        println!("Window configured to: {:?}", configure);

        self.buffer = None;
        self.width = configure.new_size.0.map(|v| v.get()).unwrap_or(256) as i32;
        self.height = configure.new_size.1.map(|v| v.get()).unwrap_or(256) as i32;

        // Initiate the first draw.
        if self.first_configure {
            self.first_configure = false;
            self.draw(conn, qh);
        }
    }
}

impl ActivationHandler for App {
    type RequestData = RequestData;

    fn new_token(&mut self, token: String, _data: &Self::RequestData) {
        self.xdg_activation
            .as_ref()
            .unwrap()
            .activate::<App>(self.window.wl_surface(), token);
    }
}

impl KeyboardHandler for App {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &WlKeyboard,
        surface: &wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        keysyms: &[Keysym],
    ) {
        if self.window.wl_surface() == surface {
            println!("Keyboard focus on window with pressed syms: {keysyms:?}");
            self.keyboard_focus = true;
        }
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
    ) {
        if self.window.wl_surface() == surface {
            println!("Release keyboard focus on window");
            self.keyboard_focus = false;
        }
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        println!("Key press: {event:?}");
        if event.raw_code == 1 {
            self.exit = true;
        }
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        println!("Key release: {event:?}");
    }

    fn update_modifiers(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &WlKeyboard, _serial: u32, _modifiers: Modifiers) {
        println!("Modifier key pressed.")
    }
}

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        use PointerEventKind::*;
        for event in events {
            // Ignore events for other surfaces
            if &event.surface != self.window.wl_surface() {
                continue;
            }

            match event.kind {
                Enter { .. } => {
                    println!("Pointer entered @{:?}", event.position);
                }
                Leave { .. } => {
                    println!("Pointer left");
                }
                Motion { .. } => {}
                Press { button, .. } => {
                    println!("Press {:x} @ {:?}", button, event.position);
                    self.shift = self.shift.xor(Some(0));
                }
                Release { button, .. } => {
                    println!("Release {:x} @ {:?}", button, event.position);
                }
                Axis { horizontal, vertical, .. } => {
                    println!("Scroll H:{horizontal:?}, V:{vertical:?}");
                }
            }
        }
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            println!("Set keyboard capability");
            let keyboard = self
                .seat_state
                .get_keyboard_with_repeat(
                    qh,
                    &seat,
                    None,
                    self.loop_handle.clone(),
                    Box::new(|_state, _wl_kbd, event| {
                        println!("Repeat: {:?} ", event);
                    }),
                )
                .expect("Failed to create keyboard");

            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer && self.pointer.is_none() {
            println!("Set pointer capability");
            let pointer = self.seat_state.get_pointer(qh, &seat).expect("Failed to create pointer");
            self.pointer = Some(pointer);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_some() {
            println!("Unset keyboard capability");
            self.keyboard.take().unwrap().release();
        }

        if capability == Capability::Pointer && self.pointer.is_some() {
            println!("Unset pointer capability");
            self.pointer.take().unwrap().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // Not needed for this example.
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        // Not needed for this example.
    }

    fn frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.draw(conn, qh);
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, GlobalData> for App {
    fn event(state: &mut Self,
             proxy: &ZwlrScreencopyFrameV1,
             event: <ZwlrScreencopyFrameV1 as Proxy>::Event,
             data: &GlobalData,
             conn: &Connection,
             qh: &QueueHandle<Self>) {

        println!("{:?}", event);
        match event {
            Event::Buffer {format, width, height, stride } => {
                let WEnum::Value(format) = format else {
                    panic!("AAAAAAAAA");
                };
                let (buffer, _) = state.pool
                    .create_buffer(width as i32, height as i32, stride as i32, format).unwrap();

                state.screen_shot_buffer = Some(buffer);
            }
            err => {
                println!("{:?}", err);}
        }
    }
}

impl Dispatch<ZwlrScreencopyManagerV1, GlobalData> for App {
    fn event(state: &mut Self,
             proxy: &ZwlrScreencopyManagerV1,
             event: <ZwlrScreencopyManagerV1 as Proxy>::Event,
             data: &GlobalData,
             conn: &Connection,
             qh: &QueueHandle<Self>,
    ) {
        println!("ScreenCopy");
    }
}

#[derive(Debug)]
pub(crate) struct ScreenCopyManager {
    zwlr_screencopy_manager: ZwlrScreencopyManagerV1,
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
        Ok(ScreenCopyManager { zwlr_screencopy_manager })
    }

    pub(crate) fn zwlr_screencopy_manager(&self) -> &ZwlrScreencopyManagerV1 {
        &self.zwlr_screencopy_manager
    }

    pub(crate) fn capture_output<U: Send + Sync + 'static, D: Dispatch<ZwlrScreencopyFrameV1, U> + 'static>(
        &self,
        overlay_cursor: i32,
        output: &WlOutput,
        qh: &QueueHandle<D>,
        udata: U,
    ) -> ZwlrScreencopyFrameV1 {
        self.zwlr_screencopy_manager.capture_output(overlay_cursor, output, &qh, udata)
    }
}