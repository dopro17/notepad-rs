//! Definição dos menus (tabela única), usada tanto pelo menu nativo `muda`
//! quanto pela barra de menus desenhada em egui (fallback).

use anyhow::Result;
use egui::{Key, KeyboardShortcut, Modifiers};
use muda::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    New,
    Open,
    Save,
    SaveAs,
    CloseTab,
    Quit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Find,
    FindNext,
    FindPrev,
    Replace,
    GoToLine,
    InsertDateTime,
    ToggleWrap,
    Font,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ToggleStatusBar,
    Preferences,
    About,
}

/// Item de um menu da tabela.
pub enum Entry {
    Item(Action),
    Check(Action),
    Separator,
    /// Itens que só aparecem fora do macOS (lá ficam no menu do app).
    NonMac(Action),
}

use Entry::{Check, Item, NonMac, Separator};

pub const MENUS: &[(&str, &[Entry])] = &[
    (
        "Arquivo",
        &[
            Item(Action::New),
            Item(Action::Open),
            Item(Action::Save),
            Item(Action::SaveAs),
            Separator,
            Item(Action::CloseTab),
            NonMac(Action::Quit),
        ],
    ),
    (
        "Editar",
        &[
            Item(Action::Undo),
            Item(Action::Redo),
            Separator,
            Item(Action::Cut),
            Item(Action::Copy),
            Item(Action::Paste),
            Separator,
            Item(Action::Find),
            Item(Action::FindNext),
            Item(Action::FindPrev),
            Item(Action::Replace),
            Item(Action::GoToLine),
            Separator,
            Item(Action::SelectAll),
            Item(Action::InsertDateTime),
        ],
    ),
    ("Formatar", &[Check(Action::ToggleWrap), Item(Action::Font)]),
    (
        "Exibir",
        &[
            Item(Action::ZoomIn),
            Item(Action::ZoomOut),
            Item(Action::ZoomReset),
            Separator,
            Check(Action::ToggleStatusBar),
        ],
    ),
    ("Ajuda", &[NonMac(Action::About)]),
];

impl Action {
    pub const ALL: [Action; 26] = [
        Action::New,
        Action::Open,
        Action::Save,
        Action::SaveAs,
        Action::CloseTab,
        Action::Quit,
        Action::Undo,
        Action::Redo,
        Action::Cut,
        Action::Copy,
        Action::Paste,
        Action::SelectAll,
        Action::Find,
        Action::FindNext,
        Action::FindPrev,
        Action::Replace,
        Action::GoToLine,
        Action::InsertDateTime,
        Action::ToggleWrap,
        Action::Font,
        Action::ZoomIn,
        Action::ZoomOut,
        Action::ZoomReset,
        Action::ToggleStatusBar,
        Action::Preferences,
        Action::About,
    ];

    pub fn id(self) -> String {
        format!("{self:?}")
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            Action::New => "Novo",
            Action::Open => "Abrir…",
            Action::Save => "Salvar",
            Action::SaveAs => "Salvar como…",
            Action::CloseTab => "Fechar aba",
            Action::Quit => "Sair",
            Action::Undo => "Desfazer",
            Action::Redo => "Refazer",
            Action::Cut => "Recortar",
            Action::Copy => "Copiar",
            Action::Paste => "Colar",
            Action::SelectAll => "Selecionar tudo",
            Action::Find => "Localizar…",
            Action::FindNext => "Localizar próxima",
            Action::FindPrev => "Localizar anterior",
            Action::Replace => "Substituir…",
            Action::GoToLine => "Ir para linha…",
            Action::InsertDateTime => "Hora/Data",
            Action::ToggleWrap => "Quebra automática de linha",
            Action::Font => "Fonte…",
            Action::ZoomIn => "Aumentar zoom",
            Action::ZoomOut => "Diminuir zoom",
            Action::ZoomReset => "Restaurar zoom padrão",
            Action::ToggleStatusBar => "Barra de status",
            Action::Preferences => "Preferências…",
            Action::About => "Sobre o notepad-rs",
        }
    }

    /// (modificadores egui, tecla egui, acelerador `muda`).
    fn binding(self) -> Option<(Modifiers, Key, &'static str)> {
        let cmd = Modifiers::COMMAND;
        let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
        let cmd_alt = Modifiers::COMMAND | Modifiers::ALT;
        Some(match self {
            Action::New => (cmd, Key::N, "CmdOrCtrl+N"),
            Action::Open => (cmd, Key::O, "CmdOrCtrl+O"),
            Action::Save => (cmd, Key::S, "CmdOrCtrl+S"),
            Action::SaveAs => (cmd_shift, Key::S, "Shift+CmdOrCtrl+S"),
            Action::CloseTab => (cmd, Key::W, "CmdOrCtrl+W"),
            Action::Quit => (cmd, Key::Q, "CmdOrCtrl+Q"),
            Action::Undo => (cmd, Key::Z, "CmdOrCtrl+Z"),
            Action::Redo => (cmd_shift, Key::Z, "Shift+CmdOrCtrl+Z"),
            Action::Cut => (cmd, Key::X, "CmdOrCtrl+X"),
            Action::Copy => (cmd, Key::C, "CmdOrCtrl+C"),
            Action::Paste => (cmd, Key::V, "CmdOrCtrl+V"),
            Action::SelectAll => (cmd, Key::A, "CmdOrCtrl+A"),
            Action::Find => (cmd, Key::F, "CmdOrCtrl+F"),
            Action::FindNext => (cmd, Key::G, "CmdOrCtrl+G"),
            Action::FindPrev => (cmd_shift, Key::G, "Shift+CmdOrCtrl+G"),
            Action::Replace => (cmd_alt, Key::F, "Alt+CmdOrCtrl+F"),
            Action::GoToLine => (cmd, Key::L, "CmdOrCtrl+L"),
            Action::InsertDateTime => (Modifiers::NONE, Key::F5, "F5"),
            Action::ZoomIn => (cmd, Key::Equals, "CmdOrCtrl+="),
            Action::ZoomOut => (cmd, Key::Minus, "CmdOrCtrl+-"),
            Action::ZoomReset => (cmd, Key::Num0, "CmdOrCtrl+0"),
            Action::Preferences => (cmd, Key::Comma, "CmdOrCtrl+,"),
            Action::ToggleWrap | Action::Font | Action::ToggleStatusBar | Action::About => {
                return None;
            }
        })
    }

    /// Ações de edição que o `TextEdit` do egui já trata pelo teclado.
    fn handled_by_text_edit(self) -> bool {
        matches!(
            self,
            Action::Undo
                | Action::Redo
                | Action::Cut
                | Action::Copy
                | Action::Paste
                | Action::SelectAll
        )
    }

    pub fn shortcut_text(self, ctx: &egui::Context) -> String {
        self.binding()
            .map(|(m, k, _)| ctx.format_shortcut(&KeyboardShortcut::new(m, k)))
            .unwrap_or_default()
    }
}

/// Consome atalhos de teclado que chegaram ao egui (quando o menu nativo não
/// os interceptou, ex.: Windows/Linux). Atalhos com mais modificadores primeiro.
pub fn consume_shortcuts(ctx: &egui::Context) -> Vec<Action> {
    let mut bound: Vec<(Action, KeyboardShortcut)> = Action::ALL
        .into_iter()
        .filter(|a| !a.handled_by_text_edit())
        .filter_map(|a| {
            a.binding()
                .map(|(m, k, _)| (a, KeyboardShortcut::new(m, k)))
        })
        .collect();
    bound.sort_by_key(|(_, s)| {
        std::cmp::Reverse(u8::from(s.modifiers.shift) + u8::from(s.modifiers.alt))
    });
    let mut actions = Vec::new();
    ctx.input_mut(|i| {
        for (action, shortcut) in &bound {
            if i.consume_shortcut(shortcut) {
                actions.push(*action);
            }
        }
        // ⌘+ também com a tecla "+" (teclados sem "=" dedicado).
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Plus)) {
            actions.push(Action::ZoomIn);
        }
    });
    actions
}

pub struct NativeMenu {
    pub menu: Menu,
    word_wrap: CheckMenuItem,
    status_bar: CheckMenuItem,
}

impl NativeMenu {
    /// Monta a barra nativa. `app_menu` (macOS) é inserido como primeiro menu.
    pub fn build(app_menu: Option<Submenu>, is_mac: bool) -> Result<Self> {
        let menu = Menu::new();
        if let Some(app_menu) = app_menu {
            menu.append(&app_menu)?;
        }
        let mut word_wrap = None;
        let mut status_bar = None;
        for (title, entries) in MENUS {
            let sub = Submenu::new(*title, true);
            for entry in *entries {
                match entry {
                    Separator => sub.append(&PredefinedMenuItem::separator())?,
                    Item(a) => sub.append(&native_item(*a))?,
                    NonMac(a) if !is_mac => sub.append(&native_item(*a))?,
                    NonMac(_) => {}
                    Check(a) => {
                        let item = CheckMenuItem::with_id(a.id(), a.label(), true, false, None);
                        sub.append(&item)?;
                        match a {
                            Action::ToggleWrap => word_wrap = Some(item),
                            _ => status_bar = Some(item),
                        }
                    }
                }
            }
            if sub.items().is_empty() {
                continue;
            }
            menu.append(&sub)?;
        }
        let missing = || anyhow::anyhow!("item de menu ausente");
        Ok(Self {
            menu,
            word_wrap: word_wrap.ok_or_else(missing)?,
            status_bar: status_bar.ok_or_else(missing)?,
        })
    }

    pub fn sync_checks(&self, word_wrap: bool, status_bar: bool) {
        if self.word_wrap.is_checked() != word_wrap {
            self.word_wrap.set_checked(word_wrap);
        }
        if self.status_bar.is_checked() != status_bar {
            self.status_bar.set_checked(status_bar);
        }
    }
}

pub fn native_item(action: Action) -> MenuItem {
    let accel = action.binding().and_then(|(_, _, a)| a.parse().ok());
    MenuItem::with_id(action.id(), action.label(), true, accel)
}

/// Barra de menus dentro da janela (Linux e fallback).
pub fn egui_menu_bar(ui: &mut egui::Ui, word_wrap: bool, status_bar: bool) -> Option<Action> {
    let mut clicked = None;
    egui::MenuBar::new().ui(ui, |ui| {
        for (title, entries) in MENUS {
            ui.menu_button(*title, |ui| {
                for entry in *entries {
                    let action = match entry {
                        Separator => {
                            ui.separator();
                            continue;
                        }
                        Item(a) | NonMac(a) | Check(a) => *a,
                    };
                    let mut label = action.label().to_owned();
                    if let Check(a) = entry {
                        let on = if *a == Action::ToggleWrap {
                            word_wrap
                        } else {
                            status_bar
                        };
                        label = format!("{} {label}", if on { "✔" } else { "   " });
                    }
                    let shortcut = action.shortcut_text(ui.ctx());
                    if ui
                        .add(egui::Button::new(label).shortcut_text(shortcut))
                        .clicked()
                    {
                        clicked = Some(action);
                        ui.close();
                    }
                }
            });
        }
    });
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_roundtrip() {
        for a in Action::ALL {
            assert_eq!(Action::from_id(&a.id()), Some(a));
        }
    }

    #[test]
    fn accelerators_parse() {
        for a in Action::ALL {
            if let Some((_, _, accel)) = a.binding() {
                assert!(
                    accel.parse::<muda::accelerator::Accelerator>().is_ok(),
                    "{accel}"
                );
            }
        }
    }
}
