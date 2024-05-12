use image::ImageFormat;
use memmap::Mmap;
use std::{fs::File, io::BufWriter, path::Path};
use wayland::{init::WaylandVars, screenshot, WaylandVarsNew};

pub mod wayland;

mod test;
mod wayland_data;
mod wayland_fractional_scale;
pub mod wayland_screencopy;

enum WindowManager {
    Wayland(WaylandVars),
}
enum WindowManagerTest {
    Wayland(WaylandVarsNew),
}
enum ScreenshotType {
    Fullscreen,
    Window,
}

struct ScreenshotPopup {
    screen: [u8; 0],
    mouse_position: Option<(u32, u32)>,
}

fn main() {
    // let mut window_manager = WindowManager::Wayland(wayland::init::create());
    let window_manager_test = WindowManagerTest::Wayland(wayland::init());
    let screenshot_type = ScreenshotType::Fullscreen;

    let mut backing_memory = tempfile::tempfile().unwrap();
    let a = match window_manager_test {
        WindowManagerTest::Wayland(vars) => screenshot(vars, &mut backing_memory),
    };

    let memory = unsafe { Mmap::map(&backing_memory).unwrap() };
    match screenshot_type {
        ScreenshotType::Fullscreen => {
            let picture_path = dirs::home_dir().unwrap().join("Pictures");

            let mut image_num = 0;
            a.iter().for_each(|b| {
                let screen_buf = &memory[b.offset..b.span];
                save_image(
                    b.screen_data.resolution,
                    screen_buf,
                    &picture_path.join(format!("output{}", image_num)),
                );
                image_num += 1;
            });
        }
        ScreenshotType::Window => todo!(),
    }

    // let screens = match window_manager {
    //     WindowManager::Wayland(ref mut vars) => wayland::get_screencopy::get_screen_data(vars),
    // };

    // match window_manager {
    //     WindowManager::Wayland(ref mut vars) => screen_shot_overlay(vars, screens),
    // }
}

fn save_image((width, height): (i32, i32), pixels: &[u8], path: &Path) {
    let img = image::ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
        let index = (y * width as u32 + x) as usize * 4;

        let r = pixels[index];
        let g = pixels[index + 1];
        let b = pixels[index + 2];

        image::Rgb([r, g, b])
    });

    //println!("{:?}", img);

    println!("Compressing");
    img.save_with_format(path, ImageFormat::Png)
        .expect("Failed to save image");
}
