# Tomato — agent working notes

Rust GTK4 + libadwaita sticky Pomodoro + todo panel for Linux/Wayland.

## Hard constraints — do not violate

- **Rust edition 2024.** Toolchain is rustc 1.93.1.
- **Do NOT change `Cargo.toml` dependency versions or feature flags.** They are pinned and verified to compile on this machine. Adding a new small dep is allowed only if unavoidable.
- **`src/timer.rs`, `src/config.rs`, `src/todo.rs` must NOT import gtk/adw/glib.** They are pure logic and must be unit-testable with plain `cargo test`.
- **GTK4 CSS is not web CSS.** No flexbox, no `gap`, no `transform`, no CSS variables. Allowed: color, background, border, border-radius, box-shadow, padding, margin, font-*, opacity, transition, min-width/min-height.
- Never `.unwrap()` on I/O or on a missing notification daemon. Missing config, corrupt JSON, and a dead notifier must all degrade gracefully, never panic.
- All persistence writes are atomic: write `<file>.tmp`, then `fs::rename` over the target.

## Environment (verified — don't re-check)

- Wayland, KDE/kwin_wayland, `zwlr_layer_shell_v1` v5 available
- gtk4 4.22.4, libadwaita 1.9.2, gtk4-layer-shell 1.3.0 system libs installed
- `cargo build` on the current Cargo.toml succeeds

## Layer-shell rules

- Call `gtk4_layer_shell::is_supported()` first. If false, skip all layer-shell calls and use a normal window — the app must still run.
- `init_layer_shell()` must be called BEFORE the window is presented.
- Use `Layer::Overlay`, `set_exclusive_zone(0)`, `KeyboardMode::OnDemand`, `set_namespace(Some("tomato"))`.
- The window cannot be moved by the compositor. Implement dragging by mutating layer-shell margins live and persisting them to config on drag end.

## Paths

- Config: `~/.config/tomato/config.toml`
- Data: `~/.local/share/tomato/todos.json`
- App ID: `dev.aamn.tomato`

## Verification before you declare done

```
cargo build
cargo test
cargo clippy -- -D warnings
```

All three must be clean. Do not report success without running them.
