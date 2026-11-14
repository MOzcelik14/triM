<div align="center">
  <h1>triM.</h1>
  <p><strong>Windows ve Linux için hızlı, güvenli ve stüdyo kalitesinde masaüstü video düzenleyici.</strong></p>
  <p>
    <a href="https://github.com/MOzcelik14/triM/releases/latest"><img src="https://img.shields.io/badge/Sürüm-1.6.2-2563eb.svg" alt="Sürüm 1.6.2" /></a>
    <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Linux-475569.svg" alt="Platform: Windows | Linux" />
    <a href="LICENSE"><img src="https://img.shields.io/badge/Lisans-GPL--3.0-16a34a.svg" alt="Lisans: GPL-3.0" /></a>
  </p>
  <p>
    <a href="README.md">English</a> ·
    <a href="https://github.com/MOzcelik14/triM/releases">İndir</a> ·
    <a href="https://mozcelik14.github.io/triM">Web Sitesi</a> ·
    <a href="https://mozcelik14.github.io/prism-suite">prisM. Suite</a> ·
    <a href="docs/SHORTCUTS.md">Kısayollar</a>
  </p>
</div>

---

![triM. Çalışma Alanı](docs/assets/screenshots/workspace.png)

## İçerik Üreticileri, Ekipler ve Günlük Prodüksiyon İçin Tasarlandı

**triM.**, **[prisM. Suite](https://mozcelik14.github.io/prism-suite)** ekosisteminin bir parçası olarak geliştirilen; abonelik ücreti, bulut yükleme zorunluluğu veya hesap girişi olmadan hız, kararlılık ve tam yaratıcı kontrol sunan modern bir masaüstü video düzenleyicisidir. Kısa sosyal medya videolarından çok kanallı profesyonel teslimatlara kadar her projede saniyeler içinde açılır ve tüm dosyalarınızı yalnızca kendi bilgisayarınızda tutar.

---

## Neden triM.?

- **Anında Açılış ve Çevrimdışı Çalışma**: İnternet bağlantısı gerekmeden saniyeler içinde kurguya başlayın.
- **Abonelik veya Filigran Yok**: 4K dışa aktarmadan gelişmiş renk düzenlemeye kadar tüm özellikler tamamen ücretsiz ve açıktır.
- **Alışması Kolay Arayüz**: **Türkçe** ve **İngilizce** dil seçenekleri, ölçeklenebilir tasarım ve Premiere Pro ile DaVinci Resolve'a uyumlu hazır kısayol profilleri.
- **Her Bilgisayarda Akıcı Deneyim**: İsteğe bağlı akıllı önizleme (proxy) modu sayesinde yüksek çözünürlüklü 4K görüntülerde bile takılmadan çalışın.

---

## Öne Çıkan Özellikler

### Çok Kanallı Kolay Zaman Çizelgesi
Video, ses, altyazı ve başlık katmanlarını manyetik yapışma, klip bağlama, işaretçiler ve hassas kesme/kaydırma araçlarıyla zahmetsizce düzenleyin.

### Stüdyo Düzeyinde Renk, Analiz ve Birleştirme
- **Hazır Renk Profilleri ve Görünümler**: Tek tıkla Otomatik Beyaz Dengesi, parlaklık/kontrast hazır ayarları, 20'den fazla sinematik/film görünümü ve özel `.cube` renk profilleri.
- **Profesyonel Renk Analiz Araçları (Scopes)**: RGB Parade, Luma Waveform, Vectorscope, Histogram, False Color ve Zebra pozlama göstergeleriyle renk ve ışık dengesini hassas biçimde kontrol edin.
- **Maskeler, Karışım Modları ve Ayar Katmanları**: Dikdörtgen, Elips ve serbest Bezier maskelerle bölgesel düzenlemeler yapın; 11 farklı karışım modu ve Ayar Katmanları ile tüm kanallara tek noktadan görünüm uygulayın.

### Net Ses ve Dahili Kayıt
Çok kanallı ses mikseri, 3 bantlı ekolayzır ve ses seviyesi dengeleme ile temiz ses elde edin; mikrofon seslendirmesi, ekran kaydı veya kamera görüntüsünü doğrudan projenize kaydedin.

### Zaman Kazandıran Akıllı Araçlar
- **Otomatik Sahne Algılama** ile uzun çekimleri tek tıkla karelere ayırın.
- **Sessizlik Kesme** ile konuşma videolarındaki boşlukları saniyeler içinde temizleyin.
- **Yerel Konuşmadan Metne Altyazı** ile internet gerektirmeden düzenlenebilir altyazılar oluşturun.

### Yayına Hazır Dışa Aktarma
YouTube, Instagram, TikTok ve yayın standartlarına uygun hazır şablonlarla MP4, MOV, WebM, ProRes veya dikey (9:16) videoları donanım hızlandırma ve arka plan kuyruğu desteğiyle hızlıca teslim edin.

---

## Uygulamadan Kareler

| Renk Düzenleme ve Analiz | Çok Kanallı Ses Mikseri |
| :---: | :---: |
| ![Renk Düzenleme](docs/assets/screenshots/color_grading.png) | ![Ses Mikseri](docs/assets/screenshots/audio_mixer.png) |

| Dışa Aktarma Stüdyosu | Dikey (9:16) Sosyal Medya Tuvali |
| :---: | :---: |
| ![Dışa Aktarma](docs/assets/screenshots/export_studio.png) | ![Dikey Tuval](docs/assets/screenshots/vertical_9_16.png) |

---

## İndirme ve Kurulum

En güncel sürümü **[GitHub Releases](https://github.com/MOzcelik14/triM/releases)** sayfasından indirebilir veya aşağıdaki tek komutlu kurulum yöntemini kullanabilirsiniz:

**Windows (PowerShell):**
```powershell
iwr -useb https://raw.githubusercontent.com/MOzcelik14/triM/master/install.ps1 | iex
```

**Linux (Terminal):**
```bash
curl -fsSL https://raw.githubusercontent.com/MOzcelik14/triM/master/install.sh | bash
```

> **Not:** Windows paketi çalışması için gereken tüm medya bileşenleriyle birlikte gelir. Linux'ta kurulum betiği gerekli medya bileşenlerini (`ffmpeg` ve `ffprobe`) otomatik olarak kontrol eder.

---

## Sistem Gereksinimleri

| Bileşen | Minimum Gereksinim |
| :--- | :--- |
| **İşletim Sistemi** | Windows 10 / 11 (64-bit) veya Linux (64-bit) |
| **Grafik** | OpenGL 3.3+ destekli güncel bir ekran kartı |
| **Depolama ve Gizlilik** | %100 yerel çalışır; kurulum sonrası internet veya hesap gerekmez |

---

## Temel Klavye Kısayolları

| Tuş | İşlem |
| :--- | :--- |
| `Space` | Oynat / Duraklat |
| `J` / `K` / `L` | Geri sar / Durdur / İleri sar |
| `V` / `C` / `B` | Seçim / Kesme / Dalga düzenleme aracı |
| `S` / `Ctrl + K` | Oynatma çizgisinde klibi böl |
| `Ctrl + M` | Videoyu dışa aktar |
| `F1` | Tüm kısayolları görüntüle |

Tüm kısayol listesi için [Klavye Kısayolları](docs/SHORTCUTS.md) sayfasını inceleyebilirsiniz.

---

<details>
<summary><strong>Geliştiriciler İçin: Kaynaktan Derleme</strong></summary>

**triM.** uygulamasını kaynaktan derlemek için **Rust 1.80+** ve Linux ortamında OpenGL/ses geliştirme kütüphaneleri gerekir:

```bash
git clone https://github.com/MOzcelik14/triM.git
cd triM
cargo build --release -p trim-app
```

</details>

## Lisans

**triM.**, **GNU General Public License v3.0 (GPL-3.0)** kapsamında yayınlanan açık kaynaklı bir yazılımdır. Ayrıntılar için [LICENSE](LICENSE) dosyasına göz atabilirsiniz.
