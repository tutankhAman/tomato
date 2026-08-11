//! KDE Wayland blur: ask the compositor to blur whatever is behind the pill
//! and dropdown, producing a frosted-glass look.
//!
//! KWin (and the better-blur-dx drop-in) advertises the `org_kde_kwin_blur_manager`
//! global from plasma-wayland-protocols. We bind it for our `wl_surface` and set
//! a rounded `wl_region` covering the pill + dropdown. Compositors without the
//! global, or non-Wayland backends, degrade to a no-op.

use std::cell::RefCell;
use std::rc::Rc;

use gdk4_wayland::prelude::WaylandSurfaceExtManual as _;
use gtk4::glib;
use gtk4::prelude::*;
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_compositor::{self, WlCompositor};
use wayland_client::protocol::wl_region::{self, WlRegion};
use wayland_client::protocol::wl_registry;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols_plasma::blur::client::org_kde_kwin_blur::{self, OrgKdeKwinBlur};
use wayland_protocols_plasma::blur::client::org_kde_kwin_blur_manager::{
    self, OrgKdeKwinBlurManager,
};

/// Corner radius (px) of the dropdown; must match `.tm-dropdown` CSS.
const DROPDOWN_RADIUS: i32 = 22;

/// Per-object user-data for the blur protocol objects.
struct State;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _state: &mut State,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qhandle: &QueueHandle<State>,
    ) {
    }
}

impl Dispatch<OrgKdeKwinBlurManager, ()> for State {
    fn event(
        _state: &mut State,
        _proxy: &OrgKdeKwinBlurManager,
        _event: org_kde_kwin_blur_manager::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<State>,
    ) {
    }
}

impl Dispatch<OrgKdeKwinBlur, ()> for State {
    fn event(
        _state: &mut State,
        _proxy: &OrgKdeKwinBlur,
        _event: org_kde_kwin_blur::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<State>,
    ) {
    }
}

impl Dispatch<WlCompositor, ()> for State {
    fn event(
        _state: &mut State,
        _proxy: &WlCompositor,
        _event: wl_compositor::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<State>,
    ) {
    }
}

impl Dispatch<WlRegion, ()> for State {
    fn event(
        _state: &mut State,
        _proxy: &WlRegion,
        _event: wl_region::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<State>,
    ) {
    }
}

/// A live blur session bound to one window surface.
struct Blur {
    conn: Connection,
    /// Keep the event queue alive so `qh` stays valid for new objects.
    #[allow(dead_code)]
    queue: EventQueue<State>,
    compositor: WlCompositor,
    qh: QueueHandle<State>,
    blur: OrgKdeKwinBlur,
    region: Option<WlRegion>,
    last: Option<Geometry>,
}

/// Pill + dropdown geometry, used to skip redundant region updates.
type Geometry = (i32, i32, i32, i32, i32, i32, i32, i32);

impl Blur {
    /// Replace the blur region to cover the given pill/dropdown rects.
    fn apply(&mut self, pill: gdk4::Rectangle, dropdown: gdk4::Rectangle) {
        let region = self.compositor.create_region(&self.qh, ());
        // The pill is a capsule: its corner radius is half its height.
        add_rounded(
            &region,
            pill.x(),
            pill.y(),
            pill.width(),
            pill.height(),
            pill.height() / 2,
        );
        if dropdown.width() > 0 && dropdown.height() > 0 {
            add_rounded(
                &region,
                dropdown.x(),
                dropdown.y(),
                dropdown.width(),
                dropdown.height(),
                DROPDOWN_RADIUS,
            );
        }
        self.blur.set_region(Some(&region));
        self.blur.commit();
        if let Some(old) = self.region.replace(region) {
            old.destroy();
        }
        let _ = self.conn.flush();
    }
}

/// Approximate a rounded rectangle with a few `wl_region` rectangles: the
/// straight top/bottom strips plus a full-width middle band. The four corner
/// squares stay unblurred, matching the rounded widget background.
fn add_rounded(region: &WlRegion, x: i32, y: i32, w: i32, h: i32, radius: i32) {
    if w <= 0 || h <= 0 {
        return;
    }
    let r = radius.clamp(0, w.min(h) / 2);
    if w - 2 * r > 0 {
        region.add(x + r, y, w - 2 * r, r);
        region.add(x + r, y + h - r, w - 2 * r, r);
    }
    region.add(x, y + r, w, h - 2 * r);
}

/// Bind the blur manager on `window`'s Wayland surface. Returns `None` when
/// the compositor doesn't support the protocol (non-KDE, non-Wayland).
fn bind(window: &gtk4::ApplicationWindow) -> Option<Blur> {
    let surface = window.surface()?.downcast::<gdk4_wayland::WaylandSurface>().ok()?;
    let wl_surface: WlSurface = surface.wl_surface()?;
    let backend = wl_surface.backend().upgrade()?;
    let conn = Connection::from_backend(backend);
    let (globals, queue) = registry_queue_init::<State>(&conn).ok()?;
    let qh = queue.handle();
    let compositor = globals.bind::<WlCompositor, State, ()>(&qh, 1..=4, ()).ok()?;
    let manager = globals.bind::<OrgKdeKwinBlurManager, State, ()>(&qh, 1..=1, ()).ok()?;
    let blur = manager.create(&wl_surface, &qh, ());
    let _ = conn.flush();
    Some(Blur { conn, queue, compositor, qh, blur, region: None, last: None })
}

/// Keep the blur region in sync with the pill/dropdown geometry.
fn refresh(state: &Rc<RefCell<Option<Blur>>>, pill: &gtk4::Widget, dropdown: &gtk4::Widget) {
    let mut borrow = state.borrow_mut();
    let Some(blur) = borrow.as_mut() else {
        return;
    };
    let p = pill.allocation();
    let d = dropdown.allocation();
    let key: Geometry = (p.x(), p.y(), p.width(), p.height(), d.x(), d.y(), d.width(), d.height());
    if blur.last == Some(key) {
        return;
    }
    blur.last = Some(key);
    blur.apply(p, d);
}

/// Request compositor blur behind `pill` and `dropdown`. Safe to call even
/// when the protocol is unavailable; it simply does nothing in that case.
pub fn install(window: &gtk4::ApplicationWindow, pill: &gtk4::Widget, dropdown: &gtk4::Widget) {
    let state: Rc<RefCell<Option<Blur>>> = Rc::new(RefCell::new(None));

    window.connect_map(glib::clone!(
        #[strong]
        state,
        #[weak]
        pill,
        #[weak]
        dropdown,
        move |win| {
            if state.borrow().is_none() {
                *state.borrow_mut() = bind(win);
            }
            refresh(&state, &pill, &dropdown);
        }
    ));

    // Refresh after any layout change (revealer animation, resize).
    // Use tick callback throttled via allocation polling; connect to
    // notify::scale-factor as a cheap layout-change hook plus a frame tick.
    window.add_tick_callback(glib::clone!(
        #[strong]
        state,
        #[weak]
        pill,
        #[weak]
        dropdown,
        #[upgrade_or]
        glib::ControlFlow::Continue,
        move |_, _| {
            refresh(&state, &pill, &dropdown);
            glib::ControlFlow::Continue
        }
    ));
}
