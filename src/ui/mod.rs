pub mod css;
pub mod settings_page;
pub mod tasks_page;
pub mod timer_page;
pub mod window;

use std::cell::RefCell;

thread_local! {
    static PROVIDER: RefCell<Option<gtk4::CssProvider>> = const { RefCell::new(None) };
}

thread_local! {
    static OPACITY: RefCell<f64> = const { RefCell::new(0.88) };
}

fn current_sheet(dark: bool, opacity: f64) -> String {
    if dark {
        css::dark_sheet_with_opacity(opacity)
    } else {
        css::light_sheet_with_opacity(opacity)
    }
}

/// Install the app stylesheet for the current color scheme. Call once at
/// startup; the provider is kept so later `reload_theme` calls swap data on it.
pub fn install_theme() {
    let opacity = OPACITY.with(|o| *o.borrow());
    install_theme_with_opacity(opacity);
}

pub fn install_theme_with_opacity(opacity: f64) {
    OPACITY.with(|o| *o.borrow_mut() = opacity.clamp(0.30, 1.0));
    let provider = gtk4::CssProvider::new();
    let dark = libadwaita::StyleManager::default().is_dark();
    let data = current_sheet(dark, opacity);
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
    let opacity = OPACITY.with(|o| *o.borrow());
    reload_theme_with_opacity(opacity);
}

pub fn reload_theme_with_opacity(opacity: f64) {
    OPACITY.with(|o| *o.borrow_mut() = opacity.clamp(0.30, 1.0));
    PROVIDER.with(|p| {
        if let Some(provider) = p.borrow().as_ref() {
            let dark = libadwaita::StyleManager::default().is_dark();
            let data = current_sheet(dark, opacity);
            provider.load_from_data(&data);
        }
    });
}
