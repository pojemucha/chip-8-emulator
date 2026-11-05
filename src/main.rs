mod core;
mod ui;

fn main() -> Result<(), slint::PlatformError> {
    // winit 0.30 does not support external drag-and-drop on native Wayland,
    // so fall back to X11 (via XWayland) when running on a Wayland session.
    // This can be removed once the winit dependency is upgraded past 0.30.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some()
        && std::env::var_os("WINIT_UNIX_BACKEND").is_none()
    {
        // SAFETY: called before any threads are spawned, so no data race is possible.
        unsafe { std::env::set_var("WINIT_UNIX_BACKEND", "x11") };
    }

    let app = ui::App::new()?;
    app.run()
}
