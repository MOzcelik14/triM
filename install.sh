#!/usr/bin/env bash
# ==============================================================================
#  triM. — Modern, Lightweight & Ultra-Fast Native Rust NLE Video Editor
#  Interactive Installer & Desktop Integration (Puresteel OS Style)
#
#  Usage:
#    • Interactive One-Line (Terminal & Piped):
#        curl -fsSL https://raw.githubusercontent.com/MOzcelik14/triM/master/install.sh | bash
#
#    • Local repository:
#        ./install.sh
#
#    • Non-interactive / Batch mode:
#        ./install.sh --yes
#
#    • Advanced flags:
#        ./install.sh --build       # Force compile from source
#        ./install.sh --uninstall   # Remove triM. and desktop shortcuts
#        ./install.sh --bin-dir DIR # Custom install directory
# ==============================================================================

set -euo pipefail

# ------------------------------------------------------------------------------
# Version & Repository Metadata
# ------------------------------------------------------------------------------
APP_NAME="triM."
APP_ID="io.github.mozcelik14.triM"
VERSION="1.2.0"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-}")" 2>/dev/null && pwd || true)"
if [[ -n "${SCRIPT_DIR}" && -f "${SCRIPT_DIR}/Cargo.toml" ]]; then
    DETECTED_VER="$(grep -m1 '^version = ' "${SCRIPT_DIR}/Cargo.toml" | cut -d '"' -f2 || true)"
    if [[ -n "${DETECTED_VER}" ]]; then
        VERSION="${DETECTED_VER}"
    fi
fi
REPO_OWNER="MOzcelik14"
REPO_NAME="triM"
GITHUB_REPO="https://github.com/${REPO_OWNER}/${REPO_NAME}"
RAW_BASE_URL="https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/master"
DEFAULT_BIN_DIR="${HOME}/.local/bin"
DEFAULT_DATA_DIR="${HOME}/.local/share"

# ------------------------------------------------------------------------------
# Color & Styling Setup (Puresteel / triM. Amber Theme)
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
# Interactive TTY Hook (Handles `curl ... | bash`)
# ------------------------------------------------------------------------------
INTERACTIVE=false
if [[ "${1:-}" =~ ^(-y|--yes|--batch|-h|--help|-u|--uninstall)$ ]]; then
    INTERACTIVE=false
elif [[ -n "${TRIM_MENU_CHOICE:-}" ]]; then
    INTERACTIVE=true
elif [[ -t 0 ]]; then
    INTERACTIVE=true
elif [[ -r /dev/tty ]] && [[ -w /dev/tty ]] && [[ -z "${CI:-}" ]] && [[ "${DEBIAN_FRONTEND:-}" != "noninteractive" ]]; then
    # Connect stdin to the controlling terminal if running through a pipe (e.g. curl ... | bash)
    exec < /dev/tty
    INTERACTIVE=true
fi

# ------------------------------------------------------------------------------
# System & Hardware Probing
# ------------------------------------------------------------------------------
OS_NAME="$(uname -s)"
ARCH_NAME="$(uname -m)"

DISTRO_PRETTY="Linux"
if [[ -f /etc/os-release ]]; then
    DISTRO_PRETTY="$(grep -E '^PRETTY_NAME=' /etc/os-release | cut -d= -f2- | tr -d '"')"
fi

DESKTOP_ENV="${XDG_CURRENT_DESKTOP:-Bilinmiyor}"
SESSION_TYPE="${XDG_SESSION_TYPE:-x11/wayland}"

FFMPEG_STATUS="Eksik"
FFMPEG_VERSION=""
FFMPEG_ENCODERS=""
if command -v ffmpeg &>/dev/null && command -v ffprobe &>/dev/null; then
    FFMPEG_VERSION="$(ffmpeg -version 2>/dev/null | head -n 1 | awk '{print $3}')"
    FFMPEG_STATUS="Hazır (v${FFMPEG_VERSION})"

    ENCS="$(ffmpeg -encoders 2>&1 || true)"
    HW_LIST=()
    echo "$ENCS" | grep -q "h264_vaapi" && HW_LIST+=("VA-API")
    echo "$ENCS" | grep -q "h264_nvenc" && HW_LIST+=("NVENC")
    echo "$ENCS" | grep -q "h264_qsv" && HW_LIST+=("QuickSync")
    if [[ ${#HW_LIST[@]} -gt 0 ]]; then
        FFMPEG_ENCODERS="${HW_LIST[*]}"
    else
        FFMPEG_ENCODERS="CPU (libx264)"
    fi
fi

CARGO_STATUS="Eksik"
if command -v cargo &>/dev/null; then
    CARGO_VER="$(cargo --version 2>/dev/null | awk '{print $2}')"
    CARGO_STATUS="Hazır (v${CARGO_VER})"
fi

EXISTING_INSTALL="Kurulu Değil"
EXISTING_PATH=""
if command -v trim &>/dev/null; then
    EXISTING_PATH="$(command -v trim)"
    EXISTING_INSTALL="Mevcut (${EXISTING_PATH})"
elif [[ -f "${DEFAULT_BIN_DIR}/trim" ]]; then
    EXISTING_PATH="${DEFAULT_BIN_DIR}/trim"
    EXISTING_INSTALL="Mevcut (${EXISTING_PATH})"
fi

# ------------------------------------------------------------------------------
# UI Helper Functions
# ------------------------------------------------------------------------------
clear_screen() {
    if [[ "$INTERACTIVE" == true ]] && [[ -z "${NO_CLEAR:-}" ]]; then
        clear 2>/dev/null || echo -e "\n\n"
    fi
}

print_banner() {
    echo -e "${AMBER}${BOLD}"
    echo "  ╭──────────────────────────────────────────────────────────────────╮"
    echo "  │   __       _ __  __                                              │"
    echo "  │  / /______(_)  \/  |   ${WHITE}triM. Video Editor${AMBER} ${GRAY}v${VERSION}${AMBER}                 │"
    echo "  │ / __/ __/ / /|_/ /     ${CYAN}Hafif, Modern & Ultra Hızlı NLE           ${AMBER}│"
    echo "  │ \__/_/ /_/_/  /_/      ${GRAY}${GITHUB_REPO}${AMBER}        │"
    echo "  ╰──────────────────────────────────────────────────────────────────╯"
    echo -e "${RESET}"
}

log_step() {
    local step="$1"
    local title="$2"
    echo -e "\n${AMBER}${BOLD}┌── [${step}]${RESET} ${BOLD}${WHITE}${title}${RESET}"
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

press_enter_to_continue() {
    if [[ "$INTERACTIVE" == true ]] && [[ -z "${TRIM_MENU_CHOICE:-}" ]] && [[ "${TEST_SINGLE_RUN:-false}" == false ]]; then
        echo ""
        echo -ne "  ${GRAY}Ana menüye dönmek için [Enter] tuşuna basın...${RESET}"
        read -r _
    fi
}

# ------------------------------------------------------------------------------
# Command-Line Arguments Parsing
# ------------------------------------------------------------------------------
ACTION="menu"
FORCE_BUILD=false
TARGET_BIN_DIR="${DEFAULT_BIN_DIR}"
TARGET_DATA_DIR="${DEFAULT_DATA_DIR}"
BATCH_MODE=false

show_help() {
    print_banner
    echo -e "${BOLD}Kullanım:${RESET} ./install.sh [seçenekler]"
    echo ""
    echo -e "${BOLD}Seçenekler:${RESET}"
    echo -e "  ${AMBER}-y, --yes, --batch${RESET}     Menüsüz otomatik kurulum (CI / hızlı kurulum)"
    echo -e "  ${AMBER}-b, --build${RESET}            GitHub sürümü yerine doğrudan kaynaktan derle"
    echo -e "  ${AMBER}-u, --uninstall${RESET}        triM. ikilisini, masaüstü kısayollarını ve simgelerini kaldır"
    echo -e "  ${AMBER}--bin-dir <dizin>${RESET}      İkili dosya hedef dizini (varsayılan: ~/.local/bin)"
    echo -e "  ${AMBER}-h, --help${RESET}             Bu yardım mesajını göster"
    echo ""
    echo -e "${BOLD}Hızlı Doğrudan Kurulum:${RESET}"
    echo -e "  ${CYAN}curl -fsSL ${RAW_BASE_URL}/install.sh | bash${RESET}"
    echo ""
    exit 0
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        -y|--yes|--batch)
            BATCH_MODE=true
            ACTION="install"
            shift
            ;;
        -b|--build)
            FORCE_BUILD=true
            ACTION="install"
            shift
            ;;
        -u|--uninstall)
            ACTION="uninstall"
            shift
            ;;
        --bin-dir)
            TARGET_BIN_DIR="$2"
            shift 2
            ;;
        -h|--help)
            show_help
            ;;
        *)
            log_err "Bilinmeyen seçenek: $1"
            echo "Yardım için: ./install.sh --help"
            exit 1
            ;;
    esac
done

# If not interactive and not batch mode, default to batch install
if [[ "$INTERACTIVE" == false ]] && [[ "$ACTION" == "menu" ]]; then
    BATCH_MODE=true
    ACTION="install"
fi

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
    echo -e "  ${GREEN}╭──────────────────────────────────────────────────────────────────╮${RESET}"
    echo -e "  ${GREEN}│${RESET}  ${BOLD}${WHITE}triM. Sisteminizden Başarıyla Kaldırıldı${RESET}                        ${GREEN}│${RESET}"
    echo -e "  ${GREEN}│${RESET}  ${GRAY}İkili dosya, masaüstü kısayolları ve simgeler temizlendi.${RESET}        ${GREEN}│${RESET}"
    echo -e "  ${GREEN}╰──────────────────────────────────────────────────────────────────╯${RESET}"
    echo ""
}

# ------------------------------------------------------------------------------
# Action: System Diagnostics
# ------------------------------------------------------------------------------
show_diagnostics() {
    clear_screen
    print_banner

    echo -e "  ${CYAN}${BOLD}╭─ 🔍 SİSTEM VE DONANIM TEŞHİS RAPORU ─────────────────────────────╮${RESET}"
    echo -e "  ${CYAN}│${RESET}                                                                  ${CYAN}│${RESET}"
    echo -e "  ${CYAN}│${RESET}  ${BOLD}İşletim Sistemi:${RESET}    ${WHITE}${DISTRO_PRETTY}${RESET} (${ARCH_NAME})"
    echo -e "  ${CYAN}│${RESET}  ${BOLD}Çekirdek (Kernel):${RESET}  $(uname -r)"
    echo -e "  ${CYAN}│${RESET}  ${BOLD}Masaüstü Ortamı:${RESET}   ${DESKTOP_ENV} (${SESSION_TYPE})"
    echo -e "  ${CYAN}│${RESET}"
    echo -e "  ${CYAN}│${RESET}  ${BOLD}FFmpeg Durumu:${RESET}      ${FFMPEG_STATUS}"
    echo -e "  ${CYAN}│${RESET}  ${BOLD}GPU Hızlandırıcı:${RESET}   ${GREEN}${FFMPEG_ENCODERS}${RESET}"
    echo -e "  ${CYAN}│${RESET}  ${BOLD}Rust / Cargo:${RESET}       ${CARGO_STATUS}"
    echo -e "  ${CYAN}│${RESET}"
    echo -e "  ${CYAN}│${RESET}  ${BOLD}Hedef Dizin:${RESET}        ${WHITE}${TARGET_BIN_DIR}${RESET}"
    if [[ -w "${HOME}/.local" ]] || [[ -w "${TARGET_BIN_DIR}" ]]; then
        echo -e "  ${CYAN}│${RESET}  ${BOLD}Yazma İzni:${RESET}         ${GREEN}Uygun (Kullanıcı Düzeyi / Root Gerekmez)${RESET}"
    else
        echo -e "  ${CYAN}│${RESET}  ${BOLD}Yazma İzni:${RESET}         ${YELLOW}Dikkat: Yazma izni kontrol edilmeli${RESET}"
    fi

    if echo ":$PATH:" | grep -q ":${TARGET_BIN_DIR}:"; then
        echo -e "  ${CYAN}│${RESET}  ${BOLD}PATH Durumu:${RESET}        ${GREEN}Hazır (~/.local/bin sistem PATH'inizde)${RESET}"
    else
        echo -e "  ${CYAN}│${RESET}  ${BOLD}PATH Durumu:${RESET}        ${YELLOW}PATH eklenmesi gerekecek (export PATH=...)${RESET}"
    fi
    echo -e "  ${CYAN}│${RESET}                                                                  ${CYAN}│${RESET}"
    echo -e "  ${CYAN}╰──────────────────────────────────────────────────────────────────╯${RESET}"

    press_enter_to_continue
}

# ------------------------------------------------------------------------------
# Action: About & Architecture Details
# ------------------------------------------------------------------------------
show_about() {
    clear_screen
    print_banner

    echo -e "  ${AMBER}${BOLD}╭─ 📖 triM. NEDİR VE SİSTEMİNİZE NE KURULACAK? ───────────────────╮${RESET}"
    echo -e "  ${AMBER}│${RESET}                                                                  ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}  ${BOLD}triM.${RESET}, Pure Rust ile geliştirilmiş, ultra hafif ve modern bir      ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}  lineer olmayan video düzenleyicidir (NLE).                         ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}                                                                  ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}  ${CYAN}•${RESET} ${BOLD}Önemli Özellikler:${RESET}                                              ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}    ⚡ 140+ FPS akıcı önizleme motoru (Zero-drop rawvideo pipe)     ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}    🎬 Akıllı Sahne Algılama ve Otomatik Bölme (Smart Cut)          ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}    🎙️ Sessizlikleri Otomatik Kaldırma (Jump-Cut generator)         ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}    📝 Yerel Whisper ile Otomatik Altyazı Deşifresi (Offline)       ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}    🎨 3 Noktalı Renk Çarkları (Lift, Gamma, Gain) ve Chroma Key    ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}    🔄 CMX 3600 EDL ve Final Cut Pro XML (FCPXML) Dışa Aktarımı     ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}                                                                  ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}  ${CYAN}•${RESET} ${BOLD}Kurulacak Dosyalar (Root / Sudo İstemez):${RESET}                       ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}    ${WHITE}${TARGET_BIN_DIR}/trim${RESET}            ${GRAY}(Çalıştırılabilir ana ikili)${RESET}  ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}    ${WHITE}${TARGET_DATA_DIR}/applications/${APP_ID}.desktop${RESET}  ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}    ${WHITE}${TARGET_DATA_DIR}/icons/hicolor/...${RESET}    ${GRAY}(16px - 512px + SVG)${RESET}        ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}                                                                  ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}  ${GREEN}✔${RESET} ${BOLD}Sistem Güvenliği:${RESET} Sistem dosyalarınız asla değiştirilmez.        ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}  ${GREEN}✔${RESET} ${BOLD}Kolay Kaldırma:${RESET}   './install.sh -u' ile anında silinebilir.       ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}                                                                  ${AMBER}│${RESET}"
    echo -e "  ${AMBER}╰──────────────────────────────────────────────────────────────────╯${RESET}"

    press_enter_to_continue
}

# ------------------------------------------------------------------------------
# Action: Perform Installation
# ------------------------------------------------------------------------------
do_install() {
    print_banner

    # Step 1: Detect Platform & Dependencies
    log_step "1/4" "Sistem ve Ortam Doğrulanıyor"

    if [[ "$OS_NAME" != "Linux" ]]; then
        log_err "triM. şu an için yalnızca Linux masaüstü ortamlarını desteklemektedir (${OS_NAME} tespit edildi)."
        exit 1
    fi
    log_ok "İşletim Sistemi: ${DISTRO_PRETTY} (${ARCH_NAME})"

    # Check FFmpeg
    if command -v ffmpeg &>/dev/null && command -v ffprobe &>/dev/null; then
        log_ok "FFmpeg & FFprobe hazır (v${FFMPEG_VERSION}) [Donanım Hızlandırma: ${GREEN}${FFMPEG_ENCODERS}${RESET}]"
    else
        log_warn "FFmpeg veya FFprobe sisteminizde bulunamadı!"
        echo -e "     ${GRAY}triM. video çözümleme ve dışa aktarma için FFmpeg kullanır.${RESET}"
        echo -e "     ${GRAY}Kurulum tamamlandıktan sonra lütfen paket yöneticinizden kurun:${RESET}"
        echo -e "     ${CYAN}• Ubuntu / Debian / Mint / Puresteel:${RESET} sudo apt install ffmpeg"
        echo -e "     ${CYAN}• Fedora / RHEL:${RESET}                      sudo dnf install ffmpeg-free"
        echo -e "     ${CYAN}• Arch Linux / Manjaro:${RESET}               sudo pacman -S ffmpeg"
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
    log_step "2/4" "triM. İkili Dosyası Temin Ediliyor"

    TEMP_DIR="$(mktemp -d /tmp/trim-install.XXXXXX)"
    cleanup() {
        rm -rf "$TEMP_DIR"
    }
    trap cleanup EXIT

    BUILT_BINARY=""

    if [[ "$IN_REPO" == true ]] && [[ "$FORCE_BUILD" == false ]] && [[ -x "${REPO_ROOT}/target/release/trim" ]]; then
        log_ok "Yerel derleme ikilisi kullanılacak: ${REPO_ROOT}/target/release/trim"
        BUILT_BINARY="${REPO_ROOT}/target/release/trim"
    elif [[ "$IN_REPO" == true ]]; then
        log_info "Rust ile release profilinde derleniyor (cargo build --release -p trim-app)..."
        if ! command -v cargo &>/dev/null; then
            log_err "'cargo' komutu bulunamadı. Lütfen Rust araç takımını kurun (https://rustup.rs)."
            exit 1
        fi
        (cd "$REPO_ROOT" && cargo build --release -p trim-app)
        BUILT_BINARY="${REPO_ROOT}/target/release/trim"
        log_ok "Derleme başarıyla tamamlandı: ${BUILT_BINARY}"
    else
        # Standalone mode via curl | bash
        FETCHED=false

        # Try downloading official release asset first if not forced to compile
        if [[ "$FORCE_BUILD" == false ]] && [[ "$ARCH_NAME" == "x86_64" ]]; then
            log_info "GitHub Releases üzerinden v${VERSION} sürümü sorgulanıyor..."
            LATEST_TAG="$(curl -sSL -H "Accept: application/vnd.github.v3+json" "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest" 2>/dev/null | grep '"tag_name":' | head -n 1 | sed -E 's/.*"([^"]+)".*/\1/' || true)"

            if [[ -n "$LATEST_TAG" ]]; then
                APPIMAGE_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${LATEST_TAG}/triM-${LATEST_TAG#v}-x86_64.AppImage"
                log_info "Resmi AppImage indiriliyor: ${APPIMAGE_URL}"
                if curl -fL --progress-bar "$APPIMAGE_URL" -o "${TEMP_DIR}/triM.AppImage"; then
                    chmod +x "${TEMP_DIR}/triM.AppImage"
                    # Extract the pure ELF binary so user does not need FUSE runtime installed
                    log_info "FUSE bağımlılığını kaldırmak için ikili dosya çıkartılıyor..."
                    if (cd "$TEMP_DIR" && ./triM.AppImage --appimage-extract "usr/bin/trim" &>/dev/null) && [[ -f "${TEMP_DIR}/squashfs-root/usr/bin/trim" ]]; then
                        BUILT_BINARY="${TEMP_DIR}/squashfs-root/usr/bin/trim"
                        FETCHED=true
                        log_ok "Yerel bağımsız ikili dosya çıkartıldı (${LATEST_TAG})."
                    else
                        BUILT_BINARY="${TEMP_DIR}/triM.AppImage"
                        FETCHED=true
                        log_ok "Kararlı sürüm AppImage olarak hazırlandı (${LATEST_TAG})."
                    fi
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
            log_info "triM. sisteminiz için optimize derleniyor (bu işlem birkaç dakika sürebilir)..."
            (cd "${TEMP_DIR}/source" && cargo build --release -p trim-app)
            BUILT_BINARY="${TEMP_DIR}/source/target/release/trim"
            REPO_ROOT="${TEMP_DIR}/source"
            IN_REPO=true
            log_ok "Yerel derleme tamamlandı!"
        fi
    fi

    # Step 3: Desktop & Filesystem Integration
    log_step "3/4" "Masaüstü Entegrasyonu ve Simgeler Yerleştiriliyor"

    mkdir -p "${TARGET_BIN_DIR}"
    mkdir -p "${TARGET_DATA_DIR}/applications"
    mkdir -p "${TARGET_DATA_DIR}/metainfo"
    mkdir -p "${TARGET_DATA_DIR}/icons/hicolor/scalable/apps"

    # Install binary
    install -Dm755 "$BUILT_BINARY" "${TARGET_BIN_DIR}/trim"
    log_ok "Uygulama ikilisi kuruldu: ${TARGET_BIN_DIR}/trim"

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
    log_ok "Masaüstü uygulama veritabanı ve simge önbelleği tazelendi."

    # Verify binary execution
    if "${TARGET_BIN_DIR}/trim" --help &>/dev/null; then
        log_ok "İkili dosya çalıştırma testi başarılı."
    fi

    # Check PATH
    PATH_NOTICE=""
    if ! echo ":$PATH:" | grep -q ":${TARGET_BIN_DIR}:"; then
        PATH_NOTICE="${TARGET_BIN_DIR}"
    fi

    # Print Final Success Card
    echo ""
    echo -e "  ${GREEN}╭──────────────────────────────────────────────────────────────────╮${RESET}"
    echo -e "  ${GREEN}│${RESET}  ${BOLD}${WHITE}🎉 triM. v${VERSION} Kurulumu Başarıyla Tamamlandı!${RESET}                 ${GREEN}│${RESET}"
    echo -e "  ${GREEN}├──────────────────────────────────────────────────────────────────┤${RESET}"
    echo -e "  ${GREEN}│${RESET}                                                                  ${GREEN}│${RESET}"
    echo -e "  ${GREEN}│${RESET}  ${CYAN}•${RESET} ${BOLD}Uygulama İkilisi:${RESET}  ${WHITE}${TARGET_BIN_DIR}/trim${RESET}"
    echo -e "  ${GREEN}│${RESET}  ${CYAN}•${RESET} ${BOLD}Masaüstü Menüsü:${RESET}   Uygulamalar > Ses ve Video > ${BOLD}${WHITE}triM.${RESET}"
    echo -e "  ${GREEN}│${RESET}  ${CYAN}•${RESET} ${BOLD}Terminalden:${RESET}       ${AMBER}${BOLD}trim${RESET} ${GRAY}veya${RESET} ${AMBER}${BOLD}trim video.mp4${RESET}"
    echo -e "  ${GREEN}│${RESET}"
    echo -e "  ${GREEN}│${RESET}  ${GRAY}Kaldırmak için: curl -fsSL ${RAW_BASE_URL}/install.sh | bash -s -- -u${RESET}"
    echo -e "  ${GREEN}╰──────────────────────────────────────────────────────────────────╯${RESET}"

    if [[ -n "$PATH_NOTICE" ]]; then
        echo ""
        echo -e "  ${YELLOW}╭─ ℹ️  PATH YAPILANDIRMASI GEREKLİ ────────────────────────────────╮${RESET}"
        echo -e "  ${YELLOW}│${RESET}"
        echo -e "  ${YELLOW}│${RESET}  ${WHITE}${PATH_NOTICE}${RESET} dizini PATH ortam değişkeninizde yer almıyor."
        echo -e "  ${YELLOW}│${RESET}  Terminalden doğrudan 'trim' yazarak başlatmak için kabuğunuza ekleyin:"
        echo -e "  ${YELLOW}│${RESET}"
        echo -e "  ${YELLOW}│${RESET}    ${WHITE}export PATH=\"${PATH_NOTICE}:\$PATH\"${RESET}"
        echo -e "  ${YELLOW}│${RESET}"
        echo -e "  ${YELLOW}│${RESET}  ${GRAY}(~/.bashrc veya ~/.zshrc dosyanıza ekleyip kabuğu yeniden açın)${RESET}"
        echo -e "  ${YELLOW}╰──────────────────────────────────────────────────────────────────╯${RESET}"
    fi
    echo ""
}

# ------------------------------------------------------------------------------
# Action: Advanced Installation Flow
# ------------------------------------------------------------------------------
do_advanced_menu() {
    clear_screen
    print_banner

    echo -e "  ${AMBER}${BOLD}╭─ ⚙️  GELİŞMİŞ KURULUM SEÇENEKLERİ ────────────────────────────────╮${RESET}"
    echo -e "  ${AMBER}│${RESET}                                                                  ${AMBER}│${RESET}"
    echo -e "  ${AMBER}│${RESET}  Buradan kurulum konumunu ve derleme tercihlerini değiştirebilirsiniz."
    echo -e "  ${AMBER}│${RESET}                                                                  ${AMBER}│${RESET}"
    echo -e "  ${AMBER}╰──────────────────────────────────────────────────────────────────╯${RESET}"
    echo ""

    # 1. Custom Bin Directory
    echo -e "  ${BOLD}1. İkili Dosya Kurulum Dizini:${RESET}"
    echo -e "     Varsayılan: ${CYAN}${DEFAULT_BIN_DIR}${RESET}"
    echo -ne "     Yeni dizin girin (boş bırakılırsa varsayılan kalır): "
    read -r CUSTOM_DIR
    if [[ -n "$CUSTOM_DIR" ]]; then
        # Expand tilde if present
        CUSTOM_DIR="${CUSTOM_DIR/#\~/$HOME}"
        TARGET_BIN_DIR="$CUSTOM_DIR"
    fi
    echo -e "     ${GREEN}✔${RESET} Seçilen dizin: ${WHITE}${TARGET_BIN_DIR}${RESET}\n"

    # 2. Build from source or download
    echo -e "  ${BOLD}2. Yükleme Yöntemi:${RESET}"
    echo -e "     ${WHITE}[1]${RESET} Resmi kararlı paketi indir (Hızlı - Önerilen)"
    echo -e "     ${WHITE}[2]${RESET} Kaynak koddan derle (Cargo gerektirir - Yerel optimizasyon)"
    echo -ne "     Seçiminiz [1]: "
    read -r BUILD_CHOICE
    BUILD_CHOICE="${BUILD_CHOICE:-1}"

    if [[ "$BUILD_CHOICE" == "2" ]]; then
        FORCE_BUILD=true
        echo -e "     ${GREEN}✔${RESET} Kaynaktan derleme seçildi.\n"
    else
        FORCE_BUILD=false
        echo -e "     ${GREEN}✔${RESET} Hazır paket indirme seçildi.\n"
    fi

    # Confirmation
    echo -e "  ${AMBER}╭─ ÖZET ───────────────────────────────────────────────────────────╮${RESET}"
    echo -e "  ${AMBER}│${RESET}  • Hedef Dizin : ${WHITE}${TARGET_BIN_DIR}${RESET}"
    echo -e "  ${AMBER}│${RESET}  • Yöntem      : $([[ "$FORCE_BUILD" == true ]] && echo "Kaynaktan Derleme (Cargo)" || echo "Resmi Paket İndirme")"
    echo -e "  ${AMBER}╰──────────────────────────────────────────────────────────────────╯${RESET}"
    echo ""
    echo -ne "  ${BOLD}Kuruluma başlansın mı? [E/h]: ${RESET}"
    read -r CONFIRM
    CONFIRM="${CONFIRM:-E}"
    if [[ "$CONFIRM" =~ ^[EeYy]$ ]]; then
        do_install
    else
        log_info "Kurulum iptal edildi."
        press_enter_to_continue
    fi
}

# ------------------------------------------------------------------------------
# Interactive Main Menu Loop
# ------------------------------------------------------------------------------
show_main_menu() {
    while true; do
        clear_screen
        print_banner

        # Quick System Bar
        echo -e "  ${CYAN}${BOLD}╭─ 💻 SİSTEM VE ORTAM ─────────────────────────────────────────────╮${RESET}"
        echo -e "  ${CYAN}│${RESET}  • Dağıtım    : ${WHITE}${DISTRO_PRETTY}${RESET} (${ARCH_NAME})"
        echo -e "  ${CYAN}│${RESET}  • Medya      : ${WHITE}FFmpeg ${FFMPEG_STATUS}${RESET}"
        echo -e "  ${CYAN}│${RESET}  • GPU Hızl.  : ${GREEN}${FFMPEG_ENCODERS}${RESET}"
        echo -e "  ${CYAN}│${RESET}  • Durum      : ${AMBER}${EXISTING_INSTALL}${RESET}"
        echo -e "  ${CYAN}╰──────────────────────────────────────────────────────────────────╯${RESET}"

        # Interactive Menu
        echo ""
        echo -e "  ${AMBER}${BOLD}╭─ 🎯 LÜTFEN BİR İŞLEM SEÇİN ──────────────────────────────────────╮${RESET}"
        echo -e "  ${AMBER}│${RESET}                                                                  ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}  ${BOLD}[1] 🚀 triM.'i Kur (Standart & Önerilen Kurulum)${RESET}                ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}      ${GRAY}↳ Hızlı kurulum, masaüstü simgeleri ve tam entegrasyon${RESET}      ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}                                                                  ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}  ${BOLD}[2] ⚙️  Gelişmiş Kurulum Seçenekleri${RESET}                            ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}      ${GRAY}↳ Kurulum dizinini değiştir, kaynaktan derle vb.${RESET}            ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}                                                                  ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}  ${BOLD}[3] 🔍 Sistem & Donanım Teşhisi Yap (Diagnostics)${RESET}              ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}      ${GRAY}↳ Codec'ler, hızlandırıcılar ve ortam detaylarını incele${RESET}    ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}                                                                  ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}  ${BOLD}[4] 📖 Kurulum Hakkında & Ne Değişecek? (Detaylı Bilgi)${RESET}         ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}      ${GRAY}↳ Kurulacak dosyalar, dizin yapısı ve sistem etkisi${RESET}         ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}                                                                  ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}  ${BOLD}[5] 🗑️  triM.'i Sistemden Kaldır (Uninstall)${RESET}                     ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}      ${GRAY}↳ İkili dosyayı, simgeleri ve masaüstü kaydını temizle${RESET}      ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}                                                                  ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}  ${BOLD}[0] 🚪 Kurulumdan Çık${RESET}                                           ${AMBER}│${RESET}"
        echo -e "  ${AMBER}│${RESET}                                                                  ${AMBER}│${RESET}"
        echo -e "  ${AMBER}╰──────────────────────────────────────────────────────────────────╯${RESET}"
        echo ""
        if [[ -n "${TRIM_MENU_CHOICE:-}" ]]; then
            MENU_CHOICE="$TRIM_MENU_CHOICE"
            echo -e "  ${AMBER}${BOLD}👉 Seçiminiz [1]: ${RESET}${WHITE}${MENU_CHOICE}${RESET}"
            unset TRIM_MENU_CHOICE
            TEST_SINGLE_RUN=true
        else
            echo -ne "  ${AMBER}${BOLD}👉 Seçiminiz [1]: ${RESET}"
            read -r MENU_CHOICE
            MENU_CHOICE="${MENU_CHOICE:-1}"
        fi

        case "$MENU_CHOICE" in
            1)
                clear_screen
                do_install
                break
                ;;
            2)
                do_advanced_menu
                ;;
            3)
                show_diagnostics
                ;;
            4)
                show_about
                ;;
            5)
                echo ""
                echo -ne "  ${RED}${BOLD}triM. sisteminizden kaldırılsın mı? [e/H]: ${RESET}"
                read -r UN_CONFIRM
                UN_CONFIRM="${UN_CONFIRM:-H}"
                if [[ "$UN_CONFIRM" =~ ^[EeYy]$ ]]; then
                    do_uninstall
                else
                    log_info "Kaldırma işlemi iptal edildi."
                    press_enter_to_continue
                fi
                ;;
            0|q|Q)
                echo -e "\n  ${GRAY}Kurulum iptal edildi. Görüşmek üzere!${RESET}\n"
                exit 0
                ;;
            *)
                echo -e "\n  ${RED}Geçersiz seçim. Lütfen [0-5] arasında bir sayı girin.${RESET}"
                sleep 1.2
                ;;
        esac

        if [[ "${TEST_SINGLE_RUN:-false}" == true ]]; then
            break
        fi
    done
}

# ------------------------------------------------------------------------------
# Entry Point
# ------------------------------------------------------------------------------
if [[ "$ACTION" == "uninstall" ]]; then
    do_uninstall
elif [[ "$ACTION" == "install" ]] && [[ "$BATCH_MODE" == true ]]; then
    do_install
else
    show_main_menu
fi
