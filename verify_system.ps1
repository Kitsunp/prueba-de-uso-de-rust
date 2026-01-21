$ErrorActionPreference = "Stop"

function Write-Header {
    param($Message)
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host $Message -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
}

function Write-Success {
    param($Message)
    Write-Host "✅ $Message" -ForegroundColor Green
}

function Write-ErrorMsg {
    param($Message)
    Write-Host "❌ $Message" -ForegroundColor Red
}

Write-Header "Starting Hybrid System Verification"

# 1. Static Analysis (Stricter than CI)
Write-Header "Step 1: Static Code Analysis (Strict)"
try {
    Write-Host "Running cargo fmt..."
    cargo fmt --all -- --check
    Write-Success "Formatting correct"

    Write-Host "Running cargo clippy (deny warnings)..."
    # Deny all warnings to ensure higher quality than standard CI
    cargo clippy --workspace --all-targets -- -D warnings
    Write-Success "Clippy clean"
} catch {
    Write-ErrorMsg "Static Analysis Failed"
    exit 1
}

# 2. CI-Approximate Tests (Base Logic)
Write-Header "Step 2: Core Logic Tests (CI Approximation)"
Write-Host "Running tests without hardware features..."
try {
    # Exclude features requiring real hardware if possible, or just standard run
    # Since we don't have separate features yet, we run standard workspace tests
    # Env var approximates CI limitation
    $Env:CI = "true" 
    cargo test --workspace --no-fail-fast
    Write-Success "Core Logic passed"
} catch {
    Write-ErrorMsg "Core Logic Tests Failed"
    exit 1
}

# 3. Local Full Tests (Hardware)
Write-Header "Step 3: Hardware Integration Tests (Local Only)"
$has_gpu = $true # Assume local has GPU for now, could be dynamic
if ($has_gpu) {
    try {
        Write-Host "Running Rendering Tests (WGPU)..."
        # In the future, this would be: cargo test --features "render_wgpu"
        # For now, we rely on the existing hybrid fallback test
        # We unset CI to allow hardware usage
        Remove-Item Env:\CI
        $Env:RUST_LOG = "vnengine=debug"
        cargo test --package vnengine_runtime -- --nocapture
        Write-Success "Hardware Tests passed"
    } catch {
        Write-ErrorMsg "Hardware Integration Tests Failed"
        # We don't exit here strictly if it's just hardware flake, but for now we do
        exit 1
    }
} else {
    Write-Host "Skipping Hardware Tests (No GPU detected)" -ForegroundColor Yellow
}

Write-Header "All Systems Operational"
Write-Success "Ready for Phase 4 Implementation"
