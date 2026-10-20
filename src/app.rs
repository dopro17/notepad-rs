//! Interface (egui): abas, editor, diálogos, barra de status.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::{Align, FontFamily, FontId, Id, ViewportCommand};

use crate::document::{Document, Encoding, LineEnding};
use crate::menu::{self, Action, NativeMenu};
use crate::platform;
use crate::search::{self, SearchOptions};
use crate::settings::{FontKind, Settings};

const APP_NAME: &str = "notepad-rs";
const SYSTEM_MONO: &str = "system-mono";

struct Tab {
    doc: Document,
    edit_id: Id,
    /// Posição do cursor (índice de caractere) no último frame.
    cursor: usize,
}

enum Modal {
    ConfirmClose(usize),
    GoToLine { input: String, error: bool },
    Font,
    About,
    Error(String),
}

#[derive(Default)]
struct FindState {
    open: bool,
    replace_mode: bool,
    query: String,
    replacement: String,
    case_sensitive: bool,
    wrap_around: bool,
    message: String,
    focus_query: bool,
}

pub struct NotepadApp {
    tabs: Vec<Tab>,
    active: usize,
    next_tab_id: u64,
    settings: Settings,
    saved_settings: Settings,
    native_menu: Option<NativeMenu>,
    menu_events: Option<Receiver<muda::MenuEvent>>,
    modal: Option<Modal>,
    find: FindState,
    pending_quit: bool,
    allow_close: bool,
    focus_editor: bool,
    scroll_to_cursor: bool,
    /// Posiciona o cursor no início do documento recém-aberto.
    reset_cursor: bool,
    title: String,
    #[cfg(feature = "screenshot")]
    debug: crate::debug::Debug,
}

impl NotepadApp {
    pub fn new(cc: &eframe::CreationContext<'_>, files: Vec<PathBuf>) -> Self {
        install_fonts(&cc.egui_ctx);
        // ⌘+/⌘- controlam só o zoom do editor, não a UI inteira.
        cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);

        let mut app = Self {
            tabs: Vec::new(),
            active: 0,
            next_tab_id: 0,
            settings: Settings::load(),
            saved_settings: Settings::default(),
            native_menu: None,
            menu_events: None,
            modal: None,
            find: FindState {
                wrap_around: true,
                ..Default::default()
            },
            pending_quit: false,
            allow_close: false,
            focus_editor: true,
            scroll_to_cursor: false,
            reset_cursor: false,
            title: String::new(),
            #[cfg(feature = "screenshot")]
            debug: Default::default(),
        };
        app.saved_settings = app.settings.clone();

        match platform::install_menu(cc) {
            Ok(Some(native)) => {
                let (tx, rx) = channel();
                let ctx = cc.egui_ctx.clone();
                muda::MenuEvent::set_event_handler(Some(move |event| {
                    let _ = tx.send(event);
                    ctx.request_repaint();
                }));
                app.native_menu = Some(native);
                app.menu_events = Some(rx);
            }
            Ok(None) => {}
            Err(e) => app.modal = Some(Modal::Error(format!("Menu nativo indisponível: {e:#}"))),
        }

        for path in files {
            app.open_path(&path);
        }
        if app.tabs.is_empty() {
            app.new_tab(Document::new(platform::default_eol()));
        }
        app
    }

    // ----------------------------------------------------------------- abas

    fn new_tab(&mut self, doc: Document) {
        self.next_tab_id += 1;
        self.tabs.push(Tab {
            doc,
            edit_id: Id::unique(("editor", self.next_tab_id)),
            cursor: 0,
        });
        self.active = self.tabs.len() - 1;
        self.focus_editor = true;
        self.reset_cursor = true;
    }

    fn tab(&self) -> &Tab {
        &self.tabs[self.active]
    }

    fn tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active]
    }

    fn open_path(&mut self, path: &Path) {
        if let Some(i) = self
            .tabs
            .iter()
            .position(|t| t.doc.path.as_deref() == Some(path))
        {
            self.active = i;
            self.focus_editor = true;
            return;
        }
        match Document::open(path, platform::default_eol()) {
            Ok(doc) => {
                let reuse = !self.tabs.is_empty() && {
                    let t = self.tab();
                    t.doc.path.is_none() && !t.doc.modified && t.doc.text.is_empty()
                };
                if reuse {
                    self.tab_mut().doc = doc;
                    self.tab_mut().cursor = 0;
                    self.focus_editor = true;
                    self.reset_cursor = true;
                } else {
                    self.new_tab(doc);
                }
            }
            Err(e) => self.modal = Some(Modal::Error(format!("{e:#}"))),
        }
    }

    fn open_dialog(&mut self) {
        if let Some(paths) = rfd::FileDialog::new().pick_files() {
            for path in paths {
                self.open_path(&path);
            }
        }
    }

    /// Salva a aba `i`. Retorna `true` se o arquivo foi gravado.
    fn save_tab(&mut self, i: usize, save_as: bool) -> bool {
        let doc = &mut self.tabs[i].doc;
        let path = match (&doc.path, save_as) {
            (Some(p), false) => p.clone(),
            _ => {
                let name = match &doc.path {
                    Some(_) => doc.file_name(),
                    None => "Sem título.txt".to_owned(),
                };
                match rfd::FileDialog::new().set_file_name(name).save_file() {
                    Some(p) => p,
                    None => return false,
                }
            }
        };
        match doc.save_as(&path) {
            Ok(()) => true,
            Err(e) => {
                self.modal = Some(Modal::Error(format!("{e:#}")));
                false
            }
        }
    }

    fn request_close_tab(&mut self, i: usize) {
        if self.tabs[i].doc.modified {
            self.active = i;
            self.modal = Some(Modal::ConfirmClose(i));
        } else {
            self.remove_tab(i);
        }
    }

    fn remove_tab(&mut self, i: usize) {
        self.tabs.remove(i);
        if self.tabs.is_empty() {
            // Como no Bloco de Notas: fechar a última aba fecha a janela.
            self.new_tab(Document::new(platform::default_eol()));
            self.pending_quit = true;
            self.continue_quit();
            return;
        }
        if self.active >= self.tabs.len() || self.active > i {
            self.active = self.active.saturating_sub(1);
        }
        self.focus_editor = true;
    }

    /// Pergunta sobre cada aba modificada; quando não restar nenhuma, fecha.
    fn continue_quit(&mut self) {
        if let Some(i) = self.tabs.iter().position(|t| t.doc.modified) {
            self.active = i;
            self.modal = Some(Modal::ConfirmClose(i));
        } else {
            self.allow_close = true;
        }
    }

    // --------------------------------------------------------------- seleção

    fn selection(&self, ctx: &egui::Context) -> (usize, usize) {
        let tab = self.tab();
        TextEditState::load(ctx, tab.edit_id)
            .and_then(|s| s.cursor.char_range())
            .map(|r| {
                let (a, b) = (r.primary.index.0, r.secondary.index.0);
                (a.min(b), a.max(b))
            })
            .unwrap_or((tab.cursor, tab.cursor))
    }

    fn set_selection(&mut self, ctx: &egui::Context, start: usize, end: usize) {
        let id = self.tab().edit_id;
        let mut state = TextEditState::load(ctx, id).unwrap_or_default();
        state.cursor.set_char_range(Some(CCursorRange::two(
            CCursor::new(start),
            CCursor::new(end),
        )));
        state.store(ctx, id);
        self.tab_mut().cursor = end;
        self.scroll_to_cursor = true;
        self.focus_editor = true;
    }

    fn selected_text(&self, ctx: &egui::Context) -> String {
        let (s, e) = self.selection(ctx);
        self.tab().doc.text.chars().skip(s).take(e - s).collect()
    }

    fn replace_selection(&mut self, ctx: &egui::Context, replacement: &str) {
        let (s, e) = self.selection(ctx);
        let doc = &mut self.tab_mut().doc;
        search::replace_range(&mut doc.text, s, e, replacement);
        doc.modified = true;
        let end = s + replacement.chars().count();
        self.set_selection(ctx, end, end);
    }

    fn undo_redo(&mut self, ctx: &egui::Context, redo: bool) {
        let id = self.tab().edit_id;
        let Some(mut state) = TextEditState::load(ctx, id) else {
            return;
        };
        let mut undoer = state.undoer();
        let current = (
            state.cursor.char_range().unwrap_or_default(),
            self.tab().doc.text.clone(),
        );
        let target = if redo {
            undoer.redo(&current)
        } else {
            undoer.undo(&current)
        }
        .cloned();
        if let Some((range, text)) = target {
            let tab = self.tab_mut();
            tab.doc.text = text;
            tab.doc.modified = true;
            tab.cursor = range.primary.index.0;
            state.cursor.set_char_range(Some(range));
            self.scroll_to_cursor = true;
        }
        state.set_undoer(undoer);
        state.store(ctx, id);
        self.focus_editor = true;
    }

    // ----------------------------------------------------------------- busca

    fn open_find(&mut self, ctx: &egui::Context, replace_mode: bool) {
        let selected = self.selected_text(ctx);
        if !selected.is_empty() && !selected.contains('\n') {
            self.find.query = selected;
        }
        self.find.open = true;
        self.find.replace_mode = replace_mode;
        self.find.focus_query = true;
        self.find.message.clear();
    }

    fn search_options(&self) -> SearchOptions {
        SearchOptions {
            case_sensitive: self.find.case_sensitive,
            wrap_around: self.find.wrap_around,
        }
    }

    fn find_next(&mut self, ctx: &egui::Context, forward: bool) {
        if self.find.query.is_empty() {
            self.open_find(ctx, false);
            return;
        }
        let (s, e) = self.selection(ctx);
        let from = if forward { e } else { s };
        let opts = self.search_options();
        match search::find(&self.tab().doc.text, &self.find.query, from, forward, opts) {
            Some((a, b)) => {
                self.find.message.clear();
                if forward {
                    self.set_selection(ctx, a, b);
                } else {
                    self.set_selection(ctx, b, a);
                }
            }
            None => {
                self.find.message = format!("Não foi possível encontrar \"{}\"", self.find.query);
            }
        }
    }

    fn replace_one(&mut self, ctx: &egui::Context) {
        let (s, e) = self.selection(ctx);
        let text = &self.tab().doc.text;
        if search::range_matches(text, s, e, &self.find.query, self.find.case_sensitive) {
            let replacement = self.find.replacement.clone();
            self.replace_selection(ctx, &replacement);
        }
        self.find_next(ctx, true);
    }

    fn replace_all(&mut self, ctx: &egui::Context) {
        let (text, count) = search::replace_all(
            &self.tab().doc.text,
            &self.find.query,
            &self.find.replacement,
            self.find.case_sensitive,
        );
        if count > 0 {
            let doc = &mut self.tab_mut().doc;
            doc.text = text;
            doc.modified = true;
            self.set_selection(ctx, 0, 0);
        }
        self.find.message = format!("{count} ocorrência(s) substituída(s)");
    }

    // ---------------------------------------------------------------- ações

    fn run(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::New => self.new_tab(Document::new(platform::default_eol())),
            Action::Open => self.open_dialog(),
            Action::Save => {
                self.save_tab(self.active, false);
            }
            Action::SaveAs => {
                self.save_tab(self.active, true);
            }
            Action::CloseTab => self.request_close_tab(self.active),
            Action::Quit => {
                self.pending_quit = true;
                self.continue_quit();
            }
            Action::Undo => self.undo_redo(ctx, false),
            Action::Redo => self.undo_redo(ctx, true),
            Action::Cut => {
                self.focus_editor = true;
                ctx.send_viewport_cmd(ViewportCommand::RequestCut);
            }
            Action::Copy => {
                self.focus_editor = true;
                ctx.send_viewport_cmd(ViewportCommand::RequestCopy);
            }
            Action::Paste => {
                self.focus_editor = true;
                ctx.send_viewport_cmd(ViewportCommand::RequestPaste);
            }
            Action::SelectAll => {
                let len = self.tab().doc.text.chars().count();
                self.set_selection(ctx, 0, len);
                self.scroll_to_cursor = false;
            }
            Action::Find => self.open_find(ctx, false),
            Action::Replace => self.open_find(ctx, true),
            Action::FindNext => self.find_next(ctx, true),
            Action::FindPrev => self.find_next(ctx, false),
            Action::GoToLine => {
                let (line, _) = self.tab().doc.line_col(self.tab().cursor);
                self.modal = Some(Modal::GoToLine {
                    input: line.to_string(),
                    error: false,
                });
            }
            Action::InsertDateTime => {
                let now = chrono::Local::now().format("%H:%M %d/%m/%Y").to_string();
                self.replace_selection(ctx, &now);
            }
            Action::ToggleWrap => self.settings.word_wrap = !self.settings.word_wrap,
            Action::ToggleStatusBar => {
                self.settings.show_status_bar = !self.settings.show_status_bar;
            }
            Action::Font | Action::Preferences => self.modal = Some(Modal::Font),
            Action::ZoomIn => self.step_zoom(0.1),
            Action::ZoomOut => self.step_zoom(-0.1),
            Action::ZoomReset => self.settings.set_zoom(1.0),
            Action::About => self.modal = Some(Modal::About),
        }
    }

    fn step_zoom(&mut self, delta: f32) {
        let z = ((self.settings.zoom + delta) * 10.0).round() / 10.0;
        self.settings.set_zoom(z);
    }

    fn collect_actions(&mut self, ui: &mut egui::Ui) -> Vec<Action> {
        let mut actions = Vec::new();
        if let Some(rx) = &self.menu_events {
            actions.extend(
                rx.try_iter()
                    .filter_map(|e| Action::from_id(e.id().as_ref())),
            );
        }
        // Sem modal aberto, atalhos que chegaram ao egui viram ações.
        if self.modal.is_none() {
            actions.extend(menu::consume_shortcuts(ui.ctx()));
        }
        actions
    }

    // ------------------------------------------------------------------- UI

    fn editor_font(&self) -> FontId {
        let family = match self.settings.font_kind {
            FontKind::Monospace => FontFamily::Monospace,
            FontKind::Proportional => FontFamily::Proportional,
        };
        FontId::new(self.settings.effective_font_size(), family)
    }

    fn tab_bar(&mut self, ui: &mut egui::Ui) -> Vec<Action> {
        let mut actions = Vec::new();
        let mut close = None;
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, tab) in self.tabs.iter().enumerate() {
                    let name = format!(
                        "{}{}",
                        tab.doc.file_name(),
                        if tab.doc.modified { " •" } else { "" }
                    );
                    let resp = ui.selectable_label(i == self.active, name).on_hover_text(
                        tab.doc
                            .path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default(),
                    );
                    if resp.clicked() {
                        self.active = i;
                        self.focus_editor = true;
                    }
                    if resp.middle_clicked() || ui.small_button("×").clicked() {
                        close = Some(i);
                    }
                    ui.separator();
                }
                if ui.small_button("+").on_hover_text("Nova aba").clicked() {
                    actions.push(Action::New);
                }
            });
        });
        if let Some(i) = close {
            self.request_close_tab(i);
        }
        actions
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            let doc = &mut self.tabs[self.active].doc;
            ui.menu_button(doc.encoding.to_string(), |ui| {
                for enc in Encoding::ALL {
                    if ui.radio(doc.encoding == enc, enc.to_string()).clicked()
                        && doc.encoding != enc
                    {
                        doc.encoding = enc;
                        doc.modified = true;
                        ui.close();
                    }
                }
            });
            ui.separator();
            ui.menu_button(doc.eol.to_string(), |ui| {
                for eol in LineEnding::ALL {
                    if ui.radio(doc.eol == eol, eol.to_string()).clicked() && doc.eol != eol {
                        doc.eol = eol;
                        doc.modified = true;
                        ui.close();
                    }
                }
            });
            ui.separator();
            ui.label(format!("{:.0}%", self.settings.zoom * 100.0));
            ui.separator();
            let tab = &self.tabs[self.active];
            let (line, col) = tab.doc.line_col(tab.cursor);
            ui.label(format!("Ln {line}, Col {col}"));
            ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                let n = tab.doc.line_count();
                ui.label(format!("{n} {}", if n == 1 { "linha" } else { "linhas" }));
            });
        });
    }

    fn editor(&mut self, ui: &mut egui::Ui) {
        let font = self.editor_font();
        let wrap = self.settings.word_wrap;
        let focus = std::mem::take(&mut self.focus_editor) && self.modal.is_none();
        let scroll_to_cursor = std::mem::take(&mut self.scroll_to_cursor);
        let tab = &mut self.tabs[self.active];

        // ⌘/Ctrl + roda do mouse (ou pinça) ajusta o zoom.
        let zoom_delta = ui.input(|i| i.zoom_delta());
        if zoom_delta != 1.0 && ui.ui_contains_pointer() {
            let z = self.settings.zoom * zoom_delta;
            self.settings.set_zoom(z);
        }

        let scroll = if wrap {
            egui::ScrollArea::vertical()
        } else {
            egui::ScrollArea::both()
        };
        scroll
            .id_salt(tab.edit_id)
            .auto_shrink(false)
            .show(ui, |ui| {
                let width = if wrap {
                    ui.available_width()
                } else {
                    f32::INFINITY
                };
                let output = egui::TextEdit::multiline(&mut tab.doc.text)
                    .id(tab.edit_id)
                    .font(font)
                    .frame(egui::Frame::NONE.inner_margin(6.0))
                    .desired_width(width)
                    .min_size(ui.available_size())
                    .lock_focus(true)
                    .show(ui);
                let response = &output.response.response;
                if response.changed() {
                    tab.doc.modified = true;
                }
                if focus {
                    response.request_focus();
                }
                if let Some(range) = output.cursor_range {
                    tab.cursor = range.primary.index.0;
                    if scroll_to_cursor {
                        let rect = output
                            .galley
                            .pos_from_cursor(range.primary)
                            .translate(output.galley_pos.to_vec2());
                        ui.scroll_to_rect(rect.expand(8.0), Some(Align::Center));
                    }
                }
            });
    }

    fn find_window(&mut self, ctx: &egui::Context) -> Vec<FindCommand> {
        let mut commands = Vec::new();
        if !self.find.open {
            return commands;
        }
        let mut open = true;
        let title = if self.find.replace_mode {
            "Substituir"
        } else {
            "Localizar"
        };
        egui::Window::new(title)
            .id(Id::unique("find-window"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_TOP, [-16.0, 40.0])
            .show(ctx, |ui| {
                let f = &mut self.find;
                egui::Grid::new("find-grid").num_columns(2).show(ui, |ui| {
                    ui.label("Localizar:");
                    let q = ui.text_edit_singleline(&mut f.query);
                    if std::mem::take(&mut f.focus_query) {
                        q.request_focus();
                    }
                    if q.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        commands.push(FindCommand::Next);
                    }
                    ui.end_row();
                    if f.replace_mode {
                        ui.label("Substituir por:");
                        ui.text_edit_singleline(&mut f.replacement);
                        ui.end_row();
                    }
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut f.case_sensitive, "Diferenciar maiúsculas");
                    ui.checkbox(&mut f.wrap_around, "Circular");
                });
                ui.horizontal(|ui| {
                    if ui.button("◀ Anterior").clicked() {
                        commands.push(FindCommand::Prev);
                    }
                    if ui.button("Próxima ▶").clicked() {
                        commands.push(FindCommand::Next);
                    }
                    if f.replace_mode {
                        if ui.button("Substituir").clicked() {
                            commands.push(FindCommand::Replace);
                        }
                        if ui.button("Substituir tudo").clicked() {
                            commands.push(FindCommand::ReplaceAll);
                        }
                    } else if ui.button("Substituir…").clicked() {
                        f.replace_mode = true;
                    }
                });
                if !f.message.is_empty() {
                    ui.colored_label(ui.visuals().warn_fg_color, &f.message);
                }
            });
        if !open {
            self.find.open = false;
            self.focus_editor = true;
        }
        commands
    }

    fn modal_ui(&mut self, ctx: &egui::Context) {
        let Some(modal) = &mut self.modal else {
            return;
        };
        let mut outcome = ModalOutcome::None;
        let response = egui::Modal::new(Id::unique("modal")).show(ctx, |ui| {
            ui.set_max_width(360.0);
            match modal {
                Modal::ConfirmClose(i) => {
                    let name = self
                        .tabs
                        .get(*i)
                        .map(|t| t.doc.file_name())
                        .unwrap_or_default();
                    ui.heading(APP_NAME);
                    ui.label(format!("Deseja salvar as alterações em \"{name}\"?"));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Salvar").clicked() {
                            outcome = ModalOutcome::Save(*i);
                        }
                        if ui.button("Não salvar").clicked() {
                            outcome = ModalOutcome::Discard(*i);
                        }
                        if ui.button("Cancelar").clicked() {
                            outcome = ModalOutcome::Cancel;
                        }
                    });
                }
                Modal::GoToLine { input, error } => {
                    ui.heading("Ir para linha");
                    let r = ui.text_edit_singleline(input);
                    r.request_focus();
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if *error {
                        ui.colored_label(
                            ui.visuals().error_fg_color,
                            "O número da linha está além do total de linhas.",
                        );
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Ir para").clicked() || enter {
                            outcome = ModalOutcome::GoTo(input.trim().parse().unwrap_or(0));
                        }
                        if ui.button("Cancelar").clicked() {
                            outcome = ModalOutcome::Cancel;
                        }
                    });
                }
                Modal::Font => {
                    ui.heading("Fonte");
                    let s = &mut self.settings;
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut s.font_kind, FontKind::Monospace, "Monoespaçada");
                        ui.radio_value(&mut s.font_kind, FontKind::Proportional, "Proporcional");
                    });
                    ui.add(egui::Slider::new(&mut s.font_size, 6.0..=72.0).text("Tamanho"));
                    ui.add_space(4.0);
                    let family = match s.font_kind {
                        FontKind::Monospace => FontFamily::Monospace,
                        FontKind::Proportional => FontFamily::Proportional,
                    };
                    ui.label(
                        egui::RichText::new("AaBbYyZz 0123 — Ação, coração!")
                            .font(FontId::new(s.font_size, family)),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut s.word_wrap, "Quebra automática de linha");
                        ui.checkbox(&mut s.show_status_bar, "Barra de status");
                    });
                    if ui.button("OK").clicked() {
                        outcome = ModalOutcome::Cancel;
                    }
                }
                Modal::About => {
                    ui.heading(APP_NAME);
                    ui.label(format!("Versão {}", env!("CARGO_PKG_VERSION")));
                    ui.label("Clone do Bloco de Notas em Rust (egui + muda).");
                    if ui.button("OK").clicked() {
                        outcome = ModalOutcome::Cancel;
                    }
                }
                Modal::Error(msg) => {
                    ui.heading("Erro");
                    ui.label(msg.as_str());
                    if ui.button("OK").clicked() {
                        outcome = ModalOutcome::Cancel;
                    }
                }
            }
        });
        if response.should_close() && matches!(outcome, ModalOutcome::None) {
            outcome = ModalOutcome::Cancel;
        }
        self.apply_modal_outcome(ctx, outcome);
    }

    fn apply_modal_outcome(&mut self, ctx: &egui::Context, outcome: ModalOutcome) {
        match outcome {
            ModalOutcome::None => {}
            ModalOutcome::Cancel => {
                self.modal = None;
                self.pending_quit = false;
                self.focus_editor = true;
            }
            ModalOutcome::Save(i) => {
                self.modal = None;
                if self.save_tab(i, false) {
                    self.after_confirm(i);
                } else if self.modal.is_none() {
                    // Usuário cancelou o diálogo de salvar.
                    self.pending_quit = false;
                }
            }
            ModalOutcome::Discard(i) => {
                self.modal = None;
                self.tabs[i].doc.modified = false;
                self.after_confirm(i);
            }
            ModalOutcome::GoTo(line) => match self.tab().doc.line_start(line) {
                Some(pos) => {
                    self.modal = None;
                    self.set_selection(ctx, pos, pos);
                }
                None => {
                    if let Some(Modal::GoToLine { error, .. }) = &mut self.modal {
                        *error = true;
                    }
                }
            },
        }
    }

    fn after_confirm(&mut self, i: usize) {
        if self.pending_quit {
            self.continue_quit();
        } else {
            self.remove_tab(i);
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let doc = &self.tab().doc;
        let title = format!(
            "{}{} — {APP_NAME}",
            if doc.modified { "*" } else { "" },
            doc.file_name()
        );
        if title != self.title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    fn persist_settings(&mut self) {
        let mut current = self.settings.clone();
        current.window_size = self.saved_settings.window_size;
        if current != self.saved_settings {
            if let Err(e) = self.settings.save() {
                self.modal = Some(Modal::Error(format!("{e:#}")));
            }
            self.saved_settings = self.settings.clone();
        }
    }
}

enum FindCommand {
    Next,
    Prev,
    Replace,
    ReplaceAll,
}

enum ModalOutcome {
    None,
    Cancel,
    Save(usize),
    Discard(usize),
    GoTo(usize),
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    if let Some(data) = platform::monospace_font() {
        fonts
            .font_data
            .insert(SYSTEM_MONO.to_owned(), Arc::new(data));
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, SYSTEM_MONO.to_owned());
    }
    ctx.set_fonts(fonts);
}

impl eframe::App for NotepadApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Fechar janela (botão vermelho, ⌘Q, ⌘W na última aba).
        if ui.input(|i| i.viewport().close_requested()) && !self.allow_close {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            if self.modal.is_none() {
                self.pending_quit = true;
                self.continue_quit();
            }
        }

        if std::mem::take(&mut self.reset_cursor) {
            self.set_selection(&ctx, 0, 0);
        }
        let mut actions = self.collect_actions(ui);
        #[cfg(feature = "screenshot")]
        actions.extend(self.debug.tick(&ctx));

        // Arquivos arrastados para a janela.
        let dropped: Vec<PathBuf> = ui.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| p.is_file())
                .collect()
        });
        for path in dropped.into_iter().chain(platform::take_open_requests()) {
            self.open_path(&path);
        }

        if self.native_menu.is_none() {
            egui::Panel::top("menu-bar").show(ui, |ui| {
                let s = &self.settings;
                if let Some(a) = menu::egui_menu_bar(ui, s.word_wrap, s.show_status_bar) {
                    actions.push(a);
                }
            });
        }
        egui::Panel::top("tab-bar").show(ui, |ui| {
            actions.extend(self.tab_bar(ui));
        });
        if self.settings.show_status_bar {
            egui::Panel::bottom("status-bar").show(ui, |ui| self.status_bar(ui));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(ui.visuals().extreme_bg_color))
            .show(ui, |ui| self.editor(ui));

        for command in self.find_window(&ctx) {
            match command {
                FindCommand::Next => self.find_next(&ctx, true),
                FindCommand::Prev => self.find_next(&ctx, false),
                FindCommand::Replace => self.replace_one(&ctx),
                FindCommand::ReplaceAll => self.replace_all(&ctx),
            }
        }
        self.modal_ui(&ctx);

        for action in actions {
            self.run(&ctx, action);
        }

        if self.allow_close {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        if let Some(native) = &self.native_menu {
            native.sync_checks(self.settings.word_wrap, self.settings.show_status_bar);
        }
        if let Some(rect) = ui.input(|i| i.viewport().inner_rect) {
            self.settings.window_size = Some([rect.width(), rect.height()]);
        }
        self.update_title(&ctx);
        self.persist_settings();
        if self.focus_editor || self.scroll_to_cursor {
            ctx.request_repaint();
        }
    }

    fn on_exit(&mut self) {
        // Erro ao salvar as configurações na saída não deve impedir o fechamento.
        let _ = self.settings.save();
    }
}
