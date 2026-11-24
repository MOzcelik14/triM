# ADR-0001: Command-Pattern Undo/Redo Mimarisi

- **Durum**: Kabul edildi
- **Tarih**: 2026-11-24

## Bağlam
Zaman çizelgesi düzenleme işlemleri (bölme, taşıma, kırpma, ripple/roll/slip/slide, efekt/renk/ses değişikliği, çoklu klip toplu işlemleri) geri alınabilir (Undo) ve yeniden uygulanabilir (Redo) olmalıdır. `trim-ui` katmanının doğrudan `Timeline` iç yapısını mutasyona uğratması hata ve tutarsızlık riskini artırır.

## Karar
Tüm kalıcı zaman çizelgesi mutasyonları `trim-commands` crate'i içinde `TimelineCommand` (`execute` / `undo`) ve bileşik `MacroCommand` nesneleri olarak modellenir. `UndoStack`, her komutu atomik olarak uygular ve geri alır. `trim-ui` yalnızca komut üretip `UndoStack::push` çağırır veya salt okunur sorgu API'lerini kullanır.

## Sonuçlar
- Her yeni düzenleme özelliği tek bir geri alma/yeniden yapma sözleşmesine uyar.
- Çok kanallı bağlı klip (linked audio/video) işlemleri `MacroCommand` veya eşlikçi komutlarla atomik kalır.
- İş mantığı `trim-ui` olmadan birim testleriyle doğrulanabilir.
