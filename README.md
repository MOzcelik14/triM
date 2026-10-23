<p align="center">
  <a href="https://mozcelik14.github.io/triM/">
    <img src="docs/assets/branding/logo_wordmark.svg" alt="triM. Logo" width="220">
  </a>
</p>

<p align="center">
  A lightweight, native, Electron-free non-linear video editor for Linux written in Rust.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square" alt="License: MIT"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-2021%20edition-orange.svg?style=flat-square" alt="Rust 2021"></a>
  <a href="https://github.com/emilk/egui"><img src="https://img.shields.io/badge/GUI-egui%20%2F%20eframe-blueviolet.svg?style=flat-square" alt="GUI: egui / eframe"></a>
  <a href="https://ffmpeg.org/"><img src="https://img.shields.io/badge/media-FFmpeg%206.0+-007808.svg?style=flat-square" alt="FFmpeg 6.0+"></a>
  <a href="https://github.com/MOzcelik14/triM/releases/tag/v0.6.0"><img src="https://img.shields.io/badge/release-v0.6.0-E07A38.svg?style=flat-square" alt="Release v0.6.0"></a>
  <img src="https://img.shields.io/badge/platform-Linux%20%7C%20Windows-28282E.svg?style=flat-square" alt="Linux | Windows">
</p>

<p align="center">
  <a href="#english"><strong>English</strong></a> &bull; <a href="#türkçe"><strong>Türkçe</strong></a> &bull; <a href="#shortcuts"><strong>Shortcuts</strong></a> &bull; <a href="#architecture"><strong>Architecture</strong></a> &bull; <a href="https://mozcelik14.github.io/triM/"><strong>Website</strong></a>
</p>

---

<a name="english"></a>
## English

**triM.** is a modern, blazing-fast, and native non-linear video editor (NLE) built from the ground up in **Rust** specifically for Linux and Windows desktop environments.

Engineered with a modular Cargo workspace architecture, **triM.** delivers precision video cutting, multi-track audio mixing, real-time color grading, and FFmpeg filtergraph exporting — completely free of Electron bloat or web runtime overhead. The compiled release binary is a self-contained, lightning-fast native application (~19 MB).

### Highlights

- **100% Rust & Native Desktop:** Zero Electron, zero Python dependencies. Instant startup and deterministic memory safety (Linux & Windows).
- **Subtitle Suite (.SRT / .VTT) & Smart Scene Detection:** Full bidirectional subtitle import/export with lower-third auto-generation, and FFmpeg scene-detection auto-cut (`gt(scene, 0.3)`) to slice clips automatically at visual cuts.
- **3D LUT Support & Cinematic Color Presets:** External `.cube` 3D LUT loading and built-in cinematic presets (Teal & Orange, Vintage Film, Monochrome Noir, Cyberpunk Neon) with real-time Program Monitor preview and FFmpeg filtergraph export.
- **Live Timeline Voiceover Recorder:** One-click track microphone capture directly on audio tracks (`A1`, `A2`...) with live timer and auto-commitment to the timeline at the playhead position.
- **Smart Proxy Workflow for 4K / UHD:** Background 720p H.264 proxy generator, `[⚡ Proxy]` timeline toggle for butter-smooth scrubbing, and 100% full-resolution master export preservation.
- **Multi-NLE Keyboard Shortcut Profiles:** Switch on the fly between triM. Default, Adobe Premiere Pro, and DaVinci Resolve shortcut profiles with a dynamic reference dialog.
- **GPU Hardware Acceleration:** Automated hardware encoder detection and pipeline export using NVIDIA NVENC (`h264_nvenc`/`hevc_nvenc`), Intel QuickSync (QSV), AMD AMF, and Linux VAAPI with graceful CPU fallback.
- **Vertical Canvas & Safe Zones:** Native 9:16 Shorts/Reels/TikTok workspace, 90% Action Safe, 80% Title Safe, and interactive social media UI overlay boundaries.
- **Pro Audio Engineering Suite:** Master audio loudness normalization (EBU R128 -23 LUFS, YouTube/Spotify -14 LUFS, Podcast -16 LUFS), Voice Clarity & Podcast EQ presets, and Noise Gate (`agate`).
- **Live Titler & Real-Time Compositing:** In-inspector live text editing with font sizing, position templates, custom X/Y sliders, drop shadows, subtitle bounding boxes, and FFmpeg `drawtext` export.
- **Portable Project Packager:** Zero-asset-loss `.trim` project bundling into standalone folders with relative media path rebasing.
- **Asynchronous Video Pipeline:** Multi-threaded `PlaybackStreamer` (235+ FPS continuous rawvideo decoding pipe) and non-blocking `AsyncFrameDecoder` for stutter-free timeline scrubbing.
- **Multi-Track Timeline:** Dynamic layered video (`V1..Vn`) and audio (`A1..An`) tracks with cross-track dragging, vertical track resizing, razor split (`S`), ripple trim (`Q`/`W`), ripple delete (`Shift+Delete`), and magnetic snapping.
- **Dedicated NLE Vector Toolbar:** Native subpixel vector-drawn tools (`V` Select, `A` Track Select, `B` Ripple, `C` Razor, `Y` Slip, `P` Pen, `H` Hand, `T` Text).
- **Multi-Channel Audio Mixer:** Live master fader, per-track dB sliders, stereo RMS/Peak VU meters, Mute/Solo, and instant audio separation ("Detach Audio").
- **Keyframing & Color Grading:** Interpolation curves (Linear, Hold, Smoothstep Ease-In-Out) across clip position, scale, opacity, rotation, brightness, contrast, and saturation.
- **Direct CLI & Welcome Launcher:** Launch directly from terminal (`trim video.mp4`) or use the startup launcher with resolution presets and recent project history.

---

### Linux Packages (.deb / .rpm / AppImage)

Prebuilt packages for the **native Rust edition** are published with each new tagged release:

- **Debian / Ubuntu / Linux Mint:** `trim_<version>_amd64.deb`
- **Fedora / RPM-based distros:** `trim-<version>-1.x86_64.rpm`
- **Other x86_64 Linux distributions:** `triM-<version>-x86_64.AppImage`

Get the latest builds from [GitHub Releases](https://github.com/MOzcelik14/triM/releases). Install and run the `trim` desktop application. FFmpeg must be available on your system for media processing and exports; the AppImage bundles compatible libraries where possible but does not bundle FFmpeg. Package compatibility should be verified on your particular distribution.

#### Publishing a new version (maintainers)

The source of truth for the Rust version is `[workspace.package].version` in `Cargo.toml`. After updating that value and committing the changes, create and push the matching tag:

```bash
git tag v0.3.1  # Example only: Cargo.toml must say 0.3.1
git push origin master
git push origin v0.3.1
```

The [Linux release workflow](.github/workflows/release-linux.yml) tests the Rust workspace, builds Debian and AppImage packages on Ubuntu and an RPM on Fedora, then publishes all three files with SHA-256 checksums to the matching GitHub Release. On changes to the packaging script or workflow, the same builds can run on `master` as CI artifacts without publishing a release. No manual packaging or uploading is needed. All packages are 100% native Rust executables.

---

### Installation & Build

#### 🚀 Quick Install

##### Linux (Bash / curl)
```bash
curl -fsSL https://raw.githubusercontent.com/MOzcelik14/triM/master/install.sh | bash
```
To cleanly uninstall:
```bash
curl -fsSL https://raw.githubusercontent.com/MOzcelik14/triM/master/install.sh | bash -s -- --uninstall
```

##### Windows (PowerShell)
```powershell
iwr -useb https://raw.githubusercontent.com/MOzcelik14/triM/master/install.ps1 | iex
```
To cleanly uninstall:
```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1 -Uninstall
```

> **Windows Note:** Ensure FFmpeg is installed (`winget install Gyan.FFmpeg`) or place `ffmpeg.exe`, `ffprobe.exe`, and `ffplay.exe` in the same folder as `trim.exe`.


#### Prerequisites (Debian / Ubuntu / Pop!_OS / Fedora / Arch)

Ensure you have Rust and the required development libraries installed:

```bash
# Debian / Ubuntu / Linux Mint
sudo apt update
sudo apt install build-essential pkg-config libasound2-dev libx11-dev libgl1-mesa-dev ffmpeg

# Fedora
sudo dnf install gcc pkgconf-pkg-config alsa-lib-devel libX11-devel mesa-libGL-devel ffmpeg

# Arch Linux
sudo pacman -S base-devel alsa-lib libx11 mesa ffmpeg
```

#### Build from Source

```bash
git clone https://github.com/MOzcelik14/triM.git
cd triM
cargo build --release
```

The optimized native binary is produced at `./target/release/trim`.

#### Running triM.

```bash
# Launch editor
./target/release/trim

# Launch with a video or project directly
./target/release/trim /path/to/video.mp4
./target/release/trim /path/to/project.trim
```

---

<a name="türkçe"></a>
## Türkçe

**triM.**, Linux masaüstü ortamları için özel olarak **Rust ile sıfırdan yeniden geliştirilmiş**, modern, ultra hızlı ve hafif bir yerel video düzenleyicidir (NLE).

Modüler Cargo çalışma alanı mimarisi üzerine kurulan **triM.**; hassas video kesme, çok kanallı ses miksajı, gerçek zamanlı renk derecelendirme ve FFmpeg filtre dışa aktarımı sunar. Electron yükü ve web motoru bağımlılıkları içermez; yaklaşık **19 MB** boyutunda bağımsız, optimize bir yerel ikili dosyadır.

### Öne Çıkan Özellikler

- **%100 Rust ve Yerel Masaüstü:** Sıfır Electron yükü, sıfır Python bağımlılığı. Anında açılış ve kararlı bellek yönetimi (Linux & Windows desteği).
- **Altyazı Süiti (.SRT / .VTT) & Akıllı Sahne Algılama:** Çift yönlü altyazı içe/dışa aktarımı, otomatik lower-third klip üretimi ve FFmpeg sahne analizi (`scene > 0.3`) ile videodaki görsel kesim noktalarından otomatik jiletleme.
- **3D LUT Desteği & Sinematik Renk Profilleri:** Harici `.cube` 3D LUT yükleme ve yerleşik sinematik hazır ayarlar (Teal & Orange, Vintage Film, Monochrome Noir, Cyberpunk Neon) ile Program Monitöründe canlı önizleme ve FFmpeg export.
- **Canlı Zaman Çizgisi Ses Kaydedicisi (Voiceover):** Ses izleri (`A1`, `A2`...) üzerinde tek tıkla stüdyo kalitesinde mikrofon kaydı, canlı sayaç rozeti ve durdurulduğunda oynatma kafası konumuna otomatik klip yerleşimi.
- **4K / UHD için Akıllı Proxy (Smart Proxy):** Arka planda 720p H.264 proxy üretimi, kasmayan kaydırma için `[⚡ Proxy]` zaman çizgisi anahtarı ve orijinal 4K master dışa aktarım garantisi.
- **Çoklu NLE Klavye Kısayol Profilleri:** triM. Varsayılan, Adobe Premiere Pro ve DaVinci Resolve kısayol profilleri arasında anında geçiş ve dinamik kısayol kılavuzu.
- **GPU Donanım Hızlandırma:** NVIDIA NVENC, Intel QuickSync (QSV), AMD AMF ve Linux VAAPI ile ultra hızlı donanım destekli dışa aktarım ve otomatik CPU yedeği.
- **Dikey Tuval & Güvenli Alan Kılavuzları (Safe Zones):** 9:16 Shorts/Reels çalışma alanı, 90% Action Safe, 80% Title Safe ve TikTok/Instagram Reels arayüz sınırları.
- **Profesyonel Ses Mühendisliği & EBU R128:** Master miksajda YouTube/Spotify (-14 LUFS), Podcast (-16 LUFS) ve Yayın (-23 LUFS) ses normalizasyonu; klip bazlı Ses Netliği, Bas Güçlendirme ve Gürültü Kapısı (Noise Gate - `agate`).
- **Canlı Titler & Altyazı Editörü:** Inspector üzerinde font boyutu, hızlı şablonlar (Alt Yazı, Orta, Üst), X/Y sürgüleri, renk paleti, gölge ve altyazı kutusu ile canlı metin düzenleme ve FFmpeg `drawtext` export desteği.
- **Taşınabilir Proje Paketleyici (Project Archiver):** Projede kullanılan tüm medyaları klasör altında toplayıp dosya yollarını göreceli hale getirerek (`media/...`) kayıpsız proje taşıma.
- **Yüksek Hızlı Asenkron Video Motoru:** `PlaybackStreamer` (arka planda 235+ FPS sürekli rawvideo borusu) ve `AsyncFrameDecoder` (sarma sırasında kilitlenmeyen oynatma kafası).
- **Dinamik Çok Kanallı Zaman Çizgisi:** Kanallar arası serbest dikey klip taşıma, dikey kanal boyu ayarlama, `[+ Video]` / `[+ Ses]` ekleme, çizgiden kesme (`S`), başı kırpıp kaydırma (`Q`), sonu kırpıp kaydırma (`W`), boşluğu kapatarak silme (`Shift+Delete`) ve manyetik yapışma.
- **Özel NLE Vektörel Araç Çubuğu:** Yerel piksel altı vektörlerle çizilen 8 profesyonel kurgu aracı (`V` Seçim, `A` İz Seçme, `B` Dalgalanma, `C` Jilet/Kesme, `Y` Kaydırma, `P` Kalem/Anahtar Kare, `H` El/Gezinme, `T` Metin).
- **Çok Kanallı Ses Mikseri Konsolu:** Canlı Master fader, kanal bazlı dB sürgüleri, stereo tepe/RMS VU metreler, Mute/Solo ve tek tıkla ses ayırma ("Sesi Ayır").
- **Gelişmiş Dışa Aktarma Stüdyosu:** YouTube 1080p, 4K UHD, Reels/Shorts 9:16 Dikey, HEVC H.265 ve Apple ProRes 422 hazır profilleri ile anlık tahmini boyut hesabı.
- **Anahtar Kare & Renk Düzenleme:** Konum, ölçek, opaklık, döndürme, parlaklık, kontrast ve doygunluk üzerinde anahtar kare enterpolasyonu (Doğrusal, Sabit, Ease-In-Out eğrisi).
- **Hızlı Başlangıç & Karşılama Ekranı:** Çözünürlük şablonları, son kullanılan projeler ve terminalden doğrudan dosya açabilme (`trim video.mp4`).
- **Geriye Dönük Uyumluluk:** `.trim` ve eski `.cutline` JSON proje dosyalarını otomatik tanıma ve her 2 dakikada bir otomatik kurtarma anlık görüntüsü alma.

### 🚀 Hızlı Kurulum

#### Linux (Bash / curl)
```bash
curl -fsSL https://raw.githubusercontent.com/MOzcelik14/triM/master/install.sh | bash
```
Temizce kaldırmak için:
```bash
curl -fsSL https://raw.githubusercontent.com/MOzcelik14/triM/master/install.sh | bash -s -- --uninstall
```

#### Windows (PowerShell)
```powershell
iwr -useb https://raw.githubusercontent.com/MOzcelik14/triM/master/install.ps1 | iex
```
Temizce kaldırmak için:
```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1 -Uninstall
```

> **Windows Notu:** Sisteminizde FFmpeg kurulu olmalıdır (`winget install Gyan.FFmpeg`) veya `ffmpeg.exe`, `ffprobe.exe` ve `ffplay.exe` dosyalarını `trim.exe` ile aynı klasöre yerleştirebilirsiniz.


---

<a name="shortcuts"></a>
## ⌨️ Keyboard Shortcuts / Klavye Kısayolları

| Kısayol | İngilizce Açıklama | Türkçe Açıklama |
| :--- | :--- | :--- |
| <kbd>Space</kbd> | Toggle Playback / Pause | Oynat / Duraklat |
| <kbd>V</kbd> | Selection Tool | Seçim Aracı |
| <kbd>A</kbd> | Track Select Forward Tool | İleri İz Seçme Aracı |
| <kbd>B</kbd> | Ripple Edit Tool | Dalgalanma Düzenleme Aracı |
| <kbd>C</kbd> | Razor / Cut Tool | Jilet / Kesme Aracı |
| <kbd>Y</kbd> | Slip Tool | Kaydırma Aracı |
| <kbd>P</kbd> | Pen / Keyframe Tool | Kalem / Anahtar Kare Aracı |
| <kbd>H</kbd> | Hand / Pan Tool | El / Gezinme Aracı |
| <kbd>T</kbd> | Type / Text Tool | Metin / Yazı Aracı |
| <kbd>S</kbd> | Split / Razor Cut at Playhead | Çizgiden Kes / Böl |
| <kbd>Q</kbd> | Ripple Trim Head to Playhead | Başı Çizgiye Kırp ve Kaydır |
| <kbd>W</kbd> | Ripple Trim Tail to Playhead | Sonu Çizgiye Kırp ve Kaydır |
| <kbd>Delete</kbd> | Delete Selected Clip(s) | Seçili Klibi Sil |
| <kbd>Shift</kbd> + <kbd>Delete</kbd> | Ripple Delete Selected Clip | Boşluğu Kapatıp Sil (Ripple Delete) |
| <kbd>J</kbd> | Shuttle Reverse (-1x, -2x, -4x, -8x) | Geriye Hızlı Oynat |
| <kbd>K</kbd> | Shuttle Pause | Oynatmayı Durdur |
| <kbd>L</kbd> | Shuttle Forward (1x, 2x, 4x, 8x) | İleriye Hızlı Oynat |
| <kbd>←</kbd> / <kbd>→</kbd> | Step 1 Frame Backward / Forward | 1 Kare Geri / İleri Adımla |
| <kbd>Shift</kbd> + <kbd>←</kbd> / <kbd>→</kbd> | Step 1 Second Backward / Forward | 1 Saniye Geri / İleri Adımla |
| <kbd>↑</kbd> / <kbd>↓</kbd> | Jump to Previous / Next Edit Point | Önceki / Sonraki Kesim Noktasına Git |
| <kbd>Home</kbd> / <kbd>End</kbd> | Jump to Beginning / End of Timeline | Zaman Çizgisi Başına / Sonuna Git |
| <kbd>M</kbd> | Add Marker at Playhead | Çizgiye İşaretçi Ekle |
| <kbd>Ctrl</kbd> + <kbd>Z</kbd> | Undo Last Action | Son İşlemi Geri Al |
| <kbd>Ctrl</kbd> + <kbd>Y</kbd> | Redo Last Action | Son İşlemi Yinele |
| <kbd>Ctrl</kbd> + <kbd>N</kbd> | New Project | Yeni Proje |
| <kbd>Ctrl</kbd> + <kbd>O</kbd> | Open Project Dialog | Proje Aç |
| <kbd>Ctrl</kbd> + <kbd>S</kbd> | Save Project | Projeyi Kaydet |
| <kbd>Ctrl</kbd> + <kbd>T</kbd> | Add Title Card Dialog | Başlık / Metin Ekle |
| <kbd>Ctrl</kbd> + <kbd>M</kbd> / <kbd>E</kbd> | Open Export Studio | Dışa Aktarma Stüdyosu |
| <kbd>F1</kbd> | Keyboard Shortcuts Guide | Klavye Kısayolları Kılavuzu |

---

<a name="architecture"></a>
## 🏗️ Architecture / Mimari Yapı

Uygulama, sorumlulukların temiz ve modüler ayrıldığı **8 üyeli Cargo Çalışma Alanı (Workspace)** olarak yapılandırılmıştır:

```
VideoEditor/
├── Cargo.toml                    # Workspace manifesti (resolver = "2")
├── crates/
│   ├── trim-core/                # Zaman, klip, kanal, anahtar kare ve proje modelleri
│   ├── trim-commands/            # 14 adet geri alınabilir (Undo/Redo) komut ve UndoStack
│   ├── trim-timeline/            # Üst düzey zaman çizgisi düzenleme algoritmaları
│   ├── trim-project/             # Atomik disk G/Ç (.tmp rename) ve AutosaveManager
│   ├── trim-media/               # VideoEngine, PlaybackStreamer, PCM mikser, ses motoru
│   ├── trim-export/              # FFmpeg filtergraph oluşturucu, arka plan işçisi
│   ├── trim-ui/                  # Saf Rust egui/eframe arayüzü, canvas, denetçi, monitör
│   └── trim-app/                 # Masaüstü ikili dosya giriş noktası (`trim`)
```

### Modül Sorumlulukları

1. **`trim-core`:**
   - SMPTE zaman kodu hesaplamaları (`seconds_to_timecode`, `timecode_to_seconds`).
   - Zaman çizgisi manyetik yapışma (`snap_time`).
   - Çok katmanlı görünür klip tespiti (`get_visible_video_clips_at`, `get_active_audio_clips_at`).
   - Anahtar kare enterpolatörleri (Linear, Hold, Smoothstep Ease-In-Out).
2. **`trim-commands`:**
   - 14 adet atomik, geri alınabilir komut (`AddClip`, `RemoveClip`, `MoveClip`, `TrimClip`, `SplitClip`, `RippleDelete`, `BatchMove`, `DetachAudio`, vb.).
3. **`trim-timeline`:**
   - Oynatma kafasından kesme (`split_at_playhead`).
   - Ripple trim baş (`ripple_trim_head`) ve kuyruk (`ripple_trim_tail`) fonksiyonları.
   - Edit noktası arama (`find_prev_edit_point`, `find_next_edit_point`).
4. **`trim-project`:**
   - Güvenli atomik kaydetme (`.tmp` -> rename).
   - `.trim` v1 ve eski `.cutline` geriye dönük uyumlu JSON serileştirme.
   - Periyodik kurtarma yöneticisi (`AutosaveManager`).
5. **`trim-media`:**
   - `VideoEngine`: Gerçek zamanlı akış (`PlaybackStreamer`), asenkron sarma (`AsyncFrameDecoder`), RAM kare önbelleği.
   - `AudioMixer`: 48kHz 16-bit stereo miksaj, arka plan ön yükleme (`preload_audio_async`), cpal ses akış cihazı (`AudioStreamPlayer`).
   - `WaveformGenerator`: Çok iş parçacıklı dalga formu analizi ve RAM/Disk önbellekleme.
   - `FFprobeAnalyzer`: JSON akış ve codec çözümleme.
6. **`trim-export`:**
   - FFmpeg karmaşık filtre grafikleri (`[v0][v1]overlay`, `scale`, `pad`, `eq`, `fade`, `afade`, `atempo`).
   - Önayarlar: 1080p, 720p, 4K, YouTube 60fps, WebM, Yalnızca Ses (WAV/MP3).
7. **`trim-ui`:**
   - Donanım hızlandırmalı egui arayüzü: Zaman çizgisi kanvası, monitör penceresi, medya havuzu, özellik denetçisi, çift kanallı VU metre.
8. **`trim-app`:**
   - Komut satırı argümanları, uygulama ikonu ve pencere ilklendirmesi.

---

### Test Kapsamı

Tüm çalışma alanı testleri Cargo üzerinden doğrudan çalıştırılabilir:

```bash
cargo test --workspace
```

Birim ve entegrasyon testleri; geri alma yığını, ses mikseri, manyetik yapışma, video çözümleme ve kurgu iş akışlarını kapsar. GitHub Actions, her ilgili değişiklikte Rust çalışma alanını yeniden derleyip test eder.

---

### License / Lisans

Bu proje **MIT Lisansı** ile lisanslanmıştır. Detaylar için [LICENSE](LICENSE) dosyasına bakabilirsiniz.
