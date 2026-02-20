// =============================================================================
// Fichier : window.rs
// Rôle    : Fenêtre principale — orchestre tous les composants
// =============================================================================

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gtk4::prelude::*;
use gtk4::{gio, glib, Box as GtkBox, FileDialog, Orientation};
use libadwaita::prelude::*;
use tokio::runtime::Runtime;

use crate::core::connection::{
    spawn_connection_actor, Connection, ConnectionCommand, ConnectionEvent, ConnectionType,
};
use crate::core::serial_manager::{SerialConfig, SerialManager};
use crate::core::settings::{SettingsManager, SshFavorite};
use crate::core::ssh_manager::{SshAuthMethod, SshConfig, SshManager};
use crate::ui::connection_panel::ConnectionPanel;
use crate::ui::header_bar::AppHeaderBar;
use crate::ui::input_panel::InputPanel;
use crate::ui::terminal_panel::TerminalPanel;
use crate::ui::theme::{Theme, ThemeManager};
use crate::ui::tools_dialog::open_tools_dialog;

/// Fenêtre principale de l'application `SerialSSHTerm`.
pub struct MainWindow {
    pub window: libadwaita::ApplicationWindow,
    pub header: AppHeaderBar,
    pub connection_panel: ConnectionPanel,
    pub terminal: TerminalPanel,
    pub input: InputPanel,
    settings: Rc<RefCell<SettingsManager>>,
    connection_tx: RefCell<Option<tokio::sync::mpsc::Sender<ConnectionCommand>>>,
    runtime: Arc<Runtime>,
    /// Overlay Adwaita pour les notifications non-bloquantes (Toast).
    toast_overlay: libadwaita::ToastOverlay,
}

impl MainWindow {
    /// Construit et affiche la fenêtre principale.
    #[allow(clippy::too_many_lines)]
    pub fn new(app: &libadwaita::Application) -> Rc<Self> {
        let settings = Rc::new(RefCell::new(SettingsManager::new()));
        let s = settings.borrow();

        let runtime = Arc::new(Runtime::new().expect("Impossible de créer le runtime Tokio"));

        let window = libadwaita::ApplicationWindow::builder()
            .application(app)
            .title("SerialSSHTerm")
            .default_width(s.settings().ui.window_width)
            .default_height(s.settings().ui.window_height)
            .build();
        drop(s);

        // Composants UI
        let header = AppHeaderBar::new();
        let connection_panel = ConnectionPanel::new();
        let terminal = TerminalPanel::new(settings.borrow().settings().ui.max_scrollback_lines);
        let input = InputPanel::new();

        // Layout principal vertical
        let main_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(0)
            .build();

        // Création de la barre de menu (MenuBar)
        let menubar_model = gio::Menu::new();
        
        let file_menu = gio::Menu::new();
        file_menu.append(Some("Sauvegarder les logs"), Some("win.save-logs"));
        file_menu.append(Some("Quitter"), Some("win.close"));
        menubar_model.append_submenu(Some("Fichier"), &file_menu);

        let edit_menu = gio::Menu::new();
        edit_menu.append(Some("Effacer le terminal"), Some("win.clear-terminal"));
        menubar_model.append_submenu(Some("Édition"), &edit_menu);

        let tools_menu = gio::Menu::new();
        tools_menu.append(Some("Calculatrice & Convertisseur"), Some("win.open-tools"));
        menubar_model.append_submenu(Some("Outils"), &tools_menu);

        let help_menu = gio::Menu::new();
        help_menu.append(Some("À propos"), Some("win.about"));
        menubar_model.append_submenu(Some("Aide"), &help_menu);

        let menu_bar = gtk4::PopoverMenuBar::from_model(Some(&menubar_model));
        main_box.append(&menu_bar);

        main_box.append(&connection_panel.container);

        let separator = gtk4::Separator::new(Orientation::Horizontal);
        main_box.append(&separator);

        main_box.append(&terminal.container);

        let separator2 = gtk4::Separator::new(Orientation::Horizontal);
        main_box.append(&separator2);

        main_box.append(&input.container);

        // Assembler la fenêtre avec ToastOverlay + ToolbarView
        let toast_overlay = libadwaita::ToastOverlay::new();
        toast_overlay.set_child(Some(&main_box));

        let toolbar_view = libadwaita::ToolbarView::new();
        toolbar_view.add_top_bar(&header.header_bar);
        toolbar_view.set_content(Some(&toast_overlay));
        window.set_content(Some(&toolbar_view));

        // Appliquer le thème initial
        let theme = Theme::from_str_name(&settings.borrow().settings().ui.theme);
        ThemeManager::apply(theme);

        let main_win = Rc::new(Self {
            window,
            header,
            connection_panel,
            terminal,
            input,
            settings,
            connection_tx: RefCell::new(None),
            runtime,
            toast_overlay,
        });

        // Restaurer les paramètres persistés dans les widgets UI
        {
            let settings = main_win.settings.borrow();
            let serial = &settings.settings().serial;
            main_win.connection_panel.serial_panel.apply_settings(
                serial.baudrate,
                serial.data_bits,
                &serial.parity,
                serial.stop_bits,
                &serial.flow_control,
            );

            // Rafraîchir puis restaurer le port précédemment sélectionné
            main_win.connection_panel.serial_panel.refresh_ports();
            main_win
                .connection_panel
                .serial_panel
                .select_port_by_device(&settings.settings().serial.port);

            let ssh = &settings.settings().ssh;
            main_win.connection_panel.ssh_panel.apply_settings(
                &ssh.host,
                ssh.port,
                &ssh.username,
                &ssh.key_path,
            );
            main_win
                .connection_panel
                .ssh_panel
                .set_favorites(&settings.settings().ssh_favorites);
        }

        // Message de bienvenue
        main_win
            .terminal
            .append_system("Bienvenue dans SerialSSHTerm !");
        main_win.terminal.append_system(
            "Sélectionnez un mode de connexion (Série ou SSH) et cliquez sur Connecter.",
        );

        // Initialiser le dropdown de fin de ligne depuis les paramètres
        {
            let le = main_win.settings.borrow().settings().ui.line_ending.clone();
            let idx = match le.as_str() {
                "CR" => 1,
                "CRLF" => 2,
                "None" => 3,
                _ => 0, // LF par défaut
            };
            main_win.input.line_ending_dropdown.set_selected(idx);
        }

        // Connecter les signaux
        Self::setup_actions(&main_win);
        Self::setup_signals(&main_win);

        main_win.window.present();
        main_win
    }

    // =========================================================================
    // Actions GIO (menu, raccourcis)
    // =========================================================================

    fn setup_actions(win: &Rc<Self>) {
        // Action : changer de thème
        let theme_action = gio::SimpleAction::new_stateful(
            "set-theme",
            Some(&String::static_variant_type()),
            &"dark".to_variant(),
        );
        {
            let w = win.clone();
            theme_action.connect_activate(move |action, param| {
                if let Some(theme_name) = param.and_then(gtk4::glib::Variant::get::<String>) {
                    let theme = Theme::from_str_name(&theme_name);
                    ThemeManager::apply(theme);
                    action.set_state(&theme_name.to_variant());
                    w.settings.borrow_mut().set_theme(theme.id());
                    w.terminal
                        .append_system(&format!("Thème changé : {}", theme.display_name()));
                }
            });
        }
        win.window.add_action(&theme_action);

        // Action : sauvegarder les logs
        let save_action = gio::SimpleAction::new("save-logs", None);
        {
            let w = win.clone();
            save_action.connect_activate(move |_, _| {
                w.save_logs();
            });
        }
        win.window.add_action(&save_action);

        // Action : ouvrir le menu Outils
        let tools_action = gio::SimpleAction::new("open-tools", None);
        {
            let w = win.clone();
            tools_action.connect_activate(move |_, _| {
                open_tools_dialog(&w.window);
            });
        }
        win.window.add_action(&tools_action);

        // Action : effacer le terminal
        let clear_action = gio::SimpleAction::new("clear-terminal", None);
        {
            let w = win.clone();
            clear_action.connect_activate(move |_, _| {
                w.terminal.clear();
                w.terminal.append_system("Terminal effacé.");
            });
        }
        win.window.add_action(&clear_action);

        // Action : à propos
        let about_action = gio::SimpleAction::new("about", None);
        {
            let w = win.clone();
            about_action.connect_activate(move |_, _| {
                let about = libadwaita::AboutDialog::builder()
                    .application_name("SerialSSHTerm")
                    .version("1.0.0")
                    .developer_name("M@nu")
                    .comments(
                        "Terminal série et SSH professionnel\nÉcrit en Rust + GTK4/Libadwaita",
                    )
                    .license_type(gtk4::License::MitX11)
                    .website("https://github.com/weedmanu/SerialSSHTerm")
                    .application_icon("utilities-terminal")
                    .build();
                about.present(Some(&w.window.clone().upcast::<gtk4::Widget>()));
            });
        }
        win.window.add_action(&about_action);

        // Action : quitter
        let close_action = gio::SimpleAction::new("close", None);
        {
            let w = win.clone();
            close_action.connect_activate(move |_, _| {
                w.window.close();
            });
        }
        win.window.add_action(&close_action);

        // Raccourcis clavier
        let app = win
            .window
            .application()
            .expect("Window doit avoir une application");
        app.set_accels_for_action("win.save-logs", &["<Ctrl>s"]);
        app.set_accels_for_action("win.clear-terminal", &["<Ctrl>l"]);
        app.set_accels_for_action("win.open-tools", &["<Ctrl>t"]);
    }

    // =========================================================================
    // Signaux (boutons, entrées, etc.)
    // =========================================================================

    #[allow(clippy::too_many_lines)]
    fn setup_signals(win: &Rc<Self>) {
        // Bouton Connecter / Déconnecter
        {
            let w = win.clone();
            win.connection_panel
                .connect_button
                .connect_clicked(move |_| {
                    w.toggle_connection();
                });
        }

        // Bouton Effacer
        {
            let w = win.clone();
            win.connection_panel.clear_button.connect_clicked(move |_| {
                w.terminal.clear();
                w.terminal.append_system("Terminal effacé.");
            });
        }

        // Bouton Rafraîchir les ports série
        {
            let w = win.clone();
            win.connection_panel
                .serial_panel
                .refresh_button
                .connect_clicked(move |_| {
                    w.connection_panel.serial_panel.refresh_ports();
                    w.terminal.append_system("Ports série rafraîchis.");
                });
        }

        // Bouton Envoyer
        {
            let w = win.clone();
            win.input.send_button.connect_clicked(move |_| {
                w.send_data();
            });
        }

        // Entrée : Envoi sur Enter
        {
            let w = win.clone();
            win.input.entry.connect_activate(move |_| {
                w.send_data();
            });
        }

        // Bouton Sauvegarder logs (header bar)
        {
            let w = win.clone();
            win.header.save_log_button.connect_clicked(move |_| {
                w.save_logs();
            });
        }

        // Synchroniser le dropdown de fin de ligne avec les paramètres
        {
            let w = win.clone();
            win.input
                .line_ending_dropdown
                .connect_selected_notify(move |dropdown| {
                    let le_str = match dropdown.selected() {
                        1 => "CR",
                        2 => "CRLF",
                        3 => "None",
                        _ => "LF",
                    };
                    w.settings.borrow_mut().set_line_ending(le_str);
                });
        }

        // Case à cocher : arrêt du défilement automatique
        {
            let terminal = win.terminal.text_view.clone();
            let w = win.clone();
            win.input
                .stop_scroll_checkbox
                .connect_toggled(move |checkbox| {
                    let auto_scroll = !checkbox.is_active();
                    w.terminal.set_auto_scroll_enabled(auto_scroll);
                    if auto_scroll {
                        let end_mark = w.terminal.buffer.create_mark(
                            None,
                            &w.terminal.buffer.end_iter(),
                            false,
                        );
                        terminal.scroll_to_mark(&end_mark, 0.0, false, 0.0, 1.0);
                        w.terminal.buffer.delete_mark(&end_mark);
                    }
                });
        }

        // Parcourir clé SSH
        {
            let w = win.clone();
            win.connection_panel
                .ssh_panel
                .key_browse_button
                .connect_clicked(move |_| {
                    let dialog = FileDialog::builder()
                        .title("Sélectionner la clé SSH")
                        .build();

                    let key_entry = w.connection_panel.ssh_panel.key_path_entry.clone();
                    dialog.open(Some(&w.window), gio::Cancellable::NONE, move |result| {
                        if let Ok(file) = result {
                            if let Some(path) = file.path() {
                                key_entry.set_text(&path.to_string_lossy());
                            }
                        }
                    });
                });
        }

        // Ajouter aux favoris SSH
        {
            let w = win.clone();
            win.connection_panel
                .ssh_panel
                .add_favorite_button
                .connect_clicked(move |_| {
                    w.add_current_ssh_favorite();
                });
        }

        // Appliquer un favori SSH sélectionné
        {
            let w = win.clone();
            win.connection_panel
                .ssh_panel
                .favorite_dropdown
                .connect_selected_notify(move |_| {
                    w.apply_selected_ssh_favorite();
                });
        }

        // Sauvegarder la taille de fenêtre à la fermeture
        {
            let w = win.clone();
            win.window.connect_close_request(move |window| {
                let (width, height) = (window.width(), window.height());
                w.settings.borrow_mut().set_window_size(width, height);
                let _ = w.settings.borrow().save();

                // Déconnecter proprement
                if let Some(tx) = w.connection_tx.borrow_mut().take() {
                    let _ = tx.try_send(ConnectionCommand::Disconnect);
                }

                log::info!("Application fermée proprement.");
                glib::Propagation::Proceed
            });
        }
    }

    // =========================================================================
    // Logique métier
    // =========================================================================

    /// Bascule connexion / déconnexion.
    fn toggle_connection(self: &Rc<Self>) {
        let is_connected = self.connection_tx.borrow().is_some();

        if is_connected {
            self.disconnect();
        } else {
            self.connect();
        }
    }

    /// Établit la connexion (série ou SSH) selon l'onglet actif.
    ///
    /// Architecture :
    ///  - Le manager est construit (validation) sur le thread GTK.
    ///  - La connexion effective a lieu dans une tâche tokio (via `spawn_connection_actor`).
    ///  - Le timer `GLib` (20 ms) pompe les événements : `HostKeyUnknown`, Connected, Data...
    ///  - Cela libère le thread GTK pendant la connexion SSH (`check_server_key`, auth).
    fn connect(self: &Rc<Self>) {
        // Validation + construction du manager (sans connexion).
        let manager: Box<dyn Connection> = match if self.connection_panel.is_serial_selected() {
            self.build_serial_manager()
        } else {
            self.build_ssh_manager()
        } {
            Ok(m) => m,
            Err(e) => {
                self.header.set_status("Erreur de configuration", false);
                self.terminal.append_error(&e);
                self.show_toast(&format!("⚠ {e}"));
                log::error!("Erreur de configuration : {e}");
                return;
            }
        };

        // Indiquer à l'UI que la connexion est en cours.
        self.header.set_status("Connexion en cours...", false);
        self.terminal.append_system("Connexion en cours...");

        // Lancer l'acteur de connexion dans le runtime tokio.
        // `runtime.enter()` établit le contexte tokio pour `tokio::spawn`
        //  sans bloquer le thread GTK (contrairement à `block_on`).
        let guard = self.runtime.enter();
        let (cmd_tx, event_rx) = spawn_connection_actor(manager);
        drop(guard);

        *self.connection_tx.borrow_mut() = Some(cmd_tx);

        // Pont async_channel → GTK main loop via GLib timer (20 ms)
        // SOLID : aucune dépendance GTK dans le core.
        let this = self.clone();
        glib::timeout_add_local(
            std::time::Duration::from_millis(20),
            move || {
                loop {
                    match event_rx.try_recv() {
                        Ok(ConnectionEvent::Connected { conn_type, description }) => {
                            let type_label = match conn_type {
                                ConnectionType::Serial => "Série",
                                ConnectionType::Ssh => "SSH",
                            };
                            this.connection_panel.set_connected(true);
                            this.header.set_status(
                                &format!("Connecté {type_label} — {description}"),
                                true,
                            );
                            this.terminal
                                .append_system(&format!("Connecté [{type_label}] {description}"));
                            this.input.grab_focus();
                        }
                        Ok(ConnectionEvent::HostKeyUnknown {
                            host,
                            key_type,
                            fingerprint,
                            is_key_changed,
                            decision_tx,
                        }) => {
                            // Afficher le dialogue de vérification de clé SSH.
                            // Le timer CONTINUE de tourner pendant que l'utilisateur répond.
                            show_host_key_dialog(
                                &this.window,
                                &host,
                                &key_type,
                                &fingerprint,
                                is_key_changed,
                                decision_tx,
                            );
                        }
                        Ok(ConnectionEvent::DataReceived(data)) => {
                            this.terminal.append_ansi(&data);
                        }
                        Ok(ConnectionEvent::Error(e)) => {
                            this.terminal.append_error(&e);
                            this.handle_disconnect();
                            return glib::ControlFlow::Break;
                        }
                        Err(async_channel::TryRecvError::Empty) => break,
                        Ok(ConnectionEvent::Disconnected)
                        | Err(async_channel::TryRecvError::Closed) => {
                            this.handle_disconnect();
                            return glib::ControlFlow::Break;
                        }
                    }
                }
                glib::ControlFlow::Continue
            },
        );
    }

    /// Traite la déconnexion — idempotente.
    ///
    /// Peut être appelée depuis :
    ///   - l'UI (bouton déconnecter) via `disconnect()`
    ///   - le timer `GLib` quand l'acteur signale Disconnected/Error/Closed
    ///
    /// Sécurité : le `take()` de `connection_tx` est atomique (thread GTK
    /// unique) et garantit qu'aucun appel simultané ne met à jour l'UI deux fois.
    fn handle_disconnect(&self) {
        // `take()` retire le sender : seul le premier appelant obtient Some.
        let had_connection = self.connection_tx.borrow().is_some();
        if let Some(tx) = self.connection_tx.borrow_mut().take() {
            // Informer l'acteur de se terminer (peut échouer si déjà fermé — normal).
            if tx.try_send(ConnectionCommand::Disconnect).is_err() {
                log::debug!("Acteur déjà fermé lors de handle_disconnect");
            }
        }
        // Mettre à jour l'UI seulement si la connexion était active.
        // (Prévient les messages 'Déconnecté' dupliquement en cas d'appels successifs.)
        if had_connection {
            self.connection_panel.set_connected(false);
            self.header.set_status("Déconnecté", false);
            self.terminal.append_system("Déconnecté");
            self.show_toast("Connexion terminée");
        }
    }

    /// Affiche une notification toast Adwaita non-bloquante (3 s par défaut).
    ///
    /// À utiliser pour les confirmations et erreurs transientes.
    /// Les erreurs critiques persistantes doivent utiliser `terminal.append_error()`.
    pub fn show_toast(&self, message: &str) {
        let toast = libadwaita::Toast::new(message);
        toast.set_timeout(3);
        self.toast_overlay.add_toast(toast);
    }


    /// Construit le manager série à partir de l'UI.
    /// La connexion effective est établie par `spawn_connection_actor`.
    fn build_serial_manager(&self) -> Result<Box<dyn Connection>, String> {
        let sp = &self.connection_panel.serial_panel;
        let port = sp
            .selected_port()
            .ok_or_else(|| "Aucun port sélectionné".to_string())?;

        let config = SerialConfig::from_params(
            &port,
            sp.selected_baudrate(),
            sp.selected_data_bits(),
            &sp.selected_parity(),
            sp.selected_stop_bits(),
            &sp.selected_flow_control(),
            self.settings.borrow().settings().serial.timeout_ms,
        );

        // Sauvegarder les paramètres série
        {
            let mut sm = self.settings.borrow_mut();
            let serial = &mut sm.settings_mut().serial;
            serial.port = port;
            serial.baudrate = sp.selected_baudrate();
            serial.data_bits = sp.selected_data_bits();
            serial.parity = sp.selected_parity();
            serial.stop_bits = sp.selected_stop_bits();
            serial.flow_control = sp.selected_flow_control();
            if let Err(e) = sm.save() {
                log::warn!("Impossible de sauvegarder les paramètres série : {e}");
            }
        }

        Ok(Box::new(SerialManager::new(config)))
    }

    /// Construit le manager SSH à partir de l'UI.
    /// La connexion effective (TCP + handshake + auth + `known_hosts`) est
    /// établie par `spawn_connection_actor` dans une tâche tokio.
    fn build_ssh_manager(&self) -> Result<Box<dyn Connection>, String> {
        let sp = &self.connection_panel.ssh_panel;
        let host = sp.host();
        let port = sp.port();
        let username = sp.username();
        let password = sp.password();
        let key_path = sp.key_path();

        if host.is_empty() || username.is_empty() {
            return Err("L'hôte et l'utilisateur sont requis.".to_string());
        }

        let auth_method = if key_path.is_empty() {
            SshAuthMethod::Password(password)
        } else {
            SshAuthMethod::KeyFile {
                private_key_path: key_path.clone(),
                passphrase: None,
            }
        };

        let config = SshConfig {
            host: host.clone(),
            port,
            username: username.clone(),
            auth_method,
            connect_timeout_secs: 10,
        };

        // Sauvegarder les paramètres SSH
        {
            let mut sm = self.settings.borrow_mut();
            let ssh = &mut sm.settings_mut().ssh;
            ssh.host = host;
            ssh.port = port;
            ssh.username = username;
            ssh.auth_method = if key_path.is_empty() {
                "password".to_string()
            } else {
                "key".to_string()
            };
            ssh.key_path = key_path;
            if let Err(e) = sm.save() {
                log::warn!("Impossible de sauvegarder les paramètres SSH : {e}");
            }
        }

        Ok(Box::new(SshManager::new(config)))
    }

    /// Ajoute ou met à jour le profil SSH courant dans les favoris persistés.
    fn add_current_ssh_favorite(&self) {
        let sp = &self.connection_panel.ssh_panel;
        let host = sp.host();
        let port = sp.port();
        let username = sp.username();
        let key_path = sp.key_path();

        if host.is_empty() || username.is_empty() {
            self.terminal
                .append_error("Favori SSH: hôte et utilisateur requis.");
            return;
        }

        let auth_method = if key_path.is_empty() {
            "password".to_string()
        } else {
            "key".to_string()
        };

        let favorite = SshFavorite {
            name: format!("{username}@{host}:{port}"),
            host,
            port,
            username,
            auth_method,
            key_path,
        };

        let mut settings = self.settings.borrow_mut();
        let favorites = &mut settings.settings_mut().ssh_favorites;

        if let Some(existing) = favorites.iter_mut().find(|f| {
            f.host == favorite.host && f.port == favorite.port && f.username == favorite.username
        }) {
            *existing = favorite.clone();
            self.show_toast(&format!("✓ Favori mis à jour : {}", favorite.name));
            self.terminal
                .append_system(&format!("Favori SSH mis à jour : {}", favorite.name));
        } else {
            favorites.push(favorite.clone());
            self.show_toast(&format!("✓ Favori ajouté : {}", favorite.name));
            self.terminal
                .append_system(&format!("Favori SSH ajouté : {}", favorite.name));
        }

        if let Err(e) = settings.save() {
            self.terminal
                .append_error(&format!("Impossible de sauvegarder les favoris SSH : {e}"));
            return;
        }

        let refreshed = settings.settings().ssh_favorites.clone();
        drop(settings);
        self.connection_panel.ssh_panel.set_favorites(&refreshed);
    }

    /// Applique les champs SSH depuis le favori sélectionné.
    fn apply_selected_ssh_favorite(&self) {
        let Some(favorite) = self.connection_panel.ssh_panel.selected_favorite() else {
            return;
        };

        self.connection_panel.ssh_panel.apply_settings(
            &favorite.host,
            favorite.port,
            &favorite.username,
            &favorite.key_path,
        );
        self.connection_panel.ssh_panel.clear_password();

        self.terminal
            .append_system(&format!("Favori SSH chargé : {}", favorite.name));
    }

    /// Déconnexion propre initiée par l'utilisateur.
    /// Délègue à `handle_disconnect()` qui envoie la commande et met à jour l'UI.
    fn disconnect(&self) {
        self.handle_disconnect();
    }

    /// Envoie les données saisies à la connexion active.
    fn send_data(&self) {
        let text = self.input.get_text();
        if text.is_empty() {
            return;
        }

        let line_ending = self.input.selected_line_ending();
        let data = format!("{text}{line_ending}");

        if let Some(tx) = self.connection_tx.borrow().as_ref() {
            if let Err(e) = tx.try_send(ConnectionCommand::SendData(data.into_bytes())) {
                self.terminal.append_error(&format!("Erreur d'envoi : {e}"));
            } else {
                self.terminal.append_sent(&format!("→ {text}\n"));
                self.input.clear();
                self.input.grab_focus();
            }
        } else {
            self.terminal
                .append_error("Non connecté — impossible d'envoyer.");
        }
    }

    /// Sauvegarde les logs dans un fichier.
    fn save_logs(&self) {
        let text = self.terminal.get_text();
        if text.is_empty() {
            self.terminal.append_system("Rien à sauvegarder.");
            return;
        }

        let dialog = FileDialog::builder()
            .title("Sauvegarder les logs")
            .initial_name(format!(
                "serial_ssh_log_{}.txt",
                chrono::Local::now().format("%Y%m%d_%H%M%S")
            ))
            .build();

        let terminal_buffer = self.terminal.buffer.clone();
        let term_text_view = self.terminal.text_view.clone();
        let sys_tag = terminal_buffer.tag_table().lookup("system");
        let toast_overlay = self.toast_overlay.clone();

        dialog.save(Some(&self.window), gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    let content = terminal_buffer
                        .text(
                            &terminal_buffer.start_iter(),
                            &terminal_buffer.end_iter(),
                            false,
                        )
                        .to_string();
                    match std::fs::write(&path, &content) {
                        Ok(()) => {
                            log::info!("Logs sauvegardés dans {}", path.display());
                            // Toast de confirmation non-bloquant
                            let toast = libadwaita::Toast::new(
                                &format!("✓ Logs sauvegardés : {}", path.display())
                            );
                            toast.set_timeout(4);
                            toast_overlay.add_toast(toast);
                            let msg = format!(
                                "\n[{}] Logs sauvegardés dans {}\n",
                                chrono::Local::now().format("%H:%M:%S"),
                                path.display()
                            );
                            let mut end = terminal_buffer.end_iter();
                            if let Some(ref tag) = sys_tag {
                                terminal_buffer.insert_with_tags(&mut end, &msg, &[tag]);
                            } else {
                                terminal_buffer.insert(&mut end, &msg);
                            }
                            let end_mark = terminal_buffer.create_mark(
                                None,
                                &terminal_buffer.end_iter(),
                                false,
                            );
                            term_text_view.scroll_to_mark(&end_mark, 0.0, false, 0.0, 1.0);
                            terminal_buffer.delete_mark(&end_mark);
                        }
                        Err(e) => {
                            log::error!("Erreur de sauvegarde : {e}");
                        }
                    }
                }
            }
        });
    }
}
// =============================================================================
// Dialogue de vérification de clé SSH (hors impl MainWindow)
// =============================================================================

/// Affiche un dialogue `adw::AlertDialog` pour la vérification TOFU de la clé SSH.
///
/// Ce dialogue est non-bloquant : le thread GTK continue, le timer `GLib`
/// continue de pomper les événements. Quand l'utilisateur répond, `decision_tx`
/// est renseigné → la tâche tokio SSH continue ou abandonne.
///
/// Sécurité : le bouton "Rejeter" est le choix par défaut.
/// Si la clé a changé (risque MITM), le bouton "Accepter" est rouge.
fn show_host_key_dialog(
    parent: &libadwaita::ApplicationWindow,
    host: &str,
    key_type: &str,
    fingerprint: &str,
    is_key_changed: bool,
    decision_tx: tokio::sync::oneshot::Sender<bool>,
) {
    let (heading, body) = if is_key_changed {
        (
            "⚠ AVERTISSEMENT : Clé SSH modifiée !".to_string(),
            format!(
                "La clé du serveur {host} a CHANGÉ depuis la dernière connexion.\n\n\
                 Cela peut indiquer une attaque de l'homme du milieu (MITM).\n\n\
                 Type : {key_type}\n\
                 Empreinte SHA256 : {fingerprint}\n\n\
                 Voulez-vous faire confiance à cette nouvelle clé ?"
            ),
        )
    } else {
        (
            format!("Clé SSH inconnue — {host}"),
            format!(
                "Le serveur {host} n'est pas encore dans vos hôtes connus.\n\n\
                 Type : {key_type}\n\
                 Empreinte SHA256 : {fingerprint}\n\n\
                 Voulez-vous faire confiance à ce serveur et enregistrer sa clé ?"
            ),
        )
    };

    let dialog = libadwaita::AlertDialog::new(Some(&heading), Some(&body));
    dialog.add_response("reject", "Rejeter");
    dialog.add_response("accept", "Accepter");
    // Par sécurité : le refus est la réponse par défaut.
    dialog.set_default_response(Some("reject"));
    // Clé changée = action destructive (rouge) ; hôte nouveau = action suggérée (bleu).
    let appearance = if is_key_changed {
        libadwaita::ResponseAppearance::Destructive
    } else {
        libadwaita::ResponseAppearance::Suggested
    };
    dialog.set_response_appearance("accept", appearance);

    let decision_tx = std::rc::Rc::new(std::cell::RefCell::new(Some(decision_tx)));
    dialog.connect_response(None, move |_, response| {
        let accepted = response == "accept";
        if let Some(tx) = decision_tx.borrow_mut().take() {
            if let Err(e) = tx.send(accepted) {
                log::warn!("SSH : impossible d'envoyer la décision host-key : {e:?}");
            }
        }
    });

    dialog.present(Some(parent));
}