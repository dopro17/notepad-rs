//! Utilitário de desenvolvimento (feature `screenshot`): executa ações listadas
//! em `NOTEPAD_RS_ACTIONS` (ex.: "Find;GoToLine") e salva um PNG da janela em
//! `NOTEPAD_RS_SCREENSHOT`, encerrando o processo em seguida.

use crate::menu::Action;

#[derive(Default)]
pub struct Debug {
    frame: u32,
}

impl Debug {
    /// Ações a executar neste frame.
    pub fn tick(&mut self, ctx: &egui::Context) -> Vec<Action> {
        self.frame += 1;
        ctx.request_repaint();
        let mut actions = Vec::new();
        if self.frame == 5
            && let Ok(list) = std::env::var("NOTEPAD_RS_ACTIONS")
        {
            actions.extend(list.split(';').filter_map(Action::from_id));
        }
        if self.frame == 20
            && let Ok(path) = std::env::var("NOTEPAD_RS_SCREENSHOT")
        {
            ctx.request_screenshot(move |img| {
                let [w, h] = img.size;
                let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                let result = image::RgbaImage::from_raw(w as u32, h as u32, bytes)
                    .map(|buf| buf.save(&path));
                eprintln!("screenshot {path}: {result:?}");
                std::process::exit(0);
            });
        }
        actions
    }
}
