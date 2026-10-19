//! Código específico de SO. O restante do app é agnóstico.

use crate::document::LineEnding;
use crate::menu::NativeMenu;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
use linux as imp;
#[cfg(target_os = "macos")]
use macos as imp;
#[cfg(target_os = "windows")]
use windows as imp;

/// Fim de linha padrão para documentos novos.
pub fn default_eol() -> LineEnding {
    imp::DEFAULT_EOL
}

/// Fonte monoespaçada do sistema (bytes + índice na coleção), se disponível.
pub fn monospace_font() -> Option<egui::FontData> {
    imp::MONOSPACE_FONTS.iter().find_map(|(path, index)| {
        let bytes = std::fs::read(path).ok()?;
        Some(egui::FontData::from_blob(
            std::sync::Arc::new(bytes),
            *index,
        ))
    })
}

/// Instala o menu nativo. `None` = usar a barra de menus desenhada em egui.
pub fn install_menu(cc: &eframe::CreationContext<'_>) -> anyhow::Result<Option<NativeMenu>> {
    imp::install_menu(cc)
}

/// Arquivos que o sistema pediu para abrir ("Abrir com", Dock) desde a última chamada.
pub fn take_open_requests() -> Vec<std::path::PathBuf> {
    imp::take_open_requests()
}

/// Inicialização que precisa ocorrer antes de o loop de eventos iniciar.
pub fn early_init() {
    imp::early_init();
}
