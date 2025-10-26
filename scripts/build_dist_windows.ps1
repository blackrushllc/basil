# Build various Basil release binaries for Windows and copy them into dist/windows
# Variants follow the requested naming scheme:
# - No --features                     -> basilc-naked.exe, bcc-naked.exe
# - --features obj-all                -> basilc.exe, bcc.exe
# - --features obj-bmx                -> basilc-bmx.exe, bcc-bmx.exe
# - --features "obj-json obj-daw obj-term obj-sqlite" -> basilc-daw.exe, bcc-daw.exe
# - --features "obj-ai obj-csv obj-curl obj-json obj-zip obj-sqlite obj-aws obj-sql obj-orm obj-net" -> basilc-web.exe, bcc-web.exe

# Stop on errors and treat uninitialized variables as errors
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Resolve repository root and move there (script may be invoked from anywhere)
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $ScriptDir '..')
Set-Location $RepoRoot

$DistDir = Join-Path $RepoRoot 'dist\windows'
New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

$basilcBin = Join-Path $RepoRoot 'target\release\basilc.exe'
$bccBin    = Join-Path $RepoRoot 'target\release\bcc.exe'

function Build-Variant {
    param(
        [Parameter(Mandatory=$true)][string]$Label,
        [string]$Features = ''
    )

    Write-Host "`n=== Building variant: $Label ===" -ForegroundColor Cyan

    if ([string]::IsNullOrWhiteSpace($Features)) {
        Write-Host "cargo build -p basilc --release"
        & cargo build -p basilc --release
    } else {
        Write-Host "cargo build -p basilc --release --features \"$Features\""
        & cargo build -p basilc --release --features "$Features"
    }

    # Build bcc (no feature flags on this crate)
    Write-Host "cargo build -p bcc --release"
    & cargo build -p bcc --release

    if (-not (Test-Path $basilcBin)) { throw "basilc binary not found at $basilcBin" }
    if (-not (Test-Path $bccBin))    { throw "bcc binary not found at $bccBin" }

    switch ($Label) {
        'naked' {
            $basilcOut = 'basilc-naked.exe'
            $bccOut    = 'bcc-naked.exe'
        }
        'all' {
            $basilcOut = 'basilc.exe'
            $bccOut    = 'bcc.exe'
        }
        'bmx' {
            $basilcOut = 'basilc-bmx.exe'
            $bccOut    = 'bcc-bmx.exe'
        }
        'daw' {
            $basilcOut = 'basilc-daw.exe'
            $bccOut    = 'bcc-daw.exe'
        }
        'web' {
            $basilcOut = 'basilc-web.exe'
            $bccOut    = 'bcc-web.exe'
        }
        default { throw "Unknown label: $Label" }
    }

    Copy-Item -Force $basilcBin (Join-Path $DistDir $basilcOut)
    Copy-Item -Force $bccBin    (Join-Path $DistDir $bccOut)
    Write-Host "Copied to $DistDir\$basilcOut and $DistDir\$bccOut" -ForegroundColor Green
}

# Variants per requirements
Build-Variant -Label 'naked' -Features ''
Build-Variant -Label 'all'   -Features 'obj-all'
Build-Variant -Label 'bmx'   -Features 'obj-bmx'
Build-Variant -Label 'daw'   -Features 'obj-json obj-daw obj-term obj-sqlite'
Build-Variant -Label 'web'   -Features 'obj-ai obj-csv obj-curl obj-json obj-zip obj-sqlite obj-aws obj-sql obj-orm obj-net'

Write-Host "`nAll Windows builds completed. Output directory: $DistDir" -ForegroundColor Yellow
