# triM. Kod Tabanı Kapsamlı Hata ve Güvenlik Denetim Raporu

**Tarih**: 25 Ekim 2026  
**Denetlenen Sürüm**: triM. v1.1.1 (`master` dalı, eae93c6)  
**Denetlenen Bileşenler**: `trim-core`, `trim-commands`, `trim-timeline`, `trim-project`, `trim-media`, `trim-export`, `trim-ui`, `trim-app`, kurulum betikleri (`install.sh`, `install.ps1`) ve CI iş akışları.  
**Metodoloji**: Statik kod analizi, derleyici bayrakları (`cargo check`, `cargo clippy` strict), birim/entegrasyon test takımı (132 test) ve uçtan uca senaryo incelemesi.

---

## 1. Yüksek Öncelikli Bulgular (Çökme, Veri Kaybı, Kilitlenme)

### 1.1. Waveform ve Thumbnail İş Parçacığı Taşması ile Uygulama Çökmesi
* **Konum**: [`crates/trim-media/src/waveform.rs:68`](file:///home/mozcelik/Projeler/triM/crates/trim-media/src/waveform.rs#L68) ve [`crates/trim-media/src/thumbnail.rs:62`](file:///home/mozcelik/Projeler/triM/crates/trim-media/src/thumbnail.rs#L62)
* **Koşul & Etki**: Medya havuzuna art arda çok sayıda video/ses dosyası aktarıldığında veya uzun klipler zaman çizgisine eklendiğinde, ses dalga formu ve küçük resim önizlemeleri oluşturmak için her çağrıda `std::thread::Builder::new().spawn(...).expect("Failed to spawn waveform worker thread")` çalıştırılmaktadır. Sistemde işletim sistemi iş parçacığı limiti (`RLIMIT_NPROC`) aşıldığında veya bellek yetersiz kaldığında `.expect()` çağrısı tüm editör sürecini doğrudan panikletip kapatır; kaydedilmemiş kullanıcı projesi kaybedilir.
* **Durum**: **[Düzeltildi (Grup 1)]** (Sabit 2 worker thread havuzuna dönüştürüldü, panic yerine hata yakalama eklendi)
* **Önerilen Düzeltme**: Her istek için yeni bir OS iş parçacığı oluşturmak yerine sabit boyutlu bir iş parçacığı havuzu (worker pool / `rayon` veya kanallı sınırlı kuyruk) kullanılmalı; `spawn` hatası durumunda paniklemek yerine `Result` döndürülüp işlem kuyruğa alınmalıdır.

---

### 1.2. Ses ve Ekran Kaydı Sırasında Pipe Tamponunun Dolması ile Süreç Kilitlenmesi (Deadlock)
* **Konum**: [`crates/trim-media/src/voiceover.rs:119-121, 144-162`](file:///home/mozcelik/Projeler/triM/crates/trim-media/src/voiceover.rs#L119-L162) ve [`crates/trim-media/src/screen_record.rs:120-122, 157-170`](file:///home/mozcelik/Projeler/triM/crates/trim-media/src/screen_record.rs#L120-L170)
* **Koşul & Etki**: FFmpeg süreçleri `stderr(Stdio::piped())` ile başlatılmakta, ancak kayıt devam ederken arka planda hiçbir thread bu pipe'ı okuyup boşaltmamaktadır. FFmpeg; ses kartı sürücüsü uyarıları, ALSA/PulseAudio/PipeWire tampon aşımı (underrun) logları veya x264 istatistikleri sebebiyle 64 KB (Linux varsayılan pipe kapasitesi) üzerinde çıktı ürettiğinde, FFmpeg `write()` sistem çağrısında bloke olmakta (deadlock) ve kayıt durmaktadır. Kullanıcı "Kaydı Durdur" butonuna bastığında stdin üzerinden gönderilen `'q'` komutu FFmpeg kilitlendiği için asla işlenememekte, 1000 ms zaman aşımı dolunca FFmpeg `kill()` ile zorla öldürülmekte ve WAV/MP4 başlığı bozuk/eksik kaydedilmektedir.
* **Durum**: **[Düzeltildi (Grup 1)]** (`drain_stderr` ile arka planda sürekli okuma eklendi)
* **Önerilen Düzeltme**: `runner.rs:88-95` bileşeninde uygulandığı gibi, stderr ayrı bir thread ile sürekli tüketilmeli (`std::io::copy(&mut err, &mut std::io::sink())`) veya loglara ihtiyaç duyulmuyorsa doğrudan `Stdio::null()` kullanılmalıdır.

---

### 1.3. Geriye Doğru Oynatmada (Reverse Playback) Parça Sonu Kilitlenme Riski
* **Konum**: [`crates/trim-media/src/video.rs:338-342`](file:///home/mozcelik/Projeler/triM/crates/trim-media/src/video.rs#L338-L342)
* **Koşul & Etki**: Geriye doğru akış modunda (`reverse: true`), `child_holder` içindeki alt süreç parça bitiminde `c.wait()` ile beklenmektedir. İleri akışta (`video.rs:260`) `let _ = c.kill();` çağrılırken burada `kill()` çağrılmamaktadır. Kullanıcı oynatmayı parça decode edilirken durdurursa veya FFmpeg çıkış borusu açık kalırsa, `c.wait()` arka plan akış iş parçacığını süresiz bloke edebilir.
* **Durum**: **[Doğrulandı]**
* **Önerilen Düzeltme**: `video.rs:260`'ta olduğu gibi parça sonlandırılırken veya durdurulurken önce `let _ = c.kill();` ardından `let _ = c.wait();` çalıştırılmalıdır.

---

### 1.4. Proje Paketlemede (Bundle Project) İsim Çakışması ile Veri Kaybı
* **Konum**: [`crates/trim-project/src/lib.rs:122-132`](file:///home/mozcelik/Projeler/triM/crates/trim-project/src/lib.rs#L122-L132)
* **Koşul & Etki**: Farklı dizinlerden içe aktarılan ancak dosya adları aynı olan medya dosyaları (örneğin `/kamera1/video.mp4` ve `/kamera2/video.mp4`) projede yer aldığında, `bundle_project` hedef `media/` klasörüne dosyaları yalın dosya adlarıyla kopyalamaktadır (`media_dir.join(&file_name)`). İkinci dosya birinci dosyanın üzerine yazılmakta ve her iki medya havuzu kaydı da `"media/video.mp4"` yoluna işaret etmektedir. İlk dosyanın içeriği pakette kalıcı olarak kaybolmaktadır.
* **Durum**: **[Düzeltildi (Grup 1)]** (Kanonik yol tekilleştirmesi ve otomatik sayaçlı adlandırma eklendi)
* **Önerilen Düzeltme**: Hedef dosya adı üretilirken medya öğesinin benzersiz ID'si eklenmeli (`format!("{}_{}", id, file_name)`) veya çakışma durumunda otomatik numara eklenerek ad tekilleştirilmelidir.

---

### 1.5. UTF-8 Olmayan Dosya Yollarında UI Çökmesi (Panic)
* **Konum**: [`crates/trim-ui/src/app/timeline_actions.rs:66`](file:///home/mozcelik/Projeler/triM/crates/trim-ui/src/app/timeline_actions.rs#L66)
* **Koşul & Etki**: Zaman çizgisinden anlık kare görüntüsü alınıp projeye eklenirken `MediaItem::new(out_png.to_str().unwrap())` çağrısı yapılmaktadır. Linux dosya sisteminde geçersiz UTF-8 baytları barındıran bir dışa aktarma yolu kullanıldığında `.to_str()` `None` döner ve `.unwrap()` arayüz iş parçacığını anında çökerterek uygulamayı kapatır.
* **Durum**: **[Doğrulandı]**
* **Önerilen Düzeltme**: `out_png.to_string_lossy().to_string()` kullanılmalı veya `match` ile güvenli hata iletisi gösterilmelidir.

---

## 2. Orta Öncelikli Bulgular (Mantık Hataları, Senkronizasyon, Geri Alma Tutarsızlıkları)

### 2.1. Kesirli FPS Zaman Kodu (Timecode) Yuvarlama ve Sapma Hatası (Drift)
* **Konum**: [`crates/trim-core/src/time.rs:6-14, 33-34`](file:///home/mozcelik/Projeler/triM/crates/trim-core/src/time.rs#L6-L34)
* **Koşul & Etki**: `seconds_to_timecode` fonksiyonu toplam saniyeyi `let fps_int = fps.round() as u64; total_seconds = total_frames / fps_int;` şeklinde hesaplarken, `timecode_to_seconds` doğrudan kesirli `fps` değeriyle çarpmaktadır. 29.97 FPS (NTSC standardı) bir projede 100 saniyelik bir süre için `seconds_to_timecode` `00:01:39:27` üretmekte, bu değer `timecode_to_seconds` fonksiyonuna verildiğinde ise `99.8998s` (yaklaşık 0.10 saniye / 3 kare fark) elde edilmektedir. Zaman çizgisinde süre uzadıkça gösterge ve kesim noktaları kaymaktadır.
* **Durum**: **[Doğrulandı]**
* **Önerilen Düzeltme**: Zaman kodu dönüşümlerinde saniye hesabı doğrudan `total_frames as f64 / fps` üzerinden tutarlı bir şekilde hesaplanmalı, NTSC standartları için drop-frame gösterimi ya da kesirli kare katsayıları eşitlenmelidir.

---

### 2.2. "Sesi Ayır" (Detach Audio) Komutunda Kanal Eşleştirme Hatası
* **Konum**: [`crates/trim-commands/src/commands.rs:782-791`](file:///home/mozcelik/Projeler/triM/crates/trim-commands/src/commands.rs#L782-L791)
* **Koşul & Etki**: Kullanıcı V2 veya V3 kanalındaki bir klibe sağ tıklayıp "Sesi Ayır" dediğinde, komut klibin bulunduğu video kanalını (`self.video_track_id`) göz ardı etmekte ve daima ilk ses kanalını (`timeline.get_audio_tracks().first()`, yani A1) seçmektedir. V2'deki bir klibin sesi A2 yerine A1'e bindirilmekte ve A1'deki mevcut kliplerle çakışmaktadır.
* **Durum**: **[Doğrulandı]**
* **Önerilen Düzeltme**: `timeline.find_or_create_corresponding_audio_track(&self.video_track_id)` çağrılarak video kanalının sıra numarasına denk gelen ses kanalı (V1->A1, V2->A2) hedeflenmelidir.

---

### 2.3. Klip Bölme ve Sesi Ayırma Eylemlerinde Undo/Redo UUID Kararsızlığı
* **Konum**: [`crates/trim-commands/src/commands.rs:411, 524, 793`](file:///home/mozcelik/Projeler/triM/crates/trim-commands/src/commands.rs#L411)
* **Koşul & Etki**: `SplitClipCommand`, `BatchSplitClipCommand` ve `DetachAudioCommand` komutlarının `execute()` fonksiyonunda yeni klip ID'si her seferinde `uuid::Uuid::new_v4().to_string()` ile dinamik üretilmektedir. Kullanıcı işlemi geri alıp (Undo) tekrar yinelediğinde (Redo), klip yepyeni bir ID kazanmaktadır. Bu durum zaman çizgisindeki seçimleri, bağlı grupları ve bu klibe referans veren diğer komut yığınlarını bozar.
* **Durum**: **[Doğrulandı]**
* **Önerilen Düzeltme**: Üretilen yeni klip ID'si ilk çalıştırmada komut nesnesinin içinde (`Option<String>`) saklanmalı; Redo sırasında `unwrap_or_else(Uuid::new_v4)` mantığıyla önceki ID yeniden kullanılmalıdır.

---

### 2.4. Klip Sürükle-Bırak İşleminde Ayrık Geri Alma (Atomic Undo Eksikliği)
* **Konum**: [`crates/trim-ui/src/app.rs:2057-2084`](file:///home/mozcelik/Projeler/triM/crates/trim-ui/src/app.rs#L2057-L2084)
* **Koşul & Etki**: Medya havuzundan zaman çizgisine video bırakıldığında, hem video klibi hem de otomatik oluşturulan ses klibi için `undo_stack.push()` fonksiyonu iki kez art arda çağrılmaktadır. Kullanıcı Ctrl+Z'ye bir kez bastığında yalnızca ses klibi silinmekte, video klibi zaman çizgisinde kalmaktadır. Tek bir kullanıcı eylemi iki ayrı geri alma adımına bölünmektedir.
* **Durum**: **[Doğrulandı]**
* **Önerilen Düzeltme**: Her iki klip ekleme eylemini tek bir işlemde toplayan bir `MacroCommand` veya `CompoundCommand` oluşturularak yığına tek seferde itilmelidir.

---

### 2.5. Whisper Transkripsiyonunda Geçici Dosya Çakışması ve Yanlış Altyazı Yüklenmesi
* **Konum**: [`crates/trim-media/src/transcribe.rs:91-95, 147-156`](file:///home/mozcelik/Projeler/triM/crates/trim-media/src/transcribe.rs#L91-L156)
* **Koşul & Etki**: Transkripsiyon çıktı dizini sabit `/tmp/trim_transcribe` olarak kodlanmıştır ve eski dosyalar silinmemektedir. Python whisper çalıştırıldığında üretilen `.srt` dosyasını aramak için `read_dir(&temp_dir)` içindeki ilk rastlanan `.srt` dosyası alınmaktadır. Kullanıcı birden fazla videoyu arka arkaya transkribe ettiğinde, sonraki video için önceki videonun eski altyazı dosyası yüklenebilmektedir.
* **Durum**: **[Doğrulandı]**
* **Önerilen Düzeltme**: Her transkripsiyon oturumu için benzersiz bir geçici klasör (`tempfile::tempdir()`) açılmalı ve iş bitiminde klasör temizlenmelidir.

---

### 2.6. Kurulum Betiklerinde Sürüm Uyuşmazlığı (`v1.0.1` vs `v1.1.1`)
* **Konum**: [`install.sh:29`](file:///home/mozcelik/Projeler/triM/install.sh#L29) ve [`install.ps1:33`](file:///home/mozcelik/Projeler/triM/install.ps1#L33)
* **Koşul & Etki**: Projedeki tüm Rust krate'leri (`Cargo.toml`) ve Flatpak meta verileri `1.1.1` sürümüne güncellenmişken, `install.sh` betiğinde `VERSION="1.0.1"` ve `install.ps1` betiğinde `$Version = "1.0.1"` sabit kalmıştır. Kullanıcı betiği çalıştırıp hazır ikili indirmeyi seçtiğinde GitHub Releases üzerinden eski `1.0.1` sürümünü indirmektedir.
* **Durum**: **[Doğrulandı]**
* **Önerilen Düzeltme**: Her iki kurulum betiğindeki sürüm sabiti `1.1.1` olarak güncellenmelidir.

---

## 3. Düşük Öncelikli Bulgular (Kırılgan Kenar Durumları, UI Sınır Aşımı)

### 3.1. Dışa Aktarma Penceresinde Sınır Kontrolsüz Dizi İndeksi
* **Konum**: [`crates/trim-ui/src/dialogs/export_dialog.rs:66-70, 362`](file:///home/mozcelik/Projeler/triM/crates/trim-ui/src/dialogs/export_dialog.rs#L66)
* **Koşul & Etki**: `presets[0]` ve `self.presets[self.selected_preset_idx]` doğrudan dizi indeksi ile çağrılmaktadır. Eğer önayarlar listesi boşalırsa veya `selected_preset_idx` geçersiz bir değere gelirse egui arayüzü her render karesinde panikleyerek çökecektir.
* **Durum**: **[Doğrulandı]**
* **Önerilen Düzeltme**: `self.presets.get(self.selected_preset_idx).or_else(|| self.presets.first())` şeklinde güvenli erişim sağlanmalıdır.

---

### 3.2. Medya İçe/Dışa Aktarmada Kök Dizin Panik Riski
* **Konum**: [`crates/trim-ui/src/app/media_actions.rs:46`](file:///home/mozcelik/Projeler/triM/crates/trim-ui/src/app/media_actions.rs#L46)
* **Koşul & Etki**: Dosya kaydetme diyaloğunda dönen yol için `out.file_name().unwrap()` çağrılmaktadır. Kullanıcı kök dizini (`/`) seçtiğinde `file_name()` `None` döner ve `.unwrap()` panikler.
* **Durum**: **[Doğrulandı]**
* **Önerilen Düzeltme**: `.and_then(|f| f.to_str()).unwrap_or("cikti")` şeklinde güvenli alternatif sunulmalıdır.

---

### 3.3. Sınırsız Büyüyen Geri Alma Yığını Bellek Tüketimi
* **Konum**: [`crates/trim-commands/src/lib.rs:31-32, 46`](file:///home/mozcelik/Projeler/triM/crates/trim-commands/src/lib.rs#L31)
* **Koşul & Etki**: `UndoStack` yapısında herhangi bir üst sınır (kapasite) bulunmamaktadır. Yüzlerce kesim ve taşıma yapılan uzun süreli kurgularda yığın sınırsız büyüyerek belleği şişirir.
* **Durum**: **[Doğrulandı]**
* **Önerilen Düzeltme**: Yığın için maksimum derinlik sınırı (örneğin 100 işlem) belirlenmeli, sınır aşıldığında en eski komut `undo_stack.remove(0)` ile silinmelidir.

---

### 3.4. Çift (Mükerrer) GitHub Actions CI Çalıştırması
* **Konum**: [`.github/workflows/ci.yml`](file:///home/mozcelik/Projeler/triM/.github/workflows/ci.yml) ve [`.github/workflows/rust-ci.yml`](file:///home/mozcelik/Projeler/triM/.github/workflows/rust-ci.yml)
* **Koşul & Etki**: Her iki iş akışı da `master` dalına push ve PR olaylarında bağımsız olarak tetiklenmekte ve her ikisi de Ubuntu üzerinde `cargo test` ve `cargo build` adımlarını tekrarlamaktadır. Bu durum GitHub Actions kotasını iki kat tüketmektedir.
* **Durum**: **[Doğrulandı]**
* **Önerilen Düzeltme**: İki dosya tek bir `ci.yml` altında çoklu platform matrisi (Linux ve Windows) olarak birleştirilmelidir.

---

## 4. Denetim Özeti ve Doğrulama Metrikleri

### 4.1. Çalıştırılan Komutlar ve Sonuçları
1. `cargo check --workspace --all-targets` ➔ **Başarılı (0 hata)**
2. `cargo clippy --workspace --all-targets -- -W clippy::all` ➔ **Başarılı (0 uyarı)**
3. `cargo test --workspace` ➔ **Başarılı (132 test geçti, 0 hata)**
4. `cargo clippy --workspace --all-targets -- -W clippy::panic -W clippy::indexing_slicing` ➔ **Doğrulandı: Doğrudan indeksleme ve unwrap noktaları tespit edildi.**

### 4.2. İncelenen Alanlar
* Tüm workspace krate'leri (`trim-core`, `trim-commands`, `trim-timeline`, `trim-project`, `trim-media`, `trim-export`, `trim-ui`, `trim-app`).
* Komut desenleri (Undo/Redo tersinirliği, macro komutlar).
* FFmpeg argüman kaçırma, pipe yönetimi, alt süreç (`ChildGuard`) yaşam döngüsü.
* Proje serileştirme/deserileştirme, göreli yol çözümleme, taşınabilir bundle paketi.
* Kurulum ve dağıtım betikleri (`install.sh`, `install.ps1`, Flatpak).

### 4.3. İncelenemeyen / Sınırlı Alanlar
* Donanım tabanlı GPU kodlayıcıları (NVENC, VA-API, QuickSync) çalıştırma ortamında fiziksel GPU bulunmadığı için çalışma anında gerçek donanım seviyesinde test edilememiş, komut satırı argümanları statik olarak doğrulanmıştır.
* `whisper.cpp` / `openai-whisper` yerel ikili dosyaları sistemde yüklü olmadığından canlı yapay zeka çıkarımı sadece mock SRT testleriyle doğrulanabilmiştir.

### 4.4. Açık Sorular
1. Proje paketlemede (`bundle_project`) aynı isimli dosyalar için `media/{uuid}_{filename}` formatı tercih edilmeli mi?
2. Klip sürükle-bırak işleminde video ve bağlı sesin tek Ctrl+Z ile birlikte geri alınması için bir `CompoundCommand` eklenmesi onaylanıyor mu?
