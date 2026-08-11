use std::cell::RefCell;
use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};


use crate::config::Config;
use crate::timer::Timer;
use crate::todo::TodoStore;
use crate::ui::{settings_page, tasks_page, timer_page};
use std::f64::consts::PI;

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
    press_x: f64,
    press_y: f64,
    corner: Corner,
    pending: Option<(i32, i32)>,
    tick: Option<gtk4::TickCallbackId>,
}

pub fn build(app: &libadwaita::Application) {
    let config = Rc::new(RefCell::new(Config::load()));

    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .build();
    window.set_decorated(false);
    window.add_css_class("tm-root");

    let cfg_borrow = config.borrow();
    let initial_opacity = cfg_borrow.window.opacity;
    // Use content alpha instead of whole-window opacity — the compositor blur
    // (better-blur/breeze) blits a rectangular slab behind the window; with
    // whole-window opacity the transparent corners still show as a sharp
    // blurred rectangle. Content alpha keeps corners fully transparent.
    crate::ui::reload_theme_with_opacity(initial_opacity);

    let root = gtk4::Overlay::new();
    window.set_child(Some(&root));

    let on_layer_shell = gtk4_layer_shell::is_supported();
    let drag_state = Rc::new(RefCell::new(DragState::default()));

    if on_layer_shell {
        let corner = parse_corner(&cfg_borrow.window.anchor);
        window.init_layer_shell();
        window.set_layer(if cfg_borrow.window.always_on_top {
            Layer::Overlay
        } else {
            Layer::Top
        });
        window.set_namespace(Some("tomato"));
        window.set_keyboard_mode(KeyboardMode::OnDemand);
        window.set_exclusive_zone(0);

        if let Some(h) = h_edge(corner) {
            window.set_anchor(h, true);
            window.set_margin(h, cfg_borrow.window.margin_x);
        }
        if let Some(v) = v_edge(corner) {
            window.set_anchor(v, true);
            window.set_margin(v, cfg_borrow.window.margin_y);
        }
        drag_state.borrow_mut().corner = corner;
    } else {
        eprintln!("tomato: gtk4-layer-shell is not supported; running as a normal window");
    }
    drop(cfg_borrow);

    // ── Pill Header ─────────────────────────────────────────────────────────
    let pill = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    pill.add_css_class("tm-pill");

    let pill_ring = gtk4::DrawingArea::new();
    pill_ring.set_size_request(24, 24);
    pill_ring.set_valign(gtk4::Align::Center);
    pill.append(&pill_ring);

    let pill_time = gtk4::Label::new(Some("25:00"));
    pill_time.add_css_class("tm-pill-time");
    pill_time.set_valign(gtk4::Align::Center);
    pill.append(&pill_time);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    pill.append(&spacer);

    // Theme toggle and Close button moved to the pill for convenience
    let pill_actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
    pill_actions.set_valign(gtk4::Align::Center);
    let theme_btn = gtk4::Button::new();
    theme_btn.set_valign(gtk4::Align::Center);
    theme_btn.add_css_class("tm-iconbtn");
    let style_manager = libadwaita::StyleManager::default();
    theme_btn.set_icon_name(if style_manager.is_dark() {
        "weather-clear-symbolic"
    } else {
        "weather-clear-night-symbolic"
    });
    theme_btn.connect_clicked(|btn| {
        let sm = libadwaita::StyleManager::default();
        if sm.is_dark() {
            sm.set_color_scheme(libadwaita::ColorScheme::ForceLight);
            btn.set_icon_name("weather-clear-night-symbolic");
        } else {
            sm.set_color_scheme(libadwaita::ColorScheme::ForceDark);
            btn.set_icon_name("weather-clear-symbolic");
        }
        crate::ui::reload_theme();
    });
    pill_actions.append(&theme_btn);

    let close_btn = gtk4::Button::from_icon_name("window-close-symbolic");
    close_btn.set_valign(gtk4::Align::Center);
    close_btn.add_css_class("tm-iconbtn");
    close_btn.connect_clicked(gtk4::glib::clone!(
        #[weak]
        window,
        move |_| {
            window.close();
        }
    ));
    pill_actions.append(&close_btn);
    pill.append(&pill_actions);

    let drag_handle = gtk4::DrawingArea::new();
    drag_handle.set_size_request(16, 24);
    drag_handle.set_valign(gtk4::Align::Center);
    drag_handle.add_css_class("tm-pill-drag");
    drag_handle.set_draw_func(|_, cr, _, _| {
        // Simple dots grid using css text color
        cr.set_source_rgba(0.5, 0.5, 0.5, 0.5);
        for x in [4.0, 10.0] {
            for y in [6.0, 12.0, 18.0] {
                cr.arc(x, y, 1.5, 0.0, 2.0 * PI);
                let _ = cr.fill();
            }
        }
    });
    pill.append(&drag_handle);
    root.add_overlay(&pill);

    // Layout contract:
    // - The pill's width animation is CSS-driven (min-width) so the layer-shell
    //   window size stays locked at 340 and the compositor never re-negotiates
    //   width mid-flight. No size-allocate loop, no dropped frames.
    // - Vertical motion is the revealer's SlideDown. Giving the root a fixed
    //   inter-child spacing keeps the pill's rounded bottom edge outside the
    //   revealer's clip — that's the "broken in half vertically" fix. The
    //   previous configuration (revealer Overflow::Hidden + root spacing 0
    //   + dropdown margin-top) put the pill edge right on the clip boundary
    //   while both animations ran, so the pill's bottom was sliced off.
    window.set_size_request(340, -1);
    root.set_halign(gtk4::Align::Fill);
    root.set_hexpand(true);
    root.set_valign(gtk4::Align::Start);
    root.set_overflow(gtk4::Overflow::Visible);
    pill.set_halign(gtk4::Align::Center);
    pill.set_hexpand(false);
    pill.set_valign(gtk4::Align::Start);
    pill.set_overflow(gtk4::Overflow::Visible);
    pill.set_size_request(-1, 38);

    let dropdown_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    dropdown_box.add_css_class("tm-dropdown");
    dropdown_box.set_size_request(340, 480);
    dropdown_box.set_overflow(gtk4::Overflow::Hidden);
    dropdown_box.set_valign(gtk4::Align::Start);

    let revealer = gtk4::Revealer::new();
    revealer.set_transition_type(gtk4::RevealerTransitionType::SlideDown);
    revealer.set_transition_duration(300);
    revealer.set_reveal_child(false);
    revealer.set_halign(gtk4::Align::Fill);
    revealer.set_hexpand(true);
    revealer.set_valign(gtk4::Align::Start);
    revealer.set_overflow(gtk4::Overflow::Hidden);
    revealer.set_child(Some(&dropdown_box));
    revealer.set_margin_top(44);
    root.set_child(Some(&revealer));

    // Bring back pill resize — but now as a CSS-driven min-width animation
    // that stays in lockstep with the revealer (same easing/duration) and
    // never breaks the pill's vertical layout. Debounce prevents double-
    // toggling mid-flight which previously left the pill half-clipped.
    let toggle = {
        let revealer = revealer.clone();
        let pill_clone = pill.clone();
        let dropdown_box_clone = dropdown_box.clone();
        Rc::new(move || {
            if revealer.is_child_revealed() != revealer.reveals_child() {
                return;
            }
            let expanding = !revealer.reveals_child();
            // Keep JS/Css timing in sync: pill CSS is 320ms, revealer 300ms —
            // close feels ~60ms snappier than open.
            let duration = if expanding { 300 } else { 240 };
            revealer.set_transition_duration(duration);
            if expanding {
                pill_clone.add_css_class("tm-pill-open");
                dropdown_box_clone.add_css_class("tm-dropdown-open");
            } else {
                pill_clone.remove_css_class("tm-pill-open");
                dropdown_box_clone.remove_css_class("tm-dropdown-open");
            }
            revealer.set_reveal_child(expanding);

            // Force full redraws during the animation to fix graphical glitches
            // (e.g. elements cut in half) caused by GTK's dirty region tracking
            // stumbling over the CSS width transition or async Wayland resizes.
            let p_clone = pill_clone.clone();
            let start = std::time::Instant::now();
            p_clone.add_tick_callback(move |w, _| {
                queue_draw_recursive(w.upcast_ref());
                if start.elapsed().as_millis() > duration as u128 + 50 {
                    gtk4::glib::ControlFlow::Break
                } else {
                    gtk4::glib::ControlFlow::Continue
                }
            });
        })
    };
    let pill_click = gtk4::GestureClick::new();
    pill_click.set_button(1);
    pill_click.connect_pressed(gtk4::glib::clone!(
        #[strong] toggle,
        move |_, n_press, _, _| {
            if n_press == 1 {
                toggle();
            }
        }
    ));
    pill.add_controller(pill_click);

    crate::ui::blur::install(&window, pill.upcast_ref(), dropdown_box.upcast_ref());

    // Drag gesture for layer-shell live movement. Margins are derived from the
    // pointer's current widget coords and the window's current margin on every
    // event, throttled to one set_margin per compositor frame via a tick
    // callback, so requested == applied and every delta lands on the pointer.
    let drag = gtk4::GestureDrag::new();
    drag.connect_drag_begin(gtk4::glib::clone!(
        #[strong]
        drag_state,
        #[weak]
        window,
        #[strong]
        on_layer_shell,
        move |_g, x, y| {
            let mut s = drag_state.borrow_mut();
            if !on_layer_shell || s.corner == Corner::Center {
                return;
            }
            s.active = true;
            s.press_x = x;
            s.press_y = y;
            s.pending = None;
            if s.tick.is_none() {
                let tick = window.add_tick_callback(gtk4::glib::clone!(
                    #[strong]
                    drag_state,
                    #[weak]
                    window,
                    #[upgrade_or]
                    gtk4::glib::ControlFlow::Continue,
                    move |_, _frame_clock| {
                        let mut s = drag_state.borrow_mut();
                        if let (Some(h), Some(v)) = (h_edge(s.corner), v_edge(s.corner))
                            && let Some((mx, my)) = s.pending.take()
                        {
                            window.set_margin(h, mx);
                            window.set_margin(v, my);
                        }
                        if s.active {
                            gtk4::glib::ControlFlow::Continue
                        } else {
                            gtk4::glib::ControlFlow::Break
                        }
                    }
                ));
                s.tick = Some(tick);
            }
        }
    ));
    drag.connect_drag_update(gtk4::glib::clone!(
        #[strong]
        drag_state,
        #[weak]
        window,
        #[strong]
        on_layer_shell,
        move |_g, dx, dy| {
            let mut s = drag_state.borrow_mut();
            if !on_layer_shell || !s.active || s.corner == Corner::Center {
                return;
            }
            let (Some(h), Some(v)) = (h_edge(s.corner), v_edge(s.corner)) else {
                return;
            };
            let x_now = s.press_x + dx;
            let y_now = s.press_y + dy;
            let margin_x = (window.margin(h) as f64
                + (x_now - s.press_x) * drag_sign_x(s.corner) as f64)
                .round()
                .max(0.0) as i32;
            let margin_y = (window.margin(v) as f64
                + (y_now - s.press_y) * drag_sign_y(s.corner) as f64)
                .round()
                .max(0.0) as i32;
            s.pending = Some((margin_x, margin_y));
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
        move |_g, _dx, _dy| {
            let mut s = drag_state.borrow_mut();
            if !on_layer_shell || !s.active || s.corner == Corner::Center {
                return;
            }
            s.active = false;
            if let Some(id) = s.tick.take() {
                id.remove();
            }
            if let (Some(h), Some(v)) = (h_edge(s.corner), v_edge(s.corner)) {
                let (margin_x, margin_y) =
                    s.pending.take().unwrap_or_else(|| (window.margin(h), window.margin(v)));
                window.set_margin(h, margin_x);
                window.set_margin(v, margin_y);
                let mut cfg = config.borrow_mut();
                cfg.window.margin_x = margin_x;
                cfg.window.margin_y = margin_y;
            }
            if let Err(e) = config.borrow().save() {
                eprintln!("tomato: failed to save config: {e}");
            }
        }
    ));
    drag_handle.add_controller(drag);

    // ── Segmented switcher ──────────────────────────────────────────────────
    let switcher = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
    switcher.add_css_class("tm-seg");
    switcher.set_homogeneous(true);

    let timer_btn = gtk4::ToggleButton::with_label("Timer");
    let tasks_btn = gtk4::ToggleButton::with_label("Tasks");
    let settings_btn = gtk4::ToggleButton::with_label("Settings");
    for b in [&timer_btn, &tasks_btn, &settings_btn] {
        b.add_css_class("tm-seg-btn");
        b.set_hexpand(true);
    }
    tasks_btn.set_group(Some(&timer_btn));
    settings_btn.set_group(Some(&timer_btn));
    timer_btn.set_active(true);

    switcher.append(&timer_btn);
    switcher.append(&tasks_btn);
    switcher.append(&settings_btn);
    dropdown_box.append(&switcher);

    // ── Pages ───────────────────────────────────────────────────────────────
    let stack = gtk4::Stack::new();
    stack.set_vexpand(true);
    stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
    stack.set_transition_duration(140);

    let timer = Rc::new(RefCell::new({
        let cfg = config.borrow();
        if let Some(snap) = crate::timer::load_timer_snapshot() {
            let before_completed = snap.completed_focus_sessions;
            let before_phase = snap.phase;
            let restored = crate::timer::Timer::restore(snap, &cfg.timer);
            // If a focus session completed while the app was closed, attribute it.
            // Defer store mutation until tasks is available (below); stash count via cfg clone.
            // Instead handle after tasks is loaded (see below).
            let _ = (before_completed, before_phase);
            restored
        } else {
            Timer::new(&cfg.timer)
        }
    }));
    let tasks = Rc::new(RefCell::new(TodoStore::load()));
    // Attribute any focus sessions that completed while the app was closed.
    {
        if let Some(snap) = crate::timer::load_timer_snapshot() {
            let before = snap.completed_focus_sessions;
            let after = timer.borrow().completed_focus_sessions();
            if after > before {
                let increments = after - before;
                let mut store = tasks.borrow_mut();
                let active_before = store.active_task().map(|t| t.id.clone());
                for _ in 0..increments {
                    store.increment_active_pomodoro();
                }
                if store.active_task().map(|t| t.id.clone()) != active_before || increments > 0 {
                    let _ = store.save();
                }
                // Notify once if we landed in a break after a completed focus.
                let cfg = config.borrow();
                if cfg.notifications.enabled
                    && matches!(timer.borrow().phase(), crate::timer::Phase::ShortBreak | crate::timer::Phase::LongBreak)
                    && snap.phase == crate::timer::Phase::Focus
                {
                    let (summary, body) = if timer.borrow().phase() == crate::timer::Phase::LongBreak {
                        ("Session finished", "Great job — enjoy a long break.")
                    } else {
                        ("Focus complete", "Nice work. Take a short break.")
                    };
                    crate::notify::notify(summary, body);
                }
            }
        }
    }

    let timer_page_widget =
        timer_page::build(Rc::clone(&config), Rc::clone(&timer), Rc::clone(&tasks), &pill_ring, &pill_time);
    stack.add_named(&timer_page_widget, Some("timer"));

    let tasks_page_widget = tasks_page::build(Rc::clone(&tasks));
    stack.add_named(&tasks_page_widget, Some("tasks"));

    let apply_config_changes = gtk4::glib::clone!(
        #[strong]
        config,
        #[weak]
        window,
        #[strong]
        on_layer_shell,
        #[strong]
        drag_state,
        move || {
            let cfg = config.borrow();
            crate::ui::reload_theme_with_opacity(cfg.window.opacity);
            if on_layer_shell {
                let corner = parse_corner(&cfg.window.anchor);
                drag_state.borrow_mut().corner = corner;
                window.set_layer(if cfg.window.always_on_top {
                    Layer::Overlay
                } else {
                    Layer::Top
                });
                for edge in [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom] {
                    window.set_anchor(edge, false);
                }
                if let Some(h) = h_edge(corner) {
                    window.set_anchor(h, true);
                    window.set_margin(h, cfg.window.margin_x);
                }
                if let Some(v) = v_edge(corner) {
                    window.set_anchor(v, true);
                    window.set_margin(v, cfg.window.margin_y);
                }
            }
        }
    );

    let settings_page_widget = settings_page::build(Rc::clone(&config), apply_config_changes);
    stack.add_named(&settings_page_widget, Some("settings"));

    dropdown_box.append(&stack);

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

    // ── Shortcuts ───────────────────────────────────────────────────────────
    let shortcut_controller = gtk4::ShortcutController::new();

    add_shortcut(
        &shortcut_controller,
        "<Control>q",
        gtk4::glib::clone!(
            #[weak]
            window,
            #[upgrade_or]
            gtk4::glib::Propagation::Proceed,
            move || {
                window.close();
                gtk4::glib::Propagation::Stop
            }
        ),
    );

    add_shortcut(
        &shortcut_controller,
        "Escape",
        gtk4::glib::clone!(
            #[weak]
            window,
            #[upgrade_or]
            gtk4::glib::Propagation::Proceed,
            move || {
                window.close();
                gtk4::glib::Propagation::Stop
            }
        ),
    );

    add_shortcut(
        &shortcut_controller,
        "<Control>1",
        gtk4::glib::clone!(
            #[weak]
            timer_btn,
            #[upgrade_or]
            gtk4::glib::Propagation::Proceed,
            move || {
                timer_btn.set_active(true);
                gtk4::glib::Propagation::Stop
            }
        ),
    );

    add_shortcut(
        &shortcut_controller,
        "<Control>2",
        gtk4::glib::clone!(
            #[weak]
            tasks_btn,
            #[upgrade_or]
            gtk4::glib::Propagation::Proceed,
            move || {
                tasks_btn.set_active(true);
                gtk4::glib::Propagation::Stop
            }
        ),
    );

    add_shortcut(
        &shortcut_controller,
        "<Control>3",
        gtk4::glib::clone!(
            #[weak]
            settings_btn,
            #[upgrade_or]
            gtk4::glib::Propagation::Proceed,
            move || {
                settings_btn.set_active(true);
                gtk4::glib::Propagation::Stop
            }
        ),
    );

    add_shortcut(
        &shortcut_controller,
        "space",
        gtk4::glib::clone!(
            #[strong]
            timer,
            move || {
                timer.borrow_mut().toggle();
                gtk4::glib::Propagation::Stop
            }
        ),
    );

    add_shortcut(
        &shortcut_controller,
        "<Control>r",
        gtk4::glib::clone!(
            #[strong]
            timer,
            #[strong]
            config,
            move || {
                let cfg = config.borrow();
                timer.borrow_mut().reset(&cfg.timer);
                gtk4::glib::Propagation::Stop
            }
        ),
    );

    add_shortcut(
        &shortcut_controller,
        "<Control>s",
        gtk4::glib::clone!(
            #[strong]
            timer,
            #[strong]
            config,
            move || {
                let cfg = config.borrow();
                timer.borrow_mut().skip(&cfg.timer);
                gtk4::glib::Propagation::Stop
            }
        ),
    );

    // Persist timer on close (space/Escape/Ctrl+Q/close button all route through window.close).
    window.connect_close_request(gtk4::glib::clone!(
        #[strong]
        timer,
        move |_| {
            let snap = timer.borrow().snapshot();
            if let Err(e) = crate::timer::save_timer_snapshot(&snap) {
                eprintln!("tomato: failed to save timer state on close: {e}");
            }
            gtk4::glib::Propagation::Proceed
        }
    ));

    window.add_controller(shortcut_controller);
    window.present();
}

fn add_shortcut<F>(controller: &gtk4::ShortcutController, trigger_str: &str, action_fn: F)
where
    F: Fn() -> gtk4::glib::Propagation + 'static,
{
    if let Some(trigger) = gtk4::ShortcutTrigger::parse_string(trigger_str) {
        let action = gtk4::CallbackAction::new(move |_, _| action_fn());
        let shortcut = gtk4::Shortcut::builder()
            .trigger(&trigger)
            .action(&action)
            .build();
        controller.add_shortcut(shortcut);
    }
}

fn queue_draw_recursive(widget: &gtk4::Widget) {
    widget.queue_draw();
    let mut child = widget.first_child();
    while let Some(c) = child {
        queue_draw_recursive(&c);
        child = c.next_sibling();
    }
}
