# Linux Entegrasyonu ve Paketleme

Bu doküman, `triM.` uygulamasının Linux masaüstü entegrasyonu, paketleme biçimleri ve doğrulama durumunu özetler.

## 1. Paketleme Biçimleri

Paketleme [scripts/package-linux.sh](../scripts/package-linux.sh) ve [.github/workflows/release-linux.yml](../.github/workflows/release-linux.yml) üzerinden yönetilir:

- **Debian/Ubuntu (`.deb`)**: `dpkg-deb` ile üretilir; `ffmpeg`, `libc6`, `libx11-6`, `libgl1`, `libasound2 | libasound2t64` bağımlılıklarını ve `pipewire-bin | pulseaudio-utils | alsa-utils` önerilerini içerir.
- **Fedora/RHEL (`.rpm`)**: `fedora:41` konteynerinde `rpmbuild` ile üretilir (`ffmpeg-free` / `/usr/bin/ffmpeg` gereksinimiyle).
- **AppImage**: `linuxdeploy` + özel `AppRun` başlatıcısı ile üretilir.
- **Taşınabilir Arşiv (`.tar.gz`)**: `usr/bin/trim` ikilisi, çalışma zamanı `trim` sarmalayıcı betiği, ikonlar ve `install.sh` ile paketlenir.
- **Flatpak**: Yol haritası kapsamı dışında bırakılmıştır (eklenmez).

## 2. Masaüstü, MIME, İkonlar ve AppStream

- **Masaüstü girdisi**: [trim.desktop](../trim.desktop) (`desktop-file-validate trim.desktop` ile doğrulandı).
- **AppStream metainfo**: [trim.metainfo.xml](../trim.metainfo.xml) (`appstreamcli validate trim.metainfo.xml` ile doğrulandı; `<developer id="com.mozcelik14">` düzeltildi).
- **MIME ilişkilendirmesi**: [resources/mime/trim.xml](../resources/mime/trim.xml) (`application/x-trim-project` → `*.trim`, `application/x-cutline-project` → `*.cutline`) paketlerde `/usr/share/mime/packages/trim.xml` altına kurulur.
- **İkon seti**: `trim.svg` (`hicolor/scalable/apps/trim.svg`) ve `resources/icons/trim_{16,32,48,64,128,256,512}x*.png` (`hicolor/<boyut>/apps/trim.png`).

## 3. Wayland, X11, XDG Portalları, Ses ve Fontlar

- **Pencere Sistemi (Wayland & X11)**: `eframe` `wayland` ve `x11` özellikleri etkin derlenir (`Cargo.toml`). Başsız (headless) ortamda canlı pencere, sürükle-bırak, pano ve HiDPI davranışı **doğrulanmadı**; Ubuntu `release-linux.yml` işinde `xvfb-run` + `llvmpipe` ile başlangıç duman testi çalıştırılır.
- **XDG Portalları**: Dosya seçici `rfd = "0.15"` (`xdg-desktop-portal` / `ashpd`) üzerinden çalışır; klasör/dosya açma eylemleri `xdg-open` kullanır (`crates/trim-ui/src/app/dialogs_dispatch.rs`).
- **Ses Yolu**: Derlemede `libasound2-dev` ve `libpipewire-0.3-dev` bağlanır; çalışma zamanı başlatıcısı `pw-cat`, `paplay` veya `aplay` varlığını kontrol eder.
- **Fontconfig / Fontlar**: Arayüz `egui` yerleşik fontlarını (`default_fonts`) kullanır; başlık/altyazı ve FFmpeg `drawtext` için kullanıcı tarafından seçilen `.ttf`/`.otf` font dosyaları desteklenir.

## 4. CI Kapsamı

[.github/workflows/ci.yml](../.github/workflows/ci.yml):
- `ubuntu-24.04`: `fmt`, `clippy`, strict panic-free `clippy`, `cargo test --workspace --locked`, `cargo-deny`, `release` derleme, `desktop-file-validate`, `appstreamcli validate`.
- `fedora:41` konteyneri: `cargo test --workspace --locked`, `desktop-file-validate`, `appstreamcli validate`.
- `windows-latest`: MSVC derleme, `clippy` ve `cargo test --workspace --locked`.
