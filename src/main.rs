mod core;
mod ui;

fn main() -> Result<(), slint::PlatformError> {
    let app = ui::App::new()?;
    app.run()
}
