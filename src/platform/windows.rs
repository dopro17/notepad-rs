use anyhow::{Context, Result};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::document::LineEnding;
use crate::menu::NativeMenu;

pub const DEFAULT_EOL: LineEnding = LineEnding::CrLf;

pub const MONOSPACE_FONTS: &[(&str, u32)] = &[(r"C:\Windows\Fonts\consola.ttf", 0)];

pub fn install_menu(cc: &eframe::CreationContext<'_>) -> Result<Option<NativeMenu>> {
    let native = NativeMenu::build(None, false)?;
    let handle = cc.window_handle().context("janela sem handle")?;
    if let RawWindowHandle::Win32(h) = handle.as_raw() {
        // SAFETY: HWND válido obtido da janela principal do winit.
        unsafe { native.menu.init_for_hwnd(h.hwnd.get())? };
        Ok(Some(native))
    } else {
        Ok(None)
    }
}

/// Pedidos de abertura vindos do sistema (só usado no macOS).
pub fn take_open_requests() -> Vec<std::path::PathBuf> {
    Vec::new()
}

pub fn early_init() {}
