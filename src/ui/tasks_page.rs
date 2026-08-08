use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

use crate::todo::{Todo, TodoStore};

pub fn build(store: Rc<RefCell<TodoStore>>) -> gtk4::Widget {
    let page = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    page.add_css_class("tomato-page");

    let entry_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let entry = gtk4::Entry::new();
    entry.set_hexpand(true);
    entry.add_css_class("tomato-entry");
    entry.set_placeholder_text(Some("Add a task…"));

    let add_btn = gtk4::Button::from_icon_name("list-add-symbolic");
    add_btn.add_css_class("tomato-addbtn");
    add_btn.set_tooltip_text(Some("Add task"));

    entry_row.append(&entry);
    entry_row.append(&add_btn);
    page.append(&entry_row);

    let scroller = gtk4::ScrolledWindow::new();
    scroller.set_vexpand(true);
    scroller.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let list = gtk4::ListBox::new();
    list.add_css_class("tomato-list");
    list.set_selection_mode(gtk4::SelectionMode::None);
    scroller.set_child(Some(&list));
    page.append(&scroller);

    let footer = gtk4::Label::new(Some(""));
    footer.add_css_class("tomato-footer");
    page.append(&footer);

    let update_ui: Rc<dyn Fn()> = Rc::new(gtk4::glib::clone!(
        #[strong]
        store,
        #[weak]
        list,
        #[weak]
        footer,
        move || {
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            let s = store.borrow();
            let mut items = s.items.clone();
            items.sort_by_key(|t| t.done);

            for todo in &items {
                let is_active = s.active_id.as_deref() == Some(&todo.id);
                let row = make_row(todo, is_active, &store, &list, &footer);
                list.append(&row);
            }

            let active_count = s.remaining_count();
            let done_count = items.len() - active_count;
            footer.set_label(&format!("{active_count} active · {done_count} done"));
        }
    ));

    let add_task = gtk4::glib::clone!(
        #[weak]
        entry,
        #[strong]
        store,
        #[strong]
        update_ui,
        move || {
            let text = entry.text();
            let title = text.trim();
            if title.is_empty() {
                return;
            }
            store.borrow_mut().add(title.to_string());
            if let Err(e) = store.borrow().save() {
                eprintln!("tomato: failed to save todo store: {e}");
            }
            entry.set_text("");
            update_ui();
        }
    );

    add_btn.connect_clicked(gtk4::glib::clone!(
        #[strong]
        add_task,
        move |_| add_task()
    ));

    entry.connect_activate(gtk4::glib::clone!(
        #[strong]
        add_task,
        move |_| add_task()
    ));

    update_ui();

    page.upcast()
}

fn make_row(
    todo: &Todo,
    is_active: bool,
    store: &Rc<RefCell<TodoStore>>,
    list: &gtk4::ListBox,
    footer: &gtk4::Label,
) -> gtk4::ListBoxRow {
    let row = gtk4::ListBoxRow::new();
    let row_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    row_box.add_css_class("tomato-task-row");
    if is_active {
        row_box.add_css_class("active-target");
    }
    row.set_child(Some(&row_box));

    let check = gtk4::CheckButton::new();
    check.set_active(todo.done);

    let title = gtk4::Label::new(Some(&todo.title));
    title.set_hexpand(true);
    title.set_xalign(0.0);
    title.set_wrap(true);
    title.add_css_class("tomato-task-title");
    if todo.done {
        title.add_css_class("done");
    }

    // Pomodoro count badge
    let pomo_text = format!("🍅 {}", todo.pomodoros_done);
    let pomo_badge = gtk4::Label::new(if todo.pomodoros_done > 0 {
        Some(&pomo_text)
    } else {
        None
    });
    pomo_badge.add_css_class("tomato-pomo-badge");

    // Active focus target button
    let star_btn = gtk4::Button::from_icon_name(if is_active {
        "starred-symbolic"
    } else {
        "non-starred-symbolic"
    });
    star_btn.add_css_class("tomato-iconbtn");
    star_btn.set_tooltip_text(Some(if is_active {
        "Current focus target"
    } else {
        "Set as focus target"
    }));

    let del_btn = gtk4::Button::from_icon_name("user-trash-symbolic");
    del_btn.add_css_class("tomato-iconbtn");
    del_btn.add_css_class("destructive");
    del_btn.set_tooltip_text(Some("Delete task"));

    row_box.append(&check);
    row_box.append(&title);
    row_box.append(&pomo_badge);
    row_box.append(&star_btn);
    row_box.append(&del_btn);

    let id = todo.id.clone();

    check.connect_toggled(gtk4::glib::clone!(
        #[strong]
        store,
        #[strong]
        id,
        #[weak]
        title,
        #[weak]
        list,
        #[weak]
        footer,
        move |c| {
            let done = c.is_active();
            store.borrow_mut().toggle(&id);
            if done {
                title.add_css_class("done");
            } else {
                title.remove_css_class("done");
            }
            if let Err(e) = store.borrow().save() {
                eprintln!("tomato: failed to save todo store: {e}");
            }
            let s = store.borrow();
            let active_count = s.remaining_count();
            let done_count = s.items.len() - active_count;
            footer.set_label(&format!("{active_count} active · {done_count} done"));
            drop(s);
            // Re-sort list visually
            list.invalidate_sort();
        }
    ));

    star_btn.connect_clicked(gtk4::glib::clone!(
        #[strong]
        store,
        #[strong]
        id,
        #[weak]
        list,
        move |_| {
            let current = store.borrow().active_id.clone();
            if current.as_deref() == Some(&id) {
                store.borrow_mut().set_active(None);
            } else {
                store.borrow_mut().set_active(Some(id.clone()));
            }
            if let Err(e) = store.borrow().save() {
                eprintln!("tomato: failed to save todo store: {e}");
            }
            list.invalidate_sort();
        }
    ));

    del_btn.connect_clicked(gtk4::glib::clone!(
        #[strong]
        store,
        #[strong]
        id,
        #[weak]
        row,
        #[weak]
        list,
        #[weak]
        footer,
        move |_| {
            store.borrow_mut().remove(&id);
            if let Err(e) = store.borrow().save() {
                eprintln!("tomato: failed to save todo store: {e}");
            }
            list.remove(&row);
            let s = store.borrow();
            let active_count = s.remaining_count();
            let done_count = s.items.len() - active_count;
            footer.set_label(&format!("{active_count} active · {done_count} done"));
        }
    ));

    row
}
