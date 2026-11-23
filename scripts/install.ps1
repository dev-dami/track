# Track and yard installer for Windows.
#
#   irm https://raw.githubusercontent.com/dev-dami/track/main/scripts/install.ps1 | iex
#   pwsh -File scripts\install.ps1            (from a checkout: builds that tree)
#
# Gets the sources (git clone if git is on PATH, else the main-branch zip),
# bootstraps trackc.exe from bootstrap\seed\trackc.exe (scripts\bootstrap.ps1),
# builds yard.exe with it, installs both to %USERPROFILE%\.track\bin and adds
# that directory to the user PATH. Needs no compiler, toolchain or admin rights.
#
# Settings (environment): TRACK_INSTALL_DIR (default %USERPROFILE%\.track\bin),
# TRACK_REF (branch or tag, default main), TRACK_NO_PATH=1 (leave PATH alone).
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Say([string]$m) { Write-Host "  > $m" }
function Ok([string]$m) { Write-Host "  + $m" -ForegroundColor Green }
function Fail([string]$m) { Write-Host "  x $m" -ForegroundColor Red; throw $m }

$repo = 'dev-dami/track'
$ref = if ($env:TRACK_REF) { $env:TRACK_REF } else { 'main' }
$bin = if ($env:TRACK_INSTALL_DIR) { $env:TRACK_INSTALL_DIR } else { Join-Path $env:USERPROFILE '.track\bin' }

Write-Host ''
Write-Host '  Track installer (Windows)' -ForegroundColor Cyan
Write-Host ''

# 1. Sources: this checkout when run as a file from one, else a fresh copy.
$src = $null
$scratch = $null
if ($PSScriptRoot -and (Test-Path (Join-Path $PSScriptRoot '..\bootstrap\seed\trackc.exe'))) {
    $src = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
    Say "using the checkout at $src"
} else {
    $scratch = Join-Path ([System.IO.Path]::GetTempPath()) ("track-install-" + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Force -Path $scratch | Out-Null
    if (Get-Command git -ErrorAction SilentlyContinue) {
        Say "git clone $repo ($ref)"
        $src = Join-Path $scratch 'track'
        & git clone --quiet --depth 1 --branch $ref "https://github.com/$repo.git" $src
        if ($LASTEXITCODE -ne 0) { Fail 'git clone failed' }
    } else {
        Say "downloading $repo ($ref) as a zip"
        $zip = Join-Path $scratch 'track.zip'
        Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/$repo/archive/refs/heads/$ref.zip" -OutFile $zip
        Expand-Archive -Path $zip -DestinationPath $scratch -Force
        $src = (Get-ChildItem -Directory $scratch | Where-Object { Test-Path (Join-Path $_.FullName 'bootstrap\seed\trackc.exe') } | Select-Object -First 1).FullName
        if (-not $src) { Fail 'the archive has no bootstrap\seed\trackc.exe' }
    }
    Ok "sources in $src"
}

try {
    # 2. Bootstrap: seed -> stage1 -> stage2 = stage3.
    Say 'bootstrapping trackc.exe (seed -> stage1 -> stage2 -> stage3)'
    $env:BOOTSTRAP_DIR = Join-Path $src 'build\bootstrap'
    & (Join-Path $src 'scripts\bootstrap.ps1')
    if ($LASTEXITCODE -ne 0) { Fail 'bootstrap failed: stage2 and stage3 differ' }
    $trackc = Join-Path $src 'build\bootstrap\stage3\trackc.exe'
    Ok 'trackc.exe reached its fixed point'

    # 3. yard.exe, built by that trackc.
    $built = Join-Path $src 'build\bin'
    New-Item -ItemType Directory -Force -Path $built | Out-Null
    Push-Location $src
    try {
        & $trackc build tools\yard\main.trk -o (Join-Path $built 'yard.exe') | Out-Null
        if ($LASTEXITCODE -ne 0) { Fail 'building yard.exe failed' }
    } finally {
        Pop-Location
    }
    Ok 'built yard.exe'

    # 4. Install.
    New-Item -ItemType Directory -Force -Path $bin | Out-Null
    Copy-Item -Force $trackc (Join-Path $bin 'trackc.exe')
    Copy-Item -Force (Join-Path $built 'yard.exe') (Join-Path $bin 'yard.exe')
    Ok "installed trackc.exe and yard.exe to $bin"

    # 5. User PATH.
    if ($env:TRACK_NO_PATH -ne '1') {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $parts = @()
        if ($userPath) { $parts = $userPath -split ';' | Where-Object { $_ } }
        if ($parts -notcontains $bin) {
            [Environment]::SetEnvironmentVariable('Path', (($parts + $bin) -join ';'), 'User')
            Ok "added $bin to the user PATH (new terminals see it)"
        } else {
            Ok "$bin is already on the user PATH"
        }
        if (($env:Path -split ';') -notcontains $bin) { $env:Path = "$env:Path;$bin" }
    }

    Write-Host ''
    & (Join-Path $bin 'trackc.exe') --version
    & (Join-Path $bin 'yard.exe') --version
    Write-Host ''
    Write-Host '  Get started:  yard init my_app; cd my_app; yard run'
    Write-Host ''
} finally {
    if ($scratch) { Remove-Item -Recurse -Force $scratch -ErrorAction SilentlyContinue }
}
