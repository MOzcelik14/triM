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
    --body "### Please confirm your submission meets all the criteria

- [x] Please describe the application briefly. triM. is a lightweight, high-performance desktop non-linear video editor (NLE) written in Rust (using egui/eframe) with FFmpeg. It features a multi-track timeline, real-time preview, color grading (wheels/curves), audio mixing/equalization, auto silence cuts, subtitles, and GPU-accelerated export.
- [x] Please attach a video showcasing the application on Linux using the Flatpak. https://raw.githubusercontent.com/MOzcelik14/triM/master/docs/assets/trim_showcase.webm
- [x] The Flatpak ID follows all the rules listed in the [Application ID requirements][appid].
- [x] I have read and followed all the [Submission requirements][reqs] and the [Submission guide][reqs2] and I agree to them.
  - [x] The application has a meaningful development history, evidence of real-world use, and a clear commitment to ongoing maintenance, as required by the [development history requirements][history]. triM. is actively developed across multiple releases (v0.1.0 through v1.0.0) with extensive unit and integration tests across Linux and Windows, documented features, and active maintenance by the author.
  - [x] I have disclosed any AI-generated material included in the application or its Flathub packaging, as required by the [Generative AI policy][ai]. **Affected parts and approximate extent:** Portions of application code, documentation, and packaging files were developed with AI pair-programming assistants under human architectural design and rigorous review. The video editor design, workflow logic, tests, and maintenance commitment are directed by the author.
  - [x] I have not used AI tools or agents to generate or automate this submission pull request or its review interactions.
- [x] I am an author/developer/upstream contributor to the project. Link: https://github.com/MOzcelik14 (Lead author of https://github.com/MOzcelik14/triM)

[appid]: https://docs.flathub.org/docs/for-app-authors/requirements#application-id
[reqs]: https://docs.flathub.org/docs/for-app-authors/requirements
[reqs2]: https://docs.flathub.org/docs/for-app-authors/submission
[history]: https://docs.flathub.org/docs/for-app-authors/requirements#insufficient-development-history
[ai]: https://docs.flathub.org/docs/for-app-authors/requirements#generative-ai-policy")

echo ""
echo "🎉 Flathub Pull Request başarıyla açıldı:"
echo "   ${PR_URL}"
