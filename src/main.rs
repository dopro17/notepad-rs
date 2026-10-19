mod app;
#[cfg(feature = "screenshot")]
mod debug;
mod document;
mod menu;
mod platform;
mod search;
mod settings;

use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let files: Vec<PathBuf> = std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .collect();
    let size = settings::Settings::load()
        .window_size
        .unwrap_or([900.0, 640.0]);

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("notepad-rs")
        .with_inner_size(size)
        .with_min_inner_size([320.0, 200.0])
        .with_drag_and_drop(true);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    platform::early_init();
    eframe::run_native(
        "notepad-rs",
        options,
        Box::new(|cc| Ok(Box::new(app::NotepadApp::new(cc, files)))),
    )
    .map_err(|e| anyhow::anyhow!("Falha ao iniciar a interface: {e}"))
}
