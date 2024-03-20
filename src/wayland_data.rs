use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::xdg::window::Window;
use smithay_client_toolkit::shell::xdg::XdgShell;
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};

struct State {
    registry_state: RegistryState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm_state: Shm,
    xdg_shell_state: XdgShell,

    pool: Option<SlotPool>,
    windows: Vec<ScreenShotWindow>,
}

struct ScreenShotWindow {
    window: Window,
    screenshot: Option<Buffer>,
    width: u32,
    height: u32,
    first_configure: bool,
    damaged: bool,
}