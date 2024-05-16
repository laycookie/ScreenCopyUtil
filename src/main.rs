use image::ImageFormat;
use std::{
    io::Read,
    path::{Path, PathBuf},
};
use wayland::{create_popup, draw_background, screenshot, types::Delegate, WaylandVarsNew};

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
    #[cfg(target_os = "linux")]
    let mut screenshots_data = screenshot(&mut wayland_vars, screenshots_file);

    // Save the screenshot
    let mut screens_buf = Vec::with_capacity(screenshots_data.file_len);
    screenshots_data.file.read_to_end(&mut screens_buf).unwrap();

    #[cfg(target_os = "linux")]
    let mut temp = create_popup(&mut wayland_vars, &screenshots_data);

    // Render overlay
    #[cfg(target_os = "linux")]
    {
        draw_background(&mut temp, settings.background);
        temp.render(&wayland_vars.qh);
        wayland_vars
            .event_queue
            .blocking_dispatch(&mut Delegate)
            .unwrap();
    }

    // Save image
    match screenshot_type {
        ScreenshotType::Fullscreen => {
            let mut image_num = 0;
            screenshots_data
                .screenshots_data
                .iter()
                .for_each(|screenshot_data| {
                    save_image(
                        screenshot_data.screen_data.resolution,
                        &screens_buf[screenshot_data.offset..screenshot_data.span],
                        &settings.path.join(format!("output{}", image_num)),
                    );
                    image_num += 1;
                });
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
}
