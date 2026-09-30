#!/usr/bin/env bash
# triM. — Installation and Desktop Setup Script
# Created by M. Özçelik
set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VENV_DIR="$PROJECT_DIR/.venv"

echo "=========================================================="
echo "  triM. — A Lightweight Non-Linear Video Editor"
echo "  Kurulum ve Masaüstü Entegrasyonu"
echo "=========================================================="

echo "[1/4] Python sanal ortamı hazırlanıyor..."
if [ ! -d "$VENV_DIR" ]; then
    python3 -m venv "$VENV_DIR"
    echo "  Sanal ortam oluşturuldu: $VENV_DIR"
else
    echo "  Mevcut sanal ortam kullanılıyor: $VENV_DIR"
fi

echo "[2/4] Bağımlılıklar yükleniyor (PySide6, PyAV, NumPy)..."
"$VENV_DIR/bin/pip" install --upgrade pip --quiet
"$VENV_DIR/bin/pip" install -r "$PROJECT_DIR/requirements.txt" --quiet

echo "[3/4] İkonlar ve masaüstü kısayolları yükleniyor..."
mkdir -p ~/.local/share/applications
mkdir -p ~/.local/share/icons/hicolor/scalable/apps
mkdir -p ~/.local/bin

# 1. Generate icons if missing
if [ ! -f "$PROJECT_DIR/trim/resources/icons/trim_512x512.png" ]; then
    "$VENV_DIR/bin/python3" "$PROJECT_DIR/scripts/generate_branding_and_icons.py"
fi

# 2. Copy scalable SVG icon
cp "$PROJECT_DIR/io.github.mozcelik14.triM.svg" ~/.local/share/icons/hicolor/scalable/apps/io.github.mozcelik14.triM.svg

# 3. Copy multi-resolution PNG icons
for sz in 16 32 48 64 128 256 512; do
    icon_src="$PROJECT_DIR/trim/resources/icons/trim_${sz}x${sz}.png"
    if [ -f "$icon_src" ]; then
        target_dir="$HOME/.local/share/icons/hicolor/${sz}x${sz}/apps"
        mkdir -p "$target_dir"
        cp "$icon_src" "$target_dir/io.github.mozcelik14.triM.png"
    fi
done

# 4. Create ~/.local/bin/trim executable wrapper
cat << EOF > ~/.local/bin/trim
#!/usr/bin/env bash
export PYTHONPATH="$PROJECT_DIR:\$PYTHONPATH"
exec "$VENV_DIR/bin/python3" -m trim.main "\$@"
EOF
chmod +x ~/.local/bin/trim

# 5. Install desktop entry
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
