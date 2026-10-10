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
  <a href="https://github.com/MOzcelik14/triM/releases/tag/v0.3.0"><img src="https://img.shields.io/badge/release-v0.3.0-E07A38.svg?style=flat-square" alt="Release v0.3.0"></a>
  <img src="https://img.shields.io/badge/platform-Linux%20(Wayland%20%7C%20X11)-28282E.svg?style=flat-square" alt="Linux">
</p>

<p align="center">
  <a href="#english"><strong>English</strong></a> &bull; <a href="#türkçe"><strong>Türkçe</strong></a> &bull; <a href="#shortcuts"><strong>Shortcuts</strong></a> &bull; <a href="#architecture"><strong>Architecture</strong></a> &bull; <a href="https://mozcelik14.github.io/triM/"><strong>Website</strong></a>
</p>

---

<a name="english"></a>
## English

**triM.** is a modern, blazing-fast, and native non-linear video editor (NLE) built from the ground up in **Rust** specifically for Linux desktop environments.

Engineered with a modular Cargo workspace architecture, **triM.** delivers precision video cutting, multi-track audio mixing, real-time color grading, and FFmpeg filtergraph exporting — completely free of Electron bloat or web runtime overhead. The compiled release binary is a self-contained, lightning-fast native Linux application (~19 MB).

### Highlights

- **100% Rust & Native Desktop:** Zero Electron, zero Python runtime dependencies. Instant startup and deterministic memory safety.
- **Asynchronous Video Pipeline:** Multi-threaded `PlaybackStreamer` (235+ FPS continuous rawvideo decoding pipe) and non-blocking `AsyncFrameDecoder` for stutter-free timeline scrubbing.
- **Multi-Track Timeline:** Layered video (`V1..Vn`) and audio (`A1..An`) tracks with razor split (`S`), ripple trim head (`Q`), ripple trim tail (`W`), ripple delete (`Shift+Delete`), and magnetic snapping.
- **Stereo Audio Engine:** Multi-track 48kHz 16-bit PCM mixing, live RMS/Peak stereo VU meter, track-level volume/mute/solo, and instant audio separation ("Detach Audio").
- **Keyframing & Color Grading:** Interpolation curves (Linear, Hold, Smoothstep Ease-In-Out) across clip position, scale, opacity, rotation, brightness, contrast, and saturation.
- **Direct CLI & Drag-and-Drop:** Open files or projects straight from your terminal (`trim path/to/video.mp4`) or drag and drop media into the pool and onto tracks.
- **Backward Compatible:** Native support for both `.trim` and legacy `.cutline` project files with automatic crash recovery snapshots.

---

### Installation & Build

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

- **%100 Rust ve Yerel Masaüstü:** Sıfır Electron yükü. Anında açılış ve kararlı bellek yönetimi.
- **Yüksek Hızlı Asenkron Video Motoru:** `PlaybackStreamer` (arka planda 235+ FPS sürekli rawvideo borusu) ve `AsyncFrameDecoder` (sarma sırasında istek birleştirme ile kilitlenmeyen oynatma kafası).
- **Çok Kanallı Zaman Çizgisi:** Video (`V1..Vn`) ve ses (`A1..An`) kanalları, oynatma kafasından kesme (`S`), başı kırpıp kaydırma (`Q`), sonu kırpıp kaydırma (`W`), boşluğu kapatarak silme (`Shift+Delete`) ve manyetik yapışma.
- **Stereo Ses Mikseri:** 48kHz 16-bit PCM miksaj, canlı RMS/Tepe çift kanallı VU göstergesi, kanal bazlı ses/mute/solo denetimleri ve tek tıkla ses ayırma ("Sesi Ayır").
- **Anahtar Kare & Renk Düzenleme:** Konum, ölçek, opaklık, döndürme, parlaklık, kontrast ve doygunluk üzerinde anahtar kare enterpolasyonu (Doğrusal, Sabit, Ease-In-Out eğrisi).
- **Komut Satırı & Sürükle-Bırak:** Dosyaları doğrudan terminalden açabilme (`trim video.mp4`) veya medya havuzuna ve kanallara sürükleyip bırakabilme.
- **Geriye Dönük Uyumluluk:** `.trim` ve eski `.cutline` JSON proje dosyalarını otomatik tanıma ve her 2 dakikada bir otomatik kurtarma anlık görüntüsü alma.

---

<a name="shortcuts"></a>
## ⌨️ Keyboard Shortcuts / Klavye Kısayolları

| Kısayol | İngilizce Açıklama | Türkçe Açıklama |
| :--- | :--- | :--- |
| <kbd>Space</kbd> | Toggle Playback / Pause | Oynat / Duraklat |
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
| <kbd>Ctrl</kbd> + <kbd>E</kbd> | Open Export Video Dialog | Videoyu Dışa Aktar |

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

Birim testleri, geri alma yığını, ses mikseri, manyetik yapışma, video çözümleme ve entegrasyon iş akışlarını kapsayan **33/33 test %100 başarılıdır.**

---

### License / Lisans

Bu proje **MIT Lisansı** ile lisanslanmıştır. Detaylar için [LICENSE](LICENSE) dosyasına bakabilirsiniz.
