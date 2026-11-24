# triM. Kullanıcı Kılavuzu

Bu kılavuz, **triM.** video düzenleyicisinin kurulumundan ilk projenin dışa aktarımına kadar temel iş akışını özetler.

> [!NOTE]
> Güncel arayüz ekran görüntüleri (ör. yerelleştirilmiş durum çubuğu, yeni Roll/Slide/Subclip/Multicam akışları) için kullanıcı tarafından sağlanacak ekran görüntüleri `docs/assets/screenshots/` altına eklenebilir.

---

## 1. Kurulum

### Linux
- **Debian / Ubuntu (`.deb`)**:
  ```bash
  sudo apt install ./trim_<sürüm>_amd64.deb
  ```
- **Fedora / RHEL (`.rpm`)**:
  ```bash
  sudo dnf install ./trim-<sürüm>-1.x86_64.rpm
  ```
- **AppImage**:
  ```bash
  chmod +x triM-<sürüm>-x86_64.AppImage
  ./triM-<sürüm>-x86_64.AppImage
  ```
- **Taşınabilir Arşiv / Kaynak Kurulum**: Arşivi açıp `./trim` ile başlatabilir veya `./install.sh` betiğini kullanabilirsiniz.
- **Gereksinimler**: Sistemde `ffmpeg` ve `ffprobe` kurulu olmalıdır (`sudo apt install ffmpeg` veya `sudo dnf install ffmpeg`).

### Windows
- Yayınlanan `.zip` arşivini çıkarın veya `install.ps1` betiğini çalıştırın (`ffmpeg.exe` ve `ffprobe.exe` ikilileri uygulama yanında veya `PATH` üzerinde bulunmalıdır).

---

## 2. İlk Proje ve Medya İçe Aktarma

1. Uygulamayı açtığınızda **Hoş Geldiniz** penceresinden **Yeni Proje** veya **Proje Aç** (`.trim`) seçebilirsiniz.
2. **Medya İçe Aktarma (`Ctrl+I`)**: Dosya seçiciyi açın ya da işletim sistemi dosya yöneticisinden video/ses/görsel dosyalarını doğrudan **Medya Havuzu**na veya **Zaman Çizelgesi**ne sürükleyip bırakın.
3. **Proxy Oluşturma**: Yüksek çözünürlüklü (ör. 4K) kliplerde akıcı oynatma için klip üzerinde sağ tıklayıp **Proxy Oluştur** seçeneğini kullanabilirsiniz; işlem alt durum çubuğundan izlenebilir ve **İptal** düğmesiyle durdurulabilir.
4. **Klasör ve Subclip**: Medya havuzunda klasörler oluşturabilir, Kaynak Monitörü'nde `I` / `O` ile aralık işaretleyip **Subclip** oluşturabilirsiniz.

---

## 3. Zaman Çizelgesi Düzenleme

- **Temel Araçlar**:
  - `V`: Seçim (Taşıma ve kenardan kırpma; `Alt` ile bağlı ses/video kliplerini bağımsız kırpma).
  - `C` / `Ctrl+K`: Jilet (Razor) ile kesme.
  - `B`: Ripple Düzenleme (sonraki klipleri otomatik kaydırma).
  - `N`: Roll Düzenleme (komşu iki klibin ortak kesme noktasını kaydırma).
  - `Y`: Slip (klip süresini değiştirmeden kaynak içi aralığı kaydırma).
  - `U`: Slide (klibi komşularının arasında kaydırma).
- **Manyetik Hizalama (Snap)**: Araç çubuğundan veya `S` kısayoluyla açılıp kapatılabilir.
- **Çoklu Sekans ve Bileşik Klip**: Sekans çubuğundan `+` ile yeni sekans ekleyebilir, birden çok klibi seçip **Compound Clip** altında gruplayabilirsiniz.
- **Çok Kameralı (Multicam) Senkron**: Seçili klipleri timecode bilgisine göre hizalayıp kamera açıları arasında geçiş yapabilirsiniz.
- **Otomatik Kayıt ve Kurtarma**: Beklenmedik kapanmalarda uygulama yeniden açıldığında en güncel `.trim` otomatik kayıt anlık görüntüsünü kurtarmanızı önerir.

---

## 4. Renk, Efekt, Başlık ve Ses

- **Renk Derecelendirme**: Sağdaki **Denetçi (Inspector)** panelinden Kamera Log profilleri, Beyaz Dengesi, Parlaklık/Kontrast, 3-Yollu Renk Tekerlekleri, RGB/Luma Eğrileri, 6-Vektör HSL, 3D HSL Qualifier, `.cube` 3D LUT ve Analog Film efektlerini uygulayabilirsiniz.
- **Animasyon (Keyframe)**: Dönüşüm, opaklık ve ses seviyesi parametreleri için doğrusal, hold veya bezier anahtar kareler ekleyebilirsiniz.
- **Ses Mikseri**: Her kanal için ses düzeyi, pan, susturma/solo, 6-bant EQ, kompresör ve ana çıkışta VU metre bulunur.

---

## 5. Dışa Aktarma (Export)

1. `Ctrl+M` veya **Dışa Aktar** düğmesiyle **Export Studio** penceresini açın.
2. Hazır profillerden (MP4 H.264/H.265, MOV ProRes, WebM vb.) veya donanım hızlandırmalı kodlayıcılardan (NVENC, VAAPI, QSV, AMF) birini seçin.
3. İsterseniz işi **Render Kuyruğu**na ekleyin veya doğrudan başlatın; işlem sırasında ilerlemeyi izleyebilir ve **İptal** ile güvenli biçimde durdurabilirsiniz.

---

## 6. Klavye Kısayolları ve Erişilebilirlik

- Uygulama içinde `F1` tuşuna basarak tüm kısayolları listeleyen yardım penceresini açabilirsiniz.
- `Ctrl+,` ile **Tercihler** penceresinden dil (`Türkçe` / `English`), kısayol profili (`triM`, `Premiere Pro`, `DaVinci Resolve`) ve otomatik kayıt aralığını özelleştirebilirsiniz.
- Ayrıntılar için [SHORTCUTS.md](file:///home/mozcelik/Projeler/triM/docs/SHORTCUTS.md) ve [ACCESSIBILITY.md](file:///home/mozcelik/Projeler/triM/docs/ACCESSIBILITY.md) dosyalarına bakın.
