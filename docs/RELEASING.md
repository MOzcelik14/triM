# Sürüm Yayınlama Rehberi (Release Process)

Bu belge, triM projesinde yeni bir sürümün hazırlanması, test edilmesi ve GitHub üzerinden paketlenerek yayınlanması sürecini belgeler.

---

## 1. Sürüm Numarasını Güncelleme

Sürüm numarası Semantic Versioning (`MAJOR.MINOR.PATCH`) standardını takip eder:

1. `Cargo.toml`:
   ```toml
   [workspace.package]
   version = "1.2.0"
   ```
2. `Cargo.lock` güncellemesi:
   ```bash
   cargo check
   ```
3. Dokümantasyon ve paketleme meta verileri:
   - `io.github.mozcelik14.triM.metainfo.xml`
   - `.github/workflows/release-linux.yml` (varsayılan fallback sürüm)
   - `docs/index.html` ve `docs/js/main.js`

---

## 2. Doğrulama ve Testler

Yayın öncesinde tüm çalışma alanının format, clippy ve test kontrollerinden geçmesi şarttır:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

---

## 3. Git Etiketi Oluşturma ve Push

Değişiklikleri commit edip sürüm etiketini oluşturun:

```bash
git add -A
git commit -m "chore: bump version to 1.2.0"
git push origin master

git tag -a v1.2.0 -m "Release v1.2.0: Kısa sürüm açıklaması"
git push origin v1.2.0
```

---

## 4. GitHub Actions ve Yayın Paketleri

Etiket (`v*`) pushlandığında `.github/workflows/release-linux.yml` otomatik olarak tetiklenir ve şu paketleri derler:
- `.deb` paketi (Ubuntu / Debian x86_64)
- `.rpm` paketi (Fedora / RHEL x86_64)
- `.AppImage` (Bağımsız çalıştırılabilir Linux paketi)
- `.zip` (Windows x86_64 ikili ve bağımlılık arşivi)
- `SHA256SUMS.txt` (Doğrulama özetleri)
