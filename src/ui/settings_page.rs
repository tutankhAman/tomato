use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

use crate::config::Config;

pub fn build<F>(config: Rc<RefCell<Config>>, on_change: F) -> gtk4::Widget
where
    F: Fn() + 'static,
{
    let on_change = Rc::new(on_change);

    let scroller = gtk4::ScrolledWindow::new();
    scroller.set_vexpand(true);
    scroller.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let page = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    page.add_css_class("tomato-page");
    scroller.set_child(Some(&page));

    // ---------- TIMER SETTINGS ----------
    let timer_title = gtk4::Label::new(Some("TIMER SETTINGS"));
    timer_title.set_xalign(0.0);
    timer_title.add_css_class("settings-group-title");
    page.append(&timer_title);

    let cfg = config.borrow();

    // Focus Minutes
    let focus_spin = gtk4::SpinButton::with_range(1.0, 180.0, 1.0);
    focus_spin.set_value(cfg.timer.focus_minutes as f64);
    let row_focus = make_setting_row("Focus (minutes)", &focus_spin);
    page.append(&row_focus);

    // Short Break
    let short_spin = gtk4::SpinButton::with_range(1.0, 60.0, 1.0);
    short_spin.set_value(cfg.timer.short_break_minutes as f64);
    let row_short = make_setting_row("Short Break (minutes)", &short_spin);
    page.append(&row_short);

    // Long Break
    let long_spin = gtk4::SpinButton::with_range(1.0, 60.0, 1.0);
    long_spin.set_value(cfg.timer.long_break_minutes as f64);
    let row_long = make_setting_row("Long Break (minutes)", &long_spin);
    page.append(&row_long);

    // Cycles before Long Break
    let cycles_spin = gtk4::SpinButton::with_range(1.0, 20.0, 1.0);
    cycles_spin.set_value(cfg.timer.cycles_before_long_break as f64);
    let row_cycles = make_setting_row("Cycles before Long Break", &cycles_spin);
    page.append(&row_cycles);

    // Auto-start Breaks
    let auto_break_switch = gtk4::Switch::new();
    auto_break_switch.set_active(cfg.timer.auto_start_breaks);
    auto_break_switch.set_valign(gtk4::Align::Center);
    let row_auto_break = make_setting_row("Auto-start Breaks", &auto_break_switch);
    page.append(&row_auto_break);

    // Auto-start Focus
    let auto_focus_switch = gtk4::Switch::new();
    auto_focus_switch.set_active(cfg.timer.auto_start_focus);
    auto_focus_switch.set_valign(gtk4::Align::Center);
    let row_auto_focus = make_setting_row("Auto-start Focus", &auto_focus_switch);
    page.append(&row_auto_focus);

    // ---------- NOTIFICATION SETTINGS ----------
    let notify_title = gtk4::Label::new(Some("NOTIFICATIONS"));
    notify_title.set_xalign(0.0);
    notify_title.add_css_class("settings-group-title");
    page.append(&notify_title);

    let notify_switch = gtk4::Switch::new();
    notify_switch.set_active(cfg.notifications.enabled);
    notify_switch.set_valign(gtk4::Align::Center);
    let row_notify = make_setting_row("Desktop Notifications", &notify_switch);
    page.append(&row_notify);

    let sound_switch = gtk4::Switch::new();
    sound_switch.set_active(cfg.notifications.sound);
    sound_switch.set_valign(gtk4::Align::Center);
    let row_sound = make_setting_row("Sound Alerts", &sound_switch);
    page.append(&row_sound);

    // ---------- WINDOW SETTINGS ----------
    let win_title = gtk4::Label::new(Some("WINDOW & DISPLAY"));
    win_title.set_xalign(0.0);
    win_title.add_css_class("settings-group-title");
    page.append(&win_title);

    // Anchor DropDown
    let anchors = ["top-right", "top-left", "bottom-right", "bottom-left", "center"];
    let model = gtk4::StringList::new(&anchors);
    let anchor_combo = gtk4::DropDown::new(Some(model), None::<&gtk4::Expression>);
    let initial_idx = anchors
        .iter()
        .position(|&a| a == cfg.window.anchor)
        .unwrap_or(0) as u32;
    anchor_combo.set_selected(initial_idx);
    let row_anchor = make_setting_row("Screen Anchor", &anchor_combo);
    page.append(&row_anchor);

    // Window Opacity
    let opacity_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.3, 1.0, 0.05);
    opacity_scale.set_value(cfg.window.opacity);
    opacity_scale.set_width_request(120);
    let row_opacity = make_setting_row("Opacity", &opacity_scale);
    page.append(&row_opacity);

    // Always on Top
    let aot_switch = gtk4::Switch::new();
    aot_switch.set_active(cfg.window.always_on_top);
    aot_switch.set_valign(gtk4::Align::Center);
    let row_aot = make_setting_row("Always on Top / Overlay", &aot_switch);
    page.append(&row_aot);

    // Compact Mode
    let compact_switch = gtk4::Switch::new();
    compact_switch.set_active(cfg.window.compact);
    compact_switch.set_valign(gtk4::Align::Center);
    let row_compact = make_setting_row("Compact Mode", &compact_switch);
    page.append(&row_compact);

    drop(cfg);

    // Connect callbacks to update config & save atomically
    let save_config = gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        on_change,
        move || {
            if let Err(e) = config.borrow().save() {
                eprintln!("tomato: failed to save config: {e}");
            }
            on_change();
        }
    );

    focus_spin.connect_value_changed(gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        save_config,
        move |spin| {
            config.borrow_mut().timer.focus_minutes = spin.value() as u32;
            save_config();
        }
    ));

    short_spin.connect_value_changed(gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        save_config,
        move |spin| {
            config.borrow_mut().timer.short_break_minutes = spin.value() as u32;
            save_config();
        }
    ));

    long_spin.connect_value_changed(gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        save_config,
        move |spin| {
            config.borrow_mut().timer.long_break_minutes = spin.value() as u32;
            save_config();
        }
    ));

    cycles_spin.connect_value_changed(gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        save_config,
        move |spin| {
            config.borrow_mut().timer.cycles_before_long_break = spin.value() as u32;
            save_config();
        }
    ));

    auto_break_switch.connect_state_set(gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        save_config,
        move |_, state| {
            config.borrow_mut().timer.auto_start_breaks = state;
            save_config();
            gtk4::glib::Propagation::Proceed
        }
    ));

    auto_focus_switch.connect_state_set(gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        save_config,
        move |_, state| {
            config.borrow_mut().timer.auto_start_focus = state;
            save_config();
            gtk4::glib::Propagation::Proceed
        }
    ));

    notify_switch.connect_state_set(gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        save_config,
        move |_, state| {
            config.borrow_mut().notifications.enabled = state;
            save_config();
            gtk4::glib::Propagation::Proceed
        }
    ));

    sound_switch.connect_state_set(gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        save_config,
        move |_, state| {
            config.borrow_mut().notifications.sound = state;
            save_config();
            gtk4::glib::Propagation::Proceed
        }
    ));

    anchor_combo.connect_selected_notify(gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        save_config,
        move |combo| {
            let idx = combo.selected() as usize;
            if idx < anchors.len() {
                config.borrow_mut().window.anchor = anchors[idx].to_string();
                save_config();
            }
        }
    ));

    opacity_scale.connect_value_changed(gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        save_config,
        move |scale| {
            config.borrow_mut().window.opacity = scale.value();
            save_config();
        }
    ));

    aot_switch.connect_state_set(gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        save_config,
        move |_, state| {
            config.borrow_mut().window.always_on_top = state;
            save_config();
            gtk4::glib::Propagation::Proceed
        }
    ));

    compact_switch.connect_state_set(gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        save_config,
        move |_, state| {
            config.borrow_mut().window.compact = state;
            save_config();
            gtk4::glib::Propagation::Proceed
        }
    ));

    scroller.upcast()
}

fn make_setting_row(label_text: &str, widget: &impl IsA<gtk4::Widget>) -> gtk4::Box {
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    row.add_css_class("settings-row");

    let lbl = gtk4::Label::new(Some(label_text));
    lbl.set_hexpand(true);
    lbl.set_xalign(0.0);
    lbl.add_css_class("settings-label");

    row.append(&lbl);
    row.append(widget);
    row
}
