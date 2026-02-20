// =============================================================================
// Fichier : connection_panel.rs
// Rôle    : Panneau de connexion avec onglets Série / SSH
// =============================================================================

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DropDown, Entry, Label, Notebook, Orientation, PasswordEntry,
    SpinButton, StringList,
};

use crate::core::serial_manager::list_serial_ports;
use crate::core::settings::SshFavorite;

// =============================================================================
// Panneau de connexion série
// =============================================================================

/// Information interne d'un port pour retrouver le nom device à partir de l'index.
struct PortEntry {
    device: String,
}

/// Panneau de configuration de la connexion série.
pub struct SerialPanel {
    pub container: GtkBox,
    pub port_dropdown: DropDown,
    pub baud_dropdown: DropDown,
    pub databits_dropdown: DropDown,
    pub parity_dropdown: DropDown,
    pub stopbits_dropdown: DropDown,
    pub flowcontrol_dropdown: DropDown,
    pub refresh_button: Button,
    port_model: StringList,
    port_entries: std::cell::RefCell<Vec<PortEntry>>,
}

impl SerialPanel {
    pub fn new() -> Self {
        let container = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        container.add_css_class("connection-panel");

        // Port série
        let port_label = Label::new(Some("Port :"));
        let port_model = StringList::new(&[]);
        let port_dropdown = DropDown::builder()
            .model(&port_model)
            .tooltip_text("Sélectionner le port série")
            .build();

        // Rafraîchir
        let refresh_button = Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Rafraîchir les ports")
            .build();

        // Vitesse
        let baud_label = Label::new(Some("Vitesse :"));
        let baud_model = StringList::new(&[
            "9600", "19200", "38400", "57600", "115200", "230400", "460800", "921600",
        ]);
        let baud_dropdown = DropDown::builder()
            .model(&baud_model)
            .selected(4) // 115200
            .build();

        // Bits de données
        let databits_model = StringList::new(&["5", "6", "7", "8"]);
        let databits_dropdown = DropDown::builder()
            .model(&databits_model)
            .selected(3) // 8
            .build();

        // Parité
        let parity_model = StringList::new(&["None", "Odd", "Even"]);
        let parity_dropdown = DropDown::builder().model(&parity_model).selected(0).build();

        // Stop bits
        let stopbits_model = StringList::new(&["1", "2"]);
        let stopbits_dropdown = DropDown::builder()
            .model(&stopbits_model)
            .selected(0)
            .build();

        // Flow control
        let flowcontrol_model = StringList::new(&["None", "Hardware", "Software"]);
        let flowcontrol_dropdown = DropDown::builder()
            .model(&flowcontrol_model)
            .selected(0)
            .build();

        // Layout
        container.append(&port_label);
        container.append(&port_dropdown);
        container.append(&refresh_button);

        let sep1 = gtk4::Separator::new(Orientation::Vertical);
        container.append(&sep1);

        container.append(&baud_label);
        container.append(&baud_dropdown);

        // Paramètres avancés
        let advanced_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(6)
            .build();

        let db_label = Label::new(Some("Bits:"));
        advanced_box.append(&db_label);
        advanced_box.append(&databits_dropdown);

        let p_label = Label::new(Some("Parité:"));
        advanced_box.append(&p_label);
        advanced_box.append(&parity_dropdown);

        let sb_label = Label::new(Some("Stop:"));
        advanced_box.append(&sb_label);
        advanced_box.append(&stopbits_dropdown);

        let fc_label = Label::new(Some("Flux:"));
        advanced_box.append(&fc_label);
        advanced_box.append(&flowcontrol_dropdown);

        container.append(&advanced_box);

        let panel = Self {
            container,
            port_dropdown,
            baud_dropdown,
            databits_dropdown,
            parity_dropdown,
            stopbits_dropdown,
            flowcontrol_dropdown,
            refresh_button,
            port_model,
            port_entries: std::cell::RefCell::new(Vec::new()),
        };

        panel.refresh_ports();
        panel
    }

    /// Rafraîchit la liste des ports série disponibles.
    pub fn refresh_ports(&self) {
        // Vider le modèle existant
        self.port_model
            .splice(0, self.port_model.n_items(), &[] as &[&str]);

        let ports = list_serial_ports();
        let mut entries = Vec::new();

        if ports.is_empty() {
            self.port_model.append("Aucun port");
            entries.push(PortEntry {
                device: String::new(),
            });
        } else {
            for port in &ports {
                let label = match (port.description.is_empty(), port.manufacturer.is_empty()) {
                    (true, true) => port.device.clone(),
                    (false, true) => format!("{} ({})", port.device, port.description),
                    (true, false) => format!("{} [{}]", port.device, port.manufacturer),
                    (false, false) => format!(
                        "{} ({}) [{}]",
                        port.device, port.description, port.manufacturer
                    ),
                };
                self.port_model.append(&label);
                entries.push(PortEntry {
                    device: port.device.clone(),
                });
            }
        }

        *self.port_entries.borrow_mut() = entries;
        self.port_dropdown.set_selected(0);
        log::info!("Ports série rafraîchis : {} trouvé(s)", ports.len());
    }

    /// Retourne le port sélectionné (nom device).
    pub fn selected_port(&self) -> Option<String> {
        let idx = self.port_dropdown.selected() as usize;
        let entries = self.port_entries.borrow();
        entries.get(idx).and_then(|e| {
            if e.device.is_empty() {
                None
            } else {
                Some(e.device.clone())
            }
        })
    }

    /// Helper pour lire la valeur textuelle d'un `DropDown` `StringList`.
    fn dropdown_text(dropdown: &DropDown) -> Option<String> {
        let model = dropdown.model()?;
        let idx = dropdown.selected();
        let item = model.item(idx)?;
        let string_obj = item.downcast::<gtk4::StringObject>().ok()?;
        Some(string_obj.string().to_string())
    }

    /// Positionne un `DropDown` `StringList` sur une valeur textuelle donnée.
    fn set_dropdown_by_text(dropdown: &DropDown, value: &str) {
        let Some(model) = dropdown.model() else {
            return;
        };

        for idx in 0..model.n_items() {
            let Some(item) = model.item(idx) else {
                continue;
            };
            let Ok(string_obj) = item.downcast::<gtk4::StringObject>() else {
                continue;
            };
            if string_obj.string() == value {
                dropdown.set_selected(idx);
                return;
            }
        }
    }

    /// Retourne le baudrate sélectionné.
    pub fn selected_baudrate(&self) -> u32 {
        Self::dropdown_text(&self.baud_dropdown)
            .and_then(|s| s.parse().ok())
            .unwrap_or(115_200)
    }

    /// Retourne les data bits sélectionnés.
    pub fn selected_data_bits(&self) -> u8 {
        Self::dropdown_text(&self.databits_dropdown)
            .and_then(|s| s.parse().ok())
            .unwrap_or(8)
    }

    /// Retourne la parité sélectionnée.
    pub fn selected_parity(&self) -> String {
        Self::dropdown_text(&self.parity_dropdown).unwrap_or_else(|| "None".to_string())
    }

    /// Retourne les stop bits sélectionnés.
    pub fn selected_stop_bits(&self) -> u8 {
        Self::dropdown_text(&self.stopbits_dropdown)
            .and_then(|s| s.parse().ok())
            .unwrap_or(1)
    }

    /// Retourne le flow control sélectionné.
    pub fn selected_flow_control(&self) -> String {
        Self::dropdown_text(&self.flowcontrol_dropdown).unwrap_or_else(|| "None".to_string())
    }

    /// Sélectionne un port par son nom device s'il existe.
    pub fn select_port_by_device(&self, device: &str) {
        if device.is_empty() {
            return;
        }

        let entries = self.port_entries.borrow();
        for (idx, entry) in entries.iter().enumerate() {
            if entry.device == device {
                self.port_dropdown.set_selected(u32::try_from(idx).unwrap_or(u32::MAX));
                return;
            }
        }
    }

    /// Applique les paramètres série à l'UI.
    pub fn apply_settings(
        &self,
        baudrate: u32,
        data_bits: u8,
        parity: &str,
        stop_bits: u8,
        flow_control: &str,
    ) {
        Self::set_dropdown_by_text(&self.baud_dropdown, &baudrate.to_string());
        Self::set_dropdown_by_text(&self.databits_dropdown, &data_bits.to_string());
        Self::set_dropdown_by_text(&self.parity_dropdown, parity);
        Self::set_dropdown_by_text(&self.stopbits_dropdown, &stop_bits.to_string());
        Self::set_dropdown_by_text(&self.flowcontrol_dropdown, flow_control);
    }
}

// =============================================================================
// Panneau de connexion SSH
// =============================================================================

/// Panneau de configuration de la connexion SSH.
pub struct SshPanel {
    pub container: GtkBox,
    pub favorite_dropdown: DropDown,
    pub add_favorite_button: Button,
    pub host_entry: Entry,
    pub port_spin: SpinButton,
    pub username_entry: Entry,
    pub password_entry: PasswordEntry,
    pub key_path_entry: Entry,
    pub key_browse_button: Button,
    favorite_model: StringList,
    favorite_entries: std::cell::RefCell<Vec<SshFavorite>>,
}

impl SshPanel {
    pub fn new() -> Self {
        let container = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        container.add_css_class("connection-panel");

        // Favoris SSH
        let favorite_label = Label::new(Some("Favori :"));
        let favorite_model = StringList::new(&["Favoris SSH"]);
        let favorite_dropdown = DropDown::builder()
            .model(&favorite_model)
            .selected(0)
            .tooltip_text("Choisir un favori SSH")
            .build();
        let add_favorite_button = Button::builder()
            .icon_name("bookmark-new-symbolic")
            .tooltip_text("Ajouter ce profil aux favoris")
            .build();

        // Hôte
        let host_label = Label::new(Some("Hôte :"));
        let host_entry = Entry::builder()
            .placeholder_text("192.168.1.1")
            .width_chars(18)
            .build();

        // Port
        let port_label = Label::new(Some("Port :"));
        let port_spin = SpinButton::with_range(1.0, 65535.0, 1.0);
        port_spin.set_value(22.0);
        port_spin.set_width_chars(6);

        // Utilisateur
        let user_label = Label::new(Some("Utilisateur :"));
        let username_entry = Entry::builder()
            .placeholder_text("root")
            .width_chars(12)
            .build();

        // Mot de passe
        let pass_label = Label::new(Some("Mot de passe :"));
        let password_entry = PasswordEntry::builder()
            .placeholder_text("••••")
            .show_peek_icon(true)
            .build();

        // Clé SSH
        let key_label = Label::new(Some("Clé :"));
        let key_path_entry = Entry::builder()
            .placeholder_text("~/.ssh/id_rsa")
            .width_chars(20)
            .build();
        let key_browse_button = Button::builder()
            .icon_name("folder-open-symbolic")
            .tooltip_text("Parcourir...")
            .build();

        container.append(&favorite_label);
        container.append(&favorite_dropdown);
        container.append(&add_favorite_button);

        let sep0 = gtk4::Separator::new(Orientation::Vertical);
        container.append(&sep0);

        container.append(&host_label);
        container.append(&host_entry);

        let sep1 = gtk4::Separator::new(Orientation::Vertical);
        container.append(&sep1);

        container.append(&port_label);
        container.append(&port_spin);

        let sep2 = gtk4::Separator::new(Orientation::Vertical);
        container.append(&sep2);

        container.append(&user_label);
        container.append(&username_entry);
        container.append(&pass_label);
        container.append(&password_entry);

        let sep3 = gtk4::Separator::new(Orientation::Vertical);
        container.append(&sep3);

        container.append(&key_label);
        container.append(&key_path_entry);
        container.append(&key_browse_button);

        Self {
            container,
            favorite_dropdown,
            add_favorite_button,
            host_entry,
            port_spin,
            username_entry,
            password_entry,
            key_path_entry,
            key_browse_button,
            favorite_model,
            favorite_entries: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Retourne l'hôte saisi.
    pub fn host(&self) -> String {
        self.host_entry.text().to_string()
    }

    /// Retourne le port saisi.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn port(&self) -> u16 {
        // Range contrainte à 1-65535 dans le constructeur SpinButton → troncature impossible.
        self.port_spin.value() as u16
    }

    /// Retourne le nom d'utilisateur.
    pub fn username(&self) -> String {
        self.username_entry.text().to_string()
    }

    /// Retourne le mot de passe.
    pub fn password(&self) -> String {
        self.password_entry.text().to_string()
    }

    /// Retourne le chemin de la clé SSH.
    pub fn key_path(&self) -> String {
        self.key_path_entry.text().to_string()
    }

    /// Efface le mot de passe affiché (sécurité UX).
    pub fn clear_password(&self) {
        self.password_entry.set_text("");
    }

    /// Applique les paramètres SSH à l'UI.
    pub fn apply_settings(&self, host: &str, port: u16, username: &str, key_path: &str) {
        self.host_entry.set_text(host);
        self.port_spin.set_value(f64::from(port));
        self.username_entry.set_text(username);
        self.key_path_entry.set_text(key_path);
    }

    /// Charge la liste des favoris SSH dans le dropdown.
    pub fn set_favorites(&self, favorites: &[SshFavorite]) {
        self.favorite_model
            .splice(0, self.favorite_model.n_items(), &["Favoris SSH"]);

        for favorite in favorites {
            self.favorite_model.append(&favorite.name);
        }

        *self.favorite_entries.borrow_mut() = favorites.to_vec();
        self.favorite_dropdown.set_selected(0);
    }

    /// Retourne le favori sélectionné, s'il y en a un.
    pub fn selected_favorite(&self) -> Option<SshFavorite> {
        let selected = self.favorite_dropdown.selected();
        if selected == 0 {
            return None;
        }

        let idx = (selected - 1) as usize;
        self.favorite_entries.borrow().get(idx).cloned()
    }
}

// =============================================================================
// Panneau de connexion combiné (Notebook tabs)
// =============================================================================

/// Panneau de connexion avec onglets Série / SSH + bouton Connecter.
pub struct ConnectionPanel {
    pub container: GtkBox,
    pub notebook: Notebook,
    pub serial_panel: SerialPanel,
    pub ssh_panel: SshPanel,
    pub connect_button: Button,
    pub clear_button: Button,
}

impl ConnectionPanel {
    pub fn new() -> Self {
        let container = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(0)
            .build();

        let notebook = Notebook::builder().show_border(true).build();

        let serial_panel = SerialPanel::new();
        let ssh_panel = SshPanel::new();

        let serial_label = Label::new(Some("🔌 Série"));
        let ssh_label = Label::new(Some("🔐 SSH"));

        notebook.append_page(&serial_panel.container, Some(&serial_label));
        notebook.append_page(&ssh_panel.container, Some(&ssh_label));

        // Barre de boutons sous les onglets
        let button_bar = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(4)
            .halign(gtk4::Align::End)
            .build();

        let connect_button = Button::builder()
            .label("Se connecter")
            .icon_name("network-wired-symbolic")
            .build();
        connect_button.add_css_class("suggested-action");

        let clear_button = Button::builder()
            .label("Effacer")
            .icon_name("edit-clear-all-symbolic")
            .build();
        clear_button.add_css_class("flat");

        button_bar.append(&clear_button);
        button_bar.append(&connect_button);

        container.append(&notebook);
        container.append(&button_bar);

        Self {
            container,
            notebook,
            serial_panel,
            ssh_panel,
            connect_button,
            clear_button,
        }
    }

    /// Indique si l'onglet série est sélectionné.
    pub fn is_serial_selected(&self) -> bool {
        self.notebook.current_page() == Some(0)
    }

    /// Met à jour le texte du bouton selon l'état de connexion.
    pub fn set_connected(&self, connected: bool) {
        if connected {
            self.connect_button.set_label("Se déconnecter");
            self.connect_button
                .set_icon_name("network-offline-symbolic");
            self.connect_button.remove_css_class("suggested-action");
            self.connect_button.add_css_class("destructive-action");
        } else {
            self.connect_button.set_label("Se connecter");
            self.connect_button.set_icon_name("network-wired-symbolic");
            self.connect_button.remove_css_class("destructive-action");
            self.connect_button.add_css_class("suggested-action");
        }
    }
}
