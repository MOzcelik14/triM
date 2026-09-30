# Cutline

Linux için modern, hızlı ve açık kaynak Non-Linear Video Editor (NLE).

## Özellikler (Milestone 1)
- **Project Bin:** Medya kütüphanesi (video, ses, görsel import ve FFprobe analizi)
- **Program Monitor:** Frame-accurate video önizleme, timecode, transport kontrolleri
- **Timeline:** Tek/çoklu track mimarisi, sürükle-bırak, split (cut), trim, delete
- **Undo / Redo:** QUndoStack tabanlı tam geri alma / yineleme
- **Proje Yönetimi:** `.cutline` JSON tabanlı proje kaydetme / yükleme ve otomatik kurtarma (autosave)
- **Export:** FFmpeg tabanlı MP4 export (arka plan işlemi, ilerleme çubuğu, iptal desteği)

## Kurulum ve Çalıştırma
```bash
./run.sh
```
veya:
```bash
source .venv/bin/activate
python3 -m cutline.main
```
