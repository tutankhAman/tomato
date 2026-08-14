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
    has_moved: bool,
    corner: Corner,
    start_mx: i32,
    start_my: i32,
    press_sx: f64,
    press_sy: f64,
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

    // Session counter — current focus session within the cycle / total.
    let pill_cycle = gtk4::Label::new(Some("0/4"));
    pill_cycle.add_css_class("tm-pill-cycle");
    pill_cycle.set_valign(gtk4::Align::Center);
    pill.append(&pill_cycle);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    pill.append(&spacer);

    let pill_actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
    pill_actions.set_valign(gtk4::Align::Center);
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
    //   window size stays locked at 300 and the compositor never re-negotiates
    //   width mid-flight. No size-allocate loop, no dropped frames.
    // - Vertical motion is the revealer's SlideDown, clipped to the dropdown's
    //   box; the pill (overlay, no clip) slides its rounded bottom edge over
    //   that clip boundary, hiding the seam.
    window.set_size_request(280, -1);
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
    dropdown_box.set_size_request(280, 460);
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
    // that stays in lockstep with the revealer (same easing/duration).
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
    // Use `released` instead of `pressed` so `GestureDrag` has a chance
    // to set `has_moved` before we decide to toggle — fixes the "hold
    // then drag toggles the dropdown" glitch.
    let pill_click = gtk4::GestureClick::new();
    pill_click.set_button(1);
    pill_click.set_propagation_phase(gtk4::PropagationPhase::Bubble);
    pill_click.connect_released(gtk4::glib::clone!(
        #[strong] toggle,
        #[weak] close_btn,
        #[weak] drag_handle,
        #[weak] pill,
        #[strong] drag_state,
        move |_, n_press, x, y| {
            if n_press != 1 { return; }
            if drag_state.borrow().has_moved {
                return;
            }
            if let Some(target) = pill.pick(x, y, gtk4::PickFlags::DEFAULT) {
                let is_descendant = |ancestor: &gtk4::Widget| {
                    let mut cur = Some(target.clone());
                    while let Some(w) = cur {
                        if &w == ancestor { return true; }
                        cur = w.parent();
                    }
                    false
                };
                if is_descendant(close_btn.upcast_ref())
                    || is_descendant(drag_handle.upcast_ref())
                {
                    return;
                }
            }
            toggle();
        }
    ));
    pill.add_controller(pill_click);

    crate::ui::blur::install(&window, pill.upcast_ref(), dropdown_box.upcast_ref());

    let (tx, rx) = std::sync::mpsc::channel::<crate::ui::tray::TrayAction>();
    let toggle_for_tray = toggle.clone();
    let window_for_tray = window.clone();
    gtk4::glib::timeout_add_local(
        std::time::Duration::from_millis(100),
        move || {
            while let Ok(action) = rx.try_recv() {
                match action {
                    crate::ui::tray::TrayAction::Toggle => toggle_for_tray(),
                    crate::ui::tray::TrayAction::Quit => {
                        window_for_tray.close();
                    }
                }
            }
            gtk4::glib::ControlFlow::Continue
        },
    );

    crate::ui::tray::spawn(tx);

    // Drag for layer-shell: move the surface via anchored margins.
    // Fixes for the jitter/lag reported on Wayland:
    //   1. `GestureDrag`'s widget-local dx/dy is computed against the drag's
    //      *start* widget origin. When we move a layer-shell surface the widget
    //      moves with it, so dx collapses — that's the feedback loop that makes
    //      the pill lag and then snap. We track the pointer in *surface-local*
    //      coordinates (`GdkDevice::surface_at_position`) instead and use the
    //      surface error `cur - press` (0 when the handle is glued to the
    //      pointer).
    //   2. `window.margin()` reflects the compositor-committed value, so the next
    //      target is `actual + sign * error` (not `start + sign*(cur-press)`,
    //      which halves each frame because the error shrinks as the window
    //      catches up). This eliminates the "half-speed" integration jitter.
    //   3. At most one `set_margin` per frame (tick coalescence) and HiDPI
    //      correction (`scale_factor`). Threshold lowered to 2px to remove GTK's
    //      default 8px drag threshold (the initial friction).
    if let Some(settings) = gtk4::Settings::default() {
        if settings.gtk_dnd_drag_threshold() > 2 {
            settings.set_gtk_dnd_drag_threshold(2);
        }
    }
    drag_handle.set_cursor_from_name(Some("grab"));
    // Surface-local pointer tracker — None when the pointer left our surface
    // (fast fling); caller falls back to the gesture's widget coords.
    let surface_pos = Rc::new({
        let window = window.clone();
        move || -> Option<(f64, f64)> {
            let display = gdk4::Display::default()?;
            let seat = display.default_seat()?;
            let dev = seat.pointer()?;
            let (surf, sx, sy) = dev.surface_at_position();
            let win_surf = window.surface()?;
            if surf.as_ref() == Some(&win_surf) {
                Some((sx, sy))
            } else {
                None
            }
        }
    });
    // Single tick coalescer — coalesce motion events to the compositor frame
    // clock so we don't fire `set_margin` faster than the surface can commit.
    let pending: Rc<RefCell<Option<(i32, i32)>>> = Rc::new(RefCell::new(None));
    let tick_id: Rc<RefCell<Option<gtk4::TickCallbackId>>> = Rc::new(RefCell::new(None));
    let ensure_tick = {
        let window = window.clone();
        let pending = Rc::clone(&pending);
        let tick_id = Rc::clone(&tick_id);
        let drag_state = Rc::clone(&drag_state);
        Rc::new(move || {
            if tick_id.borrow().is_some() {
                return;
            }
            let id = window.add_tick_callback(gtk4::glib::clone!(
                #[strong] pending,
                #[strong] drag_state,
                #[weak] window,
                #[upgrade_or] gtk4::glib::ControlFlow::Continue,
                move |_, _| {
                    if let Some((mx, my)) = pending.borrow_mut().take() {
                        if let (Some(h), Some(v)) = (h_edge(drag_state.borrow().corner), v_edge(drag_state.borrow().corner))
                        {
                            window.set_margin(h, mx);
                            window.set_margin(v, my);
                        }
                    }
                    if drag_state.borrow().active {
                        gtk4::glib::ControlFlow::Continue
                    } else {
                        gtk4::glib::ControlFlow::Break
                    }
                }
            ));
            *tick_id.borrow_mut() = Some(id);
        })
    };
    let drag = gtk4::GestureDrag::new();
    drag.set_button(gdk4::BUTTON_PRIMARY as u32);
    drag.connect_drag_begin(gtk4::glib::clone!(
        #[strong] drag_state,
        #[weak] window,
        #[strong] on_layer_shell,
        #[weak] drag_handle,
        #[strong] surface_pos,
        #[strong] pending,
        move |_g, gx, gy| {
            if !on_layer_shell || drag_state.borrow().corner == Corner::Center {
                return;
            }
            let (px, py) = surface_pos().unwrap_or((gx, gy));
            let mut s = drag_state.borrow_mut();
            s.active = true;
            s.has_moved = false;
            s.start_mx = window.margin(h_edge(s.corner).unwrap());
            // v may equal h when corner is Center — already excluded — so safe to
            // read separately via h/v; keep one read path for clarity.
            let v = v_edge(s.corner).unwrap();
            s.start_my = window.margin(v);
            s.press_sx = px;
            s.press_sy = py;
            *pending.borrow_mut() = None;
            drag_handle.set_cursor_from_name(Some("grabbing"));
        }
    ));
    // Install the tick on first motion so the surface already exists.
    let drag_update = {
        let drag_state = Rc::clone(&drag_state);
        let window = window.clone();
        let surface_pos = Rc::clone(&surface_pos);
        let pending = Rc::clone(&pending);
        let ensure_tick = Rc::clone(&ensure_tick);
        move |_g: &gtk4::GestureDrag, dx: f64, dy: f64| {
            let mut s = drag_state.borrow_mut();
            if !on_layer_shell || !s.active || s.corner == Corner::Center {
                return;
            }
            let Some(h) = h_edge(s.corner) else { return; };
            let Some(v) = v_edge(s.corner) else { return; };
            let (cur_sx, cur_sy, use_surface) = if let Some((sx, sy)) = surface_pos() {
                (sx, sy, true)
            } else {
                (s.press_sx + dx, s.press_sy + dy, false)
            };
            let err_x = cur_sx - s.press_sx;
            let err_y = cur_sy - s.press_sy;
            if !s.has_moved && err_x.hypot(err_y) < 1.8 {
                return;
            }
            s.has_moved = true;
            let scale = window
                .surface()
                .map(|surf| surf.scale_factor() as f64)
                .unwrap_or(1.0)
                .max(1.0);
            let err_x_logical = if use_surface { err_x / scale } else { err_x };
            let err_y_logical = if use_surface { err_y / scale } else { err_y };
            // Correct 1:1 tracking: global delta = err + sign*(actual - start)
            // ⇒ target = actual + sign*err (not start+sign*err, which halves).
            let mx = (window.margin(h) as f64 + err_x_logical * drag_sign_x(s.corner) as f64)
                .round().max(0.0).min(4000.0) as i32;
            let my = (window.margin(v) as f64 + err_y_logical * drag_sign_y(s.corner) as f64)
                .round().max(0.0).min(4000.0) as i32;
            *pending.borrow_mut() = Some((mx, my));
            drop(s);
            ensure_tick();
        }
    };
    drag.connect_drag_update(gtk4::glib::clone!(#[strong] drag_update, move |a,b,c| drag_update(a,b,c)));
    let drag_handle_end = drag_handle.clone();
    let do_end = {
        let drag_state = Rc::clone(&drag_state);
        let window = window.clone();
        let pending = Rc::clone(&pending);
        let tick_id = Rc::clone(&tick_id);
        let config = Rc::clone(&config);
        let drag_handle = drag_handle_end.clone();
        Rc::new(move || {
            // Fast path: flush any coalesced pending before tearing down the tick.
            if let Some((mx, my)) = pending.borrow_mut().take() {
                if let (Some(h), Some(v)) = (h_edge(drag_state.borrow().corner), v_edge(drag_state.borrow().corner)) {
                    window.set_margin(h, mx);
                    window.set_margin(v, my);
                }
            }
            if let Some(id) = tick_id.borrow_mut().take() {
                id.remove();
            }
            drag_handle.set_cursor_from_name(Some("grab"));
            if !on_layer_shell || drag_state.borrow().corner == Corner::Center {
                drag_state.borrow_mut().active = false;
                drag_state.borrow_mut().has_moved = false;
                return;
            }
            let had_move = drag_state.borrow().has_moved;
            drag_state.borrow_mut().active = false;
            if !had_move {
                return;
            }
            if let (Some(h), Some(v)) = (h_edge(drag_state.borrow().corner), v_edge(drag_state.borrow().corner)) {
                let mx = window.margin(h);
                let my = window.margin(v);
                let mut cfg = config.borrow_mut();
                cfg.window.margin_x = mx;
                cfg.window.margin_y = my;
                if let Err(e) = cfg.save() {
                    eprintln!("tomato: failed to save config: {e}");
                }
                drop(cfg);
                let st = Rc::clone(&drag_state);
                gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(350), move || {
                    st.borrow_mut().has_moved = false;
                });
            } else {
                drag_state.borrow_mut().has_moved = false;
            }
        })
    };
    drag.connect_drag_end(gtk4::glib::clone!(#[strong] do_end, move |_,_,_| do_end()));
    drag.connect_cancel(gtk4::glib::clone!(
        #[strong] drag_state,
        #[strong] pending,
        #[strong] tick_id,
        #[weak] drag_handle,
        move |_, _seq| {
            drag_handle.set_cursor_from_name(Some("grab"));
            *pending.borrow_mut() = None;
            if let Some(id) = tick_id.borrow_mut().take() { id.remove(); }
            drag_state.borrow_mut().active = false;
            let st = Rc::clone(&drag_state);
            gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(350), move || {
                st.borrow_mut().has_moved = false;
            });
        }
    ));
    drag_handle.add_controller(drag);

    // ── Segmented switcher ──────────────────────────────────────────────────
    let switcher = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
    switcher.add_css_class("tm-seg");
    switcher.add_css_class("tm-seg-switcher");
    switcher.set_homogeneous(true);

    let timer_btn = gtk4::ToggleButton::new();
    let tasks_btn = gtk4::ToggleButton::new();
    let settings_btn = gtk4::ToggleButton::new();
    timer_btn.set_icon_name("alarm-symbolic");
    tasks_btn.set_icon_name("view-list-symbolic");
    settings_btn.set_icon_name("emblem-system-symbolic");
    timer_btn.set_tooltip_text(Some("Timer"));
    tasks_btn.set_tooltip_text(Some("Tasks"));
    settings_btn.set_tooltip_text(Some("Settings"));
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

    let (timer, saved_before) = {
        let cfg = config.borrow();
        match crate::timer::load_timer_snapshot() {
            Some(snap) => {
                let before_completed = snap.completed_focus_sessions;
                let before_phase = snap.phase;
                (Timer::restore(snap, &cfg.timer), Some((before_completed, before_phase)))
            }
            None => (Timer::new(&cfg.timer), None),
        }
    };
    let timer = Rc::new(RefCell::new(timer));
    let tasks = Rc::new(RefCell::new(TodoStore::load()));
    // Attribute any focus sessions that completed while the app was closed.
    if let Some((before_completed, before_phase)) = saved_before {
        let after = timer.borrow().completed_focus_sessions();
        if after > before_completed {
            let increments = after - before_completed;
            {
                let mut store = tasks.borrow_mut();
                for _ in 0..increments {
                    store.increment_active_pomodoro();
                }
            }
            let _ = tasks.borrow().save();
            // Notify once if we landed in a break after a completed focus.
            let cfg = config.borrow();
            if cfg.notifications.enabled
                && matches!(
                    timer.borrow().phase(),
                    crate::timer::Phase::ShortBreak | crate::timer::Phase::LongBreak
                )
                && before_phase == crate::timer::Phase::Focus
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

    let timer_page_widget = timer_page::build(
        Rc::clone(&config),
        Rc::clone(&timer),
        Rc::clone(&tasks),
        &pill_ring,
        &pill_time,
        &pill_cycle,
    );
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
        #[strong]
        config,
        move |_| {
            let snap = timer.borrow().snapshot();
            if let Err(e) = crate::timer::save_timer_snapshot(&snap) {
                eprintln!("tomato: failed to save timer state on close: {e}");
            }
            // Flush any pending debounced config writes (opacity slider, spins).
            if let Err(e) = config.borrow().save() {
                eprintln!("tomato: failed to save config on close: {e}");
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
