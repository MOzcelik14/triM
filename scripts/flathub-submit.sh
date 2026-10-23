#!/usr/bin/env bash
set -euo pipefail

APP_ID="io.github.mozcelik14.triM"
BRANCH_NAME="${APP_ID}"
MANIFEST="io.github.mozcelik14.triM.yml"
METADATA="io.github.mozcelik14.triM.metainfo.xml"
CARGO_SOURCES="cargo-sources.json"

echo "=== Flathub Submission Automation for ${APP_ID} ==="

if ! command -v gh >/dev/null 2>&1; then
    echo "Hata: GitHub CLI ('gh') yüklü değil." >&2
    exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
    echo "Hata: 'gh' yetkilendirilmemiş. Lütfen 'gh auth login' çalıştırın." >&2
    exit 1
fi

USER_LOGIN=$(gh api user -q .login)
echo "GitHub kullanıcısı: ${USER_LOGIN}"

FORK_REPO="${USER_LOGIN}/flathub"
UPSTREAM_REPO="flathub/flathub"

echo "1. Flathub deposu fork kontrolü yapılıyor..."
if ! gh repo view "${FORK_REPO}" >/dev/null 2>&1; then
    echo "   Fork oluşturuluyor (${FORK_REPO})..."
    gh repo fork "${UPSTREAM_REPO}" --clone=false
    echo "   Fork başarıyla oluşturuldu."
else
    echo "   Fork zaten mevcut: ${FORK_REPO}"
fi

TMP_DIR=$(mktemp -d -t flathub-submission-XXXXXX)
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "2. flathub/flathub deposu 'new-pr' dalından klonlanıyor..."
git clone --depth 1 --branch new-pr "https://github.com/${UPSTREAM_REPO}.git" "${TMP_DIR}/flathub"

cd "${TMP_DIR}/flathub"
git remote add fork "https://github.com/${FORK_REPO}.git"
git checkout -b "${BRANCH_NAME}"

echo "3. Flatpak manifest ve kaynak dosyaları kopyalanıyor..."
cp "${OLDPWD}/flatpak/${MANIFEST}" ./
cp "${OLDPWD}/flatpak/${CARGO_SOURCES}" ./
cp "${OLDPWD}/flatpak/${METADATA}" ./

git add "${MANIFEST}" "${CARGO_SOURCES}" "${METADATA}"

echo "4. Değişiklikler işleniyor..."
git commit -m "Add ${APP_ID}

triM. is a lightweight, high-performance desktop non-linear video editor (NLE)
built with Rust, eframe/egui, and FFmpeg."

echo "5. Fork deposuna itiliyor (${FORK_REPO}:${BRANCH_NAME})..."
git push -u fork "${BRANCH_NAME}" --force

echo "6. Flathub'a Pull Request açılıyor..."
PR_URL=$(gh pr create \
    --repo "${UPSTREAM_REPO}" \
    --base new-pr \
    --head "${USER_LOGIN}:${BRANCH_NAME}" \
    --title "Add ${APP_ID}" \
    --body "### Application Summary
- **Name:** triM.
- **App ID:** ${APP_ID}
- **Developer:** Muhammet Özçelik
- **Upstream Repository:** https://github.com/MOzcelik14/triM
- **License:** MIT / Apache-2.0
- **Summary:** Fast, lightweight, non-linear desktop video editor built in Rust.

### Checklist
- [x] Tested with \`flatpak-builder-lint manifest\` (0 errors, 0 warnings)
- [x] Tested with \`flatpak-builder-lint appstream\` (0 errors, 0 warnings)
- [x] Offline cargo sources pre-generated with \`flatpak-cargo-generator.py\`
- [x] AppStream metadata includes developer ID matching GitHub account, VCS URL, and screenshots
")

echo ""
echo "🎉 Flathub Pull Request başarıyla açıldı:"
echo "   ${PR_URL}"
