# triM. Klavye Kısayolları

**triM.**, `triM Varsayılan`, `Premiere Pro` ve `DaVinci Resolve` kısayol profillerini destekler. Kısayol profili Tercihler (`Ctrl + ,`) penceresinden değiştirilebilir veya uygulama içinde `F1` tuşuna basılarak kısayol rehberi açılabilir.

Aşağıdaki liste `crates/trim-ui/src/app.rs` içindeki gerçek tuş bağlamalarına göre doğrulanmıştır.

---

## 1. Oynatma ve Gezinme (Transport)

| Kısayol | İşlev |
| :--- | :--- |
| `Boşluk` | Oynat / Duraklat |
| `J` | Geri oynat (tekrar basıldığında 2x, 4x, 8x hızlanır) |
| `K` | Oynatmayı durdur |
| `L` | İleri oynat (tekrar basıldığında 2x, 4x, 8x hızlanır) |
| `Sol Ok` / `Sağ Ok` | 1 kare geri / ileri git |
| `Shift + Sol Ok` / `Shift + Sağ Ok` | 1 saniye geri / ileri git |
| `Yukarı Ok` / `Aşağı Ok` | Önceki / sonraki kesim noktasına atla |
| `Home` / `End` | Zaman çizelgesinin başına / sonuna git |
| `I` / `O` | Giriş (Mark In) / Çıkış (Mark Out) işaretle |
| `Alt + X` | Giriş / Çıkış işaretlerini temizle |
| `F11` / `Esc` | Tam ekran Program Monitörünü aç / kapat |

---

## 2. Araçlar ve Zaman Çizelgesi Düzenleme

| triM. Varsayılan | Premiere Pro | DaVinci Resolve | İşlev |
| :--- | :--- | :--- | :--- |
| `V` | `V` | `A` | Seçim Aracı (Selection Tool) |
| `A` | `A` | - | Kanal Seçim Aracı (Track Select Tool) |
| `B` | `B` | - | Dalgalı Düzenleme Aracı (Ripple Edit Tool) |
| `C` | `C` | `B` | Jilet / Kesme Aracı (Razor / Cut Tool) |
| `Y` | `Y` | `Y` | Kaydırma Aracı (Slip Edit Tool) |
| `P` | `P` | `P` | Kalem / Keyframe Aracı (Pen Tool) |
| `H` | `H` | `H` | El / Kaydırma Aracı (Hand / Pan Tool) |
| `T` | `T` | `T` | Metin Aracı (Type / Text Tool) |
| `S` | `Ctrl + K` | `Ctrl + \` | Oynatma kafasında klibi böl |
| `Q` | `Q` | `Q` | Oynatma kafasına kadar dalgalı kırp (Baş) |
| `W` | `W` | `W` | Oynatma kafasından sona dalgalı kırp (Son) |
| `Delete` / `Backspace` | `Delete` / `Backspace` | `Delete` / `Backspace` | Seçili klipleri sil |
| `Shift + Delete` | `Shift + Delete` | `Shift + Delete` | Seçili klipleri dalgalı sil (Ripple Delete) |
| `,` / `.` | `,` / `.` | `,` / `.` | Kaynak Monitöründen Araya Ekle / Üzerine Yaz |
| `M` | `M` | `M` | Oynatma kafasına işaretçi (Marker) ekle |
| `N` | `N` | `N` | Manyetik kenetlenmeyi aç / kapat (Snap) |
| `1` - `4` | `1` - `4` | `1` - `4` | Çoklu Kamera (Multicam) modunda aktif açıyı değiştir |
| `Ctrl + =` / `Ctrl + -` | `Ctrl + =` / `Ctrl + -` | `Ctrl + =` / `Ctrl + -` | Zaman çizelgesini yakınlaştır / uzaklaştır |

---

## 3. Proje, Pano ve Paneller

| Kısayol | İşlev |
| :--- | :--- |
| `Ctrl + N` | Yeni proje oluştur |
| `Ctrl + O` | Proje aç |
| `Ctrl + S` | Projeyi kaydet |
| `Ctrl + Z` | Geri al (Undo) |
| `Ctrl + Y` veya `Ctrl + Shift + Z` | Yinele (Redo) |
| `Ctrl + C` / `Ctrl + X` / `Ctrl + V` | Klipleri kopyala / kes / yapıştır |
| `Ctrl + Alt + V` | Özellik Yapıştır (Paste Attributes) penceresini aç |
| `Ctrl + L` | Seçili klipleri bağla / ayır (Link / Unlink) |
| `Ctrl + R` | Klip Hızı / Süresi penceresini aç |
| `Ctrl + T` | Yeni Başlık (Title) penceresini aç |
| `Ctrl + I` | Medya dosyası içe aktar |
| `Ctrl + M` veya `Ctrl + E` | Dışa Aktarma Stüdyosu'nu aç |
| `Ctrl + Shift + E` | Render Kuyruğu penceresini aç / kapat |
| `Ctrl + ,` | Tercihler penceresini aç |
| `F1` | Klavye kısayolları penceresini aç / kapat |
| `Ctrl + Q` | Uygulamadan çık |
