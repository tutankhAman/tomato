use tomato::ui::window;

use gtk4::prelude::*;

fn main() {
    let app = libadwaita::Application::builder()
        .application_id("dev.aamn.tomato")
        .build();

    app.connect_startup(|_| {
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(include_str!("../data/style.css"));
        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        libadwaita::StyleManager::default().set_color_scheme(libadwaita::ColorScheme::ForceDark);
    });

    app.connect_activate(|app| {
        window::build(app);
    });

    app.run();
}
