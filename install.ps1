# ==============================================================================
#  triM. — Modern, Lightweight & Ultra-Fast Native Rust NLE Video Editor
#  Windows Installer & Desktop Integration Script (PowerShell)
#
#  Usage:
#    • One-line PowerShell install:
#        iwr -useb https://raw.githubusercontent.com/MOzcelik14/triM/master/install.ps1 | iex
#
#    • From cloned repository:
#        powershell -ExecutionPolicy Bypass -File .\install.ps1
#
#    • Force build from source:
#        powershell -ExecutionPolicy Bypass -File .\install.ps1 -Build
#
#    • Uninstall:
#        powershell -ExecutionPolicy Bypass -File .\install.ps1 -Uninstall
# ==============================================================================

[CmdletBinding()]
param (
    [switch]$Build,
    [switch]$Uninstall,
    [string]$BinDir = "",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

# ------------------------------------------------------------------------------
# Version & Metadata
$AppName = "triM."
$Version = "1.1.2"
if ($PSScriptRoot) {
    $CargoTomlPath = Join-Path $PSScriptRoot "Cargo.toml"
    if (Test-Path $CargoTomlPath) {
        $CargoContent = Get-Content $CargoTomlPath -Raw
        if ($CargoContent -match '(?m)^version\s*=\s*"([^"]+)"') {
            $Version = $Matches[1]
        }
    }
}
$RepoOwner = "MOzcelik14"
$RepoName = "triM"
$GitHubRepo = "https://github.com/$RepoOwner/$RepoName"
$RawBaseUrl = "https://raw.githubusercontent.com/$RepoOwner/$RepoName/master"

if ([string]::IsNullOrEmpty($BinDir)) {
    $InstallDir = Join-Path $env:LOCALAPPDATA "triM"
    $TargetBinDir = Join-Path $InstallDir "bin"
} else {
    $TargetBinDir = $BinDir
    $InstallDir = Split-Path -Parent $TargetBinDir
}

# ------------------------------------------------------------------------------
# Styling & Colors (Precision Amber)
# ------------------------------------------------------------------------------
$ESC = [char]27
$AMBER  = "$ESC[38;2;224;122;56m"
$CYAN   = "$ESC[38;2;0;210;210m"
$GREEN  = "$ESC[38;2;46;204;113m"
$RED    = "$ESC[38;2;231;76;60m"
$YELLOW = "$ESC[38;2;241;196;15m"
$GRAY   = "$ESC[38;2;130;130;130m"
$WHITE  = "$ESC[38;2;245;245;245m"
$BOLD   = "$ESC[1m"
$RESET  = "$ESC[0m"

function Print-Banner {
    Write-Host "$AMBER$BOLD"
    Write-Host "    __       _ __  __ "
    Write-Host "   / /______(_)  \/  |   $WHITE$AppName$RESET $GRAY`v$Version (Windows)$RESET"
    Write-Host "  / __/ __/ / /|_/ /     $CYAN`Hafif, Modern ve Hızlı Video Düzenleyici$RESET"
    Write-Host "  \__/_/ /_/_/  /_/      $GRAY$GitHubRepo$RESET"
    Write-Host "$RESET"
}

function Log-Step($step, $title) {
    Write-Host "`n$AMBER$BOLD[$step]$RESET $BOLD$WHITE$title$RESET"
}

function Log-Ok($msg) {
    Write-Host "  $GREEN✔$RESET $msg"
}

function Log-Info($msg) {
    Write-Host "  $CYAN➜$RESET $msg"
}

function Log-Warn($msg) {
    Write-Host "  $YELLOW▲$RESET $YELLOW$msg$RESET"
}

function Log-Err($msg) {
    Write-Host "  $RED✖$RESET $RED$msg$RESET"
}

# ------------------------------------------------------------------------------
# Help
# ------------------------------------------------------------------------------
if ($Help) {
    Print-Banner
    Write-Host "$BOLD`Kullanım:$RESET powershell -ExecutionPolicy Bypass -File .\install.ps1 [seçenekler]"
    Write-Host ""
    Write-Host "$BOLD`Seçenekler:$RESET"
    Write-Host "  $AMBER-Build$RESET          GitHub Releases yerine doğrudan kaynaktan derle"
    Write-Host "  $AMBER-Uninstall$RESET      triM. uygulamasını, masaüstü kısayollarını ve dosyalarını kaldır"
    Write-Host "  $AMBER-BinDir <dizin>$RESET Özel ikili dosya hedef dizini"
    Write-Host "  $AMBER-Help$RESET           Bu yardım mesajını göster"
    Write-Host ""
    Write-Host "$BOLD`Hızlı Kurulum (PowerShell):$RESET"
    Write-Host "  $GRAY`iwr -useb $RawBaseUrl/install.ps1 | iex$RESET"
    Write-Host ""
    exit 0
}

# ------------------------------------------------------------------------------
# Action: Uninstall
# ------------------------------------------------------------------------------
if ($Uninstall) {
    Print-Banner
    Log-Step "1/2" "triM. Windows Bileşenleri Kaldırılıyor..."

    $TargetExe = Join-Path $TargetBinDir "trim.exe"
    if (Test-Path $TargetExe) {
        Remove-Item -Force $TargetExe -ErrorAction SilentlyContinue
        Log-Ok "İkili dosya kaldırıldı: $TargetExe"
    }

    # Remove Desktop Shortcut
    $DesktopShortcut = Join-Path ([Environment]::GetFolderPath("Desktop")) "triM..lnk"
    if (Test-Path $DesktopShortcut) {
        Remove-Item -Force $DesktopShortcut -ErrorAction SilentlyContinue
        Log-Ok "Masaüstü kısayolu kaldırıldı"
    }

    # Remove Start Menu Shortcut
    $StartMenuDir = Join-Path ([Environment]::GetFolderPath("Programs")) "triM."
    if (Test-Path $StartMenuDir) {
        Remove-Item -Recurse -Force $StartMenuDir -ErrorAction SilentlyContinue
        Log-Ok "Başlat Menüsü kısayolu kaldırıldı"
    }

    Log-Step "2/2" "Temizlik Doğrulanıyor..."
    Log-Ok "Kaldırma işlemi tamamlandı."

    Write-Host ""
    Write-Host "  $GREEN┌──$RESET $BOLD$WHITE`triM. Sistemden Başarıyla Kaldırıldı$RESET $GREEN─────────────────────────$RESET"
    Write-Host "  $GREEN│$RESET  $GRAY`İkili dosya, Başlat Menüsü ve Masaüstü kısayolları temizlendi.$RESET"
    Write-Host "  $GREEN└───────────────────────────────────────────────────────────────────────$RESET`n"
    exit 0
}

# ------------------------------------------------------------------------------
# Action: Install
# ------------------------------------------------------------------------------
Print-Banner

# Step 1: Detect Platform & FFmpeg
Log-Step "1/4" "Sistem ve FFmpeg Ortamı Kontrol Ediliyor"

$Arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
Log-Ok "İşletim Sistemi: Windows ($Arch)"

$FfmpegCmd = Get-Command ffmpeg -ErrorAction SilentlyContinue
if ($FfmpegCmd) {
    try {
        $FfVersion = (& ffmpeg -version 2>$null | Select-Object -First 1) -replace "ffmpeg version ([^ ]+).*", '$1'
        Log-Ok "FFmpeg tespit edildi ($GRAY`v$FfVersion$RESET)"
    } catch {
        Log-Ok "FFmpeg tespit edildi"
    }
} else {
    Log-Warn "FFmpeg sisteminizde (PATH) bulunamadı!"
    Write-Host "     $GRAY`triM. video çözümleme, dalga formu çıkarma ve dışa aktarma için FFmpeg kullanır.$RESET"
    Write-Host "     $GRAY`Kurulum sonrasında tek bir komutla kolayca yükleyebilirsiniz:$RESET"
    Write-Host "     $CYAN`• winget install Gyan.FFmpeg$RESET"
    Write-Host "     $GRAY`veya https://ffmpeg.org/download.html adresinden indirip 'ffmpeg.exe'yi triM klasörüne atabilirsiniz.$RESET"
}

# Determine if in git clone
$InRepo = $false
$RepoRoot = ""
if (Test-Path "Cargo.toml") {
    $cargoContent = Get-Content "Cargo.toml" -Raw -ErrorAction SilentlyContinue
    if ($cargoContent -match "trim-app") {
        $InRepo = $true
        $RepoRoot = (Get-Location).Path
    }
}

# Step 2: Binary Acquisition
Log-Step "2/4" "triM. İkili Dosyası Hazırlanıyor"

$TempDir = Join-Path $env:TEMP ("trim-install-" + [guid]::NewGuid().ToString().Substring(0, 8))
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

$BuiltBinary = ""

if ($InRepo -and -not $Build -and (Test-Path (Join-Path $RepoRoot "target\release\trim.exe"))) {
    $BuiltBinary = Join-Path $RepoRoot "target\release\trim.exe"
    Log-Ok "Mevcut yerel derleme bulundu: $BuiltBinary"
} elseif ($InRepo) {
    Log-Info "Rust ile release profilinde derleniyor (cargo build --release -p trim-app)..."
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Log-Err "'cargo' komutu bulunamadı. Lütfen Rust araç takımını kurun (https://rustup.rs)."
        exit 1
    }
    Push-Location $RepoRoot
    & cargo build --release -p trim-app
    Pop-Location
    $BuiltBinary = Join-Path $RepoRoot "target\release\trim.exe"
    Log-Ok "Yerel derleme tamamlandı: $BuiltBinary"
} else {
    $Fetched = $false
    # Try downloading release ZIP from GitHub if available
    if (-not $Build) {
        try {
            Log-Info "GitHub Releases üzerinden son kararlı sürüm kontrol ediliyor..."
            $LatestReleaseUrl = "https://api.github.com/repos/$RepoOwner/$RepoName/releases/latest"
            $ReleaseJson = Invoke-RestMethod -Uri $LatestReleaseUrl -Headers @{ "User-Agent" = "triM-Installer" } -ErrorAction SilentlyContinue
            
            if ($ReleaseJson -and $ReleaseJson.assets) {
                $ZipAsset = $ReleaseJson.assets | Where-Object { $_.name -like "*windows-x86_64.zip" -or $_.name -like "*win64.zip" } | Select-Object -First 1
                if ($ZipAsset) {
                    $ZipDest = Join-Path $TempDir "trim.zip"
                    Log-Info "İndiriliyor: $($ZipAsset.browser_download_url)..."
                    Invoke-WebRequest -Uri $ZipAsset.browser_download_url -OutFile $ZipDest -UseBasicParsing
                    Expand-Archive -Path $ZipDest -DestinationPath $TempDir -Force
                    $FoundExe = Get-ChildItem -Path $TempDir -Filter "trim.exe" -Recurse | Select-Object -First 1
                    if ($FoundExe) {
                        $BuiltBinary = $FoundExe.FullName
                        $Fetched = $true
                        Log-Ok "Kararlı sürüm ($($ReleaseJson.tag_name)) başarıyla indirildi."
                    }
                }
            }
        } catch {
            Log-Warn "GitHub Releases varlığı indirilemedi, kaynak koddan derlemeye geçiliyor."
        }
    }

    if (-not $Fetched) {
        if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
            Log-Err "Rust derleyicisi (cargo) sisteminizde bulunamadı."
            Write-Host "`n  $BOLD`Çözüm:$RESET Lütfen Rust araç takımını kurduktan sonra tekrar deneyin:"
            Write-Host "  $CYAN`winget install Rustlang.Rustup$RESET veya $CYAN`https://rustup.rs$RESET`n"
            exit 1
        }

        Log-Info "Kaynak kodu GitHub'dan çekiliyor ($GitHubRepo)..."
        $SourceDir = Join-Path $TempDir "source"
        & git clone --depth 1 $GitHubRepo $SourceDir
        Log-Info "triM. yerel sisteminiz için optimize derleniyor..."
        Push-Location $SourceDir
        & cargo build --release -p trim-app
        Pop-Location
        $BuiltBinary = Join-Path $SourceDir "target\release\trim.exe"
        Log-Ok "Yerel derleme tamamlandı!"
    }
}

# Step 3: Desktop & Filesystem Integration
Log-Step "3/4" "Windows Masaüstü Entegrasyonu ve Kısayollar Kuruluyor"

New-Item -ItemType Directory -Path $TargetBinDir -Force | Out-Null
$FinalExe = Join-Path $TargetBinDir "trim.exe"
Copy-Item -Path $BuiltBinary -Destination $FinalExe -Force
Log-Ok "İkili dosya yerleştirildi: $FinalExe"

# Download or copy icon
$IconDir = Join-Path $InstallDir "resources"
New-Item -ItemType Directory -Path $IconDir -Force | Out-Null
$TargetIcon = Join-Path $IconDir "trim.ico"

if ($InRepo -and (Test-Path (Join-Path $RepoRoot "resources\icons\trim.ico"))) {
    Copy-Item -Path (Join-Path $RepoRoot "resources\icons\trim.ico") -Destination $TargetIcon -Force
} else {
    try {
        Invoke-WebRequest -Uri "$RawBaseUrl/resources/icons/trim.ico" -OutFile $TargetIcon -UseBasicParsing
    } catch {}
}
Log-Ok "Yüksek çözünürlüklü Windows simgesi (.ico) yerleştirildi"

# Create Desktop and Start Menu Shortcuts using WScript.Shell
$WshShell = New-Object -ComObject WScript.Shell

# 1. Desktop Shortcut
$DesktopPath = [Environment]::GetFolderPath("Desktop")
$DesktopLnk = Join-Path $DesktopPath "triM..lnk"
$Shortcut = $WshShell.CreateShortcut($DesktopLnk)
$Shortcut.TargetPath = $FinalExe
$Shortcut.WorkingDirectory = [Environment]::GetFolderPath("UserProfile")
$Shortcut.Description = "triM. — Fast, Minimalist Native Rust NLE Video Editor"
if (Test-Path $TargetIcon) {
    $Shortcut.IconLocation = "$TargetIcon, 0"
}
$Shortcut.Save()
Log-Ok "Masaüstü kısayolu oluşturuldu"

# 2. Start Menu Shortcut
$StartMenuPrograms = [Environment]::GetFolderPath("Programs")
$StartMenuAppDir = Join-Path $StartMenuPrograms "triM."
New-Item -ItemType Directory -Path $StartMenuAppDir -Force | Out-Null
$StartMenuLnk = Join-Path $StartMenuAppDir "triM..lnk"
$Shortcut = $WshShell.CreateShortcut($StartMenuLnk)
$Shortcut.TargetPath = $FinalExe
$Shortcut.WorkingDirectory = [Environment]::GetFolderPath("UserProfile")
$Shortcut.Description = "triM. — Fast, Minimalist Native Rust NLE Video Editor"
if (Test-Path $TargetIcon) {
    $Shortcut.IconLocation = "$TargetIcon, 0"
}
$Shortcut.Save()
Log-Ok "Başlat Menüsü program listesine eklendi"

# Step 4: Verification & PATH check
Log-Step "4/4" "Ortam Değişkenleri Doğrulanıyor"

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
$PathNotice = $false
if ($UserPath -notlike "*$TargetBinDir*") {
    $PathNotice = $true
    try {
        [Environment]::SetEnvironmentVariable("Path", "$UserPath;$TargetBinDir", "User")
        $env:Path = "$env:Path;$TargetBinDir"
        Log-Ok "Hedef dizin ($TargetBinDir) Kullanıcı PATH ortam değişkenine otomatik eklendi."
        $PathNotice = $false
    } catch {
        Log-Warn "Hedef dizin PATH değişkenine otomatik eklenemedi."
    }
} else {
    Log-Ok "Hedef dizin ($TargetBinDir) PATH içinde mevcut."
}

# Cleanup
Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue

# Final Success Card
Write-Host ""
Write-Host "  $GREEN┌──$RESET $BOLD$WHITE`triM. v$Version Windows Kurulumu Başarıyla Tamamlandı!$RESET $GREEN──────────────────$RESET"
Write-Host "  $GREEN│$RESET"
Write-Host "  $GREEN│$RESET  $CYAN•$RESET $BOLD`Uygulama İkilisi:$RESET  $WHITE$FinalExe$RESET"
Write-Host "  $GREEN│$RESET  $CYAN•$RESET $BOLD`Başlat Menüsü:$RESET     Başlat Menüsü > $BOLD$WHITE`triM.$RESET"
Write-Host "  $GREEN│$RESET  $CYAN•$RESET $BOLD`Masaüstü Kısayolu:$RESET Masaüstü > $BOLD$WHITE`triM.$RESET"
Write-Host "  $GREEN│$RESET  $CYAN•$RESET $BOLD`Çalıştırmak İçin:$RESET  $AMBER$BOLD`trim$RESET $GRAY`veya$RESET $AMBER$BOLD`trim C:\path\to\video.mp4$RESET"
Write-Host "  $GREEN│$RESET"
Write-Host "  $GREEN│$RESET  $GRAY`Kaldırmak için: powershell -ExecutionPolicy Bypass -File .\install.ps1 -Uninstall$RESET"
Write-Host "  $GREEN└────────────────────────────────────────────────────────────────────────$RESET"

if ($PathNotice) {
    Write-Host ""
    Write-Host "  $YELLOW┌──$RESET $BOLD$YELLOW`BİLGİ: PATH Yapılandırması$RESET $YELLOW─────────────────────────────────$RESET"
    Write-Host "  $YELLOW│$RESET"
    Write-Host "  $YELLOW│$RESET  Uygulamayı herhangi bir komut satırından 'trim' yazarak başlatabilmek için:"
    Write-Host "  $YELLOW│$RESET  [Environment]::SetEnvironmentVariable('Path', `"`$env:Path;$TargetBinDir`", 'User')"
    Write-Host "  $YELLOW└────────────────────────────────────────────────────────────────────────$RESET"
}
Write-Host ""
