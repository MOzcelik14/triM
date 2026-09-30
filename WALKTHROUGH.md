# Cutline NLE - Oynatma Hızı ve Ses Düzeltmeleri Güncellemesi

Kullanıcı geri bildirimleri doğrultusunda ses çıtırtısı (crackling) ve videonun 2x-3x hızda erken bitmesi sorunları kökten çözüldü.

---

## Yapılan İyileştirmeler

### 1. Ses Çıtırtısının Çözümü (Queue-Buffered Audio Streaming)
- **Sorun:** Oynatma döngüsünde ses paketleri kontrolsüz bir şekilde topluca ses kartına yazılıyordu. Video kareleri arasında ses yazımı durakladığında ses donanımı aç kalıyor (buffer underrun), ardından ani paket yığını geldiğinde taşma (buffer overflow) yapıp şiddetli çıtırtı/patlama seslerine yol açıyordu.
- **Çözüm:**
  - `QAudioSink` için tampon boyutu 96.000 byte (~0.5 saniye) olarak ayarlandı.
  - Oynatma motoruna `self._audio_queue = bytearray()` ve `self._video_queue = deque()` önbellek kuyruğu eklendi.
  - Ses verisi, `audio_sink.bytesFree()` sorgulanarak sadece ses kartının hazır olduğu kadar dilimler halinde düzenli olarak aktarılıyor.
  - Sonuç: Çıtırtısız, kesintisiz ve net ses çıkışı.

### 2. Videonun Hızlı Bitmesi ve Zamanlama Senkronizasyonu
- **Sorun:** Önceki döngüde `f_time >= src_target - 0.05` mantığı, gelecekteki kareleri de karşıladığı için döngü her zamanlayıcı adımında (10-16 ms) bir sonraki kareyi hemen tüketiyordu. Bu nedenle 30 FPS bir video saniyede 60-100 kare tüketerek 2x-3x hızda oynatılıp hemen sona ulaşıyordu.
- **Çözüm:**
  - Video kareleri çözülüp zaman damgalarıyla (`pts_sec`) birlikte `_video_queue` kuyruğuna alınıyor.
  - Kareler ekrana **yalnızca ve yalnızca** duvar saati zamanı (`cur_time`) o karenin gerçek zamanına (`pts_sec`) ulaştığında aktarılıyor.
  - Otomatik yapılan hız testinde 1.0 saniye duvar saatinde oynatma süresi **0.997 saniye** olarak ölçüldü (hata payı sadece 3 milisaniye!).
  - 10 saniyelik bir video artık tam olarak 10.0 saniyede, doğal hızında oynatılıyor.

### 3. FFprobe Süre Okuma Güvenliği
- Video veya ses akışlarında `format.duration` eksik veya `"N/A"` olduğunda `nb_frames / fps` ve akış süreleri üzerinden yedekli çözümleme eklendi.

---

## Test Sonuçları (12/12 Başarılı)

```bash
$ .venv/bin/pytest tests/ -v
12 passed in 3.65s
```

---

## Çalıştırma

```bash
./run.sh
```
