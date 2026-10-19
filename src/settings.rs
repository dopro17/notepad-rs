//! Configurações persistentes (JSON no diretório de config do usuário).

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontKind {
    Monospace,
    Proportional,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub word_wrap: bool,
    pub show_status_bar: bool,
    pub font_kind: FontKind,
    pub font_size: f32,
    /// Fator de zoom (1.0 = 100%).
    pub zoom: f32,
    pub window_size: Option<[f32; 2]>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            word_wrap: true,
            show_status_bar: true,
            font_kind: FontKind::Monospace,
            font_size: 14.0,
            zoom: 1.0,
            window_size: None,
        }
    }
}

pub const MIN_ZOOM: f32 = 0.1;
pub const MAX_ZOOM: f32 = 5.0;

impl Settings {
    fn path() -> Option<PathBuf> {
        directories::ProjectDirs::from("br.com", "prodocimo", "notepad-rs")
            .map(|d| d.config_dir().join("settings.json"))
    }

    /// Carrega do disco; em caso de falha usa o padrão.
    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path().context("Diretório de configuração indisponível")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(self)?)
            .with_context(|| format!("Não foi possível salvar {}", path.display()))
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    }

    pub fn effective_font_size(&self) -> f32 {
        (self.font_size * self.zoom).max(4.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_json_and_defaults_for_missing_fields() -> Result<()> {
        let s: Settings = serde_json::from_str(r#"{"word_wrap": false}"#)?;
        assert!(!s.word_wrap);
        assert_eq!(s.font_size, 14.0);
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s)?)?;
        assert!(!back.word_wrap);
        Ok(())
    }

    #[test]
    fn zoom_is_clamped() {
        let mut s = Settings::default();
        s.set_zoom(100.0);
        assert_eq!(s.zoom, MAX_ZOOM);
        s.set_zoom(0.0);
        assert_eq!(s.zoom, MIN_ZOOM);
    }
}
