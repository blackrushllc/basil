param(
    [switch]$NoBuild
)

Write-Host "=== BASIC_IDE_STEP1: Running Debug API test and analyzer/demo ===" -ForegroundColor Cyan

if (-not $NoBuild) {
    Write-Host "Building workspace..."
    cargo build
    if ($LASTEXITCODE -ne 0) { Write-Error "Build failed"; exit 1 }
}

Write-Host "\n-- 1) Running Rust test: basil-vm/tests/debug_api.rs --" -ForegroundColor Yellow
cargo test -p basil-vm --test debug_api -- --nocapture
if ($LASTEXITCODE -ne 0) { Write-Error "Debug API test failed"; exit 1 }

Write-Host "\n-- 2) Running compiler analysis (--analyze) on examples/debug_demo.basil --" -ForegroundColor Yellow
cargo run -p basilc -- --analyze examples\debug_demo.basil --json
if ($LASTEXITCODE -ne 0) { Write-Warning "Analyzer run returned non-zero" }

Write-Host "\n-- 3) Running VM in debug mode (--debug) to show JSON events --" -ForegroundColor Yellow
# Expect Started, Output, Exited events
cargo run -p basilc -- --debug examples\debug_demo.basil | Out-Host

Write-Host "\nAll steps finished." -ForegroundColor Green
