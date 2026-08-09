use tomato::ui::{self, window};

use gtk4::prelude::*;

fn main() {
    let app = libadwaita::Application::builder()
        .application_id("dev.aamn.tomato")
        .build();

    app.connect_startup(|_| {
        libadwaita::StyleManager::default().set_color_scheme(libadwaita::ColorScheme::ForceDark);
        // Keep startup path in sync with the saved window opacity; the
        // actual opacity is re-applied in window::build once config is loaded
        // so no extra arg is needed here.
        ui::install_theme();
    });

    app.connect_activate(|app| {
        window::build(app);
    });

    app.run();
}
