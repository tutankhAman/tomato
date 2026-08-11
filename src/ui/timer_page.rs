use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;
use std::time::Duration;

use gtk4::prelude::*;

use crate::config::Config;
use crate::timer::{Phase, Status, Timer};
use crate::todo::TodoStore;

/// Diameter of the progress ring.
const RING: i32 = 204;
/// Ring stroke width.
const RING_W: f64 = 9.0;

pub fn build(
    config: Rc<RefCell<Config>>,
    timer: Rc<RefCell<Timer>>,
    store: Rc<RefCell<TodoStore>>,
    pill_ring: &gtk4::DrawingArea,
    pill_time: &gtk4::Label,
) -> gtk4::Widget {
    let page = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    page.add_css_class("tm-page");

    // ── Phase label ─────────────────────────────────────────────────────────
    let phase_lbl = gtk4::Label::new(Some("FOCUS"));
    phase_lbl.add_css_class("tm-phase");
    phase_lbl.add_css_class("tm-phase-focus");
    phase_lbl.set_halign(gtk4::Align::Center);
    phase_lbl.set_margin_top(6);
    page.append(&phase_lbl);

    // ── Ring ────────────────────────────────────────────────────────────────
    let overlay = gtk4::Overlay::new();
    overlay.set_halign(gtk4::Align::Center);
    overlay.set_valign(gtk4::Align::Center);
    overlay.set_size_request(RING, RING);
    overlay.set_margin_top(8);

    let ring = gtk4::DrawingArea::new();
    ring.set_size_request(RING, RING);
    ring.set_content_width(RING);
    ring.set_content_height(RING);

    let time = gtk4::Label::new(Some("25:00"));
    time.add_css_class("tm-time");

    let sub = gtk4::Label::new(Some("REMAINING"));
    sub.add_css_class("tm-time-sub");

    let center = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    center.set_valign(gtk4::Align::Center);
    center.set_halign(gtk4::Align::Center);
    center.append(&time);
    center.append(&sub);

    overlay.set_child(Some(&ring));
    overlay.add_overlay(&center);
    page.append(&overlay);

    // ── Session dots ────────────────────────────────────────────────────────
    let dots = gtk4::Box::new(gtk4::Orientation::Horizontal, 7);
    dots.add_css_class("tm-dots");
    dots.set_halign(gtk4::Align::Center);
    dots.set_margin_top(12);
    page.append(&dots);

    // ── Active task chip ────────────────────────────────────────────────────
    let chip = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    chip.add_css_class("tm-chip");
    chip.set_halign(gtk4::Align::Center);
    chip.set_margin_top(10);

    let chip_dot = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    chip_dot.add_css_class("tm-chip-dot");
    chip_dot.set_valign(gtk4::Align::Center);
    chip.append(&chip_dot);

    let chip_text = gtk4::Label::new(None);
    chip_text.add_css_class("tm-chip-text");
    chip_text.set_ellipsize(pango::EllipsizeMode::End);
    chip_text.set_max_width_chars(26);
    chip.append(&chip_text);
    page.append(&chip);

    // ── Controls ────────────────────────────────────────────────────────────
    let controls = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    controls.add_css_class("tm-ctl");
    controls.set_halign(gtk4::Align::Center);
    controls.set_margin_top(16);

    let reset_btn = gtk4::Button::from_icon_name("view-refresh-symbolic");
    reset_btn.add_css_class("tm-btn-ghost");
    reset_btn.set_tooltip_text(Some("Reset (Ctrl+R)"));

    let start_btn = gtk4::Button::with_label("Start Focus");
    start_btn.add_css_class("tm-btn-main");

    let skip_btn = gtk4::Button::from_icon_name("media-skip-forward-symbolic");
    skip_btn.add_css_class("tm-btn-ghost");
    skip_btn.set_tooltip_text(Some("Skip phase (Ctrl+S)"));

    controls.append(&reset_btn);
    controls.append(&start_btn);
    controls.append(&skip_btn);
    page.append(&controls);

    // ── Smooth ring animation state ─────────────────────────────────────────
    // Displayed progress eases toward the timer's real progress every frame.
    let shown = Rc::new(RefCell::new(0.0f64));
    // Real progress at last refresh; a big jump (reset/skip/phase change) snaps.
    let target = Rc::new(RefCell::new(0.0f64));

    ring.set_draw_func(gtk4::glib::clone!(
        #[strong]
        shown,
        move |area, cr, width, height| {
            let p = *shown.borrow();
            let w = width as f64;
            let h = height as f64;
            let cx = w / 2.0;
            let cy = h / 2.0;
            let radius = (w.min(h) - RING_W) / 2.0;

            // Track: derive from the widget's themed foreground color.
            #[allow(deprecated)]
            let fg = area.style_context().color();
            let (fr, fg_, fb) = (f64::from(fg.red()), f64::from(fg.green()), f64::from(fg.blue()));

            cr.set_source_rgba(fr, fg_, fb, 0.10);
            cr.set_line_width(RING_W - 2.0);
            cr.arc(cx, cy, radius, 0.0, 2.0 * PI);
            let _ = cr.stroke();

            if p <= 0.0005 {
                return;
            }

            // Phase accent comes from the widget's `color`, set by the
            // tm-ring-* CSS phase classes.
            #[allow(deprecated)]
            let accent = area.style_context().color();
            let (ar, ag, ab) = (
                f64::from(accent.red()),
                f64::from(accent.green()),
                f64::from(accent.blue()),
            );

            let start = -PI / 2.0;
            let end = start + p * 2.0 * PI;

            // Soft halo under the arc
            cr.set_source_rgba(ar, ag, ab, 0.16);
            cr.set_line_width(RING_W + 6.0);
            cr.set_line_cap(cairo::LineCap::Round);
            cr.arc(cx, cy, radius, start, end);
            let _ = cr.stroke();

            // Main arc
            cr.set_source_rgba(ar, ag, ab, 1.0);
            cr.set_line_width(RING_W);
            cr.arc(cx, cy, radius, start, end);
            let _ = cr.stroke();

            // Leading knob
            let kx = cx + radius * end.cos();
            let ky = cy + radius * end.sin();
            cr.set_source_rgba(ar, ag, ab, 1.0);
            cr.arc(kx, ky, RING_W / 2.0 + 1.5, 0.0, 2.0 * PI);
            let _ = cr.fill();

            let _ = (fr, fg_, fb);
        }
    ));

    // ── Pill Ring ───────────────────────────────────────────────────────────
    pill_ring.set_draw_func(gtk4::glib::clone!(
        #[strong]
        shown,
        move |area, cr, width, height| {
            let p = *shown.borrow();
            let w = width as f64;
            let h = height as f64;
            let cx = w / 2.0;
            let cy = h / 2.0;
            let ring_w = 4.0; // smaller stroke for pill
            let radius = (w.min(h) - ring_w) / 2.0;

            #[allow(deprecated)]
            let fg = area.style_context().color();
            let (fr, fg_, fb) = (f64::from(fg.red()), f64::from(fg.green()), f64::from(fg.blue()));

            cr.set_source_rgba(fr, fg_, fb, 0.10);
            cr.set_line_width(ring_w - 1.0);
            cr.arc(cx, cy, radius, 0.0, 2.0 * PI);
            let _ = cr.stroke();

            if p <= 0.0005 {
                return;
            }

            #[allow(deprecated)]
            let accent = area.style_context().color();
            let (ar, ag, ab) = (
                f64::from(accent.red()),
                f64::from(accent.green()),
                f64::from(accent.blue()),
            );

            let start = -PI / 2.0;
            let end = start + p * 2.0 * PI;

            cr.set_source_rgba(ar, ag, ab, 1.0);
            cr.set_line_width(ring_w);
            cr.arc(cx, cy, radius, start, end);
            let _ = cr.stroke();
        }
    ));

    // Frame callback eases `shown` toward `target`. Attach to pill_ring since it's always visible.
    pill_ring.add_tick_callback(gtk4::glib::clone!(
        #[strong]
        shown,
        #[strong]
        target,
        #[weak]
        ring,
        #[weak(rename_to = pill)]
        pill_ring,
        #[upgrade_or]
        gtk4::glib::ControlFlow::Break,
        move |_, _clock| {
            let t = *target.borrow();
            let mut s = shown.borrow_mut();
            let diff = t - *s;
            if diff.abs() < 0.000001 {
                if (*s - t).abs() > f64::EPSILON {
                    *s = t;
                    ring.queue_draw();
                    pill.queue_draw();
                }
                return gtk4::glib::ControlFlow::Continue;
            }
            let frame = 1.0f64 / 60.0;
            let k = 1.0 - (-5.0f64 * frame).exp();
            *s += diff * k;
            drop(s);
            ring.queue_draw();
            pill.queue_draw();
            gtk4::glib::ControlFlow::Continue
        }
    ));

    // ── Refresh ─────────────────────────────────────────────────────────────
    let refresh = gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        timer,
        #[strong]
        store,
        #[weak]
        phase_lbl,
        #[weak]
        time,
        #[weak]
        sub,
        #[weak]
        ring,
        #[weak]
        dots,
        #[weak]
        chip,
        #[weak]
        chip_text,
        #[weak]
        start_btn,
        #[weak(rename_to = pill_ring_w)]
        pill_ring,
        #[weak(rename_to = pill_time_w)]
        pill_time,
        #[strong]
        target,
        #[strong]
        shown,
        move || {
            let cfg = config.borrow();
            let t = timer.borrow();

            let (phase_text, phase_class) = match t.phase() {
                Phase::Focus => ("FOCUS", "tm-phase-focus"),
                Phase::ShortBreak => ("SHORT BREAK", "tm-phase-short"),
                Phase::LongBreak => ("LONG BREAK", "tm-phase-long"),
            };
            phase_lbl.set_label(phase_text);
            for c in ["tm-phase-focus", "tm-phase-short", "tm-phase-long"] {
                phase_lbl.remove_css_class(c);
                ring.remove_css_class(c);
                pill_ring_w.remove_css_class(c);
            }
            phase_lbl.add_css_class(phase_class);
            
            let ring_class = match t.phase() {
                Phase::Focus => "tm-ring-focus",
                Phase::ShortBreak => "tm-ring-short",
                Phase::LongBreak => "tm-ring-long",
            };
            ring.add_css_class(ring_class);
            pill_ring_w.add_css_class(ring_class);

            let mmss = t.remaining_mmss();
            time.set_label(&mmss);
            pill_time_w.set_label(&mmss);
            
            sub.set_label(match t.status() {
                Status::Running => "REMAINING",
                Status::Paused => "PAUSED",
                Status::Idle => "READY",
            });

            let real = t.progress(&cfg.timer);
            let mut tg = target.borrow_mut();
            // Snap on large jumps (reset / skip / phase change) — else ease.
            if (*tg - real).abs() > 0.03 || real < *tg - 0.001 && real < 0.02 {
                *shown.borrow_mut() = real;
                ring.queue_draw();
                pill_ring_w.queue_draw();
            }
            *tg = real;
            drop(tg);

            // Session dots
            while let Some(child) = dots.first_child() {
                dots.remove(&child);
            }
            let cycles = cfg.timer.cycles_before_long_break.max(1) as usize;
            let done = (t.completed_focus_sessions() % cfg.timer.cycles_before_long_break.max(1))
                as usize;
            for i in 0..cycles.min(8) {
                let d = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
                d.add_css_class("tm-dot");
                if i < done {
                    d.add_css_class("tm-dot-on");
                    d.add_css_class(phase_class);
                }
                dots.append(&d);
            }

            // Active task chip
            let s = store.borrow();
            if let Some(active) = s.active_task() {
                chip_text.set_label(&active.title);
                chip.set_visible(true);
            } else {
                chip.set_visible(false);
            }
            drop(s);

            // Start / pause button
            if t.status() == Status::Running {
                start_btn.set_label("Pause");
                start_btn.add_css_class("tm-btn-running");
            } else {
                start_btn.set_label(match t.phase() {
                    Phase::Focus => "Start Focus",
                    Phase::ShortBreak | Phase::LongBreak => "Start Break",
                });
                start_btn.remove_css_class("tm-btn-running");
            }
        }
    );

    let save_timer = {
        let timer = Rc::clone(&timer);
        Rc::new(move || {
            let snap = timer.borrow().snapshot();
            if let Err(e) = crate::timer::save_timer_snapshot(&snap) {
                eprintln!("tomato: failed to save timer state: {e}");
            }
        })
    };
    // Throttled save while running: one flush per ~1s
    let save_throttled = {
        let save_timer = Rc::clone(&save_timer);
        let last_save = Rc::new(RefCell::new(std::time::Instant::now()));
        Rc::new(move || {
            let now = std::time::Instant::now();
            if now.duration_since(*last_save.borrow()) >= Duration::from_secs(1) {
                *last_save.borrow_mut() = now;
                save_timer();
            }
        })
    };

    refresh();

    start_btn.connect_clicked(gtk4::glib::clone!(
        #[strong]
        timer,
        #[strong]
        refresh,
        #[strong]
        save_timer,
        move |_| {
            timer.borrow_mut().toggle();
            save_timer();
            refresh();
        }
    ));

    reset_btn.connect_clicked(gtk4::glib::clone!(
        #[strong]
        timer,
        #[strong]
        config,
        #[strong]
        refresh,
        #[strong]
        save_timer,
        move |_| {
            let cfg = config.borrow();
            timer.borrow_mut().reset(&cfg.timer);
            drop(cfg);
            save_timer();
            refresh();
        }
    ));

    skip_btn.connect_clicked(gtk4::glib::clone!(
        #[strong]
        timer,
        #[strong]
        config,
        #[strong]
        refresh,
        #[strong]
        save_timer,
        move |_| {
            let cfg = config.borrow();
            timer.borrow_mut().skip(&cfg.timer);
            drop(cfg);
            save_timer();
            refresh();
        }
    ));

    gtk4::glib::timeout_add_local(
        Duration::from_millis(250),
        gtk4::glib::clone!(
            #[strong]
            config,
            #[strong]
            timer,
            #[strong]
            store,
            #[strong]
            refresh,
            #[strong]
            save_timer,
            #[strong]
            save_throttled,
            move || {
                let status = timer.borrow().status();
                if status == Status::Running {
                    let cfg = config.borrow();
                    let new_phase = timer
                        .borrow_mut()
                        .tick(Duration::from_millis(250), &cfg.timer);
                    if let Some(phase) = new_phase {
                        if (phase == Phase::ShortBreak || phase == Phase::LongBreak)
                            && store.borrow_mut().increment_active_pomodoro()
                        {
                            let _ = store.borrow().save();
                        }
                        if cfg.notifications.enabled {
                            let (summary, body) = match phase {
                                Phase::Focus => ("Break over", "Back to work — stay focused."),
                                Phase::ShortBreak => {
                                    ("Focus complete", "Nice work. Take a short break.")
                                }
                                Phase::LongBreak => {
                                    ("Session finished", "Great job — enjoy a long break.")
                                }
                            };
                            crate::notify::notify(summary, body);
                        }
                        drop(cfg);
                        save_timer();
                    } else {
                        drop(cfg);
                        save_throttled();
                    }
                }
                refresh();
                gtk4::glib::ControlFlow::Continue
            }
        ),
    );

    page.upcast()
}
