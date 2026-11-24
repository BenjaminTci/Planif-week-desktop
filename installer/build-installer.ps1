# Compile Planif'Week en version finale puis génère l'installeur Windows.
# Usage : powershell -ExecutionPolicy Bypass -File installer\build-installer.ps1
# Résultat : target\installer\PlanifWeek-Setup-<version>.exe

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

# Inno Setup : dans le PATH, sinon aux emplacements d'installation habituels.
$iscc = (Get-Command ISCC.exe -ErrorAction SilentlyContinue).Source
if (-not $iscc) {
    $iscc = @(
        "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
    ) | Where-Object { Test-Path $_ } | Select-Object -First 1
}
if (-not $iscc) {
    throw "Inno Setup 6 introuvable. Installez-le avec : winget install JRSoftware.InnoSetup"
}

# Version lue dans Cargo.toml (section [package]).
$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value

Write-Host "Compilation de Planif'Week $version..."
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "La compilation a échoué." }

Write-Host "Création de l'installeur..."
& $iscc "/DAppVersion=$version" "/Q" "installer\planif-week.iss"
if ($LASTEXITCODE -ne 0) { throw "Inno Setup a échoué." }

$setup = Join-Path $root "target\installer\PlanifWeek-Setup-$version.exe"
Write-Host "Installeur prêt : $setup"
