// =============================================================================
// Fichier : app.rs
// Rôle    : Configuration et lancement de l'application GTK4/Libadwaita
// =============================================================================

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

use crate::ui::window::MainWindow;

/// Construit et lance l'application.
pub fn run() -> glib::ExitCode {
    let app = libadwaita::Application::builder()
        .application_id("com.github.weedmanu.serial-ssh-term")
        .build();

    // Stocker la référence à la fenêtre pour éviter le drop prématuré
    let main_window: Rc<RefCell<Option<Rc<MainWindow>>>> = Rc::new(RefCell::new(None));

    let mw = main_window;
    app.connect_activate(move |app| {
        let win = MainWindow::new(app);
        *mw.borrow_mut() = Some(win);
    });

    app.run()
}

use gtk4::glib;
