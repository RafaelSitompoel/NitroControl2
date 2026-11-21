# ================================================================
#  Deploy-NitroControl.ps1
#  Double-click script ini — otomatis copy file + push ke GitHub
#  Tidak perlu setting apapun, cukup Git sudah terinstall
# ================================================================

$REPO = "C:\Users\ACER NITRO\Documents\NitroControl2"
$SCRIP = Split-Path -Parent $MyInvocation.MyCommand.Path  # folder script ini

Write-Host ""
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  NitroControl — Auto Deploy ke GitHub" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# ── Cek repo ada ─────────────────────────────────────────────────
if (-not (Test-Path "$REPO\.git")) {
    Write-Host "ERROR: Folder repo tidak ditemukan di:" -ForegroundColor Red
    Write-Host "  $REPO" -ForegroundColor Yellow
    Write-Host "Pastikan path sudah benar lalu jalankan lagi." -ForegroundColor Red
    pause; exit 1
}

# ── Copy file ke repo ─────────────────────────────────────────────
Write-Host ">> Menyalin file ke repo..." -ForegroundColor Green

# Root files
Copy-Item "$SCRIP\build.yml"          "$REPO\.github\workflows\build.yml"   -Force
Copy-Item "$SCRIP\NitroControlApp.cs" "$REPO\NitroControlApp.cs"            -Force
Copy-Item "$SCRIP\NitroControl.csproj" "$REPO\NitroControl.csproj"          -Force
Copy-Item "$SCRIP\NitroControl.ico"   "$REPO\NitroControl.ico"              -Force
Copy-Item "$SCRIP\LICENSE.txt"        "$REPO\LICENSE.txt"                   -Force
Copy-Item "$SCRIP\installer.nsi"      "$REPO\installer.nsi"                 -Force

# Rust source fixes
$hwPath = "$REPO\crates\nitrocontrol-core\src\hardware"
New-Item -ItemType Directory -Force -Path "$hwPath\acer" | Out-Null
Copy-Item "$SCRIP\wmi.rs"        "$hwPath\acer\wmi.rs"   -Force
Copy-Item "$SCRIP\discovery.rs"  "$hwPath\discovery.rs"  -Force

Write-Host "   Selesai copy file." -ForegroundColor Green

# ── Git commit + push ────────────────────────────────────────────
Write-Host ""
Write-Host ">> Commit dan push ke GitHub..." -ForegroundColor Green

Set-Location $REPO

git add .
git commit -m "Add installer, uninstaller, desktop EXE with icon, fix Nitro V15 fan control"
git push origin main

# ── Tag versi (trigger GitHub Actions release) ───────────────────
Write-Host ""
Write-Host ">> Membuat tag v1.1.0 untuk trigger release..." -ForegroundColor Green

# Hapus tag lama kalau ada
git tag -d v1.1.0 2>$null
git push origin :refs/tags/v1.1.0 2>$null

git tag v1.1.0
git push origin v1.1.0

Write-Host ""
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  SELESAI! GitHub Actions sedang build..." -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Buka link ini untuk pantau progress build:" -ForegroundColor Yellow
Write-Host "  https://github.com/RafaelSitompoel/NitroControl2/actions" -ForegroundColor White
Write-Host ""
Write-Host "Setelah ~10 menit, installer .exe akan muncul di:" -ForegroundColor Yellow
Write-Host "  https://github.com/RafaelSitompoel/NitroControl2/releases" -ForegroundColor White
Write-Host ""

pause
