// =============================================================================
// SerialSSHTerm — Terminal série et SSH professionnel
// =============================================================================
//
// Architecture :
//   core/   — Logique métier (serial, ssh, settings, connection trait)
//   ui/     — Interface GTK4/Libadwaita (window, panels, themes)
//   app.rs  — Bootstrap de l'application
//
// Technologies :
//   Rust + GTK4 + Libadwaita + serialport + ssh2
//
// Auteur : M@nu
// Licence : MIT
// =============================================================================

mod app;
mod core;
mod ui;

fn main() -> glib::ExitCode {
    // Initialiser le logger avec un niveau détaillé
    crate::core::logger::init_logger(log::LevelFilter::Info);
    log::info!("Démarrage de SerialSSHTerm v1.0.0");

    app::run()
}

use gtk4::glib;
