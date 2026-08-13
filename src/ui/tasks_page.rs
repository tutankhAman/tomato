use std::cell::RefCell;
use std::rc::{Rc, Weak};

use gtk4::prelude::*;

use crate::todo::{Todo, TodoStore};

pub fn build(store: Rc<RefCell<TodoStore>>) -> gtk4::Widget {
    let page = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
    page.add_css_class("tm-page");
    page.set_margin_top(6);

    // ── Entry row ───────────────────────────────────────────────────────────
    let entry_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let entry = gtk4::Entry::new();
    entry.set_hexpand(true);
    entry.add_css_class("tm-entry");
    entry.set_placeholder_text(Some("Add a task…"));

    let add_btn = gtk4::Button::from_icon_name("list-add-symbolic");
    add_btn.add_css_class("tm-add");
    add_btn.set_tooltip_text(Some("Add task"));

    entry_row.append(&entry);
    entry_row.append(&add_btn);
    page.append(&entry_row);

    // ── List ────────────────────────────────────────────────────────────────
    let scroller = gtk4::ScrolledWindow::new();
    scroller.set_vexpand(true);
    scroller.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let list = gtk4::ListBox::new();
    list.add_css_class("tm-list");
    list.set_selection_mode(gtk4::SelectionMode::None);
    scroller.set_child(Some(&list));
    page.append(&scroller);

    // ── Footer ──────────────────────────────────────────────────────────────
    let footer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    footer.set_margin_top(2);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    footer.append(&spacer);

    let clear_btn = gtk4::Button::with_label("Clear done");
    clear_btn.add_css_class("tm-link");
    clear_btn.set_valign(gtk4::Align::Center);
    footer.append(&clear_btn);
    page.append(&footer);

    type RebuildFn = dyn Fn();
    // ── Rebuild ─────────────────────────────────────────────────────────────
    // Shared rebuild closure. Rows need a callback to trigger a rebuild, but
    // the closure also creates the rows — a direct self-capture would be an Rc
    // reference cycle. Instead the closure holds the cell weakly and the cell
    // holds only a weak ref back; the closure's own strong capture keeps the
    // cell alive, so both are dropped together when the page dies.
    let update_ui: Rc<RebuildFn> = {
        let store_c = Rc::clone(&store);
        let list_c = list.clone();
        let clear_c = clear_btn.clone();
        let cell: Rc<RefCell<Option<Weak<RebuildFn>>>> = Rc::new(RefCell::new(None));
        let cell_clone = Rc::clone(&cell);
        let mk: Rc<RebuildFn> = Rc::new(move || {
            while let Some(child) = list_c.first_child() {
                list_c.remove(&child);
            }
            let s = store_c.borrow();
            let mut items = s.items.clone();
            items.sort_by_key(|t| t.done);
            let cb = cell_clone.borrow().as_ref().and_then(|w| w.upgrade());
            for todo in &items {
                let is_active = s.active_id.as_deref() == Some(&todo.id);
                let row = if let Some(ref f) = cb {
                    make_row(todo, is_active, &store_c, f)
                } else {
                    make_row_fallback(todo, is_active, &store_c)
                };
                list_c.append(&row);
            }
            clear_c.set_visible(items.iter().any(|t| t.done));
        });
        *cell.borrow_mut() = Some(Rc::downgrade(&mk));
        mk
    };

    let add_task = gtk4::glib::clone!(
        #[weak]
        entry,
        #[strong]
        store,
        #[strong]
        update_ui,
        move || {
            let title = entry.text();
            let title = title.trim();
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

    clear_btn.connect_clicked(gtk4::glib::clone!(
        #[strong]
        store,
        #[strong]
        update_ui,
        move |_| {
            store.borrow_mut().clear_completed();
            if let Err(e) = store.borrow().save() {
                eprintln!("tomato: failed to save todo store: {e}");
            }
            update_ui();
        }
    ));

    update_ui();

    page.upcast()
}

fn make_row(
    todo: &Todo,
    is_active: bool,
    store: &Rc<RefCell<TodoStore>>,
    update_ui: &Rc<dyn Fn()>,
) -> gtk4::ListBoxRow {
    let row = gtk4::ListBoxRow::new();
    row.set_activatable(false);

    let row_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    row_box.add_css_class("tm-row");
    if is_active {
        row_box.add_css_class("tm-row-active");
    }
    row.set_child(Some(&row_box));

    // Round check
    let check = gtk4::CheckButton::new();
    check.add_css_class("tm-check");
    check.set_active(todo.done);
    check.set_valign(gtk4::Align::Center);

    // Title label + editable entry (stacked)
    let title_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    title_box.set_hexpand(true);
    title_box.set_valign(gtk4::Align::Center);
    let title = gtk4::Label::new(Some(&todo.title));
    title.set_hexpand(true);
    title.set_xalign(0.0);
    title.set_wrap(true);
    title.set_wrap_mode(pango::WrapMode::WordChar);
    title.add_css_class("tm-task");
    if todo.done {
        title.add_css_class("tm-task-done");
    }
    let edit = gtk4::Entry::new();
    edit.add_css_class("tm-entry");
    edit.add_css_class("tm-entry-inline");
    edit.set_text(&todo.title);
    edit.set_hexpand(true);
    edit.set_visible(false);
    title_box.append(&title);
    title_box.append(&edit);

    // Pomodoro badge: done/estimated or just done
    let pomo_badge = gtk4::Label::new(None);
    pomo_badge.add_css_class("tm-count");
    if is_active {
        pomo_badge.add_css_class("tm-count-active");
    }
    pomo_badge.set_valign(gtk4::Align::Center);
    update_pomo_label(&pomo_badge, todo.pomodoros_done, todo.pomodoros_estimated);

    // Estimate stepper (- / +)
    let est_minus = gtk4::Button::from_icon_name("list-remove-symbolic");
    est_minus.add_css_class("tm-rowbtn");
    est_minus.add_css_class("tm-rowbtn-sm");
    est_minus.set_tooltip_text(Some("Decrease estimate"));
    let est_plus = gtk4::Button::from_icon_name("list-add-symbolic");
    est_plus.add_css_class("tm-rowbtn");
    est_plus.add_css_class("tm-rowbtn-sm");
    est_plus.set_tooltip_text(Some("Increase estimate"));
    let est_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 1);
    est_box.set_valign(gtk4::Align::Center);
    est_box.append(&est_minus);
    est_box.append(&est_plus);

    // Focus target star
    let star_btn = gtk4::Button::from_icon_name(if is_active {
        "starred-symbolic"
    } else {
        "non-starred-symbolic"
    });
    star_btn.add_css_class("tm-rowbtn");
    if is_active {
        star_btn.add_css_class("tm-rowbtn-on");
    }
    star_btn.set_tooltip_text(Some(if is_active {
        "Current focus target"
    } else {
        "Set as focus target"
    }));

    // Delete
    let del_btn = gtk4::Button::from_icon_name("user-trash-symbolic");
    del_btn.add_css_class("tm-rowbtn");
    del_btn.set_tooltip_text(Some("Delete task"));

    // Hover-revealed secondary actions: estimate stepper + delete
    let actions_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
    actions_box.add_css_class("tm-row-actions");
    actions_box.set_valign(gtk4::Align::Center);
    actions_box.append(&est_box);
    actions_box.append(&del_btn);

    row_box.append(&check);
    row_box.append(&title_box);
    row_box.append(&pomo_badge);
    row_box.append(&star_btn);
    row_box.append(&actions_box);

    let id = todo.id.clone();
    let initial_title = todo.title.clone();

    // Double-click title to edit
    let click = gtk4::GestureClick::new();
    click.connect_pressed(gtk4::glib::clone!(
        #[weak]
        title,
        #[weak]
        edit,
        move |_, n_press, _, _| {
            if n_press == 2 {
                title.set_visible(false);
                edit.set_visible(true);
                edit.grab_focus();
                edit.select_region(0, -1);
            }
        }
    ));
    title_box.add_controller(click);

    let commit_edit = gtk4::glib::clone!(
        #[strong]
        store,
        #[strong]
        id,
        #[weak]
        title,
        #[weak]
        edit,
        move || {
            let new_text = edit.text().to_string();
            if new_text.trim().is_empty() {
                edit.set_text(title.label().as_ref());
                title.set_visible(true);
                edit.set_visible(false);
                return;
            }
            if store.borrow_mut().rename(&id, new_text.clone()) {
                if let Err(e) = store.borrow().save() {
                    eprintln!("tomato: failed to save todo store: {e}");
                }
                title.set_label(new_text.trim());
            }
            title.set_visible(true);
            edit.set_visible(false);
        }
    );
    let cancel_edit = gtk4::glib::clone!(
        #[weak]
        title,
        #[weak]
        edit,
        #[strong]
        initial_title,
        move || {
            edit.set_text(&initial_title);
            title.set_visible(true);
            edit.set_visible(false);
        }
    );
    edit.connect_activate(gtk4::glib::clone!(
        #[strong]
        commit_edit,
        move |_| commit_edit()
    ));
    // Use key controller for Escape
    let key = gtk4::EventControllerKey::new();
    key.connect_key_pressed(gtk4::glib::clone!(
        #[strong]
        cancel_edit,
        #[strong]
        commit_edit,
        move |_, keyval, _, _| {
            if keyval == gtk4::gdk::Key::Escape {
                cancel_edit();
                return gtk4::glib::Propagation::Stop;
            }
            if keyval == gtk4::gdk::Key::Return || keyval == gtk4::gdk::Key::KP_Enter {
                commit_edit();
                return gtk4::glib::Propagation::Stop;
            }
            gtk4::glib::Propagation::Proceed
        }
    ));
    edit.add_controller(key);
    // Commit on focus out
    let focus_ctrl = gtk4::EventControllerFocus::new();
    focus_ctrl.connect_leave(gtk4::glib::clone!(
        #[weak]
        edit,
        #[strong]
        commit_edit,
        move |_| {
            if gtk4::prelude::WidgetExt::is_visible(&edit) {
                commit_edit();
            }
        }
    ));
    edit.add_controller(focus_ctrl);

    let id_clone = id.clone();
    est_minus.connect_clicked(gtk4::glib::clone!(
        #[strong]
        store,
        #[strong]
        id_clone,
        #[weak]
        pomo_badge,
        move |_| {
            let mut s = store.borrow_mut();
            let cur = s.items.iter().find(|t| t.id == id_clone).map(|t| t.pomodoros_estimated).unwrap_or(0);
            let next = cur.saturating_sub(1);
            if s.set_estimate(&id_clone, next) {
                let done = s.items.iter().find(|t| t.id == id_clone).map(|t| t.pomodoros_done).unwrap_or(0);
                drop(s);
                update_pomo_label(&pomo_badge, done, next);
                if let Err(e) = store.borrow().save() {
                    eprintln!("tomato: failed to save todo store: {e}");
                }
            }
        }
    ));
    let id_clone2 = id.clone();
    est_plus.connect_clicked(gtk4::glib::clone!(
        #[strong]
        store,
        #[strong]
        id_clone2,
        #[weak]
        pomo_badge,
        move |_| {
            let mut s = store.borrow_mut();
            let cur = s.items.iter().find(|t| t.id == id_clone2).map(|t| t.pomodoros_estimated).unwrap_or(0);
            let next = (cur + 1).min(20);
            if s.set_estimate(&id_clone2, next) {
                let done = s.items.iter().find(|t| t.id == id_clone2).map(|t| t.pomodoros_done).unwrap_or(0);
                drop(s);
                update_pomo_label(&pomo_badge, done, next);
                if let Err(e) = store.borrow().save() {
                    eprintln!("tomato: failed to save todo store: {e}");
                }
            }
        }
    ));

    // Capture update_ui for delete/star toggles
    let update_ui_clone = Rc::clone(update_ui);

    check.connect_toggled(gtk4::glib::clone!(
        #[strong]
        store,
        #[strong]
        id,
        #[strong]
        update_ui_clone,
        move |c| {
            let _done = c.is_active();
            store.borrow_mut().toggle(&id);
            if let Err(e) = store.borrow().save() {
                eprintln!("tomato: failed to save todo store: {e}");
            }
            update_ui_clone();
        }
    ));

    let update_ui_clone2 = Rc::clone(update_ui);
    star_btn.connect_clicked(gtk4::glib::clone!(
        #[strong]
        store,
        #[strong]
        id,
        #[strong]
        update_ui_clone2,
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
            update_ui_clone2();
        }
    ));

    del_btn.connect_clicked(gtk4::glib::clone!(
        #[strong]
        store,
        #[strong]
        id,
        #[strong]
        update_ui,
        move |_| {
            store.borrow_mut().remove(&id);
            if let Err(e) = store.borrow().save() {
                eprintln!("tomato: failed to save todo store: {e}");
            }
            update_ui();
        }
    ));

    row
}

fn update_pomo_label(label: &gtk4::Label, done: u32, estimated: u32) {
    if done == 0 && estimated == 0 {
        label.set_label("");
        label.set_visible(false);
        return;
    }
    label.set_visible(true);
    if estimated > 0 {
        label.set_label(&format!("🍅 {done}/{estimated}"));
    } else {
        label.set_label(&format!("🍅 {done}"));
    }
}

fn make_row_fallback(
    todo: &Todo,
    is_active: bool,
    store: &Rc<RefCell<TodoStore>>,
) -> gtk4::ListBoxRow {
    let noop: Rc<dyn Fn()> = Rc::new(|| {});
    make_row(todo, is_active, store, &noop)
}
