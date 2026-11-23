# Bootstrap the self-hosted Track compiler on Windows and verify the fixed point.
#
#   seed     bootstrap\seed\trackc.exe, a committed x86_64-windows PE
#            (the fixed-point trackc), compiles compiler\src\main.trk -> stage1
#   stage1   compiles compiler\src\main.trk                           -> stage2
#   stage2   compiles compiler\src\main.trk                           -> stage3
#
# The PowerShell twin of scripts/bootstrap.sh. No C compiler, assembler or
# linker is used; trackc.exe needs only kernel32. The gate: stage2 and
# stage3 are byte-identical (Get-FileHash).
#
#   pwsh -File scripts\bootstrap.ps1      (or powershell -ExecutionPolicy Bypass -File ...)
#
# Artifacts go to build\bootstrap\ (override with $env:BOOTSTRAP_DIR); the
# result is build\bootstrap\stage3\trackc.exe.
$ErrorActionPreference = 'Stop'

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$out = if ($env:BOOTSTRAP_DIR) { $env:BOOTSTRAP_DIR } else { Join-Path $root 'build\bootstrap' }
if (-not [System.IO.Path]::IsPathRooted($out)) { $out = Join-Path (Get-Location).Path $out }
$seed = Join-Path $root 'bootstrap\seed\trackc.exe'

if (-not (Test-Path $seed)) { Write-Error 'bootstrap: missing bootstrap\seed\trackc.exe'; exit 1 }
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
foreach ($d in 'seed', 'stage1', 'stage2', 'stage3') {
    New-Item -ItemType Directory -Force -Path (Join-Path $out $d) | Out-Null
}
Copy-Item $seed (Join-Path $out 'seed\trackc.exe')

# Every stage compiles the same sources from the repository root, so the
# loader resolves identical module paths at every stage.
Push-Location $root
try {
    $prev = Join-Path $out 'seed\trackc.exe'
    foreach ($s in 'stage1', 'stage2', 'stage3') {
        Write-Host "==> ${s}: $(Split-Path -Leaf (Split-Path -Parent $prev)) compiles compiler\src\main.trk"
        $exe = Join-Path $out "$s\trackc.exe"
        & $prev build --target x86_64-windows compiler\src\main.trk -o $exe | Out-Null
        if ($LASTEXITCODE -ne 0) { Write-Error "bootstrap: $s failed (exit $LASTEXITCODE)"; exit 1 }
        $prev = $exe
    }
} finally {
    Pop-Location
}

$status = 0
$h2 = (Get-FileHash (Join-Path $out 'stage2\trackc.exe') -Algorithm SHA256).Hash
$h3 = (Get-FileHash (Join-Path $out 'stage3\trackc.exe') -Algorithm SHA256).Hash
$size = (Get-Item (Join-Path $out 'stage3\trackc.exe')).Length
if ($h2 -eq $h3) {
    Write-Host "OK stage2 and stage3 are byte-identical ($size bytes)"
} else {
    Write-Host 'FAIL stage2 and stage3 differ'
    $status = 1
}
# A stale seed is reported here and gated by scripts/update_seed.sh --check.
if ((Get-FileHash $seed -Algorithm SHA256).Hash -eq $h3) {
    Write-Host 'OK bootstrap\seed\trackc.exe matches the fixed point'
} else {
    Write-Host '! bootstrap\seed\trackc.exe differs from the fixed point; run scripts/update_seed.sh'
}
& (Join-Path $out 'stage3\trackc.exe') --version
exit $status
