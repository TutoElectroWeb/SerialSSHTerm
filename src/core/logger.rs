// =============================================================================
// Fichier : logger.rs
// Rôle    : Initialisation et configuration du système de logging
// =============================================================================

use std::io::Write;

use chrono::Local;
use env_logger::Builder;
use log::LevelFilter;

/// Initialise le système de logging avec un format professionnel.
///
/// Format : `[YYYY-MM-DD HH:MM:SS] LEVEL module - message`
pub fn init_logger(level: LevelFilter) {
    Builder::new()
        .filter_level(level)
        .format(|buf, record| {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let level = record.level();
            let target = record.target();
            writeln!(buf, "[{timestamp}] {level:<5} {target} - {}", record.args())
        })
        .init();
}
