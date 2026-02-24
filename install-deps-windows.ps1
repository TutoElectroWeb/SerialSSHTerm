<#
Script d'installation des dépendances Windows pour SerialSSHTerm

Usage (PowerShell admin recommandé) :
  powershell -ExecutionPolicy Bypass -File .\install-deps-windows.ps1
#>

$ErrorActionPreference = "Stop"

Write-Host "===============================================================" -ForegroundColor Cyan
Write-Host "  SerialSSHTerm - Installation dépendances Windows" -ForegroundColor Cyan
Write-Host "===============================================================" -ForegroundColor Cyan

if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    Write-Error "winget n'est pas disponible. Installez 'App Installer' depuis Microsoft Store."
}

function Ensure-Command {
    param(
        [string]$Command,
        [string]$WingetId,
        [string]$DisplayName
    )

    if (Get-Command $Command -ErrorAction SilentlyContinue) {
        Write-Host "✓ $DisplayName déjà installé"
        return
    }

    Write-Host "📦 Installation $DisplayName..."
    winget install --id $WingetId --accept-source-agreements --accept-package-agreements --silent
}

Ensure-Command -Command "cargo" -WingetId "Rustlang.Rustup" -DisplayName "Rust"
Ensure-Command -Command "git" -WingetId "Git.Git" -DisplayName "Git"

if (-not (Test-Path "C:\msys64\usr\bin\bash.exe")) {
    Write-Host "📦 Installation MSYS2 (GTK runtime/build)..."
    winget install --id "MSYS2.MSYS2" --accept-source-agreements --accept-package-agreements --silent
} else {
    Write-Host "✓ MSYS2 déjà installé"
}

if (Test-Path "C:\msys64\usr\bin\bash.exe") {
    Write-Host "↻ Mise à jour MSYS2 et installation toolchain mingw64 GTK4..."

    & "C:\msys64\usr\bin\bash.exe" -lc "pacman -Syu --noconfirm" | Out-Host
    & "C:\msys64\usr\bin\bash.exe" -lc "pacman -Su --noconfirm" | Out-Host
    & "C:\msys64\usr\bin\bash.exe" -lc "pacman -S --noconfirm --needed mingw-w64-x86_64-toolchain mingw-w64-x86_64-gtk4 mingw-w64-x86_64-libadwaita mingw-w64-x86_64-openssl" | Out-Host
}

Write-Host ""
Write-Host "✓ Dépendances Windows installées" -ForegroundColor Green
Write-Host "Étape suivante :" -ForegroundColor Yellow
Write-Host "  powershell -ExecutionPolicy Bypass -File .\build-exe.ps1"
