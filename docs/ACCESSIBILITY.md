# triM. Erişilebilirlik (Accessibility), Klavye ve HiDPI Notları

## 1. Klavye ile Tam Yönetim ve Özelleştirme
- Tüm temel düzenleme, taşıma, oynatma (`J`/`K`/`L`, `Space`) ve araç seçimi (`V`, `A`, `B`, `R`, `C`, `Y`, `U`, `P`, `H`, `T`) eylemleri klavye kısayollarıyla erişilebilirdir.
- `F1` veya Yardım menüsünden açılan **Klavye Kısayolları** penceresi (`ShortcutsDialog`) tüm kısayolları tek yerde listeler ve profil seçimi (`triM. Varsayılan`, `Premiere Pro`, `DaVinci Resolve`, `Final Cut Pro`) sunar.

## 2. HiDPI ve Ölçekleme
- `egui::Context::pixels_per_point()` üzerinden sistem ölçeklemesi otomatik desteklenir; vektör tabanlı ikonlar (`toolbar.rs`) ve `Painter` çizimleri ölçeklemeden etkilenmeden keskin kalır.

## 3. Çok Dilli Arayüz (TR / EN)
- Tercihler (`Ctrl+,`) üzerinden Türkçe ve İngilizce arasında anında geçiş yapılabilir (`crates/trim-ui/src/i18n.rs`).

## 4. Bilinen Sınırlar (`accesskit` ve Linux IME)
- **`egui` `accesskit` Durumu:** `Cargo.toml` içindeki `eframe = "0.29"` yapılandırmasında `accesskit` özelliği **etkindir** (`accesskit_winit`, `accesskit_unix` / AT-SPI2); `cargo deny --offline --target x86_64-unknown-linux-gnu check licenses` uyumu doğrulanmıştır. Standart `egui` düğmeleri, etiketleri ve diyalog pencereleri erişilebilirlik ağacına aktarılır.
- **Özel Çizim Alanları:** `Painter` ile özel çizilen zaman çizelgesi kanvası (`canvas/tracks.rs`), dalga formları ve monitör içi katmanlar ayrıntılı AT-SPI2 düğümleri üretmez; bu alanlarda tam klavye navigasyonu ve `F1` kısayol rehberi birincil erişim yoludur.
- **Linux X11 IME (Türkçe Karakterler):** `egui-winit` 0.29'un Linux X11'de `Ime::Commit` olaylarını yok sayması nedeniyle, metin girişi aktifken `suppress_linux_ime_for_text_input` (`crates/trim-ui/src/app.rs`) ile IME geçici olarak devre dışı bırakılır; böylece `ç, ğ, ı, ö, ş, ü, İ` tuşları doğrudan layout karakteri olarak alınır.
