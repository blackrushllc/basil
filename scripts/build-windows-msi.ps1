$ErrorActionPreference = "Stop"

# Paths
$RepoRoot   = (Resolve-Path "$PSScriptRoot\..").Path
$WixDir     = Join-Path $RepoRoot "installer\windows\wix"
$BuildDir   = Join-Path $RepoRoot "target\release"

# Staging folders
$StageRoot     = Join-Path $RepoRoot "installer\windows\_stage"
$DocsStage     = Join-Path $StageRoot "docs"
$ExamplesStage = Join-Path $StageRoot "examples"

# Clean stage
if (Test-Path $StageRoot) { Remove-Item $StageRoot -Recurse -Force }
New-Item -ItemType Directory -Path $DocsStage | Out-Null
New-Item -ItemType Directory -Path $ExamplesStage | Out-Null

Write-Host "1) Build Rust in Release..."
# cargo build --release # <-- comment this out to avoid rebuilding

# Build basilc with features
cargo build -p basilc --release --features obj-all

# Build the others normally (adjust features if needed)
cargo build -p bcc --release
cargo build -p basil-serve --release
# cargo build -p basil-serve --release --features obj-all

Write-Host "2) Stage DOCS (docs/** + README.md + WHATS_NEW.md)..."
# Copy docs/**
$DocsSrc = Join-Path $RepoRoot "docs"
if (Test-Path $DocsSrc) {
  robocopy $DocsSrc $DocsStage /MIR /NFL /NDL /NJH /NJS /NC /NS | Out-Null
}

# Add top-level README.md and WHATS_NEW.md into docs
$rootReadme = Join-Path $RepoRoot "README.md"
$rootWhats  = Join-Path $RepoRoot "WHATS_NEW.md"
if (Test-Path $rootReadme) { Copy-Item $rootReadme -Destination (Join-Path $DocsStage "README.md") }
if (Test-Path $rootWhats)  { Copy-Item $rootWhats  -Destination (Join-Path $DocsStage "WHATS_NEW.md") }

Write-Host "3) Stage EXAMPLES (examples/**) with exclusions..."
$ExamplesSrc = Join-Path $RepoRoot "examples"
if (Test-Path $ExamplesSrc) {
  # First copy everything, then prune by your rules (simplest + robust)
  robocopy $ExamplesSrc $ExamplesStage /MIR /NFL /NDL /NJH /NJS /NC /NS | Out-Null

  # Remove *.basilx and .env
  Get-ChildItem $ExamplesStage -Recurse -File -Include *.basilx,.env | Remove-Item -Force

  # Remove any file/dir whose name starts with "." or "_"
  Get-ChildItem $ExamplesStage -Recurse -Force | Where-Object {
    $_.Name -match '^[._]'  # leading dot or underscore
  } | ForEach-Object {
    if ($_.PSIsContainer) { Remove-Item $_.FullName -Recurse -Force }
    else { Remove-Item $_.FullName -Force }
  }

  # Remove examples/.basil (if still present after the above)
  $basilHidden = Join-Path $ExamplesStage ".basil"
  if (Test-Path $basilHidden) { Remove-Item $basilHidden -Recurse -Force }

  # Remove examples/basilbasic.com/.basilcache
  $cacheDir = Join-Path $ExamplesStage "basilbasic.com\.basilcache"
  if (Test-Path $cacheDir) { Remove-Item $cacheDir -Recurse -Force }
}

Write-Host "4) Build MSI via WiX v4..."
Push-Location $WixDir

# Clean WiX obj/bin to force re-harvest after project changes
if (Test-Path (Join-Path $WixDir "obj")) { Remove-Item (Join-Path $WixDir "obj") -Recurse -Force }
if (Test-Path (Join-Path $WixDir "bin")) { Remove-Item (Join-Path $WixDir "bin") -Recurse -Force }

# Ensure Wix SDK and build assets are restored
dotnet restore

# Pass paths as MSBuild properties (used by .wixproj DefineConstants)
dotnet build -c Release `
  -p:RepoRoot="$RepoRoot" `
  -p:BuildDir="$BuildDir" `
  -p:DocsStage="$DocsStage" `
  -p:ExamplesStage="$ExamplesStage"

Pop-Location

Write-Host "Done. MSI should be under: installer\\windows\\wix\\bin\\Release\\"
