#!/usr/bin/env bash
# triM. — Installation and Desktop Setup Script
# Installs native Rust triM desktop editor.
set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_DIR"

echo "=========================================================="
echo "  triM. — Native Rust Video Editor"
echo "  Kurulum ve Masaüstü Entegrasyonu"
echo "=========================================================="

echo "[1/4] Rust / Cargo kontrol ediliyor..."
if ! command -v cargo &>/dev/null; then
    echo "Hata: 'cargo' bulunamadı. Lütfen Rust araç setini kurun (https://rustup.rs)." >&2
    exit 1
fi

echo "[2/4] Release modunda derleniyor (trim-app)..."
cargo build --release -p trim-app

echo "[3/4] İkili dosya, masaüstü girdisi ve ikonlar kuruluyor..."
mkdir -p ~/.local/bin
mkdir -p ~/.local/share/applications
mkdir -p ~/.local/share/metainfo
mkdir -p ~/.local/share/icons/hicolor/scalable/apps

install -Dm755 "$PROJECT_DIR/target/release/trim" ~/.local/bin/trim
install -Dm644 "$PROJECT_DIR/io.github.mozcelik14.triM.svg" ~/.local/share/icons/hicolor/scalable/apps/io.github.mozcelik14.triM.svg
install -Dm644 "$PROJECT_DIR/io.github.mozcelik14.triM.metainfo.xml" ~/.local/share/metainfo/io.github.mozcelik14.triM.metainfo.xml

for sz in 16 32 48 64 128 256 512; do
    icon_src="$PROJECT_DIR/resources/icons/trim_${sz}x${sz}.png"
    if [ -f "$icon_src" ]; then
        target_dir="$HOME/.local/share/icons/hicolor/${sz}x${sz}/apps"
        mkdir -p "$target_dir"
        cp "$icon_src" "$target_dir/io.github.mozcelik14.triM.png"
    fi
done

cat << EOF > ~/.local/share/applications/io.github.mozcelik14.triM.desktop
[Desktop Entry]
Name=triM.
GenericName=Video Editor
GenericName[tr]=Video Düzenleyici
Comment=A lightweight non-linear video editor
Comment[tr]=Hafif ve modern lineer olmayan video düzenleyici
Exec=$HOME/.local/bin/trim %F
Icon=io.github.mozcelik14.triM
Terminal=false
Type=Application
Categories=AudioVideo;Video;AudioVideoEditing;
MimeType=application/x-trim-project;application/x-cutline-project;
Keywords=video;editor;cut;trim;timeline;ffmpeg;nle;
StartupNotify=true
StartupWMClass=triM.
EOF

update-desktop-database ~/.local/share/applications 2>/dev/null || true
gtk-update-icon-cache -f -t ~/.local/share/icons/hicolor 2>/dev/null || true

echo "[4/4] Kurulum başarıyla tamamlandı!"
echo ""
echo "  Uygulamayı başlatmak için:"
echo "  • Terminalden: trim"
echo "  • Veya proje dizininden: ./run.sh"
echo "  • Veya uygulama menüsünden: triM. (Ses ve Video altında)"
echo "=========================================================="
