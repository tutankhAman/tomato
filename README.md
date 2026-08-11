# tomato

a pomodoro timer that refuses to leave.

rust, gtk4, libadwaita, wayland. it pins itself to your desktop and counts
down while you work. the todo panel sits beside it. no electron, no cloud,
no account.

## build

    cargo build --release
    ./install.sh

the binary lands in `~/.local/bin/tomato`.

## run

    tomato

## config

`~/.config/tomato/config.toml`

## license

mit
