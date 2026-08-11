tomato

a sticky pomodoro timer and todo tracker for linux wayland. rust, gtk4,
libadwaita. it pins itself to a screen corner and counts down while you work.
no electron, no cloud, no account.

features

- pomodoro cycles: focus, short break, long break. set the lengths and how
  many focus blocks before a long break, and auto-start breaks or focus.
- todo panel: add, check off, rename, delete, and estimate tasks in pomodoros.
  each task tracks pomodoros done against its estimate.
- finishing a focus session bumps the active task's pomodoro count.
- daily stats: focus sessions and minutes, for today and the last 7 days.
- stays docked: runs on the gtk4 layer shell, pinned to a corner (top-right by
  default), always on top, draggable, with your compositor blur behind it.
- system tray: click to toggle the panel, quit from the menu.
- survives restarts: timer state is saved and fast-forwards through time spent
  closed, so a running session resumes where it left off.
- notifications on phase change, silent when no daemon is around.
- dark, compact, opacity configurable.

build

    cargo build --release
    ./install.sh

the binary lands in ~/.local/bin/tomato. icons and the desktop entry go to
your xdg directories.

run

    tomato

config

    ~/.config/tomato/config.toml

every field has a default, so a partial file is fine. open settings in the
panel to change values live, or hand-edit the file.

license

mit
