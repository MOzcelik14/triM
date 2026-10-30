# triM

[![Sürüm](https://img.shields.io/badge/sürüm-1.2.2-blue.svg)](Cargo.toml)
[![Lisans: GPL-3.0](https://img.shields.io/badge/lisans-GPL--3.0-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20Windows-lightgrey.svg)](#sistem-gereksinimleri)
[![CI](https://github.com/MOzcelik14/triM/actions/workflows/ci.yml/badge.svg)](https://github.com/MOzcelik14/triM/actions)

[English README](README.md) · [Klavye Kısayolları](docs/SHORTCUTS.md) · [Web Sitesi](https://mozcelik14.github.io/triM)

triM, Rust ve egui ile geliştirdiğim, arka planda FFmpeg kullanan hafif bir masaüstü video düzenleyicisidir.

![triM Çalışma Alanı](docs/assets/screenshots/workspace.png)

---

## Neden Geliştirdim?

Mevcut video kurgu programlarının çoğu ya yüzlerce megabaytlık kurulum paketleriyle geliyor ya da tarayıcı motorları (Electron) üzerinde çalışarak yüksek bellek tüketiyor. Kişisel kurgu ihtiyaçlarımda hızlı açılan, harici bulut servislerine bağımlı olmayan, doğrudan yerel komut satırı araçlarıyla (FFmpeg, PipeWire) çalışan ve sistemi yormayan bir düzenleyiciye ihtiyaç duyuyordum.

triM'i bu amaçla tek başıma geliştirdim. Arayüzü `egui`/`eframe` ile çiziyor, video ve ses işlemlerini sistemdeki `ffmpeg` ve `ffprobe` süreçlerine devrediyor. Derlenmiş ikili dosya boyutu yaklaşık 13 MB'tır.

---

## Temel Özellikler

### Kurgu ve Zaman Çizgisi
- Çok kanallı video ve ses zaman çizgisi (V1..Vn, A1..An)
- Jilet (Razor), dalgalanma (Ripple), kaydırma (Slip) ve uzatma (Rate Stretch) araçları
- Oynatma kafasına ve klip kenarlarına manyetik kenetlenme
- Otomatik video-ses klip bağlama ve gruplama (V1 -> A1, V2 -> A2 eşleştirmesi)
- Zaman çizgisi üzerinde görsel işaretçiler (Markers)

### Renk ve Efektler
- 3 yönlü renk çarkları (Lift, Gamma, Gain)
- `.cube` formatında 3D LUT uygulama desteği
- Renk anahtarı (Chroma Key / yeşil perde temizleme)
- Opaklık ve doğrusal geçişler (fade in, fade out)
- Konum, ölçek, döndürme ve opaklık için anahtar kare (keyframe) desteği

### Ses İşleme
- Çok kanallı yazılımsal PCM ses mikseri (48 kHz stereo)
- Kanal başına kazanç (dB), sessize alma (Mute) ve solo kontrolleri
- 3 bant parametrik ekolayzır ve ses hazır ayarları (Konuşma netliği, bas güçlendirme vb.)
- Doğrudan zaman çizgisine mikrofonla ses kaydı (Voiceover)
- Çıkışta ses tepe noktalarını sınırlayan 5 ms tuğla duvar sınırlayıcı (Lookahead Limiter)

### Dışa Aktarma ve Standartlar
- H.264, HEVC, VP9, AV1, ProRes ve DNxHR formatlarında çıktı alma
- Donanım hızlandırma tespiti (NVIDIA NVENC, Intel QSV, AMD AMF, Linux VAAPI)
- EBU R128 standardında ses seviyesi normalizasyonu (`loudnorm`)
- Final Cut Pro (`.fcpxml` v1.8/v1.9) ve CMX 3600 (`.edl`) sekans aktarımı

---

## Ekran Görüntüleri

| Renk Düzenleme (Lift/Gamma/Gain) | Ses Mikseri ve Ekolayzır |
|---|---|
| ![Renk Düzenleme](docs/assets/screenshots/color_grading.png) | ![Ses Mikseri](docs/assets/screenshots/audio_mixer.png) |

| Dışa Aktarma Stüdyosu | Dikey Kurgu (9:16) ve Güvenli Alanlar |
|---|---|
| ![Dışa Aktarma](docs/assets/screenshots/export_studio.png) | ![Dikey Tuval](docs/assets/screenshots/vertical_9_16.png) |

---

## Kurulum

### Sistem Gereksinimleri
- **İşletim Sistemi:** Linux (x86_64) veya Windows (x86_64)
- **Grafik:** OpenGL 3.3+ destekleyen sürücü
- **Gerekli Araçlar:** Sistemde kurulu `ffmpeg` ve `ffprobe`
- **Linux Ses Çıktısı:** PipeWire (`pw-cat`), PulseAudio (`paplay`) veya ALSA (`aplay`)

### Hazır İkili Dosyalar
[GitHub Sürümler](https://github.com/MOzcelik14/triM/releases) sayfasından Linux için `.deb`, `.rpm`, `.AppImage` veya Windows için `.zip` paketlerini indirebilirsiniz.

Linux üzerinde tek komutla kurulum:
```bash
curl -fsSL https://raw.githubusercontent.com/MOzcelik14/triM/master/install.sh | bash
```

### Kaynaktan Derleme
Rust toolchain (1.80+) kurulu olmalıdır:

```bash
git clone https://github.com/MOzcelik14/triM.git
cd triM
cargo build --release -p trim-app
install -m 0755 target/release/trim ~/.local/bin/trim
```

---

## Kullanım

Uygulamayı başlatın veya doğrudan bir proje dosyasını açın:

```bash
trim
trim /yol/proje.trim
```

### Temel Kısayollar

| Tuş | Eylem |
|---|---|
| `Boşluk` | Oynat / Duraklat |
| `J` / `K` / `L` | Geri oynat / Durdur / İleri oynat (2x, 4x, 8x hızlanma) |
| `C` | Jilet aracı (Razor) |
| `S` | Oynatma kafasında klibi böl |
| `V` | Seçim aracı |
| `Delete` | Klibi sil |
| `Shift + Delete` | Boşluğu kapatıp sil (Ripple Delete) |
| `Ctrl + E` | Dışa aktarma penceresini aç |
| `F1` | Kısayol referans penceresini aç |

Tüm klavye kısayolları için [docs/SHORTCUTS.md](docs/SHORTCUTS.md) belgesine bakın.

---

## Mimari

Proje, bağımsız katmanlara ayrılmış 8 Cargo crate'inden oluşur:

```
triM/
├── crates/
│   ├── trim-core/       # Zaman, klip, kanal ve proje veri modelleri
│   ├── trim-commands/   # Geri alınabilir (Undo/Redo) komut nesneleri
│   ├── trim-timeline/   # Kırpma ve kesim algoritmaları
│   ├── trim-project/    # Dosya kaydetme (.trim), otomatik yedekleme ve kurtarma
│   ├── trim-media/      # FFmpeg süreç yöneticisi, kare çözümleme, ses mikseri
│   ├── trim-export/     # FFmpeg filtergraph derleyicisi, EDL ve FCPXML aktarımı
│   ├── trim-ui/         # egui arayüzü, monitörler, zaman çizgisi çizimi
│   └── trim-app/        # Masaüstü çalıştırılabilir ikili giriş noktası
```

Daha fazla ayrıntı için [ARCHITECTURE.md](ARCHITECTURE.md) belgesini inceleyebilirsiniz.

---

## Bilinen Sınırlamalar

1. **Önizleme Çözünürlüğü:** Zaman çizgisi önizlemesi CPU üzerinde sabit 640x360 çözünürlükte işlenip OpenGL dokusu olarak yüklenir. Çok sayıda eşzamanlı katman ve ağır efekt içeren sahnelerde önizleme CPU tüketebilir.
2. **Alt Süreç Tabanlı Medya Hattı:** C kütüphanelerini (`libavcodec`) bağlamak yerine FFmpeg izole bir alt süreç (`ChildGuard`) olarak çağrılır. Bu tercih arayüzün kararsız codec çökmelerinden etkilenmesini engeller; ancak uzun GOP aralığına sahip dosyalarda ilk kare arama süresi 15-40 ms civarında gecikme oluşturabilir. Bunun önüne geçmek için proxy modu kullanılabilir.
3. **Platform Kapsamı:** Ses kaydı (Voiceover) ve ekran kaydı özellikleri Linux'a (PulseAudio/ALSA ve x11grab) özgüdür. Windows derlemesinde ses oynatımı düşük gecikmeli `ffplay` üzerinden sağlanır; doğrudan mikrofon kaydı henüz Windows'ta desteklenmemektedir.
4. **Renk Alanı:** Önizleme 8-bit sRGB alanında çalışır. 10-bit HDR kaynak medyalar önizleme sırasında 8-bite indirgenir.

---

## Lisans

Bu proje [GNU General Public License v3.0 or later (GPL-3.0-or-later)](LICENSE) altında lisanslanmıştır.
