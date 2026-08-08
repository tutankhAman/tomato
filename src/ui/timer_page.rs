use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk4::prelude::*;

use crate::config::Config;
use crate::timer::{Phase, Status, Timer};

pub fn build(config: Rc<RefCell<Config>>, timer: Rc<RefCell<Timer>>) -> gtk4::Widget {
    let page = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    page.add_css_class("tomato-page");
    page.set_valign(gtk4::Align::Center);

    let pill = gtk4::Label::new(Some("FOCUS"));
    pill.add_css_class("tomato-phase-pill");

    let time = gtk4::Label::new(Some("00:00"));
    time.add_css_class("tomato-time");

    let progress = gtk4::ProgressBar::new();
    progress.add_css_class("tomato-progress");
    progress.set_show_text(false);
    progress.set_hexpand(true);

    let sessions = gtk4::Label::new(Some(""));
    sessions.add_css_class("tomato-sessions");

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
    page.append(&controls);

    let refresh: Rc<RefCell<Box<dyn FnMut()>>> = Rc::new(RefCell::new(Box::new(gtk4::glib::clone!(
        #[strong]
        config,
        #[strong]
        timer,
        #[weak]
        pill,
        #[weak]
        time,
        #[weak]
        progress,
        #[weak]
        sessions,
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
            start_btn.set_label(if t.status() == Status::Running {
                "PAUSE"
            } else {
                "START"
            });
        }
    ))));

    refresh.borrow_mut()();

    start_btn.connect_clicked(gtk4::glib::clone!(
        #[strong]
        timer,
        #[strong]
        refresh,
        move |_| {
            timer.borrow_mut().toggle();
            refresh.borrow_mut()();
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
            {
                let cfg = config.borrow();
                timer.borrow_mut().reset(&cfg.timer);
            }
            refresh.borrow_mut()();
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
            {
                let cfg = config.borrow();
                timer.borrow_mut().skip(&cfg.timer);
            }
            refresh.borrow_mut()();
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
            refresh,
            move || {
                {
                    let cfg = config.borrow();
                    let new_phase = timer.borrow_mut().tick(Duration::from_secs(1), &cfg.timer);
                    if let Some(phase) = new_phase
                        && cfg.notifications.enabled
                    {
                        let (summary, body) = match phase {
                            Phase::Focus => ("Break over", "Back to work"),
                            Phase::ShortBreak => ("Focus complete", "Nice work, take a break"),
                            Phase::LongBreak => ("Focus complete", "Great session, long break"),
                        };
                        crate::notify::notify(summary, body);
                    }
                }
                refresh.borrow_mut()();
                gtk4::glib::ControlFlow::Continue
            }
        ),
    );

    page.upcast()
}
