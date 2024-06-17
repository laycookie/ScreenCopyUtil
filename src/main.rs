use image::ImageFormat;
use std::{
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::SystemTime,
};
use types::BuffersStore;
use wayland::{
    create_popup, screenshot,
    types::{Delegate, Popup},
    WaylandVarsNew,
};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

use crate::types::Screenshot;

pub mod types;
pub mod wayland;

mod test;
mod wayland_data;
mod wayland_fractional_scale;
pub mod wayland_screencopy;

enum WindowManagerTest {
    Wayland(WaylandVarsNew),
}

enum ScreenshotType {
    Fullscreen { single_monitor: bool },
    Window,
}

struct Settings {
    background: [u8; 4],
    path: PathBuf,
}

fn main() {
    let time = SystemTime::now();
    let screenshot_type = ScreenshotType::Fullscreen {
        single_monitor: false,
    };
    let settings = Settings {
        background: [5, 5, 5, 80],
        #[cfg(target_os = "linux")]
        path: dirs::home_dir()
            .expect("Home dir not found")
            .join("Pictures"),
    };

    #[cfg(target_os = "linux")]
    let mut wayland_vars = wayland::init();

    // Create and save the screenshot with the screen info
    let screenshots_file = tempfile::tempfile().unwrap();
    println!("Screenshoting");
    #[cfg(target_os = "linux")]
    let mut screenshots_data = match screenshot_type {
        ScreenshotType::Fullscreen { .. } => screenshot(&mut wayland_vars, screenshots_file),
        ScreenshotType::Window => screenshot(&mut wayland_vars, screenshots_file),
    };

    println!("Creating popups");
    #[cfg(target_os = "linux")]
    let mut popups = create_popup(&mut wayland_vars, &screenshots_data);

    // Render overlay
    #[cfg(target_os = "linux")]
    {
        popups.buffers_metadata.iter().for_each(|a| {
            a.surface.attach(Some(&a.buffer), 0, 0);
            a.surface.commit();
        });
        wayland_vars
            .event_queue
            .blocking_dispatch(&mut Delegate)
            .unwrap();

        // Remove all the popups that dont have the cursor on them

        if matches!(
            screenshot_type,
            ScreenshotType::Fullscreen { single_monitor } if single_monitor
        ) {
            let qh = &wayland_vars.qh;

            let mut seat: Option<WlSeat> = None;
            for global in wayland_vars.globals.contents().clone_list() {
                if "wl_seat" == &global.interface[..] {
                    seat = Some(wayland_vars.globals.registry().bind(
                        global.name,
                        global.version,
                        qh,
                        (),
                    ));
                };
            }
            wayland_vars.event_queue.roundtrip(&mut Delegate).unwrap();

            let hovering_over_surface = Arc::new(Mutex::new(None));
            seat.expect("WlSeat was not found")
                .get_pointer(qh, hovering_over_surface.clone());

            wayland_vars
                .event_queue
                .blocking_dispatch(&mut Delegate)
                .unwrap();

            let hovering_over_surface = hovering_over_surface.lock().unwrap();
            let hovering_over_surface = hovering_over_surface.clone().unwrap();

            popups.buffers_metadata.retain(|e| {
                if e.surface != hovering_over_surface {
                    e.surface.attach(None, 0, 0);
                    e.surface.commit();
                    false
                } else {
                    true
                }
            });
        }

        draw_background(&mut popups, settings.background);
        popups.buffers_metadata.iter().for_each(|a| {
            let (width, height) = a.screen_data.screen_data.resolution;
            a.surface.damage_buffer(0, 0, width, height);
            a.surface.attach(Some(&a.buffer), 0, 0);
            a.surface.commit();
        });
        wayland_vars
            .event_queue
            .blocking_dispatch(&mut Delegate)
            .unwrap();
    }

    #[cfg(target_os = "linux")]
    {
        let qh = &wayland_vars.qh;
        let screencopy_manager: ZwlrScreencopyManagerV1 =
            wayland_vars.globals.bind(qh, 1..=1, ()).unwrap();

        let screencopying_counter = Arc::new(Mutex::new(0u8));
        popups.buffers_metadata.iter().for_each(|e| {
            e.screen_data.wayland_data.screencopy(
                screencopying_counter.clone(),
                qh,
                &screencopy_manager,
            )
        });

        loop {
            wayland_vars
                .event_queue
                .blocking_dispatch(&mut Delegate)
                .unwrap();

            let val = screencopying_counter.lock().unwrap();
            if *val == 0 {
                break;
            }
        }
    }

    // Save the screenshot
    let mut screens_buf = Vec::with_capacity(screenshots_data.file_len);
    screenshots_data
        .buffer_file
        .read_to_end(&mut screens_buf)
        .unwrap();

    // println!("{:?}", screens_buf);

    // Save image
    match screenshot_type {
        ScreenshotType::Fullscreen { .. } => {
            let mut image_num = 0;
            let test = Arc::new(screens_buf);

            let handles = popups
                .buffers_metadata
                .iter()
                .map(|e| {
                    let screenshot_data = &e.screen_data;

                    let pixels = test.clone();
                    let offset = screenshot_data.offset;
                    let span = screenshot_data.span;
                    let resolution = screenshot_data.screen_data.resolution;
                    let path = settings.path.join(format!("output{}", image_num));
                    image_num += 1;
                    println!("Saving");

                    thread::spawn(move || save_image(resolution, &pixels[offset..span], &path))
                })
                .collect::<Vec<_>>();

            for handle in handles {
                handle.join().unwrap();
            }
        }
        ScreenshotType::Window => todo!(),
    }
    println!("{:?}", time.elapsed());
}

fn save_image((width, height): (i32, i32), pixels: &[u8], path: &Path) {
    let img = image::ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
        let index = (y * width as u32 + x) as usize * 4;

        let r = pixels[index + 2];
        let g = pixels[index + 1];
        let b = pixels[index];

        image::Rgb([r, g, b])
    });

    //println!("{:?}", img);

    println!("Compressing");
    img.save_with_format(path, ImageFormat::WebP)
        .expect("Failed to save image");
    println!("Finished compressing");
}

pub(crate) fn draw_background(popups_mem: &mut BuffersStore<Popup>, background: [u8; 4]) {
    for popup in popups_mem.buffers_metadata.iter() {
        popups_mem
            .buffer_file
            .seek(SeekFrom::Start(popup.screen_data.offset as u64))
            .unwrap();

        let cashed = u32::from_ne_bytes(background);
        let compressed_buf = vec![cashed; popup.screen_data.span / 4];
        let uncompressed_buf = bytemuck::cast_slice::<u32, u8>(&compressed_buf[..]);

        popups_mem.buffer_file.write_all(uncompressed_buf).unwrap();
    }
}
