# tests/windows/run.ps1 — Windows checks for the CI `windows` job.
#
#   pwsh -File tests\windows\run.ps1 [path\to\trackc.exe]
#
# With trackc.exe (default build\bootstrap\stage3\trackc.exe, from
# scripts\bootstrap.ps1): hello and every example build, run and match
# their goldens (tests\cases\examples; @HOSTNAME_BYTES@ is 0, Windows has no
# /etc/hostname); *_err examples are rejected; yard.exe builds and a
# sample project passes `yard test` (one passing, one failing test).
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$trackc = if ($args.Count -gt 0) { (Resolve-Path $args[0]).Path } else { Join-Path $root 'build\bootstrap\stage3\trackc.exe' }
$work = Join-Path $root 'build\windows-tests'
if (Test-Path $work) { Remove-Item -Recurse -Force $work }
New-Item -ItemType Directory -Force -Path $work | Out-Null
$failed = 0

# Run `exe` in `dir`, its stdout saved byte for byte to `out`; returns the exit code.
function Run-Raw([string]$dir, [string]$exe, [string]$out) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $exe
    $psi.WorkingDirectory = $dir
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $p = [System.Diagnostics.Process]::Start($psi)
    $fs = [System.IO.File]::Create($out)
    try { $p.StandardOutput.BaseStream.CopyTo($fs) } finally { $fs.Close() }
    $p.WaitForExit()
    return $p.ExitCode
}

Push-Location $root
try {
    foreach ($f in Get-ChildItem (Join-Path $root 'examples') -Filter '*.trk' | Sort-Object Name) {
        $s = $f.BaseName
        if ($s.EndsWith('_err')) {
            & $trackc check "examples\$($f.Name)" *> $null
            if ($LASTEXITCODE -eq 0) { Write-Host "FAIL $s : check accepted it"; $failed++ } else { Write-Host "ok   rejected $s" }
            continue
        }
        $exe = Join-Path $work "$s.exe"
        & $trackc build "examples\$($f.Name)" -o $exe | Out-Null
        if ($LASTEXITCODE -ne 0) { Write-Host "FAIL $s : build"; $failed++; continue }
        $out = Join-Path $work "$s.out"
        $rc = Run-Raw (Join-Path $root 'examples') $exe $out
        $want = ([System.IO.File]::ReadAllText((Join-Path $root "tests\cases\examples\$s.out"))).Replace('@HOSTNAME_BYTES@', '0')
        $got = [System.IO.File]::ReadAllText($out)
        $code = [int]([System.IO.File]::ReadAllText((Join-Path $root "tests\cases\examples\$s.code")).Trim())
        if ($got -ne $want) { Write-Host "FAIL $s : output differs"; $failed++ }
        elseif ($rc -ne $code) { Write-Host "FAIL $s : exit $rc, want $code"; $failed++ }
        else { Write-Host "ok   $s" }
    }

    # yard.exe, beside trackc.exe so it finds it.
    $bin = Join-Path $work 'bin'
    New-Item -ItemType Directory -Force -Path $bin | Out-Null
    Copy-Item $trackc (Join-Path $bin 'trackc.exe')
    & $trackc build tools\yard\main.trk -o (Join-Path $bin 'yard.exe') | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'yard.exe build failed' }
    $yard = Join-Path $bin 'yard.exe'
} finally {
    Pop-Location
}

Push-Location $work
try {
    & $yard init sample | Out-Null
    if ($LASTEXITCODE -ne 0) { Write-Host 'FAIL yard init'; $failed++ }
    Set-Location sample
    New-Item -ItemType Directory -Force -Path tests\fixtures | Out-Null
    Set-Content -NoNewline -Path tests\fixtures\f.txt -Value 'data'
    Set-Content -Path src\util.trk -Value 'fn helper(x: i64) -> i64 { return x + 1; }'
    Set-Content -Path tests\basic.trk -Value @'
import "util";
fn main() -> i64 {
    if helper(1) != 2 { return 1; }
    if file_exists("tests/fixtures/f.txt") == 0 { return 2; }
    return 0;
}
'@
    & $yard test
    if ($LASTEXITCODE -ne 0) { Write-Host 'FAIL yard test (passing project)'; $failed++ } else { Write-Host 'ok   yard test passes' }
    Set-Content -Path tests\failing.trk -Value 'fn main() -> i64 { return 3; }'
    & $yard test
    if ($LASTEXITCODE -eq 0) { Write-Host 'FAIL yard test passed a failing test'; $failed++ } else { Write-Host 'ok   yard test reports a failing test' }
    Remove-Item tests\failing.trk
    & $yard run
    if ($LASTEXITCODE -ne 0) { Write-Host 'FAIL yard run'; $failed++ } else { Write-Host 'ok   yard run' }
} finally {
    Pop-Location
}

if ($failed -gt 0) { Write-Host "$failed failure(s)"; exit 1 }
Write-Host 'all Windows checks passed'
