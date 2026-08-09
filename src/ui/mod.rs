pub mod css;
pub mod settings_page;
pub mod tasks_page;
pub mod timer_page;
pub mod window;

use std::cell::RefCell;

thread_local! {
    static PROVIDER: RefCell<Option<gtk4::CssProvider>> = const { RefCell::new(None) };
}

fn current_sheet(dark: bool) -> String {
    if dark {
        css::dark_sheet()
    } else {
        css::light_sheet()
    }
}

/// Install the app stylesheet for the current color scheme. Call once at
/// startup; the provider is kept so later `reload_theme` calls swap data on it.
pub fn install_theme() {
    let provider = gtk4::CssProvider::new();
    let dark = libadwaita::StyleManager::default().is_dark();
    let data = current_sheet(dark);
    provider.load_from_data(&data);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    PROVIDER.with(|p| *p.borrow_mut() = Some(provider));
}

/// Re-load stylesheet data after a color-scheme flip.
pub fn reload_theme() {
    PROVIDER.with(|p| {
        if let Some(provider) = p.borrow().as_ref() {
            let dark = libadwaita::StyleManager::default().is_dark();
            let data = current_sheet(dark);
            provider.load_from_data(&data);
        }
    });
}
