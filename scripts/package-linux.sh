#!/usr/bin/env bash
# Build Linux packages for the native Rust edition of triM.
# Usage: bash scripts/package-linux.sh deb|rpm|appimage|tarball
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
FORMAT="${1:-}"
case "$FORMAT" in deb|rpm|appimage|tarball) ;; *) echo "Usage: $0 deb|rpm|appimage|tarball" >&2; exit 2 ;; esac

VERSION="$(sed -nE 's/^version = "([0-9]+\.[0-9]+\.[0-9]+)"$/\1/p' Cargo.toml | head -n1)"
if [[ -z "$VERSION" ]]; then echo "Cannot determine Cargo workspace version" >&2; exit 1; fi
if [[ "${GITHUB_REF_TYPE:-}" == "tag" && "${GITHUB_REF_NAME:-}" != "v$VERSION" ]]; then
  echo "Tag ${GITHUB_REF_NAME} does not match Cargo version v$VERSION" >&2
  exit 1
fi
BIN="$ROOT/target/release/trim"
DESKTOP="$ROOT/io.github.mozcelik14.triM.desktop"
METAINFO="$ROOT/io.github.mozcelik14.triM.metainfo.xml"
ICON="$ROOT/io.github.mozcelik14.triM.svg"
DIST="$ROOT/dist"
mkdir -p "$DIST"
for path in "$BIN" "$DESKTOP" "$METAINFO" "$ICON"; do
  [[ -f "$path" ]] || { echo "Missing input: $path" >&2; exit 1; }
done
[[ -x "$BIN" ]] || { echo "Not executable: $BIN" >&2; exit 1; }

# Keep AppStream version metadata in sync without hand-editing XML each release.
METAINFO_SOURCE="$METAINFO"
METAINFO="$ROOT/build/linux/io.github.mozcelik14.triM.metainfo.xml"
mkdir -p "$(dirname "$METAINFO")"
python3 - "$METAINFO_SOURCE" "$METAINFO" "$VERSION" <<'PY'
import datetime
import sys
import xml.etree.ElementTree as ET

source, destination, version = sys.argv[1:]
tree = ET.parse(source)
releases = tree.getroot().find("releases")
if releases is not None and not any(
    release.get("version") == version for release in releases.findall("release")
):
    releases.insert(0, ET.Element(
        "release",
        {"version": version, "date": datetime.date.today().isoformat()},
    ))
tree.write(destination, encoding="utf-8", xml_declaration=True)
PY

install_payload() {
  local root="$1"
  install -Dm755 "$BIN" "$root/usr/bin/trim"
  install -Dm644 "$DESKTOP" "$root/usr/share/applications/io.github.mozcelik14.triM.desktop"
  install -Dm644 "$METAINFO" "$root/usr/share/metainfo/io.github.mozcelik14.triM.metainfo.xml"
  install -Dm644 "$ICON" "$root/usr/share/icons/hicolor/scalable/apps/io.github.mozcelik14.triM.svg"
  local size
  for size in 16 32 48 64 128 256 512; do
    local image="$ROOT/resources/icons/trim_${size}x${size}.png"
    [[ -f "$image" ]] || continue
    install -Dm644 "$image" "$root/usr/share/icons/hicolor/${size}x${size}/apps/io.github.mozcelik14.triM.png"
  done
}

write_runtime_launcher() {
  local target_script="$1"
  local rel_bin="$2"
  cat > "$target_script" <<EOF
#!/usr/bin/env bash
set -euo pipefail
HERE="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")" && pwd)"

# Dynamic library fallback paths (bundled libs first, then standard host paths for PipeWire/ALSA/Mesa/Wayland/X11)
for libdir in "\$HERE/usr/lib" "\$HERE/usr/lib/x86_64-linux-gnu" "\$HERE/lib"; do
  if [[ -d "\$libdir" ]]; then
    export LD_LIBRARY_PATH="\$libdir\${LD_LIBRARY_PATH:+:\$LD_LIBRARY_PATH}"
  fi
done

for bindir in "\$HERE/usr/bin" "\$HERE/bin"; do
  if [[ -d "\$bindir" ]]; then
    export PATH="\$bindir:\$PATH"
  fi
done

# Runtime FFmpeg / FFprobe resolution with host fallback
if [[ -z "\${TRIM_FFMPEG_PATH:-}" ]]; then
  if [[ -x "\$HERE/usr/bin/ffmpeg" ]]; then
    export TRIM_FFMPEG_PATH="\$HERE/usr/bin/ffmpeg"
  elif [[ -x "\$HERE/bin/ffmpeg" ]]; then
    export TRIM_FFMPEG_PATH="\$HERE/bin/ffmpeg"
  elif command -v ffmpeg >/dev/null 2>&1; then
    export TRIM_FFMPEG_PATH="\$(command -v ffmpeg)"
  fi
fi

if [[ -z "\${TRIM_FFPROBE_PATH:-}" ]]; then
  if [[ -x "\$HERE/usr/bin/ffprobe" ]]; then
    export TRIM_FFPROBE_PATH="\$HERE/usr/bin/ffprobe"
  elif [[ -x "\$HERE/bin/ffprobe" ]]; then
    export TRIM_FFPROBE_PATH="\$HERE/bin/ffprobe"
  elif command -v ffprobe >/dev/null 2>&1; then
    export TRIM_FFPROBE_PATH="\$(command -v ffprobe)"
  fi
fi

# Verify PipeWire / PulseAudio / ALSA runtime audio tool availability
if ! command -v pw-cat >/dev/null 2>&1 && ! command -v paplay >/dev/null 2>&1 && ! command -v aplay >/dev/null 2>&1; then
  echo "[triM] Uyari: Sistemde pw-cat (PipeWire), paplay (PulseAudio) veya aplay (ALSA) bulunamadi." >&2
fi

exec "\$HERE/$rel_bin" "\$@"
EOF
  chmod +x "$target_script"
}

build_deb() {
  command -v dpkg-deb >/dev/null
  local stage="$ROOT/build/linux/deb"
  rm -rf "$stage"
  install_payload "$stage"
  # A previously installed per-user ~/.local/bin/trim Python launcher can
  # shadow /usr/bin/trim on PATH; use an absolute Exec for distro packages.
  sed -i 's|^Exec=trim |Exec=/usr/bin/trim |' "$stage/usr/share/applications/io.github.mozcelik14.triM.desktop"
  mkdir -p "$stage/DEBIAN"
  cat > "$stage/DEBIAN/control" <<EOF
Package: trim
Version: $VERSION
Section: video
Priority: optional
Architecture: amd64
Maintainer: triM. contributors <198776086+MOzcelik14@users.noreply.github.com>
Depends: ffmpeg, libc6, libx11-6, libgl1, libasound2 | libasound2t64
Recommends: pipewire-bin | alsa-utils
Homepage: https://github.com/MOzcelik14/triM
Description: Native open-source video editor for Linux
 triM. is a native desktop video editor written in Rust.
 Features multi-track editing, sound mixing and FFmpeg-based export.
EOF
  dpkg-deb --build --root-owner-group "$stage" "$DIST/trim_${VERSION}_amd64.deb"
  dpkg-deb --info "$DIST/trim_${VERSION}_amd64.deb"
  dpkg-deb --contents "$DIST/trim_${VERSION}_amd64.deb" | grep -F '/usr/bin/trim'
}

build_rpm() {
  command -v rpmbuild >/dev/null
  local top="$ROOT/build/linux/rpmbuild"
  rm -rf "$top"
  mkdir -p "$top"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
  install -m755 "$BIN" "$top/SOURCES/trim"
  cp "$DESKTOP" "$top/SOURCES/triM.desktop"
  sed -i 's|^Exec=trim |Exec=/usr/bin/trim |' "$top/SOURCES/triM.desktop"
  cp "$METAINFO" "$top/SOURCES/triM.metainfo.xml"
  cp "$ICON" "$top/SOURCES/triM.svg"
  local size
  for size in 16 32 48 64 128 256 512; do
    local image="$ROOT/resources/icons/trim_${size}x${size}.png"
    if [[ -f "$image" ]]; then cp "$image" "$top/SOURCES/trim_${size}.png"; fi
  done
  cat > "$top/SPECS/trim.spec" <<EOF
Name: trim
Version: $VERSION
Release: 1
Summary: Native video editor for Linux
License: GPL-3.0-or-later
URL: https://github.com/MOzcelik14/triM
BuildArch: x86_64
Requires: /usr/bin/ffmpeg
%global debug_package %{nil}
%description
triM. is a native open-source desktop video editor written in Rust.

%prep

%build

%install
install -Dm755 %{_sourcedir}/trim %{buildroot}%{_bindir}/trim
install -Dm644 %{_sourcedir}/triM.desktop %{buildroot}%{_datadir}/applications/io.github.mozcelik14.triM.desktop
install -Dm644 %{_sourcedir}/triM.metainfo.xml %{buildroot}%{_datadir}/metainfo/io.github.mozcelik14.triM.metainfo.xml
install -Dm644 %{_sourcedir}/triM.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/io.github.mozcelik14.triM.svg
EOF
  for size in 16 32 48 64 128 256 512; do
    if [[ -f "$top/SOURCES/trim_${size}.png" ]]; then
      echo "install -Dm644 %{_sourcedir}/trim_${size}.png %{buildroot}%{_datadir}/icons/hicolor/${size}x${size}/apps/io.github.mozcelik14.triM.png" >> "$top/SPECS/trim.spec"
    fi
  done
  cat >> "$top/SPECS/trim.spec" <<'EOF'

%files
%{_bindir}/trim
%{_datadir}/applications/io.github.mozcelik14.triM.desktop
%{_datadir}/metainfo/io.github.mozcelik14.triM.metainfo.xml
%{_datadir}/icons/hicolor/

%changelog
EOF
  rpmbuild -bb --define "_topdir $top" "$top/SPECS/trim.spec"
  find "$top/RPMS" -type f -name '*.rpm' -exec cp -v {} "$DIST/" \;
  test -n "$(find "$DIST" -maxdepth 1 -name "trim-${VERSION}-*.rpm" -print -quit)"
  rpm -qip "$DIST"/trim-"$VERSION"-*.rpm
}

build_appimage() {
  local tools="$ROOT/build/linux/tools"
  local work="$ROOT/build/linux/appimage"
  local appdir="$work/triM.AppDir"
  rm -rf "$work"
  mkdir -p "$tools" "$work"
  install_payload "$appdir"
  write_runtime_launcher "$appdir/AppRun" "usr/bin/trim"
  local linuxdeploy="$tools/linuxdeploy-x86_64.AppImage"
  if [[ ! -s "$linuxdeploy" ]]; then
    curl --fail --location --retry 3 --silent --show-error \
      "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage" \
      --output "$linuxdeploy"
    chmod +x "$linuxdeploy"
  fi
  (
    cd "$work"
    export ARCH=x86_64 VERSION="$VERSION" APPIMAGE_EXTRACT_AND_RUN=1
    "$linuxdeploy" --appdir "$appdir" \
      --executable "$BIN" --desktop-file "$DESKTOP" --icon-file "$ICON" \
      --custom-apprun "$appdir/AppRun" \
      --output appimage
  )
  local output
  output="$(find "$work" -maxdepth 1 -type f -name '*.AppImage' -print -quit)"
  [[ -n "$output" ]] || { echo "linuxdeploy did not create an AppImage" >&2; exit 1; }
  mv "$output" "$DIST/triM-${VERSION}-x86_64.AppImage"
  chmod +x "$DIST/triM-${VERSION}-x86_64.AppImage"
  file "$DIST/triM-${VERSION}-x86_64.AppImage"
}

build_tarball() {
  local stage_parent="$ROOT/build/linux/tarball"
  local folder_name="triM-${VERSION}-linux-x86_64"
  local stage="$stage_parent/$folder_name"
  rm -rf "$stage_parent"
  mkdir -p "$stage"
  install_payload "$stage"
  write_runtime_launcher "$stage/trim" "usr/bin/trim"
  if [[ -d "$ROOT/resources" ]]; then
    cp -r "$ROOT/resources" "$stage/resources"
  fi
  cp "$ROOT/README.md" "$stage/README.md"
  if [[ -f "$ROOT/install.sh" ]]; then
    install -Dm755 "$ROOT/install.sh" "$stage/install.sh"
  fi
  tar -C "$stage_parent" -czf "$DIST/${folder_name}.tar.gz" "$folder_name"
  test -s "$DIST/${folder_name}.tar.gz"
}

case "$FORMAT" in
  deb) build_deb ;;
  rpm) build_rpm ;;
  appimage) build_appimage ;;
  tarball) build_tarball ;;
esac
echo "Built $FORMAT for triM. v$VERSION"
