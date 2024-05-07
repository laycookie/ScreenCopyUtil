use wayland::init::{screen_shot_overlay, WaylandVars};
pub mod wayland;

mod test;
mod wayland_data;
mod wayland_fractional_scale;
pub mod wayland_screencopy;

enum WindowManager {
    Wayland(WaylandVars),
}

fn main() {
    let mut window_manager = WindowManager::Wayland(wayland::init::create());

    let screens = match window_manager {
        WindowManager::Wayland(ref mut vars) => wayland::get_screencopy::get_screen_data(vars),
    };

    match window_manager {
        WindowManager::Wayland(ref mut vars) => screen_shot_overlay(vars, screens),
    }
}
