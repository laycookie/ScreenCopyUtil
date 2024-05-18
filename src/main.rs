use image::ImageFormat;
use std::{
    io::{Read, Seek, SeekFrom, Write},
    mem,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::SystemTime,
};
use types::BuffersStore;
use wayland::{
    create_popup, screenshot,
    types::{Delegate, Popup},
    WaylandVarsNew,
};

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
    Fullscreen,
    Window,
}

struct Settings {
    background: [u8; 4],
    path: PathBuf,
}

fn main() {
    let screenshot_type = ScreenshotType::Fullscreen;
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
    let mut screenshots_data = screenshot(&mut wayland_vars, screenshots_file);

    println!("Copying screenshots");
    // Save the screenshot
    let mut screens_buf = Vec::with_capacity(screenshots_data.file_len);
    screenshots_data
        .buffer_file
        .read_to_end(&mut screens_buf)
        .unwrap();

    println!("Creating popups");
    #[cfg(target_os = "linux")]
    let mut popups = create_popup(&mut wayland_vars, &screenshots_data);

    // Render overlay
    #[cfg(target_os = "linux")]
    {
        println!("Drawing");
        draw_background(&mut popups, settings.background);
        println!("Done");
        popups.buffers_metadata.iter().for_each(|a| {
            a.surface.frame(&wayland_vars.qh, ());
            a.surface.attach(Some(&a.buffer), 0, 0);
            a.surface.commit();
        });
        wayland_vars
            .event_queue
            .blocking_dispatch(&mut Delegate)
            .unwrap();
    }

    // Save image
    match screenshot_type {
        ScreenshotType::Fullscreen => {
            let mut image_num = 0;
            let test = Arc::new(screens_buf.clone());

            let handles = screenshots_data
                .buffers_metadata
                .iter()
                .map(|screenshot_data| {
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
        let uncompressed_buf = unsafe {
            let len = compressed_buf.len() * 4;
            let ptr = compressed_buf.as_ptr() as *const u8;
            mem::forget(compressed_buf);

            Vec::from_raw_parts(ptr as *mut u8, len, len)
        };

        popups_mem
            .buffer_file
            .write_all(&uncompressed_buf[..])
            .unwrap();
    }
}
