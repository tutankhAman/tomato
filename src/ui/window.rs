use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use libadwaita::prelude::*;

use crate::config::Config;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    #[default]
    Center,
}

fn parse_corner(anchor: &str) -> Corner {
    match anchor {
        "top-left" => Corner::TopLeft,
        "top-right" => Corner::TopRight,
        "bottom-left" => Corner::BottomLeft,
        "bottom-right" => Corner::BottomRight,
        "center" => Corner::Center,
        _ => Corner::TopRight,
    }
}

fn h_edge(corner: Corner) -> Option<Edge> {
    match corner {
        Corner::TopLeft | Corner::BottomLeft => Some(Edge::Left),
        Corner::TopRight | Corner::BottomRight => Some(Edge::Right),
        Corner::Center => None,
    }
}

fn v_edge(corner: Corner) -> Option<Edge> {
    match corner {
        Corner::TopLeft | Corner::TopRight => Some(Edge::Top),
        Corner::BottomLeft | Corner::BottomRight => Some(Edge::Bottom),
        Corner::Center => None,
    }
}

fn drag_sign_x(corner: Corner) -> i32 {
    match corner {
        Corner::TopLeft | Corner::BottomLeft => 1,
        Corner::TopRight | Corner::BottomRight => -1,
        Corner::Center => 0,
    }
}

fn drag_sign_y(corner: Corner) -> i32 {
    match corner {
        Corner::TopLeft | Corner::TopRight => 1,
        Corner::BottomLeft | Corner::BottomRight => -1,
        Corner::Center => 0,
    }
}

#[derive(Default)]
struct DragState {
    active: bool,
    start_x: i32,
    start_y: i32,
    corner: Corner,
}

pub fn build(app: &libadwaita::Application) {
    let config = Rc::new(RefCell::new(Config::load()));

    let window = libadwaita::ApplicationWindow::builder()
        .application(app)
        .build();
    window.set_default_size(360, 470);
    window.set_decorated(false);
    window.add_css_class("tomato-root");

    let root = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    root.add_css_class("tomato-card");
    window.set_content(Some(&root));

    let on_layer_shell = gtk4_layer_shell::is_supported();
    let drag_state = Rc::new(RefCell::new(DragState::default()));

    if on_layer_shell {
        let cfg = config.borrow();
        let corner = parse_corner(&cfg.window.anchor);
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_namespace(Some("tomato"));
        window.set_keyboard_mode(KeyboardMode::OnDemand);
        window.set_exclusive_zone(0);
        if let Some(h) = h_edge(corner) {
            window.set_anchor(h, true);
            window.set_margin(h, cfg.window.margin_x);
        }
        if let Some(v) = v_edge(corner) {
            window.set_anchor(v, true);
            window.set_margin(v, cfg.window.margin_y);
        }
        drag_state.borrow_mut().corner = corner;
    } else {
        eprintln!("tomato: gtk4-layer-shell is not supported; running as a normal window");
    }

    let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    header.add_css_class("tomato-header");

    let title = gtk4::Label::new(Some("TOMATO"));
    title.add_css_class("tomato-title");
    header.append(&title);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    header.append(&spacer);

    let close_btn = gtk4::Button::from_icon_name("window-close-symbolic");
    close_btn.add_css_class("tomato-iconbtn");
    close_btn.add_css_class("destructive");
    close_btn.connect_clicked(gtk4::glib::clone!(
        #[weak]
        window,
        move |_| {
            window.close();
        }
    ));
    header.append(&close_btn);

    root.append(&header);

    let drag = gtk4::GestureDrag::new();
    drag.connect_drag_begin(gtk4::glib::clone!(
        #[strong]
        drag_state,
        #[weak]
        window,
        #[strong]
        on_layer_shell,
        move |_g, _x, _y| {
            let mut s = drag_state.borrow_mut();
            if !on_layer_shell || s.corner == Corner::Center {
                return;
            }
            let h = h_edge(s.corner).expect("corner has horizontal edge");
            let v = v_edge(s.corner).expect("corner has vertical edge");
            s.active = true;
            s.start_x = window.margin(h);
            s.start_y = window.margin(v);
        }
    ));
    drag.connect_drag_update(gtk4::glib::clone!(
        #[strong]
        drag_state,
        #[weak]
        window,
        #[strong]
        on_layer_shell,
        move |_g, x, y| {
            let s = drag_state.borrow_mut();
            if !on_layer_shell || !s.active || s.corner == Corner::Center {
                return;
            }
            let h = h_edge(s.corner).expect("corner has horizontal edge");
            let v = v_edge(s.corner).expect("corner has vertical edge");
            let margin_x = (s.start_x + x as i32 * drag_sign_x(s.corner)).max(0);
            let margin_y = (s.start_y + y as i32 * drag_sign_y(s.corner)).max(0);
            window.set_margin(h, margin_x);
            window.set_margin(v, margin_y);
        }
    ));
    drag.connect_drag_end(gtk4::glib::clone!(
        #[strong]
        drag_state,
        #[weak]
        window,
        #[strong]
        config,
        #[strong]
        on_layer_shell,
        move |_g, _x, _y| {
            let mut s = drag_state.borrow_mut();
            if !on_layer_shell || !s.active || s.corner == Corner::Center {
                return;
            }
            let h = h_edge(s.corner).expect("corner has horizontal edge");
            let v = v_edge(s.corner).expect("corner has vertical edge");
            let margin_x = window.margin(h);
            let margin_y = window.margin(v);
            s.active = false;
            {
                let mut cfg = config.borrow_mut();
                cfg.window.margin_x = margin_x;
                cfg.window.margin_y = margin_y;
            }
            if let Err(e) = config.borrow().save() {
                eprintln!("tomato: failed to save config: {e}");
            }
        }
    ));
    header.add_controller(drag);

    let switcher = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    switcher.add_css_class("tomato-switcher");

    let timer_btn = gtk4::ToggleButton::with_label("TIMER");
    let tasks_btn = gtk4::ToggleButton::with_label("TASKS");
    let settings_btn = gtk4::ToggleButton::with_label("SETTINGS");
    timer_btn.set_hexpand(true);
    tasks_btn.set_hexpand(true);
    settings_btn.set_hexpand(true);

    tasks_btn.set_group(Some(&timer_btn));
    settings_btn.set_group(Some(&timer_btn));
    timer_btn.set_active(true);

    switcher.append(&timer_btn);
    switcher.append(&tasks_btn);
    switcher.append(&settings_btn);
    root.append(&switcher);

    let stack = gtk4::Stack::new();
    stack.set_vexpand(true);

    let timer_label = gtk4::Label::new(Some("Timer"));
    timer_label.set_halign(gtk4::Align::Center);
    timer_label.set_valign(gtk4::Align::Center);
    stack.add_named(&timer_label, Some("timer"));

    let tasks_label = gtk4::Label::new(Some("Tasks"));
    tasks_label.set_halign(gtk4::Align::Center);
    tasks_label.set_valign(gtk4::Align::Center);
    stack.add_named(&tasks_label, Some("tasks"));

    let settings_label = gtk4::Label::new(Some("Settings"));
    settings_label.set_halign(gtk4::Align::Center);
    settings_label.set_valign(gtk4::Align::Center);
    stack.add_named(&settings_label, Some("settings"));

    root.append(&stack);

    timer_btn.connect_toggled(gtk4::glib::clone!(
        #[weak]
        stack,
        move |b| {
            if b.is_active() {
                stack.set_visible_child_name("timer");
            }
        }
    ));
    tasks_btn.connect_toggled(gtk4::glib::clone!(
        #[weak]
        stack,
        move |b| {
            if b.is_active() {
                stack.set_visible_child_name("tasks");
            }
        }
    ));
    settings_btn.connect_toggled(gtk4::glib::clone!(
        #[weak]
        stack,
        move |b| {
            if b.is_active() {
                stack.set_visible_child_name("settings");
            }
        }
    ));

    let shortcut_controller = gtk4::ShortcutController::new();
    let trigger = gtk4::ShortcutTrigger::parse_string("<Control>q")
        .expect("tomato: hardcoded shortcut should parse");
    let action = gtk4::CallbackAction::new(gtk4::glib::clone!(
        #[weak]
        window,
        #[upgrade_or]
        gtk4::glib::Propagation::Proceed,
        move |_, _| {
            window.close();
            gtk4::glib::Propagation::Stop
        }
    ));
    let shortcut = gtk4::Shortcut::builder()
        .trigger(&trigger)
        .action(&action)
        .build();
    shortcut_controller.add_shortcut(shortcut);
    window.add_controller(shortcut_controller);

    window.present();
}
