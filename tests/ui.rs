//! Headless UI smoke tests. Run with: cargo test --test ui.
//! These verify that each page actually builds a widget tree with the expected
//! elements, without needing a visible display.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use tomato::config::Config;
use tomato::timer::Timer;
use tomato::todo::TodoStore;
use tomato::ui::{tasks_page, timer_page};

static INIT: std::sync::Once = std::sync::Once::new();

fn init_gtk() {
    INIT.call_once(|| {
        let _ = gtk4::init();
    });
}

fn find_widgets<T: IsA<gtk4::Widget>>(root: &impl IsA<gtk4::Widget>) -> Vec<T> {
    let root = root.as_ref();
    let mut result = Vec::new();
    if let Ok(w) = root.clone().downcast::<T>() {
        result.push(w);
    }
    let mut child = root.first_child();
    while let Some(c) = child {
        result.extend(find_widgets::<T>(&c));
        child = c.next_sibling();
    }
    result
}

#[test]
fn ui_pages_build_expected_widgets() {
    init_gtk();

    // 1. Timer Page
    {
        let config = Rc::new(RefCell::new(Config::default()));
        let timer = Rc::new(RefCell::new(Timer::new(&config.borrow().timer)));
        let widget = timer_page::build(Rc::clone(&config), Rc::clone(&timer));

        let page = widget.downcast::<gtk4::Box>().expect("timer page is a Box");
        assert!(page.has_css_class("tomato-page"), "timer page has tomato-page class");

        let labels: Vec<gtk4::Label> = find_widgets(&page);
        let found_time = labels.iter().any(|lbl| lbl.label() == "25:00");
        let found_progress = !find_widgets::<gtk4::ProgressBar>(&page).is_empty();
        let buttons: Vec<gtk4::Button> = find_widgets(&page);

        assert!(found_time, "countdown shows default 25:00");
        assert!(found_progress, "progress bar present");
        assert!(buttons.len() >= 3, "start, reset, skip buttons present");
    }

    // 2. Tasks Page
    {
        let mut store = TodoStore::default();
        store.add("Write the integration test".to_string());
        store.add("Verify the tasks page renders".to_string());
        let store = Rc::new(RefCell::new(store));
        let widget = tasks_page::build(Rc::clone(&store));

        let page = widget.downcast::<gtk4::Box>().expect("tasks page is a Box");
        assert!(page.has_css_class("tomato-page"), "tasks page has tomato-page class");

        let entries: Vec<gtk4::Entry> = find_widgets(&page);
        let buttons: Vec<gtk4::Button> = find_widgets(&page);
        let rows: Vec<gtk4::ListBoxRow> = find_widgets(&page);
        let labels: Vec<gtk4::Label> = find_widgets(&page);
        let found_footer = labels.iter().any(|lbl| lbl.label().contains("active"));

        assert!(!entries.is_empty(), "add-task entry present");
        assert!(!buttons.is_empty(), "add (+) button present");
        assert_eq!(rows.len(), 2, "two seeded tasks rendered as rows");
        assert!(found_footer, "footer shows active/done counts");
    }
}
