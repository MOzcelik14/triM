# ADR-0002: FFmpeg/FFprobe Alt Süreç (Subprocess) Mimarisi

- **Durum**: Kabul edildi
- **Tarih**: 2026-11-24

## Bağlam
Video çözme, küçük resim/dalga formu üretimi, donanım hızlandırma (`vaapi`/`cuda`/`vulkan`) ve nihai dışa aktarma işlemleri geniş codec/kapsayıcı desteği gerektirir. C FFI (`libav*`) bağlamak derleme karmaşıklığını, ABI kırılmalarını ve bellek güvenliği riskini artırır.

## Karar
`trim-media` ve `trim-export`, sistemdeki veya paketlenmiş `ffmpeg` / `ffprobe` ikililerini alt süreç olarak çalıştırır. Tüm alt süreçler `ChildGuard` / `TrackedChildPool` (RAII `Drop` ile sonlandırma, `panic = "unwind"` profili) altında yönetilir; kullanıcı metinleri ve dosya yolları `filter_complex` ve protokol enjeksiyonuna karşı temizlenir; donanım hızlandırma çalışma zamanında `probe_working_hwaccel()` ile doğrulanıp başarısızlıkta yazılıma geri düşer.

## Sonuçlar
- Bellek güvenliği ve çökme yalıtımı korunur; alt süreç zombileri `ChildGuard` ile önlenir.
- Yeni FFmpeg sürümleri yeniden derleme gerektirmeden kullanılabilir.
- Export ve önizleme tutarlılığı `parity_test` ile gerçek `ffmpeg` çıktısı üzerinden test edilir.
