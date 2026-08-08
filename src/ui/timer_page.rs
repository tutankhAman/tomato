use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk4::prelude::*;

use crate::config::Config;
use crate::timer::{Phase, Status, Timer};
use crate::todo::TodoStore;

pub fn build(
    config: Rc<RefCell<Config>>,
    timer: Rc<RefCell<Timer>>,
    store: Rc<RefCell<TodoStore>>,
) -> gtk4::Widget {
    let page = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    page.add_css_class("tomato-page");
    page.set_valign(gtk4::Align::Center);

    let pill = gtk4::Label::new(Some("FOCUS"));
    pill.add_css_class("tomato-phase-pill");
    pill.add_css_class("phase-focus");

    let time = gtk4::Label::new(Some("25:00"));
    time.add_css_class("tomato-time");

    let progress = gtk4::ProgressBar::new();
    progress.add_css_class("tomato-progress");
    progress.set_show_text(false);
    progress.set_hexpand(true);

    let sessions = gtk4::Label::new(Some("SESSION 1 / 4"));
    sessions.add_css_class("tomato-sessions");

    let active_task_label = gtk4::Label::new(None);
    active_task_label.add_css_class("tomato-active-task");
    active_task_label.set_ellipsize(pango::EllipsizeMode::End);

    let controls = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    controls.add_css_class("tomato-controls");
    controls.set_homogeneous(true);

    let start_btn = gtk4::Button::with_label("START");
    start_btn.add_css_class("tomato-btn");
    start_btn.add_css_class("primary");

    let reset_btn = gtk4::Button::with_label("RESET");
    reset_btn.add_css_class("tomato-btn");

    let skip_btn = gtk4::Button::with_label("SKIP");
    skip_btn.add_css_class("tomato-btn");

    controls.append(&start_btn);
    controls.append(&reset_btn);
    controls.append(&skip_btn);

    page.append(&pill);
    page.append(&time);
    page.append(&progress);
    page.append(&sessions);
    page.append(&active_task_label);
    page.append(&controls);

    let refresh = gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        timer,
        #[strong]
        store,
        #[weak]
        pill,
        #[weak]
        time,
        #[weak]
        progress,
        #[weak]
        sessions,
        #[weak]
        active_task_label,
        #[weak]
        start_btn,
        move || {
            let cfg = config.borrow();
            let t = timer.borrow();

            let (phase_text, phase_class) = match t.phase() {
                Phase::Focus => ("FOCUS", "phase-focus"),
                Phase::ShortBreak => ("SHORT BREAK", "phase-short"),
                Phase::LongBreak => ("LONG BREAK", "phase-long"),
            };
            pill.set_label(phase_text);
            pill.remove_css_class("phase-focus");
            pill.remove_css_class("phase-short");
            pill.remove_css_class("phase-long");
            pill.add_css_class(phase_class);

            time.set_label(&t.remaining_mmss());
            progress.set_fraction(t.progress(&cfg.timer));

            sessions.set_label(&format!(
                "SESSION {} / {}",
                t.completed_focus_sessions() + 1,
                cfg.timer.cycles_before_long_break.max(1)
            ));

            let s = store.borrow();
            if let Some(active) = s.active_task() {
                active_task_label.set_label(&format!("🎯 {}", active.title));
                active_task_label.set_visible(true);
            } else {
                active_task_label.set_visible(false);
            }

            if t.status() == Status::Running {
                start_btn.set_label("PAUSE");
                start_btn.add_css_class("running");
            } else {
                start_btn.set_label("START");
                start_btn.remove_css_class("running");
            }
        }
    );

    refresh();

    start_btn.connect_clicked(gtk4::glib::clone!(
        #[strong]
        timer,
        #[strong]
        refresh,
        move |_| {
            timer.borrow_mut().toggle();
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
        move |_| {
            let cfg = config.borrow();
            timer.borrow_mut().reset(&cfg.timer);
            drop(cfg);
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
        move |_| {
            let cfg = config.borrow();
            timer.borrow_mut().skip(&cfg.timer);
            drop(cfg);
            refresh();
        }
    ));

    gtk4::glib::timeout_add_local(
        Duration::from_secs(1),
        gtk4::glib::clone!(
            #[strong]
            config,
            #[strong]
            timer,
            #[strong]
            store,
            #[strong]
            refresh,
            move || {
                let status = timer.borrow().status();
                if status == Status::Running {
                    let cfg = config.borrow();
                    let new_phase = timer.borrow_mut().tick(Duration::from_secs(1), &cfg.timer);
                    if let Some(phase) = new_phase {
                        if (phase == Phase::ShortBreak || phase == Phase::LongBreak)
                            && store.borrow_mut().increment_active_pomodoro()
                        {
                            let _ = store.borrow().save();
                        }

                        if cfg.notifications.enabled {
                            let (summary, body) = match phase {
                                Phase::Focus => ("Break Over", "Back to work! Stay focused."),
                                Phase::ShortBreak => ("Focus Complete", "Nice work! Take a short break."),
                                Phase::LongBreak => ("Session Finished", "Great job! Enjoy a long break."),
                            };
                            crate::notify::notify(summary, body);
                        }
                    }
                    drop(cfg);
                    refresh();
                } else {
                    refresh();
                }
                gtk4::glib::ControlFlow::Continue
            }
        ),
    );

    page.upcast()
}
