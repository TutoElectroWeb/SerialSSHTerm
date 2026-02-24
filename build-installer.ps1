<#
Script de création d'un installateur Windows (.exe) via Inno Setup

Usage :
  powershell -ExecutionPolicy Bypass -File .\build-installer.ps1

Options :
  -Configuration release|debug
  -IncludeGtkRuntime      Copie aussi les DLL GTK dans la distribution
  -SkipBuild              N'exécute pas build-exe.ps1 avant packaging
#>

param(
    [ValidateSet("release", "debug")]
    [string]$Configuration = "release",
    [switch]$IncludeGtkRuntime,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

Write-Host "===============================================================" -ForegroundColor Cyan
Write-Host "  SerialSSHTerm - Build installateur Windows" -ForegroundColor Cyan
Write-Host "===============================================================" -ForegroundColor Cyan

$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $projectRoot

$cargoTomlPath = Join-Path $projectRoot "Cargo.toml"
if (-not (Test-Path $cargoTomlPath)) {
    throw "Cargo.toml introuvable dans $projectRoot"
}

$cargoToml = Get-Content $cargoTomlPath -Raw
$versionMatch = [regex]::Match($cargoToml, 'version\s*=\s*"([^"]+)"')
if (-not $versionMatch.Success) {
    throw "Impossible de déterminer la version depuis Cargo.toml"
}
$appVersion = $versionMatch.Groups[1].Value

$buildExeScript = Join-Path $projectRoot "build-exe.ps1"
if (-not (Test-Path $buildExeScript)) {
    throw "build-exe.ps1 introuvable."
}

if (-not $SkipBuild) {
    Write-Host "🔨 Build de l'application (préparation distribution)..."
    $buildArgs = @(
        "-ExecutionPolicy", "Bypass",
        "-File", $buildExeScript,
        "-Configuration", $Configuration
    )
    if ($IncludeGtkRuntime) {
        $buildArgs += "-IncludeGtkRuntime"
    }

    & powershell @buildArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Échec de build-exe.ps1"
    }
}

$appSource = Join-Path $projectRoot "dist\windows\SerialSSHTerm"
if (-not (Test-Path (Join-Path $appSource "serial-ssh-term.exe"))) {
    throw "Binaire distributable introuvable: $appSource\serial-ssh-term.exe"
}

$iscc = Get-Command iscc.exe -ErrorAction SilentlyContinue
if (-not $iscc) {
    $defaultIscc = "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
    if (Test-Path $defaultIscc) {
        $isccPath = $defaultIscc
    } else {
        throw "Inno Setup (iscc.exe) introuvable. Installez via: winget install JRSoftware.InnoSetup"
    }
} else {
    $isccPath = $iscc.Source
}

$installerDir = Join-Path $projectRoot "dist\windows\installer"
if (-not (Test-Path $installerDir)) {
    New-Item -Path $installerDir -ItemType Directory -Force | Out-Null
}

$outputBase = "serial-ssh-term-setup-win64-v$appVersion"
$setupScriptPath = Join-Path $installerDir "serial-ssh-term.iss"

$issContent = @"
[Setup]
AppId={{8D546EF5-5CA7-4CF7-A2AC-7D2F207E7D9E}
AppName=SerialSSHTerm
AppVersion=$appVersion
AppPublisher=M@nu
DefaultDirName={autopf}\SerialSSHTerm
DefaultGroupName=SerialSSHTerm
AllowNoIcons=yes
OutputDir=$installerDir
OutputBaseFilename=$outputBase
Compression=lzma2
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
DisableProgramGroupPage=yes
LicenseFile=

[Languages]
Name: ""french""; MessagesFile: ""compiler:Languages\French.isl""
Name: ""english""; MessagesFile: ""compiler:Default.isl""

[Files]
Source: ""$appSource\*""; DestDir: ""{app}""; Flags: ignoreversion recursesubdirs createallsubdirs

[Tasks]
Name: ""desktopicon""; Description: ""{cm:CreateDesktopIcon}""; GroupDescription: ""{cm:AdditionalIcons}""; Flags: unchecked

[Icons]
Name: ""{group}\SerialSSHTerm""; Filename: ""{app}\serial-ssh-term.exe""
Name: ""{autodesktop}\SerialSSHTerm""; Filename: ""{app}\serial-ssh-term.exe""; Tasks: desktopicon

[Run]
Filename: ""{app}\serial-ssh-term.exe""; Description: ""{cm:LaunchProgram,SerialSSHTerm}""; Flags: nowait postinstall skipifsilent
"@

Set-Content -Path $setupScriptPath -Value $issContent -Encoding UTF8

Write-Host "📦 Génération installateur avec Inno Setup..."
& $isccPath $setupScriptPath
if ($LASTEXITCODE -ne 0) {
    throw "Échec Inno Setup"
}

$setupExe = Join-Path $installerDir "$outputBase.exe"

Write-Host ""
Write-Host "✓ Installateur généré" -ForegroundColor Green
Write-Host "Script ISS : $setupScriptPath"
Write-Host "Installateur : $setupExe"
