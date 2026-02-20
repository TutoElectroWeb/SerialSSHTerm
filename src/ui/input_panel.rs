// =============================================================================
// Fichier : input_panel.rs
// Rôle    : Barre de saisie et envoi de commandes
// =============================================================================

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, CheckButton, DropDown, Entry, Label, Orientation, StringList};

/// Panneau de saisie en bas de la fenêtre.
///
/// Contient un champ de texte, un sélecteur de fin de ligne et un bouton Envoyer.
pub struct InputPanel {
    pub container: GtkBox,
    pub entry: Entry,
    pub send_button: Button,
    pub line_ending_dropdown: DropDown,
    pub stop_scroll_checkbox: CheckButton,
}

impl InputPanel {
    /// Crée le panneau de saisie.
    pub fn new() -> Self {
        let container = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(4)
            .margin_bottom(8)
            .build();

        // Champ de saisie
        let entry = Entry::builder()
            .placeholder_text("Tapez votre commande ici...")
            .hexpand(true)
            .build();
        entry.add_css_class("input-entry");

        // Sélecteur de fin de ligne
        let le_label = Label::new(Some("Fin :"));
        let line_endings = StringList::new(&["LF (\\n)", "CR (\\r)", "CRLF (\\r\\n)", "Aucun"]);
        let line_ending_dropdown = DropDown::builder().model(&line_endings).selected(0).build();

        // Bouton Envoyer
        let send_button = Button::builder()
            .label("Envoyer")
            .icon_name("mail-send-symbolic")
            .build();
        send_button.add_css_class("suggested-action");

        // Case à cocher : arrêt du défilement automatique
        let stop_scroll_checkbox = CheckButton::builder()
            .label("Arrêt défilement")
            .tooltip_text("Bloque le défilement automatique du terminal")
            .build();

        container.append(&entry);
        container.append(&le_label);
        container.append(&line_ending_dropdown);
        container.append(&stop_scroll_checkbox);
        container.append(&send_button);

        Self {
            container,
            entry,
            send_button,
            line_ending_dropdown,
            stop_scroll_checkbox,
        }
    }

    /// Retourne le texte saisi.
    pub fn get_text(&self) -> String {
        self.entry.text().to_string()
    }

    /// Efface le champ de saisie.
    pub fn clear(&self) {
        self.entry.set_text("");
    }

    /// Retourne le suffixe de fin de ligne sélectionné.
    pub fn selected_line_ending(&self) -> &str {
        match self.line_ending_dropdown.selected() {
            0 => "\n",
            1 => "\r",
            2 => "\r\n",
            _ => "",
        }
    }

    /// Remet le focus sur le champ de saisie.
    pub fn grab_focus(&self) {
        self.entry.grab_focus();
    }
}
