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

    let page = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    page.add_css_class("tm-page");
    scroller.set_child(Some(&page));

    let cfg = config.borrow();

    // ── Timer ───────────────────────────────────────────────────────────────
    page.append(&group_title("TIMER"));

    let timer_group = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    timer_group.add_css_class("tm-group");

    // Preset segmented control — minimal labels, no dedicated row chrome.
    let preset_wrap = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
    preset_wrap.add_css_class("tm-seg");
    preset_wrap.add_css_class("tm-seg-preset");
    preset_wrap.set_homogeneous(true);

    let classic_btn = gtk4::ToggleButton::with_label("25/5");
    let deep_btn = gtk4::ToggleButton::with_label("50/10");
    let custom_btn = gtk4::ToggleButton::with_label("Custom");
    classic_btn.set_tooltip_text(Some("Classic · 25 focus / 5 break / 15 long, every 4"));
    deep_btn.set_tooltip_text(Some("Deep · 50 focus / 10 break / 20 long, every 4"));
    custom_btn.set_tooltip_text(Some("Manual values below"));
    for b in [&classic_btn, &deep_btn, &custom_btn] {
        b.add_css_class("tm-seg-btn");
        b.set_hexpand(true);
    }
    deep_btn.set_group(Some(&classic_btn));
    custom_btn.set_group(Some(&classic_btn));

    preset_wrap.append(&classic_btn);
    preset_wrap.append(&deep_btn);
    preset_wrap.append(&custom_btn);

    // Determine initial preset selection
    let is_classic = cfg.timer.focus_minutes == 25
        && cfg.timer.short_break_minutes == 5
        && cfg.timer.long_break_minutes == 15
        && cfg.timer.cycles_before_long_break == 4;
    let is_deep = cfg.timer.focus_minutes == 50
        && cfg.timer.short_break_minutes == 10
        && cfg.timer.long_break_minutes == 20
        && cfg.timer.cycles_before_long_break == 4;
    if is_classic {
        classic_btn.set_active(true);
    } else if is_deep {
        deep_btn.set_active(true);
    } else {
        custom_btn.set_active(true);
    }

    timer_group.append(&preset_wrap);

    let focus_spin = spin(1.0, 180.0, cfg.timer.focus_minutes as f64);
    let focus_box = with_suffix(&focus_spin, "min");
    append_row(&timer_group, "Focus", &focus_box, true);

    let short_spin = spin(1.0, 60.0, cfg.timer.short_break_minutes as f64);
    let short_box = with_suffix(&short_spin, "min");
    append_row(&timer_group, "Short break", &short_box, true);

    let long_spin = spin(1.0, 60.0, cfg.timer.long_break_minutes as f64);
    let long_box = with_suffix(&long_spin, "min");
    append_row(&timer_group, "Long break", &long_box, true);

    // Cycles row with live dots preview
    let cycles_spin = spin(1.0, 12.0, cfg.timer.cycles_before_long_break as f64);
    let dots_preview = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);
    dots_preview.add_css_class("tm-dots");
    dots_preview.set_valign(gtk4::Align::Center);
    rebuild_dots(&dots_preview, cfg.timer.cycles_before_long_break);
    let cycles_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    cycles_box.append(&cycles_spin);
    cycles_box.append(&dots_preview);
    cycles_box.set_valign(gtk4::Align::Center);
    append_row(&timer_group, "Sessions before long break", &cycles_box, false);

    page.append(&timer_group);

    // ── Automation ──────────────────────────────────────────────────────────
    page.append(&group_title("AUTOMATION"));
    let auto_group = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    auto_group.add_css_class("tm-group");

    let auto_break_switch = toggle(cfg.timer.auto_start_breaks);
    append_row(&auto_group, "Auto-start breaks", &auto_break_switch, false);

    let auto_focus_switch = toggle(cfg.timer.auto_start_focus);
    append_row(&auto_group, "Auto-start focus sessions", &auto_focus_switch, true);
    page.append(&auto_group);

    // ── Window ──────────────────────────────────────────────────────────────
    page.append(&group_title("WINDOW"));

    let win_group = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    win_group.add_css_class("tm-group");

    let anchors = ["top-right", "top-left", "bottom-right", "bottom-left", "center"];
    let model = gtk4::StringList::new(&anchors);
    let anchor_combo = gtk4::DropDown::new(Some(model), None::<&gtk4::Expression>);
    anchor_combo.add_css_class("tm-spin");
    let initial_idx = anchors
        .iter()
        .position(|&a| a == cfg.window.anchor)
        .unwrap_or(0) as u32;
    anchor_combo.set_selected(initial_idx);
    append_row(&win_group, "Screen corner", &anchor_combo, false);

    let opacity_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.3, 1.0, 0.05);
    opacity_scale.add_css_class("tm-scale");
    opacity_scale.set_value(cfg.window.opacity);
    opacity_scale.set_size_request(140, -1);
    opacity_scale.set_valign(gtk4::Align::Center);
    opacity_scale.set_draw_value(false);
    let opacity_val = gtk4::Label::new(Some(&format!("{:.0}%", cfg.window.opacity * 100.0)));
    opacity_val.add_css_class("tm-opacity-val");
    opacity_val.set_width_chars(4);
    opacity_val.set_xalign(1.0);
    let opacity_reset = gtk4::Button::from_icon_name("view-refresh-symbolic");
    opacity_reset.add_css_class("tm-iconbtn");
    opacity_reset.set_tooltip_text(Some("Reset to 97%"));
    let opacity_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    opacity_box.set_valign(gtk4::Align::Center);
    opacity_box.append(&opacity_scale);
    opacity_box.append(&opacity_val);
    opacity_box.append(&opacity_reset);
    append_row(&win_group, "Opacity", &opacity_box, true);

    let aot_switch = toggle(cfg.window.always_on_top);
    append_row(&win_group, "Always on top (overlay layer)", &aot_switch, true);
    page.append(&win_group);

    // ── Notifications ───────────────────────────────────────────────────────
    page.append(&group_title("NOTIFICATIONS"));
    let notif_group = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    notif_group.add_css_class("tm-group");
    let notify_switch = toggle(cfg.notifications.enabled);
    append_row(&notif_group, "Desktop notifications on phase change", &notify_switch, false);
    page.append(&notif_group);

    drop(cfg);

    // ── Appearance ──────────────────────────────────────────────────────────
    page.append(&group_title("APPEARANCE"));
    let theme_group = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    theme_group.add_css_class("tm-group");

    let theme_btns = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
    theme_btns.add_css_class("tm-seg");
    theme_btns.add_css_class("tm-seg-preset");
    theme_btns.set_homogeneous(true);
    let sys_btn = gtk4::ToggleButton::with_label("System");
    let light_btn = gtk4::ToggleButton::with_label("Light");
    let dark_btn = gtk4::ToggleButton::with_label("Dark");
    light_btn.set_group(Some(&sys_btn));
    dark_btn.set_group(Some(&sys_btn));
    for b in [&sys_btn, &light_btn, &dark_btn] {
        b.add_css_class("tm-seg-btn");
        b.set_hexpand(true);
    }
    // Only System or Dark is reachable if the app was launched in dark mode,
    // since in-app toggling previously used ForceLight/ForceDark. Pick by state.
    let style_manager = libadwaita::StyleManager::default();
    if style_manager.is_dark() {
        dark_btn.set_active(true);
    } else {
        light_btn.set_active(true);
    }
    theme_btns.append(&sys_btn);
    theme_btns.append(&light_btn);
    theme_btns.append(&dark_btn);

    let set_scheme = Rc::new(move |scheme: libadwaita::ColorScheme| {
        libadwaita::StyleManager::default().set_color_scheme(scheme);
        crate::ui::reload_theme();
    });
    sys_btn.connect_toggled(gtk4::glib::clone!(#[strong] set_scheme, move |b| {
        if b.is_active() {
            set_scheme(libadwaita::ColorScheme::Default);
        }
    }));
    light_btn.connect_toggled(gtk4::glib::clone!(#[strong] set_scheme, move |b| {
        if b.is_active() {
            set_scheme(libadwaita::ColorScheme::ForceLight);
        }
    }));
    dark_btn.connect_toggled(gtk4::glib::clone!(#[strong] set_scheme, move |b| {
        if b.is_active() {
            set_scheme(libadwaita::ColorScheme::ForceDark);
        }
    }));

    theme_group.append(&theme_btns);
    page.append(&theme_group);

    // ── Persistence ─────────────────────────────────────────────────────────
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

    // Guard to avoid feedback loops between preset toggles and spins.
    let guard = Rc::new(RefCell::new(false));

    // Helpers to apply a preset atomically.
    let apply_classic = gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        save_config,
        #[strong]
        guard,
        #[weak]
        focus_spin,
        #[weak]
        short_spin,
        #[weak]
        long_spin,
        #[weak]
        cycles_spin,
        move || {
            *guard.borrow_mut() = true;
            {
                let mut cfg = config.borrow_mut();
                cfg.timer.focus_minutes = 25;
                cfg.timer.short_break_minutes = 5;
                cfg.timer.long_break_minutes = 15;
                cfg.timer.cycles_before_long_break = 4;
            }
            focus_spin.set_value(25.0);
            short_spin.set_value(5.0);
            long_spin.set_value(15.0);
            cycles_spin.set_value(4.0);
            *guard.borrow_mut() = false;
            save_config();
        }
    );
    let apply_deep = gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        save_config,
        #[strong]
        guard,
        #[weak]
        focus_spin,
        #[weak]
        short_spin,
        #[weak]
        long_spin,
        #[weak]
        cycles_spin,
        move || {
            *guard.borrow_mut() = true;
            {
                let mut cfg = config.borrow_mut();
                cfg.timer.focus_minutes = 50;
                cfg.timer.short_break_minutes = 10;
                cfg.timer.long_break_minutes = 20;
                cfg.timer.cycles_before_long_break = 4;
            }
            focus_spin.set_value(50.0);
            short_spin.set_value(10.0);
            long_spin.set_value(20.0);
            cycles_spin.set_value(4.0);
            *guard.borrow_mut() = false;
            save_config();
        }
    );

    classic_btn.connect_toggled(gtk4::glib::clone!(
        #[strong]
        guard,
        #[strong]
        apply_classic,
        move |b| {
            if *guard.borrow() {
                return;
            }
            if b.is_active() {
                apply_classic();
            }
        }
    ));
    deep_btn.connect_toggled(gtk4::glib::clone!(
        #[strong]
        guard,
        #[strong]
        apply_deep,
        move |b| {
            if *guard.borrow() {
                return;
            }
            if b.is_active() {
                apply_deep();
            }
        }
    ));
    // Custom toggle does nothing on its own; it gets activated by manual edits.

    // Sync preset selection when spins change
    let sync_preset = gtk4::glib::clone!(
        #[strong]
        guard,
        #[weak]
        classic_btn,
        #[weak]
        deep_btn,
        #[weak]
        custom_btn,
        #[weak]
        focus_spin,
        #[weak]
        short_spin,
        #[weak]
        long_spin,
        #[weak]
        cycles_spin,
        move || {
            if *guard.borrow() {
                return;
            }
            let f = focus_spin.value() as u32;
            let s = short_spin.value() as u32;
            let l = long_spin.value() as u32;
            let c = cycles_spin.value() as u32;
            let is_c = f == 25 && s == 5 && l == 15 && c == 4;
            let is_d = f == 50 && s == 10 && l == 20 && c == 4;
            *guard.borrow_mut() = true;
            if is_c {
                classic_btn.set_active(true);
            } else if is_d {
                deep_btn.set_active(true);
            } else {
                custom_btn.set_active(true);
            }
            *guard.borrow_mut() = false;
        }
    );

    macro_rules! bind_spin {
        ($spin:expr, $field:ident, $sub:ident) => {
            $spin.connect_value_changed(gtk4::glib::clone!(
                #[strong]
                config,
                #[strong]
                save_config,
                #[strong]
                guard,
                #[strong]
                sync_preset,
                #[weak]
                dots_preview,
                move |s| {
                    if *guard.borrow() {
                        // Still update dots if this is the cycles spin
                        if stringify!($field) == "cycles_before_long_break" {
                            rebuild_dots(&dots_preview, s.value() as u32);
                        }
                        return;
                    }
                    config.borrow_mut().$sub.$field = s.value() as u32;
                    if stringify!($field) == "cycles_before_long_break" {
                        rebuild_dots(&dots_preview, s.value() as u32);
                    }
                    sync_preset();
                    save_config();
                }
            ));
        };
    }
    bind_spin!(focus_spin, focus_minutes, timer);
    bind_spin!(short_spin, short_break_minutes, timer);
    bind_spin!(long_spin, long_break_minutes, timer);
    bind_spin!(cycles_spin, cycles_before_long_break, timer);

    macro_rules! bind_switch {
        ($sw:expr, $field:ident, $sub:ident) => {
            $sw.connect_state_set(gtk4::glib::clone!(
                #[strong]
                config,
                #[strong]
                save_config,
                move |_, state| {
                    config.borrow_mut().$sub.$field = state;
                    save_config();
                    gtk4::glib::Propagation::Proceed
                }
            ));
        };
    }
    bind_switch!(auto_break_switch, auto_start_breaks, timer);
    bind_switch!(auto_focus_switch, auto_start_focus, timer);
    bind_switch!(notify_switch, enabled, notifications);
    bind_switch!(aot_switch, always_on_top, window);

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
        #[weak]
        opacity_val,
        move |scale| {
            let v = scale.value();
            config.borrow_mut().window.opacity = v;
            opacity_val.set_label(&format!("{:.0}%", v * 100.0));
            save_config();
        }
    ));
    opacity_reset.connect_clicked(gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        save_config,
        #[weak]
        opacity_scale,
        #[weak]
        opacity_val,
        move |_| {
            opacity_scale.set_value(0.97);
            opacity_val.set_label("97%");
            config.borrow_mut().window.opacity = 0.97;
            save_config();
        }
    ));

    scroller.upcast()
}

fn group_title(text: &str) -> gtk4::Label {
    let lbl = gtk4::Label::new(Some(text));
    lbl.set_xalign(0.0);
    lbl.add_css_class("tm-group-title");
    lbl
}

fn spin(min: f64, max: f64, value: f64) -> gtk4::SpinButton {
    let s = gtk4::SpinButton::with_range(min, max, 1.0);
    s.add_css_class("tm-spin");
    s.set_value(value);
    s.set_valign(gtk4::Align::Center);
    s
}

fn toggle(active: bool) -> gtk4::Switch {
    let s = gtk4::Switch::new();
    s.set_active(active);
    s.set_valign(gtk4::Align::Center);
    s
}

fn with_suffix(spin: &gtk4::SpinButton, suffix: &str) -> gtk4::Box {
    let b = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    b.set_valign(gtk4::Align::Center);
    b.append(spin);
    let lbl = gtk4::Label::new(Some(suffix));
    lbl.add_css_class("tm-suffix");
    lbl.set_valign(gtk4::Align::Center);
    b.append(&lbl);
    b
}

fn rebuild_dots(container: &gtk4::Box, cycles: u32) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
    let n = cycles.clamp(1, 12) as usize;
    for i in 0..n.min(8) {
        let d = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        d.add_css_class("tm-dot");
        if i == 0 {
            d.add_css_class("tm-dot-on");
        }
        container.append(&d);
    }
    if n > 8 {
        let more = gtk4::Label::new(Some(&format!("+{}", n - 8)));
        more.add_css_class("tm-dots-more");
        container.append(&more);
    }
}

fn append_row(group: &gtk4::Box, label: &str, widget: &impl IsA<gtk4::Widget>, separator: bool) {
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    row.add_css_class("tm-setrow");
    if separator {
        row.add_css_class("tm-setrow-sep");
    }
    let lbl = gtk4::Label::new(Some(label));
    lbl.set_hexpand(true);
    lbl.set_xalign(0.0);
    lbl.add_css_class("tm-setlabel");
    row.append(&lbl);
    row.append(widget);
    group.append(&row);
}
