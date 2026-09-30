# Cutline NLE - Milestone 1 Tamamlandı

Linux için modern, hızlı ve açık kaynak Non-Linear Video Editor (NLE) projesi **Cutline**, Milestone 1 hedefleri eksiksiz olarak tamamlanarak hayata geçirildi.

---

## Gerçekleştirilen Sistemler ve Mimarisi

- **Project Bin:** Video, ses ve görsel dosyalarını içe aktarma, FFprobe ile metadata okuma, küçük resim (thumbnail) önbellekleme, timeline'a sürükle-bırak desteği.
- **Program Monitor:** Aspect-ratio korumalı, letterbox destekli video önizleme, transport kontrolleri (Oynat/Duraklat, Durdur, 1 kare ileri/geri), `HH:MM:SS:FF` timecode gösterimi.
- **Timeline:** Ayrık veri modeli (`TimelineModel`, `Track`, `Clip`), klip taşıma, sol/sağ kenardan trimleme (kırpma), `S` kısayoluyla kesme (split/cut), normal silme ve boşluksuz silme (ripple delete), kenarlara ve oynatma ibresine kenetlenme (snapping).
- **Undo / Redo:** `QUndoStack` tabanlı tam komut deseni (`Ctrl+Z`, `Ctrl+Y`).
- **Proje Yönetimi:** `.cutline` JSON proje kaydetme ve açma, atomik kayıt, göreceli dosya yolu (relative path) desteği, her 2 dakikada bir otomatik kurtarma (autosave) ve çökme sonrası kurtarma diyaloğu.
- **Export Engine:** FFmpeg tabanlı arka plan dışa aktarma (MP4 1080p, 720p, Source Match), gerçek zamanlı ilerleme yüzdesi ve iptal desteği.

---

## Çalıştırma

```bash
./run.sh
```

Veya sanal ortam üzerinden:

```bash
source .venv/bin/activate
python3 -m cutline.main
```

## Test Süiti

```bash
.venv/bin/pytest tests/ -v
```
