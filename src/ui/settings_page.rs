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

    let focus_spin = spin(1.0, 180.0, cfg.timer.focus_minutes as f64);
    append_row(&timer_group, "Focus minutes", &focus_spin, false);

    let short_spin = spin(1.0, 60.0, cfg.timer.short_break_minutes as f64);
    append_row(&timer_group, "Short break", &short_spin, true);

    let long_spin = spin(1.0, 60.0, cfg.timer.long_break_minutes as f64);
    append_row(&timer_group, "Long break", &long_spin, true);

    let cycles_spin = spin(1.0, 20.0, cfg.timer.cycles_before_long_break as f64);
    append_row(&timer_group, "Cycles before long break", &cycles_spin, true);
    page.append(&timer_group);

    // ── Automation ──────────────────────────────────────────────────────────
    page.append(&group_title("AUTOMATION"));
    let auto_group = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    auto_group.add_css_class("tm-group");

    let auto_break_switch = toggle(cfg.timer.auto_start_breaks);
    append_row(&auto_group, "Auto-start breaks", &auto_break_switch, false);

    let auto_focus_switch = toggle(cfg.timer.auto_start_focus);
    append_row(&auto_group, "Auto-start focus", &auto_focus_switch, true);
    page.append(&auto_group);

    // ── Notifications ───────────────────────────────────────────────────────
    page.append(&group_title("NOTIFICATIONS"));
    let notif_group = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    notif_group.add_css_class("tm-group");

    let notify_switch = toggle(cfg.notifications.enabled);
    append_row(&notif_group, "Desktop notifications", &notify_switch, false);
    page.append(&notif_group);

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
    append_row(&win_group, "Screen anchor", &anchor_combo, false);

    let opacity_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.3, 1.0, 0.05);
    opacity_scale.add_css_class("tm-scale");
    opacity_scale.set_value(cfg.window.opacity);
    opacity_scale.set_size_request(120, -1);
    opacity_scale.set_valign(gtk4::Align::Center);
    append_row(&win_group, "Opacity", &opacity_scale, true);

    let aot_switch = toggle(cfg.window.always_on_top);
    append_row(&win_group, "Always on top", &aot_switch, true);
    page.append(&win_group);

    drop(cfg);

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

    macro_rules! bind_spin {
        ($spin:expr, $field:ident, $sub:ident) => {
            $spin.connect_value_changed(gtk4::glib::clone!(
                #[strong]
                config,
                #[strong]
                save_config,
                move |s| {
                    config.borrow_mut().$sub.$field = s.value() as u32;
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
        move |scale| {
            config.borrow_mut().window.opacity = scale.value();
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
