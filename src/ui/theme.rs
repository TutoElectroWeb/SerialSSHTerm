// =============================================================================
// Fichier : theme.rs
// Rôle    : Gestionnaire de thèmes (Clair, Sombre, Hacker)
// =============================================================================

use gtk4::CssProvider;

/// Thèmes disponibles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
    Hacker,
}

impl Theme {
    /// Convertit depuis une chaîne.
    pub fn from_str_name(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "light" | "clair" => Self::Light,
            "hacker" | "matrix" => Self::Hacker,
            _ => Self::Dark,
        }
    }

    /// Nom d'affichage.
    pub const fn display_name(&self) -> &str {
        match self {
            Self::Light => "Clair",
            Self::Dark => "Sombre",
            Self::Hacker => "Hacker",
        }
    }

    /// Nom technique.
    pub const fn id(&self) -> &str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::Hacker => "hacker",
        }
    }

    /// Liste de tous les thèmes.
    pub const fn all() -> &'static [Self] {
        &[Self::Light, Self::Dark, Self::Hacker]
    }
}

/// Gestionnaire de thèmes pour l'application.
pub struct ThemeManager;

impl ThemeManager {
    /// Applique le thème sélectionné à l'application.
    pub fn apply(theme: Theme) {
        // Configurer le color scheme Adwaita
        let style_manager = libadwaita::StyleManager::default();
        match theme {
            Theme::Light => {
                style_manager.set_color_scheme(libadwaita::ColorScheme::ForceLight);
            }
            Theme::Dark | Theme::Hacker => {
                style_manager.set_color_scheme(libadwaita::ColorScheme::ForceDark);
            }
        }

        // CSS personnalisé par thème
        let css = Self::css_for_theme(theme);
        let provider = CssProvider::new();
        provider.load_from_string(&css);

        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        log::info!("Thème appliqué : {}", theme.display_name());
    }

    /// Génère le CSS personnalisé pour un thème donné.
    fn css_for_theme(theme: Theme) -> String {
        match theme {
            Theme::Light => r#"
                .terminal-view {
                    background-color: #fafafa;
                    color: #2e2e2e;
                    font-family: "Monospace";
                    font-size: 11pt;
                    padding: 8px;
                }
                .input-entry {
                    font-family: "Monospace";
                    font-size: 11pt;
                    min-height: 36px;
                }
                .connection-panel {
                    padding: 6px 12px;
                }
                .status-connected {
                    color: #26a269;
                    font-weight: bold;
                }
                .status-disconnected {
                    color: #c01c28;
                    font-weight: bold;
                }
            "#
            .to_string(),

            Theme::Dark => r#"
                .terminal-view {
                    background-color: #1e1e2e;
                    color: #cdd6f4;
                    font-family: "Monospace";
                    font-size: 11pt;
                    padding: 8px;
                }
                .input-entry {
                    font-family: "Monospace";
                    font-size: 11pt;
                    min-height: 36px;
                }
                .connection-panel {
                    padding: 6px 12px;
                }
                .status-connected {
                    color: #a6e3a1;
                    font-weight: bold;
                }
                .status-disconnected {
                    color: #f38ba8;
                    font-weight: bold;
                }
            "#
            .to_string(),

            Theme::Hacker => r#"
                .terminal-view {
                    background-color: #0a0a0a;
                    color: #00ff41;
                    font-family: "Monospace";
                    font-size: 11pt;
                    padding: 8px;
                    text-shadow: 0 0 3px rgba(0, 255, 65, 0.3);
                }
                .input-entry {
                    font-family: "Monospace";
                    font-size: 11pt;
                    min-height: 36px;
                    color: #00ff41;
                }
                .connection-panel {
                    padding: 6px 12px;
                }
                .status-connected {
                    color: #00ff41;
                    font-weight: bold;
                }
                .status-disconnected {
                    color: #ff3333;
                    font-weight: bold;
                }
                .hacker-title {
                    color: #00ff41;
                    font-weight: bold;
                }
            "#
            .to_string(),
        }
    }
}
