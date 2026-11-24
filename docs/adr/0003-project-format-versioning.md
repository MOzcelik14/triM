# ADR-0003: Proje Formatı Sürümleme ve Geriye Dönük Uyumluluk

- **Durum**: Kabul edildi
- **Tarih**: 2026-11-24

## Bağlam
`.trim` proje dosyaları ve otomatik kayıt (`autosave`) anlık görüntüleri zaman içinde yeni klip efektleri, renk derecelendirme alanları ve çoklu sekans yapılarıyla genişlemektedir. Eski sürümlerle kaydedilmiş projelerin yeni uygulama sürümlerinde bozulmadan açılması gerekir.

## Karar
Proje dosyaları `trim-project` üzerinden sürümlü JSON şeması olarak serileştirilir (`CURRENT_PROJECT_VERSION`). Yeni eklenen tüm alanlar `trim-core` içinde `#[serde(default)]` ile tanımlanır ve şema değişiklikleri `trim-project` içindeki sürümlü migrasyon fonksiyonlarıyla işlenir. Kayıt işlemleri geçici dosya + atomik yeniden adlandırma (`atomic write`) ve okuma tarafında boyut/symlink sınırlarıyla korunur.

## Sonuçlar
- Eski `.trim` projeleri veri kaybı olmadan açılır ve güncel şemaya taşınır.
- Bozuk veya aşırı büyük girdi dosyaları ayrıştırma aşamasında güvenli biçimde reddedilir.
