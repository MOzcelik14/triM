#!/usr/bin/env bash
# ==============================================================================
#  triM. — Modern, Lightweight & Ultra-Fast Native Rust NLE Video Editor
#  Installer & Desktop Integration Script
#
#  Usage:
#    • One-line curl install:
#        curl -fsSL https://raw.githubusercontent.com/MOzcelik14/triM/master/install.sh | bash
#
#    • From cloned repository:
#        ./install.sh
#
#    • Force build from source:
#        ./install.sh --build
#
#    • Uninstall:
#        ./install.sh --uninstall
# ==============================================================================

set -euo pipefail

# ------------------------------------------------------------------------------
# Version & Repository Metadata
# ------------------------------------------------------------------------------
APP_NAME="triM."
APP_ID="io.github.mozcelik14.triM"
VERSION="0.3.0"
REPO_OWNER="MOzcelik14"
REPO_NAME="triM"
GITHUB_REPO="https://github.com/${REPO_OWNER}/${REPO_NAME}"
RAW_BASE_URL="https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/master"
DEFAULT_BIN_DIR="${HOME}/.local/bin"
DEFAULT_DATA_DIR="${HOME}/.local/share"

# ------------------------------------------------------------------------------
# Color & Styling Setup (Precision Amber Theme)
# ------------------------------------------------------------------------------
setup_colors() {
    if [[ -t 1 ]] && [[ -z "${NO_COLOR:-}" ]] && [[ "${TERM:-}" != "dumb" ]]; then
        AMBER="\033[38;2;224;122;56m"
        CYAN="\033[38;2;0;210;210m"
        GREEN="\033[38;2;46;204;113m"
        RED="\033[38;2;231;76;60m"
        YELLOW="\033[38;2;241;196;15m"
        GRAY="\033[38;2;130;130;130m"
        WHITE="\033[38;2;245;245;245m"
        BOLD="\033[1m"
        DIM="\033[2m"
        RESET="\033[0m"
    else
        AMBER=""
        CYAN=""
        GREEN=""
        RED=""
        YELLOW=""
        GRAY=""
        WHITE=""
        BOLD=""
        DIM=""
        RESET=""
    fi
}

setup_colors

# ------------------------------------------------------------------------------
# UI Helpers & Logging
# ------------------------------------------------------------------------------
print_banner() {
    echo -e "${AMBER}${BOLD}"
    echo "    __       _ __  __ "
    echo "   / /______(_)  \/  |   ${WHITE}triM.${RESET} ${GRAY}v${VERSION}${RESET}"
    echo -e "  / __/ __/ / /|_/ /     ${CYAN}Hafif, Modern ve Hızlı Video Düzenleyici${RESET}"
    echo -e "  \__/_/ /_/_/  /_/      ${GRAY}${GITHUB_REPO}${RESET}"
    echo -e "${RESET}"
}

log_step() {
    local step="$1"
    local title="$2"
    echo -e "\n${AMBER}${BOLD}[${step}]${RESET} ${BOLD}${WHITE}${title}${RESET}"
}

log_ok() {
    echo -e "  ${GREEN}✔${RESET} $1"
}

log_info() {
    echo -e "  ${CYAN}➜${RESET} $1"
}

log_warn() {
    echo -e "  ${YELLOW}▲${RESET} ${YELLOW}$1${RESET}"
}

log_err() {
    echo -e "  ${RED}✖${RESET} ${RED}$1${RESET}" >&2
}

# ------------------------------------------------------------------------------
# Command-Line Arguments
# ------------------------------------------------------------------------------
ACTION="install"
FORCE_BUILD=false
TARGET_BIN_DIR="${DEFAULT_BIN_DIR}"
TARGET_DATA_DIR="${DEFAULT_DATA_DIR}"

show_help() {
    print_banner
    echo -e "${BOLD}Kullanım:${RESET} ./install.sh [seçenekler]"
    echo ""
    echo -e "${BOLD}Seçenekler:${RESET}"
    echo -e "  ${AMBER}--build, -b${RESET}         GitHub Releases yerine doğrudan kaynaktan derle"
    echo -e "  ${AMBER}--uninstall, -u${RESET}     triM. ikilisini, masaüstü kısayollarını ve simgelerini kaldır"
    echo -e "  ${AMBER}--bin-dir <dizin>${RESET}   İkili dosya hedef dizini (varsayılan: ~/.local/bin)"
    echo -e "  ${AMBER}--help, -h${RESET}          Bu yardım mesajını göster"
    echo ""
    echo -e "${BOLD}Hızlı Kurulum (curl):${RESET}"
    echo -e "  ${GRAY}curl -fsSL ${RAW_BASE_URL}/install.sh | bash${RESET}"
    echo ""
    exit 0
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --build|-b)
            FORCE_BUILD=true
            shift
            ;;
        --uninstall|-u)
            ACTION="uninstall"
            shift
            ;;
        --bin-dir)
            TARGET_BIN_DIR="$2"
            shift 2
            ;;
        --help|-h)
            show_help
            ;;
        *)
            log_err "Bilinmeyen seçenek: $1"
            echo "Yardım için: ./install.sh --help"
            exit 1
            ;;
    esac
done

# ------------------------------------------------------------------------------
# Action: Uninstall
# ------------------------------------------------------------------------------
do_uninstall() {
    print_banner
    log_step "1/2" "triM. Bileşenleri Kaldırılıyor..."

    local removed_any=false

    # Remove binary
    if [[ -f "${TARGET_BIN_DIR}/trim" ]]; then
        rm -f "${TARGET_BIN_DIR}/trim"
        log_ok "İkili dosya kaldırıldı: ${TARGET_BIN_DIR}/trim"
        removed_any=true
    fi

    # Remove desktop entry
    local desktop_file="${TARGET_DATA_DIR}/applications/${APP_ID}.desktop"
    if [[ -f "$desktop_file" ]]; then
        rm -f "$desktop_file"
        log_ok "Masaüstü kısayolu kaldırıldı: ${desktop_file}"
        removed_any=true
    fi

    # Remove metainfo
    local metainfo_file="${TARGET_DATA_DIR}/metainfo/${APP_ID}.metainfo.xml"
    if [[ -f "$metainfo_file" ]]; then
        rm -f "$metainfo_file"
        log_ok "Metainfo kaldırıldı: ${metainfo_file}"
        removed_any=true
    fi

    # Remove scalable icon
    local svg_icon="${TARGET_DATA_DIR}/icons/hicolor/scalable/apps/${APP_ID}.svg"
    if [[ -f "$svg_icon" ]]; then
        rm -f "$svg_icon"
        log_ok "Vektör simge kaldırıldı: ${svg_icon}"
        removed_any=true
    fi

    # Remove raster icons
    for sz in 16 32 48 64 128 256 512; do
        local png_icon="${TARGET_DATA_DIR}/icons/hicolor/${sz}x${sz}/apps/${APP_ID}.png"
        if [[ -f "$png_icon" ]]; then
            rm -f "$png_icon"
            removed_any=true
        fi
    done
    log_ok "Tüm çözünürlüklerdeki PNG simgeleri temizlendi."

    log_step "2/2" "Masaüstü ve Simge Önbelleği Güncelleniyor..."
    update-desktop-database "${TARGET_DATA_DIR}/applications" 2>/dev/null || true
    gtk-update-icon-cache -f -t "${TARGET_DATA_DIR}/icons/hicolor" 2>/dev/null || true
    log_ok "Önbellekler yenilendi."

    echo ""
    echo -e "  ${GREEN}┌──${RESET} ${BOLD}${WHITE}triM. Sistemden Kaldırıldı${RESET} ${GREEN}─────────────────────────────────────────${RESET}"
    echo -e "  ${GREEN}│${RESET}  ${GRAY}İkili dosya, masaüstü kısayolu ve tüm simgeler temizlendi.${RESET}"
    echo -e "  ${GREEN}└───────────────────────────────────────────────────────────────────────${RESET}"
    echo ""
    exit 0
}

if [[ "$ACTION" == "uninstall" ]]; then
    do_uninstall
fi

# ------------------------------------------------------------------------------
# Action: Install
# ------------------------------------------------------------------------------
print_banner

# Step 1: Detect Platform & Dependencies
log_step "1/4" "Sistem ve Ortam Gereksinimleri Kontrol Ediliyor"

OS_NAME="$(uname -s)"
ARCH_NAME="$(uname -m)"

if [[ "$OS_NAME" != "Linux" ]]; then
    log_err "triM. şu an için yalnızca Linux masaüstü ortamlarını desteklemektedir (${OS_NAME} tespit edildi)."
    exit 1
fi
log_ok "İşletim Sistemi: Linux (${ARCH_NAME})"

# Check FFmpeg / FFprobe
if command -v ffmpeg &>/dev/null && command -v ffprobe &>/dev/null; then
    FF_VER="$(ffmpeg -version 2>/dev/null | head -n 1 | awk '{print $3}')"
    log_ok "FFmpeg ve FFprobe tespit edildi (${GRAY}v${FF_VER}${RESET})"
else
    log_warn "FFmpeg veya FFprobe sisteminizde bulunamadı!"
    echo -e "     ${GRAY}triM. video çözümleme, dalga formu çıkarma ve dışa aktarma için FFmpeg kullanır.${RESET}"
    echo -e "     ${GRAY}Kurulumu tamamladıktan sonra lütfen paket yöneticinizden FFmpeg'i kurun:${RESET}"
    echo -e "     ${CYAN}• Ubuntu / Debian / Mint:${RESET} sudo apt install ffmpeg"
    echo -e "     ${CYAN}• Fedora / RHEL:${RESET}          sudo dnf install ffmpeg-free"
    echo -e "     ${CYAN}• Arch Linux / Manjaro:${RESET}   sudo pacman -S ffmpeg"
    echo -e "     ${CYAN}• openSUSE:${RESET}               sudo zypper install ffmpeg"
fi

# Determine whether we are running inside the triM repository or standalone (curl)
IN_REPO=false
REPO_ROOT=""

if [[ -f "Cargo.toml" ]] && grep -q "trim-app" "Cargo.toml" 2>/dev/null; then
    IN_REPO=true
    REPO_ROOT="$(pwd)"
elif [[ -n "${BASH_SOURCE[0]:-}" ]] && [[ -f "$(dirname "${BASH_SOURCE[0]}")/Cargo.toml" ]]; then
    IN_REPO=true
    REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fi

# Step 2: Acquire Binary (Release Asset or Cargo Build)
log_step "2/4" "triM. İkili Dosyası Hazırlanıyor"

TEMP_DIR="$(mktemp -d /tmp/trim-install.XXXXXX)"
cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

BUILT_BINARY=""

if [[ "$IN_REPO" == true ]] && [[ "$FORCE_BUILD" == false ]] && [[ -x "${REPO_ROOT}/target/release/trim" ]]; then
    log_ok "Mevcut yerel derleme bulundu: ${REPO_ROOT}/target/release/trim"
    BUILT_BINARY="${REPO_ROOT}/target/release/trim"
elif [[ "$IN_REPO" == true ]]; then
    log_info "Rust ile release profilinde derleniyor (cargo build --release -p trim-app)..."
    if ! command -v cargo &>/dev/null; then
        log_err "'cargo' komutu bulunamadı. Lütfen Rust araç takımını kurun (https://rustup.rs)."
        exit 1
    fi
    (cd "$REPO_ROOT" && cargo build --release -p trim-app)
    BUILT_BINARY="${REPO_ROOT}/target/release/trim"
    log_ok "Derleme tamamlandı: ${BUILT_BINARY}"
else
    # Standalone mode via curl | bash
    FETCHED=false

    # Try downloading AppImage or release binary if not forced to build
    if [[ "$FORCE_BUILD" == false ]] && [[ "$ARCH_NAME" == "x86_64" ]]; then
        log_info "GitHub Releases üzerinden son kararlı sürüm kontrol ediliyor..."
        LATEST_TAG="$(curl -sSL -H "Accept: application/vnd.github.v3+json" "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest" 2>/dev/null | grep '"tag_name":' | head -n 1 | sed -E 's/.*"([^"]+)".*/\1/' || true)"

        if [[ -n "$LATEST_TAG" ]]; then
            APPIMAGE_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${LATEST_TAG}/triM-${LATEST_TAG#v}-x86_64.AppImage"
            log_info "İndiriliyor: ${APPIMAGE_URL}"
            if curl -fL --progress-bar "$APPIMAGE_URL" -o "${TEMP_DIR}/trim"; then
                chmod +x "${TEMP_DIR}/trim"
                BUILT_BINARY="${TEMP_DIR}/trim"
                FETCHED=true
                log_ok "Kararlı sürüm (${LATEST_TAG}) başarıyla indirildi."
            fi
        fi
    fi

    # Fallback to cloning & compiling if prebuilt not available or requested
    if [[ "$FETCHED" == false ]]; then
        if ! command -v cargo &>/dev/null; then
            log_err "Rust derleyicisi (cargo) bulunamadı ve hazır ikili paket temin edilemedi."
            echo -e "\n  ${BOLD}Çözüm:${RESET} Lütfen Rust araç takımını kurduktan sonra tekrar deneyin:"
            echo -e "  ${CYAN}curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${RESET}\n"
            exit 1
        fi

        log_info "Kaynak kodu GitHub'dan çekiliyor (${GITHUB_REPO})..."
        git clone --depth 1 "$GITHUB_REPO" "${TEMP_DIR}/source"
        log_info "triM. yerel sisteminiz için optimize derleniyor..."
        (cd "${TEMP_DIR}/source" && cargo build --release -p trim-app)
        BUILT_BINARY="${TEMP_DIR}/source/target/release/trim"
        REPO_ROOT="${TEMP_DIR}/source"
        IN_REPO=true
        log_ok "Yerel derleme tamamlandı!"
    fi
fi

# Step 3: Desktop & Filesystem Integration
log_step "3/4" "Masaüstü Entegrasyonu ve Simgeler Kuruluyor"

mkdir -p "${TARGET_BIN_DIR}"
mkdir -p "${TARGET_DATA_DIR}/applications"
mkdir -p "${TARGET_DATA_DIR}/metainfo"
mkdir -p "${TARGET_DATA_DIR}/icons/hicolor/scalable/apps"

# Install binary
install -Dm755 "$BUILT_BINARY" "${TARGET_BIN_DIR}/trim"
log_ok "İkili dosya kuruldu: ${TARGET_BIN_DIR}/trim"

# Helper to copy from local repo or fetch from GitHub raw
fetch_or_copy() {
    local rel_path="$1"
    local dest_path="$2"

    if [[ "$IN_REPO" == true ]] && [[ -f "${REPO_ROOT}/${rel_path}" ]]; then
        install -Dm644 "${REPO_ROOT}/${rel_path}" "$dest_path"
    else
        mkdir -p "$(dirname "$dest_path")"
        curl -fsSL "${RAW_BASE_URL}/${rel_path}" -o "$dest_path"
    fi
}

# Install SVG icon
fetch_or_copy "io.github.mozcelik14.triM.svg" "${TARGET_DATA_DIR}/icons/hicolor/scalable/apps/${APP_ID}.svg"
log_ok "Vektörel SVG simge kuruldu"

# Install Metainfo XML
fetch_or_copy "io.github.mozcelik14.triM.metainfo.xml" "${TARGET_DATA_DIR}/metainfo/${APP_ID}.metainfo.xml"

# Install PNG icons for standard desktop sizes
for sz in 16 32 48 64 128 256 512; do
    target_dir="${TARGET_DATA_DIR}/icons/hicolor/${sz}x${sz}/apps"
    mkdir -p "$target_dir"
    fetch_or_copy "resources/icons/trim_${sz}x${sz}.png" "${target_dir}/${APP_ID}.png"
done
log_ok "Yüksek çözünürlüklü simgeler (16px - 512px) yerleştirildi"

# Generate Desktop Entry
cat << EOF > "${TARGET_DATA_DIR}/applications/${APP_ID}.desktop"
[Desktop Entry]
Name=triM.
GenericName=Video Editor
GenericName[tr]=Video Düzenleyici
Comment=A lightweight non-linear video editor
Comment[tr]=Hafif ve modern lineer olmayan video düzenleyici
Exec=${TARGET_BIN_DIR}/trim %F
Icon=${APP_ID}
Terminal=false
Type=Application
Categories=AudioVideo;Video;AudioVideoEditing;
MimeType=application/x-trim-project;application/x-cutline-project;
Keywords=video;editor;cut;trim;timeline;ffmpeg;nle;
StartupNotify=true
StartupWMClass=triM.
EOF
log_ok "Masaüstü kısayolu oluşturuldu: ${TARGET_DATA_DIR}/applications/${APP_ID}.desktop"

# Step 4: Verification & Cache Refresh
log_step "4/4" "Önbellekler Yenileniyor ve Kurulum Doğrulanıyor"

update-desktop-database "${TARGET_DATA_DIR}/applications" 2>/dev/null || true
gtk-update-icon-cache -f -t "${TARGET_DATA_DIR}/icons/hicolor" 2>/dev/null || true
log_ok "Masaüstü veri tabanı ve simge önbelleği tazelendi."

# Check PATH
PATH_NOTICE=""
if ! echo ":$PATH:" | grep -q ":${TARGET_BIN_DIR}:"; then
    PATH_NOTICE="${TARGET_BIN_DIR}"
fi

# Print Final Success Card
echo ""
echo -e "  ${GREEN}┌──${RESET} ${BOLD}${WHITE}triM. v${VERSION} Kurulumu Başarıyla Tamamlandı!${RESET} ${GREEN}──────────────────────────${RESET}"
echo -e "  ${GREEN}│${RESET}"
echo -e "  ${GREEN}│${RESET}  ${CYAN}•${RESET} ${BOLD}Uygulama İkilisi:${RESET}  ${WHITE}${TARGET_BIN_DIR}/trim${RESET}"
echo -e "  ${GREEN}│${RESET}  ${CYAN}•${RESET} ${BOLD}Masaüstü Menüsü:${RESET}   Uygulamalar > Ses ve Video > ${BOLD}${WHITE}triM.${RESET}"
echo -e "  ${GREEN}│${RESET}  ${CYAN}•${RESET} ${BOLD}Çalıştırmak İçin:${RESET}  ${AMBER}${BOLD}trim${RESET} ${GRAY}veya${RESET} ${AMBER}${BOLD}trim /path/to/video.mp4${RESET}"
echo -e "  ${GREEN}│${RESET}"
echo -e "  ${GREEN}│${RESET}  ${GRAY}Kaldırmak için: curl -fsSL ${RAW_BASE_URL}/install.sh | bash -s -- -u${RESET}"
echo -e "  ${GREEN}└────────────────────────────────────────────────────────────────────────${RESET}"

if [[ -n "$PATH_NOTICE" ]]; then
    echo ""
    echo -e "  ${YELLOW}┌──${RESET} ${BOLD}${YELLOW}BİLGİ: PATH Yapılandırması Gerekli${RESET} ${YELLOW}─────────────────────────────────${RESET}"
    echo -e "  ${YELLOW}│${RESET}"
    echo -e "  ${YELLOW}│${RESET}  ${WHITE}${PATH_NOTICE}${RESET} dizini PATH ortam değişkeninizde bulunmuyor."
    echo -e "  ${YELLOW}│${RESET}  Uygulamayı doğrudan 'trim' yazarak açmak için kabuk ayarınıza ekleyin:"
    echo -e "  ${YELLOW}│${RESET}"
    echo -e "  ${YELLOW}│${RESET}    ${WHITE}export PATH=\"${PATH_NOTICE}:\$PATH\"${RESET}"
    echo -e "  ${YELLOW}│${RESET}"
    echo -e "  ${YELLOW}│${RESET}  ${GRAY}(~/.bashrc veya ~/.zshrc dosyanıza ekleyip kabuğunuzu yenileyin)${RESET}"
    echo -e "  ${YELLOW}└────────────────────────────────────────────────────────────────────────${RESET}"
fi
echo ""

