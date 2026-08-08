# Tomato — Sticky Pomodoro + Todo Implementation Plan

**Goal:** A single Rust GTK4/libadwaita desktop app for Linux: configurable Pomodoro timer + todo tracker, in a slim always-on-top sticky panel with polished modern UI.

**Architecture:** One binary, GTK4 + libadwaita for widgets/theming, `gtk4-layer-shell` (zwlr_layer_shell_v1) for true always-on-top sticky behaviour on Wayland with an X11/xdg-shell fallback. State (todos) and config (TOML) persist to `~/.config/tomato/` and `~/.local/share/tomato/`. Timer is a pure state machine driven by a 100ms GLib tick; UI is a thin render layer over it.

**Tech Stack:** Rust 2024, gtk4 0.11, libadwaita 0.9, gtk4-layer-shell 0.8 (feature `v1_3`), serde/serde_json/toml, notify-rust, chrono, anyhow, dirs.

**Verified environment (checked, do not re-litigate):**
- rustc 1.93.1 / cargo 1.93.1
- Wayland session, KDE/kwin_wayland, `zwlr_layer_shell_v1` v5 present → layer-shell path works
- System libs present: gtk4 4.22.4, libadwaita-1 1.9.2, gtk4-layer-shell-0 1.3.0
- Full dep set compiles clean on this machine (verified in a scratch crate)

---

## Non-negotiable requirements

1. **Sticky / always-on-top.** Wayland: layer-shell `Layer::Overlay`, anchored, `KeyboardMode::OnDemand`, `set_exclusive_zone(0)` so it floats *over* windows without reserving space. X11 fallback: `window.set_keep_above(true)` equivalent via `gdk_x11` hints; if unavailable, still run as a normal window (never crash).
2. **Configurable.** Every duration, the cycle count, anchor edge, margins, opacity, autostart behaviour, and notification/sound toggles live in `config.toml` and are editable **from the UI**, not just the file.
3. **Polished UI.** libadwaita styling, custom CSS, rounded 16px card, a real animated progress ring (custom `DrawingArea` snapshot, not a GtkProgressBar), smooth transitions, monospace tabular-figure timer digits so numbers don't jitter.
4. **No data loss.** Todos written atomically (temp file + rename). Config write likewise.

## Repo layout

```
~/repos/tomato/
├── Cargo.toml
├── README.md
├── .gitignore
├── install.sh
├── data/
│   ├── dev.aamn.tomato.desktop
│   └── style.css                # embedded via include_str!
└── src/
    ├── main.rs                  # app bootstrap, layer-shell init, CSS load
    ├── config.rs                # Config struct, load/save, defaults
    ├── timer.rs                 # pure Pomodoro state machine (unit-tested)
    ├── todo.rs                  # Todo model + atomic JSON store (unit-tested)
    ├── notify.rs                # desktop notifications wrapper
    └── ui/
        ├── mod.rs
        ├── window.rs            # root window, layer-shell, drag, view stack
        ├── ring.rs              # custom-drawn progress ring
        ├── timer_view.rs        # timer page
        ├── todo_view.rs         # todo list page
        └── settings_view.rs     # settings page
```

## Data contracts (fix these first — everything else depends on them)

### `config.toml` → `~/.config/tomato/config.toml`

```toml
[timer]
focus_minutes = 25
short_break_minutes = 5
long_break_minutes = 15
cycles_before_long_break = 4
auto_start_breaks = true
auto_start_focus = false

[notifications]
enabled = true
sound = true

[window]
anchor = "top-right"      # top-left|top-right|bottom-left|bottom-right|center
margin_x = 16
margin_y = 16
opacity = 0.97
always_on_top = true
compact = false
```

### `todos.json` → `~/.local/share/tomato/todos.json`

```json
{
  "version": 1,
  "items": [
    {
      "id": "01J...",
      "title": "Ship the ring widget",
      "done": false,
      "pomodoros_done": 2,
      "pomodoros_estimated": 4,
      "created_at": "2026-08-08T12:00:00Z",
      "completed_at": null
    }
  ]
}
```

### Timer state machine (`timer.rs`)

```rust
pub enum Phase { Focus, ShortBreak, LongBreak }
pub enum Status { Idle, Running, Paused }

pub struct Timer {
    phase: Phase,
    status: Status,
    remaining: Duration,
    completed_focus_sessions: u32,
}
```

Transitions:
- `start()` Idle/Paused → Running
- `pause()` Running → Paused
- `reset()` any → Idle, `remaining = phase duration`
- `skip()` → immediately advance phase
- `tick(dt)` decrements; at zero → `advance()` which increments `completed_focus_sessions` after Focus, picks LongBreak when `completed_focus_sessions % cycles_before_long_break == 0`, else ShortBreak; break → Focus. Honours `auto_start_*`.

**This module must be pure** — no GTK imports — so it is unit-testable with `cargo test`.

## Task breakdown

Each task ends with a build/test and a commit.

### Task 1 — Scaffold
`cargo init --name tomato`, edition 2024. Add deps with the exact versions verified above. Add `.gitignore` (`/target`), MIT-ish README stub.
Verify: `cargo build` → Finished. Commit `chore: scaffold tomato crate`.

### Task 2 — `config.rs`
Serde structs `Config { timer, notifications, window }` with `#[serde(default)]` on every field so a partial/older config never fails to load. `Config::load()` → read `~/.config/tomato/config.toml`, on missing/parse-error return `Config::default()` and log to stderr (never panic). `Config::save()` → create dirs, write to `config.toml.tmp`, `fs::rename` over the real path.
Tests: default round-trips through toml; a config file with only `[timer] focus_minutes = 50` loads with all other fields defaulted.
Verify: `cargo test config` → 2 passed. Commit.

### Task 3 — `timer.rs`
Implement the state machine above. No GTK.
Tests (all required):
- `tick` past zero on Focus with `cycles_before_long_break = 4` and 3 prior sessions → LongBreak
- same with 1 prior session → ShortBreak
- `pause` then `tick` does not decrement
- `reset` restores full phase duration
- `skip` from Focus advances without incrementing beyond the correct count
Verify: `cargo test timer` → 5 passed. Commit.

### Task 4 — `todo.rs`
`Todo` struct + `TodoStore { items }` with `add/toggle/remove/clear_completed/increment_pomodoro/save/load`. IDs from timestamp+counter (no extra crate). Atomic save via tmp+rename. Corrupt JSON → back it up to `todos.json.bak` and start empty rather than dying.
Tests: add→save→load round-trip; toggle sets `completed_at`; corrupt file yields empty store and writes `.bak`.
Verify: `cargo test todo` → 3 passed. Commit.

### Task 5 — `notify.rs`
Thin wrapper over `notify-rust`: `notify(summary, body)` with app name "Tomato", `Timeout::Milliseconds(6000)`, urgency Normal. Errors are logged, never propagated — a missing notification daemon must not kill the timer.
Verify: `cargo build`. Commit.

### Task 6 — `main.rs` + window shell (the risky one)
`adw::Application` with id `dev.aamn.tomato`. On activate:
1. Load CSS from `include_str!("../data/style.css")` into a `CssProvider` at `STYLE_PROVIDER_PRIORITY_APPLICATION`.
2. Build `adw::ApplicationWindow`, default size ~360×460, `set_decorated(false)`, `add_css_class("tomato-root")`.
3. **Sticky:** if `gtk4_layer_shell::is_supported()` → `init_layer_shell()`, `set_layer(Layer::Overlay)`, `set_namespace(Some("tomato"))`, `set_keyboard_mode(KeyboardMode::OnDemand)`, `set_exclusive_zone(0)`, anchor to the two edges implied by `config.window.anchor`, `set_margin` per axis. Else fall back to a plain window and print one warning line.
4. Custom drag: `GestureDrag` on the header bar area → move the window. Note: on layer-shell there is no compositor move; implement drag by mutating margins live (`set_margin(Edge::Top/Left, ...)`) and persisting to config on drag end.
5. `Ctrl+Q` quit, `Esc` hide-to-tray-less minimize (just `set_visible(false)` if a second instance can reveal it — otherwise omit; YAGNI).

**Pitfall (must respect):** `init_layer_shell()` must be called **before** the window is first presented, and `set_respect_close(true)` + a `close-request` handler returning `Propagation::Stop` is needed if we want to survive compositor `.closed` events. Default in v1.3 is not to forward, which is what we want — do nothing extra.

Verify: `cargo run` launches a frameless card floating above other windows on kwin_wayland. Screenshot it. Commit.

### Task 7 — `ui/ring.rs` progress ring
`gtk::DrawingArea` with `set_draw_func`. Draw: background arc (2px, low-alpha), progress arc (8px, rounded caps, phase-tinted, `-π/2` start, sweeping clockwise by `progress`), centered time text `MM:SS` in a tabular-figures font, phase label beneath. Repaint via `queue_draw()` from the tick.
Phase palette: Focus `#ff6b6b`→`#ee5a52`, Short `#4ecdc4`, Long `#5b8def`.
Verify: ring renders and sweeps. Commit.

### Task 8 — `ui/timer_view.rs`
Ring + phase pill + session counter dots (filled = completed this cycle) + controls: Start/Pause (single toggle, primary pill button), Reset, Skip. A 100ms `glib::timeout_add_local` drives `timer.tick()`; on phase change fire `notify::notify()` and, if the active todo exists and the finished phase was Focus, `increment_pomodoro` on it.
Verify: run, watch a 6-second test focus (temporarily set config) roll into a break with a notification. Commit.

### Task 9 — `ui/todo_view.rs`
`AdwEntryRow` for input (Enter adds), `gtk::ListBox` of rows: checkbox, title (strikethrough + dim when done), `2/4` pomodoro badge, delete button on hover. "Active" todo is selectable and highlighted — that's the one the timer credits. Empty state with an icon + one line of copy. Footer: "N left" + "Clear completed".
Verify: add/toggle/delete/persist across restart. Commit.

### Task 10 — `ui/settings_view.rs`
`adw::PreferencesPage` with groups: Durations (three `SpinRow` 1–120), Cycle (`SpinRow` 2–8), Automation (two `SwitchRow`), Notifications (two `SwitchRow`), Appearance (anchor `ComboRow`, opacity `SpinRow`/scale, compact `SwitchRow`). Every change writes config immediately and applies live where possible (opacity, anchor, margins instantly; durations apply to the next phase, or to the current one if Idle).
Verify: change focus to 1 min, reset, confirm ring shows 01:00; restart app, setting persisted. Commit.

### Task 11 — `data/style.css`
Dark glassy card: `background: rgba(24,24,29,0.92)`, `border-radius: 16px`, subtle 1px `rgba(255,255,255,0.08)` border, soft shadow. Pill buttons with hover/active transitions (`transition: all 150ms ease`). Tabular numerals on the timer digits. Respect `AdwStyleManager` dark preference; force dark since the panel is an overlay.
Verify: visually inspect via screenshot. Commit.

### Task 12 — Packaging + polish
- `data/dev.aamn.tomato.desktop` (`Categories=Utility;`, `StartupWMClass=tomato`)
- `install.sh`: `cargo build --release`, copy binary to `~/.local/bin/tomato`, desktop file to `~/.local/share/applications/`
- `README.md`: what it is, screenshot, build, install, config reference, keybinds
- `cargo clippy -- -D warnings` clean, `cargo fmt`
Verify: `./install.sh` then launch `tomato` from the app launcher. Commit + tag `v0.1.0`.

## Risks & mitigations

| Risk | Mitigation |
|---|---|
| Layer-shell window can't be dragged by the compositor | Drag = live margin mutation, persisted to config. Planned in Task 6, not discovered later. |
| Overlay layer swallows clicks meant for apps below | `set_exclusive_zone(0)` + `KeyboardMode::OnDemand`; the surface only covers its own 360×460 footprint. |
| gtk4-layer-shell absent on some machine / X11 session | `is_supported()` guard with plain-window fallback. Never a hard dependency at runtime. |
| GTK4 CSS is not web CSS | Only use GTK-supported properties. No flexbox, no `gap`, no `transform`. Box-shadow and border-radius are fine. |
| Notification daemon missing | Errors swallowed and logged in `notify.rs`. |
| Edition 2024 + gtk-rs closure borrow pain | Use `glib::clone!` macro with `@weak`/`@strong` consistently; shared state as `Rc<RefCell<T>>`. |

## Verification checklist (definition of done)

- [ ] `cargo build --release` clean
- [ ] `cargo test` — all timer/config/todo tests pass
- [ ] `cargo clippy -- -D warnings` clean
- [ ] App launches, renders as a floating card above other windows on kwin_wayland
- [ ] Timer counts down, ring animates, phase advances, notification fires
- [ ] Todos add/toggle/delete and survive a restart
- [ ] Settings change durations and appearance, and survive a restart
- [ ] Screenshot captured as visual evidence

## Execution

Delegated to OpenCode with `merge-gateway/deepseek/deepseek-v4-flash`, one `opencode run` per task group, with `cargo build`/`cargo test` gating between groups and Hermes verifying each result independently (no self-reported success accepted).




