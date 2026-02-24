<#
Script de build Windows (.exe) pour SerialSSHTerm

Usage :
  powershell -ExecutionPolicy Bypass -File .\build-exe.ps1

Options :
  -IncludeGtkRuntime       Copie les DLL GTK depuis C:\msys64\mingw64\bin
  -Configuration release   release | debug
#>

param(
    [ValidateSet("release", "debug")]
    [string]$Configuration = "release",
    [switch]$IncludeGtkRuntime
)

$ErrorActionPreference = "Stop"

Write-Host "===============================================================" -ForegroundColor Cyan
Write-Host "  SerialSSHTerm - Build Windows EXE" -ForegroundColor Cyan
Write-Host "===============================================================" -ForegroundColor Cyan

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "cargo introuvable. Exécutez d'abord .\install-deps-windows.ps1"
}

$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $projectRoot

Write-Host "🔨 Compilation Rust ($Configuration)..."
if ($Configuration -eq "release") {
    cargo build --release
    $exePath = Join-Path $projectRoot "target\release\serial-ssh-term.exe"
} else {
    cargo build
    $exePath = Join-Path $projectRoot "target\debug\serial-ssh-term.exe"
}

if (-not (Test-Path $exePath)) {
    throw "EXE non généré: $exePath"
}

$distRoot = Join-Path $projectRoot "dist\windows"
$appFolder = Join-Path $distRoot "SerialSSHTerm"
if (Test-Path $appFolder) {
    Remove-Item $appFolder -Recurse -Force
}
New-Item -ItemType Directory -Path $appFolder -Force | Out-Null

Copy-Item $exePath (Join-Path $appFolder "serial-ssh-term.exe") -Force

if (Test-Path (Join-Path $projectRoot "README.md")) {
    Copy-Item (Join-Path $projectRoot "README.md") (Join-Path $appFolder "README.md") -Force
}

if (Test-Path (Join-Path $projectRoot "assets\icon.svg")) {
    New-Item -ItemType Directory -Path (Join-Path $appFolder "assets") -Force | Out-Null
    Copy-Item (Join-Path $projectRoot "assets\icon.svg") (Join-Path $appFolder "assets\icon.svg") -Force
}

if ($IncludeGtkRuntime) {
    $gtkBin = "C:\msys64\mingw64\bin"
    if (-not (Test-Path $gtkBin)) {
        throw "GTK runtime introuvable ($gtkBin). Installez MSYS2 et les paquets mingw64 requis."
    }

    Write-Host "📦 Copie runtime GTK depuis $gtkBin..."
    Get-ChildItem $gtkBin -Filter "*.dll" | ForEach-Object {
        Copy-Item $_.FullName (Join-Path $appFolder $_.Name) -Force
    }
}

$archiveName = "serial-ssh-term-win64-$Configuration.zip"
$archivePath = Join-Path $distRoot $archiveName
if (Test-Path $archivePath) {
    Remove-Item $archivePath -Force
}

Write-Host "🗜️ Création archive $archiveName..."
Compress-Archive -Path (Join-Path $appFolder "*") -DestinationPath $archivePath -CompressionLevel Optimal

Write-Host ""
Write-Host "✓ Build terminé" -ForegroundColor Green
Write-Host "EXE : $exePath"
Write-Host "Dossier distrib : $appFolder"
Write-Host "Archive : $archivePath"
