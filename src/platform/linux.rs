use anyhow::Result;

use crate::document::LineEnding;
use crate::menu::NativeMenu;

pub const DEFAULT_EOL: LineEnding = LineEnding::Lf;

pub const MONOSPACE_FONTS: &[(&str, u32)] = &[
    ("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", 0),
    ("/usr/share/fonts/TTF/DejaVuSansMono.ttf", 0),
    ("/usr/share/fonts/dejavu/DejaVuSansMono.ttf", 0),
];

/// O winit não expõe a janela GTK que o `muda` exige; usa-se o menu em egui.
pub fn install_menu(_cc: &eframe::CreationContext<'_>) -> Result<Option<NativeMenu>> {
    Ok(None)
}

/// Pedidos de abertura vindos do sistema (só usado no macOS).
pub fn take_open_requests() -> Vec<std::path::PathBuf> {
    Vec::new()
}

pub fn early_init() {}
