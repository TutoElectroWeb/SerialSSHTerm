// =============================================================================
// Fichier : terminal_panel.rs
// Rôle    : Panneau d'affichage du terminal (zone de texte scrollable) avec support ANSI
// =============================================================================

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{ScrolledWindow, TextBuffer, TextTagTable, TextView, TextTag};
use vte::{Parser, Perform};

/// Panneau d'affichage du terminal.
///
/// Contient un `TextView` en lecture seule avec auto-scroll et gestion
/// du scrollback, ainsi qu'un parseur ANSI pour les couleurs.
pub struct TerminalPanel {
    pub container: ScrolledWindow,
    pub text_view: TextView,
    pub buffer: TextBuffer,
    pub max_lines: u32,
    auto_scroll_enabled: Rc<Cell<bool>>,
    ansi_parser: Rc<RefCell<Parser>>,
    ansi_performer: Rc<RefCell<AnsiPerformer>>,
}

struct AnsiPerformer {
    buffer: TextBuffer,
    pending_text: String,
    current_fg: Option<u8>,
    current_bg: Option<u8>,
    bold: bool,
    italic: bool,
    underline: bool,
}

impl AnsiPerformer {
    const fn new(buffer: TextBuffer) -> Self {
        Self {
            buffer,
            pending_text: String::new(),
            current_fg: None,
            current_bg: None,
            bold: false,
            italic: false,
            underline: false,
        }
    }

    fn flush(&mut self) {
        if self.pending_text.is_empty() {
            return;
        }

        let mut end_iter = self.buffer.end_iter();
        let mut tag_names = Vec::new();

        if let Some(fg) = self.current_fg {
            tag_names.push(format!("fg_{fg}"));
        }
        if let Some(bg) = self.current_bg {
            tag_names.push(format!("bg_{bg}"));
        }
        if self.bold {
            tag_names.push("bold".to_string());
        }
        if self.italic {
            tag_names.push("italic".to_string());
        }
        if self.underline {
            tag_names.push("underline".to_string());
        }

        if tag_names.is_empty() {
            self.buffer.insert(&mut end_iter, &self.pending_text);
        } else {
            let tag_table = self.buffer.tag_table();
            let tags: Vec<TextTag> = tag_names
                .iter()
                .filter_map(|name| tag_table.lookup(name))
                .collect();
            let tags_refs: Vec<&TextTag> = tags.iter().collect();
            self.buffer.insert_with_tags(&mut end_iter, &self.pending_text, &tags_refs);
        }

        self.pending_text.clear();
    }
}

impl Perform for AnsiPerformer {
    fn print(&mut self, c: char) {
        self.pending_text.push(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | b'\r' | b'\t' | b'\x08' => {
                self.pending_text.push(byte as char);
            }
            _ => {}
        }
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn csi_dispatch(&mut self, params: &vte::Params, _intermediates: &[u8], _ignore: bool, action: char) {
        if action == 'm' {
            self.flush();
            let mut has_params = false;
            for param in params {
                has_params = true;
                let p = if param.is_empty() { 0 } else { param[0] };
                match p {
                    0 => {
                        self.current_fg = None;
                        self.current_bg = None;
                        self.bold = false;
                        self.italic = false;
                        self.underline = false;
                    }
                    1 => self.bold = true,
                    3 => self.italic = true,
                    4 => self.underline = true,
                    22 => self.bold = false,
                    23 => self.italic = false,
                    24 => self.underline = false,
                    // Les plages de match garantissent que le résultat tient dans u8 (0-15).
                    30..=37 => self.current_fg = Some(u8::try_from(p - 30).unwrap_or(0)),
                    39 => self.current_fg = None,
                    40..=47 => self.current_bg = Some(u8::try_from(p - 40).unwrap_or(0)),
                    49 => self.current_bg = None,
                    90..=97 => self.current_fg = Some(u8::try_from(p - 90 + 8).unwrap_or(8)),
                    100..=107 => self.current_bg = Some(u8::try_from(p - 100 + 8).unwrap_or(8)),
                    _ => {}
                }
            }
            if !has_params {
                self.current_fg = None;
                self.current_bg = None;
                self.bold = false;
                self.italic = false;
                self.underline = false;
            }
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

impl TerminalPanel {
    /// Crée un nouveau panneau terminal.
    pub fn new(max_lines: u32) -> Self {
        let tag_table = TextTagTable::new();

        // Tag pour les données envoyées (TX)
        let tx_tag = gtk4::TextTag::builder()
            .name("tx")
            .foreground("orange")
            .build();
        tag_table.add(&tx_tag);

        // Tag pour les données reçues (RX)
        let rx_tag = gtk4::TextTag::builder().name("rx").build();
        tag_table.add(&rx_tag);

        // Tag pour les messages système
        let sys_tag = gtk4::TextTag::builder()
            .name("system")
            .foreground("#888888")
            .style(gtk4::pango::Style::Italic)
            .build();
        tag_table.add(&sys_tag);

        // Tag pour les erreurs
        let err_tag = gtk4::TextTag::builder()
            .name("error")
            .foreground("#ff4444")
            .weight(700)
            .build();
        tag_table.add(&err_tag);

        // Tags ANSI
        let colors = [
            "#000000", "#CD0000", "#00CD00", "#CDCD00", "#0000EE", "#CD00CD", "#00CDCD", "#E5E5E5", // 0-7
            "#7F7F7F", "#FF0000", "#00FF00", "#FFFF00", "#5C5CFF", "#FF00FF", "#00FFFF", "#FFFFFF", // 8-15
        ];
        for (i, color) in colors.iter().enumerate() {
            let fg_tag = gtk4::TextTag::builder()
                .name(format!("fg_{i}"))
                .foreground(*color)
                .build();
            tag_table.add(&fg_tag);

            let bg_tag = gtk4::TextTag::builder()
                .name(format!("bg_{i}"))
                .background(*color)
                .build();
            tag_table.add(&bg_tag);
        }

        let bold_tag = gtk4::TextTag::builder()
            .name("bold")
            .weight(700)
            .build();
        tag_table.add(&bold_tag);

        let italic_tag = gtk4::TextTag::builder()
            .name("italic")
            .style(gtk4::pango::Style::Italic)
            .build();
        tag_table.add(&italic_tag);

        let underline_tag = gtk4::TextTag::builder()
            .name("underline")
            .underline(gtk4::pango::Underline::Single)
            .build();
        tag_table.add(&underline_tag);

        let buffer = TextBuffer::new(Some(&tag_table));

        let text_view = TextView::builder()
            .buffer(&buffer)
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(gtk4::WrapMode::Char)
            .monospace(true)
            .top_margin(4)
            .bottom_margin(4)
            .left_margin(8)
            .right_margin(8)
            .vexpand(true)
            .hexpand(true)
            .build();

        text_view.add_css_class("terminal-view");

        let container = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .child(&text_view)
            .build();

        let auto_scroll_enabled = Rc::new(Cell::new(true));
        let ansi_parser = Rc::new(RefCell::new(Parser::new()));
        let ansi_performer = Rc::new(RefCell::new(AnsiPerformer::new(buffer.clone())));

        Self {
            container,
            text_view,
            buffer,
            max_lines,
            auto_scroll_enabled,
            ansi_parser,
            ansi_performer,
        }
    }

    /// Ajoute des données reçues (RX) au terminal en parsant les séquences ANSI.
    pub fn append_ansi(&self, data: &[u8]) {
        let mut parser = self.ansi_parser.borrow_mut();
        let mut performer = self.ansi_performer.borrow_mut();
        
        parser.advance(&mut *performer, data);
        performer.flush();

        self.trim_scrollback();
        if self.auto_scroll_enabled.get() {
            self.scroll_to_bottom();
        }
    }

    /// Ajoute du texte envoyé (TX) au terminal — écho local.
    pub fn append_sent(&self, text: &str) {
        self.append_with_tag(text, "tx");
    }

    /// Ajoute un message système.
    pub fn append_system(&self, text: &str) {
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        self.append_with_tag(&format!("[{timestamp}] {text}\n"), "system");
    }

    /// Ajoute un message d'erreur.
    pub fn append_error(&self, text: &str) {
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        self.append_with_tag(&format!("[{timestamp}] ERREUR: {text}\n"), "error");
    }

    /// Ajoute du texte avec un tag donné et fait défiler vers le bas.
    fn append_with_tag(&self, text: &str, tag_name: &str) {
        let mut end_iter = self.buffer.end_iter();

        let tag_table = self.buffer.tag_table();
        if let Some(tag) = tag_table.lookup(tag_name) {
            self.buffer.insert_with_tags(&mut end_iter, text, &[&tag]);
        } else {
            self.buffer.insert(&mut end_iter, text);
        }

        // Limiter le scrollback
        self.trim_scrollback();

        // Auto-scroll vers le bas
        if self.auto_scroll_enabled.get() {
            self.scroll_to_bottom();
        }
    }

    /// Supprime les anciennes lignes au-delà de la limite de scrollback.
    fn trim_scrollback(&self) {
        let line_count = self.buffer.line_count();
        let max_lines_i32 = i32::try_from(self.max_lines).unwrap_or(i32::MAX);
        if line_count > max_lines_i32 {
            let lines_to_remove = line_count - max_lines_i32;
            let mut start = self.buffer.start_iter();
            let mut end = self.buffer.iter_at_line(lines_to_remove).unwrap_or(start);
            // S'assurer que end est bien au début de la ligne
            if end.line_offset() != 0 {
                end.forward_to_line_end();
                end.forward_char();
            }
            self.buffer.delete(&mut start, &mut end);
        }
    }

    /// Fait défiler le terminal vers le bas.
    fn scroll_to_bottom(&self) {
        let end_mark = self
            .buffer
            .create_mark(None, &self.buffer.end_iter(), false);
        self.text_view
            .scroll_to_mark(&end_mark, 0.0, false, 0.0, 1.0);
        self.buffer.delete_mark(&end_mark);
    }

    /// Efface tout le contenu du terminal.
    pub fn clear(&self) {
        self.buffer
            .delete(&mut self.buffer.start_iter(), &mut self.buffer.end_iter());
    }

    /// Retourne tout le texte du terminal.
    pub fn get_text(&self) -> String {
        self.buffer
            .text(&self.buffer.start_iter(), &self.buffer.end_iter(), false)
            .to_string()
    }

    /// Active/désactive le défilement automatique.
    pub fn set_auto_scroll_enabled(&self, enabled: bool) {
        self.auto_scroll_enabled.set(enabled);
    }

    /// Retourne un handle partagé de l'état auto-scroll.
    #[allow(dead_code)]
    pub fn auto_scroll_handle(&self) -> Rc<Cell<bool>> {
        self.auto_scroll_enabled.clone()
    }
}
