#!/usr/bin/env bash
# triM. — Debian (.deb) Package Builder
# Builds a standard Linux package for Debian, Ubuntu, Linux Mint, Pop!_OS
set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_DIR"

PKG_NAME="trim"
PKG_VERSION="0.2.0"
PKG_ARCH="all"
DEB_NAME="${PKG_NAME}_${PKG_VERSION}_${PKG_ARCH}"
STAGE_DIR="$PROJECT_DIR/build/deb/$DEB_NAME"
DIST_DIR="$PROJECT_DIR/dist"

echo "=========================================================="
echo "  triM. — Building Debian Package ($DEB_NAME.deb)"
echo "=========================================================="

# Clean up previous build
rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR"
mkdir -p "$DIST_DIR"

# 1. Generate fresh icons and branding if not present
if [ ! -f "$PROJECT_DIR/trim/resources/icons/trim_512x512.png" ]; then
    echo "Generating icons..."
    python3 "$PROJECT_DIR/scripts/generate_branding_and_icons.py"
fi

# 2. Setup directory tree
mkdir -p "$STAGE_DIR/DEBIAN"
mkdir -p "$STAGE_DIR/usr/bin"
mkdir -p "$STAGE_DIR/usr/share/trim"
mkdir -p "$STAGE_DIR/usr/share/applications"
mkdir -p "$STAGE_DIR/usr/share/metainfo"
mkdir -p "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps"

# 3. Create DEBIAN/control
cat << EOF > "$STAGE_DIR/DEBIAN/control"
Package: $PKG_NAME
Version: $PKG_VERSION
Section: video
Priority: optional
Architecture: $PKG_ARCH
Maintainer: M. Özçelik <198776086+MOzcelik14@users.noreply.github.com>
Depends: python3 (>= 3.10), python3-pip | python3-venv, ffmpeg
Recommends: libxcb-cursor0, libegl1
Homepage: https://github.com/MOzcelik14/triM
Description: A lightweight non-linear video editor
 triM. is a modern, fast, and native non-linear video editor for Linux.
 Built with Python 3.12, PySide6 (Qt6), FFmpeg, and PyAV, it provides a clean,
 precision-oriented workflow for video cutting, trimming, audio mixing, and exporting.
EOF

# 4. Create DEBIAN/postinst
cat << 'EOF' > "$STAGE_DIR/DEBIAN/postinst"
#!/bin/sh
set -e

if which update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi

if which gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
fi

exit 0
EOF
chmod 755 "$STAGE_DIR/DEBIAN/postinst"

# 5. Create DEBIAN/postrm
cat << 'EOF' > "$STAGE_DIR/DEBIAN/postrm"
#!/bin/sh
set -e

if which update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi

if which gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
fi

exit 0
EOF
chmod 755 "$STAGE_DIR/DEBIAN/postrm"

# 6. Copy application source
cp -r "$PROJECT_DIR/trim" "$STAGE_DIR/usr/share/trim/"
cp -r "$PROJECT_DIR/cutline" "$STAGE_DIR/usr/share/trim/"
cp "$PROJECT_DIR/pyproject.toml" "$STAGE_DIR/usr/share/trim/"
cp "$PROJECT_DIR/requirements.txt" "$STAGE_DIR/usr/share/trim/"
cp "$PROJECT_DIR/README.md" "$STAGE_DIR/usr/share/trim/"

# Clean any pycache or temporary files in package
find "$STAGE_DIR/usr/share/trim" -name "__pycache__" -type d -exec rm -rf {} + 2>/dev/null || true
find "$STAGE_DIR/usr/share/trim" -name "*.pyc" -delete 2>/dev/null || true

# 7. Create launcher /usr/bin/trim
cat << 'EOF' > "$STAGE_DIR/usr/bin/trim"
#!/usr/bin/env bash
# triM. System Launcher
set -e

APP_DIR="/usr/share/trim"
ENV_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/trim/env"

# Check for user isolated virtualenv or create it automatically
if [ ! -f "$ENV_DIR/bin/python3" ] || [ ! -f "$ENV_DIR/.installed" ]; then
    echo "=========================================================="
    echo "  triM.: İlk çalıştırma için ortam hazırlanıyor..."
    echo "=========================================================="
    mkdir -p "$(dirname "$ENV_DIR")"
    python3 -m venv "$ENV_DIR"
    "$ENV_DIR/bin/pip" install --upgrade pip --quiet
    "$ENV_DIR/bin/pip" install -r "$APP_DIR/requirements.txt" --quiet
    touch "$ENV_DIR/.installed"
    echo "  Ortam başarıyla hazırlandı."
fi

export PYTHONPATH="$APP_DIR:$PYTHONPATH"
exec "$ENV_DIR/bin/python3" -m trim.main "$@"
EOF
chmod 755 "$STAGE_DIR/usr/bin/trim"

# 8. Install desktop file and metadata
cp "$PROJECT_DIR/io.github.mozcelik14.triM.desktop" "$STAGE_DIR/usr/share/applications/"
cp "$PROJECT_DIR/io.github.mozcelik14.triM.metainfo.xml" "$STAGE_DIR/usr/share/metainfo/"

# 9. Install scalable and multi-size icons
cp "$PROJECT_DIR/io.github.mozcelik14.triM.svg" "$STAGE_DIR/usr/share/icons/hicolor/scalable/apps/"

for sz in 16 32 48 64 128 256 512; do
    icon_src="$PROJECT_DIR/trim/resources/icons/trim_${sz}x${sz}.png"
    if [ -f "$icon_src" ]; then
        target_dir="$STAGE_DIR/usr/share/icons/hicolor/${sz}x${sz}/apps"
        mkdir -p "$target_dir"
        cp "$icon_src" "$target_dir/io.github.mozcelik14.triM.png"
    fi
done

# 10. Fix file permissions
find "$STAGE_DIR" -type d -exec chmod 755 {} +
find "$STAGE_DIR/usr/share" -type f -exec chmod 644 {} +
chmod 755 "$STAGE_DIR/usr/bin/trim"
chmod 755 "$STAGE_DIR/DEBIAN/postinst"
chmod 755 "$STAGE_DIR/DEBIAN/postrm"

# 11. Build package using dpkg-deb
echo "Packaging into $DIST_DIR/$DEB_NAME.deb..."
dpkg-deb --build --root-owner-group "$STAGE_DIR" "$DIST_DIR/$DEB_NAME.deb"

echo "=========================================================="
echo "  Build Complete: $DIST_DIR/$DEB_NAME.deb"
echo "  To install: sudo apt install ./dist/$DEB_NAME.deb"
echo "=========================================================="
