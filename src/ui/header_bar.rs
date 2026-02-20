// =============================================================================
// Fichier : header_bar.rs
// Rôle    : Barre d'en-tête avec menu et actions rapides
// =============================================================================

use gtk4::gio;
use gtk4::prelude::*;
use gtk4::{Button, Label, MenuButton, PopoverMenu};
use libadwaita::HeaderBar;

use crate::ui::theme::Theme;

/// Barre d'en-tête de l'application.
pub struct AppHeaderBar {
    pub header_bar: HeaderBar,
    pub status_label: Label,
    pub save_log_button: Button,
}

impl AppHeaderBar {
    pub fn new() -> Self {
        let header_bar = HeaderBar::new();

        // Label de statut à gauche
        let status_label = Label::builder().label("Déconnecté").build();
        status_label.add_css_class("status-disconnected");
        header_bar.pack_start(&status_label);

        // Bouton sauvegarde logs
        let save_log_button = Button::builder()
            .icon_name("document-save-symbolic")
            .tooltip_text("Sauvegarder les logs")
            .build();

        // Menu hamburger
        let main_menu = gio::Menu::new();

        // Sous-menu Thèmes
        let theme_menu = gio::Menu::new();
        for theme in Theme::all() {
            theme_menu.append(
                Some(theme.display_name()),
                Some(&format!("win.set-theme::{}", theme.id())),
            );
        }
        main_menu.append_submenu(Some("Thème"), &theme_menu);

        // Actions directes
        main_menu.append(Some("Outils"), Some("win.open-tools"));
        main_menu.append(Some("Sauvegarder les logs"), Some("win.save-logs"));
        main_menu.append(Some("Effacer le terminal"), Some("win.clear-terminal"));

        let sep = gio::Menu::new();
        sep.append(Some("À propos"), Some("win.about"));
        main_menu.append_section(None, &sep);

        let popover = PopoverMenu::from_model(Some(&main_menu));
        let menu_button = MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .popover(&popover)
            .tooltip_text("Menu")
            .build();

        header_bar.pack_end(&menu_button);
        header_bar.pack_end(&save_log_button);

        Self {
            header_bar,
            status_label,
            save_log_button,
        }
    }

    /// Met à jour le label de statut.
    pub fn set_status(&self, text: &str, connected: bool) {
        self.status_label.set_label(text);
        if connected {
            self.status_label.remove_css_class("status-disconnected");
            self.status_label.add_css_class("status-connected");
        } else {
            self.status_label.remove_css_class("status-connected");
            self.status_label.add_css_class("status-disconnected");
        }
    }
}
