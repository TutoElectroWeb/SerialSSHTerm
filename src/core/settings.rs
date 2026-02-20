// =============================================================================
// Fichier : settings.rs
// Rôle    : Gestion de la configuration persistante (JSON)
// =============================================================================

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// =============================================================================
// Structures de configuration
// =============================================================================

/// Configuration complète de l'application.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub serial: SerialSettings,
    pub ssh: SshSettings,
    pub ssh_favorites: Vec<SshFavorite>,
    pub ui: UiSettings,
    pub log: LogSettings,
}

/// Favori SSH enregistrable pour réutilisation rapide.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SshFavorite {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: String,
    pub key_path: String,
}

/// Paramètres de connexion série.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SerialSettings {
    pub port: String,
    pub baudrate: u32,
    pub data_bits: u8,
    pub parity: String,
    pub stop_bits: u8,
    pub flow_control: String,
    pub timeout_ms: u64,
}

/// Paramètres de connexion SSH.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SshSettings {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: String, // "password" | "key"
    pub key_path: String,
}

/// Paramètres d'interface utilisateur.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    pub theme: String, // "light" | "dark" | "hacker"
    pub font_family: String,
    pub font_size: u32,
    pub window_width: i32,
    pub window_height: i32,
    pub show_line_numbers: bool,
    pub max_scrollback_lines: u32,
    pub line_ending: String, // "LF" | "CR" | "CRLF"
}

/// Paramètres de logging.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LogSettings {
    pub enabled: bool,
    pub level: String,
    pub log_to_file: bool,
    pub log_directory: String,
}

// =============================================================================
// Implémentations par défaut
// =============================================================================

impl Default for SerialSettings {
    fn default() -> Self {
        Self {
            port: String::new(),
            baudrate: 115_200,
            data_bits: 8,
            parity: "None".to_string(),
            stop_bits: 1,
            flow_control: "None".to_string(),
            timeout_ms: 1000,
        }
    }
}

impl Default for SshSettings {
    fn default() -> Self {
        Self {
            host: "192.168.1.1".to_string(),
            port: 22,
            username: String::new(),
            auth_method: "password".to_string(),
            key_path: String::new(),
        }
    }
}

impl Default for SshFavorite {
    fn default() -> Self {
        Self {
            name: String::new(),
            host: String::new(),
            port: 22,
            username: String::new(),
            auth_method: "password".to_string(),
            key_path: String::new(),
        }
    }
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            font_family: "Monospace".to_string(),
            font_size: 11,
            window_width: 1100,
            window_height: 750,
            show_line_numbers: false,
            max_scrollback_lines: 10000,
            line_ending: "LF".to_string(),
        }
    }
}

impl Default for LogSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            level: "INFO".to_string(),
            log_to_file: false,
            log_directory: "logs".to_string(),
        }
    }
}

// =============================================================================
// Gestionnaire de configuration
// =============================================================================

/// Gestionnaire de configuration avec chargement/sauvegarde JSON.
#[derive(Debug, Clone)]
pub struct SettingsManager {
    settings: AppSettings,
    config_path: PathBuf,
}

impl SettingsManager {
    /// Crée un nouveau gestionnaire en chargeant depuis le chemin par défaut.
    pub fn new() -> Self {
        let config_path = Self::default_config_path();
        let settings = Self::load_from_path(&config_path).unwrap_or_default();
        Self {
            settings,
            config_path,
        }
    }

    /// Chemin par défaut du fichier de configuration.
    fn default_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("serial-ssh-term")
            .join("settings.json")
    }

    /// Charge la configuration depuis un fichier JSON.
    fn load_from_path(path: &PathBuf) -> Result<AppSettings> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Impossible de lire {}", path.display()))?;
        let settings: AppSettings =
            serde_json::from_str(&content).context("Format JSON invalide")?;
        log::info!("Configuration chargée depuis {}", path.display());
        Ok(settings)
    }

    /// Sauvegarde la configuration dans le fichier JSON.
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Impossible de créer {}", parent.display()))?;
        }
        let json =
            serde_json::to_string_pretty(&self.settings).context("Erreur de sérialisation JSON")?;
        fs::write(&self.config_path, json)
            .with_context(|| format!("Impossible d'écrire {}", self.config_path.display()))?;
        log::info!(
            "Configuration sauvegardée dans {}",
            self.config_path.display()
        );
        Ok(())
    }

    /// Accès en lecture aux paramètres.
    pub const fn settings(&self) -> &AppSettings {
        &self.settings
    }

    /// Accès en écriture aux paramètres.
    pub fn settings_mut(&mut self) -> &mut AppSettings {
        &mut self.settings
    }

    /// Met à jour le thème et sauvegarde.
    pub fn set_theme(&mut self, theme: &str) {
        self.settings.ui.theme = theme.to_string();
        let _ = self.save();
    }

    /// Met à jour la taille de fenêtre.
    pub fn set_window_size(&mut self, width: i32, height: i32) {
        self.settings.ui.window_width = width;
        self.settings.ui.window_height = height;
    }

    /// Met à jour la terminaison de ligne.
    pub fn set_line_ending(&mut self, ending: &str) {
        self.settings.ui.line_ending = ending.to_string();
        let _ = self.save();
    }
}
