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
$Version = "1.3.3"
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

    foreach ($bin in @("trim.exe", "ffmpeg.exe", "ffprobe.exe", "ffplay.exe")) {
        $binPath = Join-Path $TargetBinDir $bin
        if (Test-Path $binPath) {
            Remove-Item -Force $binPath -ErrorAction SilentlyContinue
            Log-Ok "İkili dosya kaldırıldı: $bin"
        }
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
    Write-Host "  $GREEN│$RESET  $GRAY`İkili dosya, FFmpeg motoru, Başlat Menüsü ve Masaüstü kısayolları temizlendi.$RESET"
    Write-Host "  $GREEN└───────────────────────────────────────────────────────────────────────$RESET`n"
    exit 0
}

# ------------------------------------------------------------------------------
# Helper: Ensure FFmpeg & FFprobe are Installed
# ------------------------------------------------------------------------------
function Ensure-FFmpeg-Installed {
    param (
        [string]$DestinationDir,
        [string]$ExtractSourceDir = ""
    )

    $DestFfmpeg = Join-Path $DestinationDir "ffmpeg.exe"
    $DestFfprobe = Join-Path $DestinationDir "ffprobe.exe"

    # 1. Check if already present in destination directory
    if ((Test-Path $DestFfmpeg) -and (Test-Path $DestFfprobe)) {
        Log-Ok "FFmpeg ve FFprobe hedef dizinde hazır ($DestinationDir)"
        return
    }

    # 2. Check if present in extracted source / zip directory
    if (![string]::IsNullOrEmpty($ExtractSourceDir) -and (Test-Path $ExtractSourceDir)) {
        $SourceFfmpeg = Get-ChildItem -Path $ExtractSourceDir -Filter "ffmpeg.exe" -Recurse | Select-Object -First 1
        $SourceFfprobe = Get-ChildItem -Path $ExtractSourceDir -Filter "ffprobe.exe" -Recurse | Select-Object -First 1
        if ($SourceFfmpeg -and $SourceFfprobe) {
            Copy-Item -Path $SourceFfmpeg.FullName -Destination $DestFfmpeg -Force
            Copy-Item -Path $SourceFfprobe.FullName -Destination $DestFfprobe -Force
            $SourceFfplay = Get-ChildItem -Path $ExtractSourceDir -Filter "ffplay.exe" -Recurse | Select-Object -First 1
            if ($SourceFfplay) {
                Copy-Item -Path $SourceFfplay.FullName -Destination (Join-Path $DestinationDir "ffplay.exe") -Force
            }
            Log-Ok "FFmpeg ve FFprobe indirilen paketten yerleştirildi ($DestinationDir)"
            return
        }
    }

    # 3. Check if available on system PATH
    $SysFfmpeg = Get-Command ffmpeg -ErrorAction SilentlyContinue
    $SysFfprobe = Get-Command ffprobe -ErrorAction SilentlyContinue
    if ($SysFfmpeg -and $SysFfprobe) {
        try {
            Copy-Item -Path $SysFfmpeg.Source -Destination $DestFfmpeg -Force
            Copy-Item -Path $SysFfprobe.Source -Destination $DestFfprobe -Force
            Log-Ok "Sistemdeki FFmpeg ikilileri triM klasörüne entegre edildi"
            return
        } catch {
            Log-Ok "Sistem FFmpeg ikilisi kullanılacak: $($SysFfmpeg.Source)"
            return
        }
    }

    # 4. If missing, automatically download and install official static FFmpeg
    Log-Info "FFmpeg sistemde bulunamadı. triM. için FFmpeg resmi kararlı sürümü kuruluyor..."

    $Installed = $false

    # Strategy A: winget
    if (Get-Command winget -ErrorAction SilentlyContinue) {
        try {
            Log-Info "winget ile Gyan.FFmpeg kuruluyor..."
            $proc = Start-Process -FilePath "winget" -ArgumentList "install --id Gyan.FFmpeg -e --accept-source-agreements --accept-package-agreements --silent" -Wait -PassThru -NoNewWindow
            if ($proc.ExitCode -eq 0) {
                $SysFfmpeg = Get-Command ffmpeg -ErrorAction SilentlyContinue
                $SysFfprobe = Get-Command ffprobe -ErrorAction SilentlyContinue
                if ($SysFfmpeg -and $SysFfprobe) {
                    Copy-Item -Path $SysFfmpeg.Source -Destination $DestFfmpeg -Force
                    Copy-Item -Path $SysFfprobe.Source -Destination $DestFfprobe -Force
                    Log-Ok "FFmpeg winget ile başarıyla kuruldu ve entegre edildi!"
                    $Installed = $true
                }
            }
        } catch {
            Log-Warn "winget kurulumu atlandı, doğrudan taşınabilir paket indirilmesine geçiliyor."
        }
    }

    # Strategy B: Direct download of Gyan FFmpeg release essentials zip
    if (-not $Installed) {
        try {
            Log-Info "Resmi taşınabilir FFmpeg paketi indiriliyor (Gyan.dev)..."
            $FfTempZip = Join-Path $env:TEMP ("ffmpeg-" + [guid]::NewGuid().ToString().Substring(0, 8) + ".zip")
            $FfTempExtract = Join-Path $env:TEMP ("ffmpeg-extract-" + [guid]::NewGuid().ToString().Substring(0, 8))
            New-Item -ItemType Directory -Path $FfTempExtract -Force | Out-Null

            [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
            Invoke-WebRequest -Uri "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip" -OutFile $FfTempZip -UseBasicParsing

            Log-Info "FFmpeg paketi açılıyor..."
            Expand-Archive -Path $FfTempZip -DestinationPath $FfTempExtract -Force

            $FoundFfmpeg = Get-ChildItem -Path $FfTempExtract -Filter "ffmpeg.exe" -Recurse | Select-Object -First 1
            $FoundFfprobe = Get-ChildItem -Path $FfTempExtract -Filter "ffprobe.exe" -Recurse | Select-Object -First 1
            $FoundFfplay = Get-ChildItem -Path $FfTempExtract -Filter "ffplay.exe" -Recurse | Select-Object -First 1

            if ($FoundFfmpeg -and $FoundFfprobe) {
                Copy-Item -Path $FoundFfmpeg.FullName -Destination $DestFfmpeg -Force
                Copy-Item -Path $FoundFfprobe.FullName -Destination $DestFfprobe -Force
                if ($FoundFfplay) {
                    Copy-Item -Path $FoundFfplay.FullName -Destination (Join-Path $DestinationDir "ffplay.exe") -Force
                }
                Log-Ok "FFmpeg ve FFprobe başarıyla kuruldu: $DestinationDir"
                $Installed = $true
            }

            Remove-Item -Recurse -Force $FfTempZip -ErrorAction SilentlyContinue
            Remove-Item -Recurse -Force $FfTempExtract -ErrorAction SilentlyContinue
        } catch {
            Log-Warn "Gyan.dev üzerinden indirme başarısız oldu, GitHub aynası deneniyor..."
            try {
                $FfTempZip = Join-Path $env:TEMP ("ffmpeg-gh-" + [guid]::NewGuid().ToString().Substring(0, 8) + ".zip")
                $FfTempExtract = Join-Path $env:TEMP ("ffmpeg-gh-ext-" + [guid]::NewGuid().ToString().Substring(0, 8))
                New-Item -ItemType Directory -Path $FfTempExtract -Force | Out-Null

                Invoke-WebRequest -Uri "https://github.com/GyanD/codexffmpeg/releases/download/7.1/ffmpeg-7.1-essentials_build.zip" -OutFile $FfTempZip -UseBasicParsing
                Expand-Archive -Path $FfTempZip -DestinationPath $FfTempExtract -Force

                $FoundFfmpeg = Get-ChildItem -Path $FfTempExtract -Filter "ffmpeg.exe" -Recurse | Select-Object -First 1
                $FoundFfprobe = Get-ChildItem -Path $FfTempExtract -Filter "ffprobe.exe" -Recurse | Select-Object -First 1
                $FoundFfplay = Get-ChildItem -Path $FfTempExtract -Filter "ffplay.exe" -Recurse | Select-Object -First 1

                if ($FoundFfmpeg -and $FoundFfprobe) {
                    Copy-Item -Path $FoundFfmpeg.FullName -Destination $DestFfmpeg -Force
                    Copy-Item -Path $FoundFfprobe.FullName -Destination $DestFfprobe -Force
                    if ($FoundFfplay) {
                        Copy-Item -Path $FoundFfplay.FullName -Destination (Join-Path $DestinationDir "ffplay.exe") -Force
                    }
                    Log-Ok "FFmpeg ve FFprobe GitHub aynasından kuruldu: $DestinationDir"
                    $Installed = $true
                }

                Remove-Item -Recurse -Force $FfTempZip -ErrorAction SilentlyContinue
                Remove-Item -Recurse -Force $FfTempExtract -ErrorAction SilentlyContinue
            } catch {
                Log-Err "FFmpeg otomatik kurulamadı: $_"
            }
        }
    }
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
$DestFfmpeg = Join-Path $TargetBinDir "ffmpeg.exe"
if ($FfmpegCmd) {
    try {
        $FfVersion = (& ffmpeg -version 2>$null | Select-Object -First 1) -replace "ffmpeg version ([^ ]+).*", '$1'
        Log-Ok "FFmpeg sistem ortamında tespit edildi ($GRAY`v$FfVersion$RESET)"
    } catch {
        Log-Ok "FFmpeg tespit edildi"
    }
} elseif (Test-Path $DestFfmpeg) {
    Log-Ok "FFmpeg hedef klasörde mevcut ($DestFfmpeg)"
} else {
    Log-Info "FFmpeg sistemde henüz yok — Kurulum sırasında triM. ile birlikte otomatik yüklenecek."
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

# Step 3: Desktop, Filesystem & FFmpeg Integration
Log-Step "3/4" "Windows Masaüstü Entegrasyonu ve FFmpeg Kuruluyor"

New-Item -ItemType Directory -Path $TargetBinDir -Force | Out-Null
$FinalExe = Join-Path $TargetBinDir "trim.exe"
Copy-Item -Path $BuiltBinary -Destination $FinalExe -Force
Log-Ok "İkili dosya yerleştirildi: $FinalExe"

# Ensure FFmpeg and FFprobe are present alongside trim.exe
Ensure-FFmpeg-Installed -DestinationDir $TargetBinDir -ExtractSourceDir $TempDir

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
