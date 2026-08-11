#!/usr/bin/env bash
set -e

echo "🍅 Building Tomato (release)..."
cargo build --release

echo "📦 Installing binary to ~/.local/bin/tomato..."
mkdir -p "$HOME/.local/bin"
install -m 755 target/release/tomato "$HOME/.local/bin/tomato"

echo "🎨 Installing icons..."
mkdir -p "$HOME/.local/share/icons/hicolor/scalable/apps"
mkdir -p "$HOME/.local/share/pixmaps"
cp data/dev.aamn.tomato.svg "$HOME/.local/share/icons/hicolor/scalable/apps/dev.aamn.tomato.svg"
cp data/dev.aamn.tomato.svg "$HOME/.local/share/pixmaps/dev.aamn.tomato.svg"

if command -v rsvg-convert >/dev/null 2>&1; then
    for sz in 16 32 48 64 128 256 512; do
        mkdir -p "$HOME/.local/share/icons/hicolor/${sz}x${sz}/apps"
        rsvg-convert -w "$sz" -h "$sz" data/dev.aamn.tomato.svg -o "$HOME/.local/share/icons/hicolor/${sz}x${sz}/apps/dev.aamn.tomato.png"
    done
    cp "$HOME/.local/share/icons/hicolor/128x128/apps/dev.aamn.tomato.png" "$HOME/.local/share/pixmaps/dev.aamn.tomato.png"
fi

echo "🖥️ Installing desktop entry..."
mkdir -p "$HOME/.local/share/applications"
cp data/dev.aamn.tomato.desktop "$HOME/.local/share/applications/dev.aamn.tomato.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

echo "✅ Tomato has been installed successfully!"
echo "   You can now launch 'Tomato' from your application menu or run 'tomato' in your terminal."
