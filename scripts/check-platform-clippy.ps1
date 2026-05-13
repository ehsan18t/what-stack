#!/usr/bin/env pwsh
# what-stack - Cross-target Clippy gate.

$ErrorActionPreference = "Stop"

$targets = @(
    "x86_64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc"
)

function Assert-Command {
    param([string]$Name)

    if (Get-Command $Name -ErrorAction SilentlyContinue) {
        return
    }

    Write-Output ""
    Write-Output "ERROR: REQUIRED COMMAND NOT FOUND"
    Write-Output "  '$Name' is required to run the cross-target Clippy gate."
    exit 1
}

function Add-RustToolchainPath {
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        return
    }

    $cargoBin = Join-Path $HOME ".cargo\bin"
    $cargoExe = Join-Path $cargoBin "cargo.exe"

    if (Test-Path $cargoExe) {
        $env:PATH = "$cargoBin;$env:PATH"
    }
}

Add-RustToolchainPath

Assert-Command cargo
Assert-Command rustc
Assert-Command rustup

$installedTargets = @(rustup target list --installed | ForEach-Object { $_.Trim() })
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$hostTargetLine = rustc -vV | Where-Object { $_ -like 'host:*' } | Select-Object -First 1
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$hostTarget = $hostTargetLine -replace '^host:\s*', ''

$missingTargets = @($targets | Where-Object { $_ -notin $installedTargets })
if ($missingTargets.Count -gt 0) {
    Write-Output ""
    Write-Output "ERROR: MISSING RUST TARGETS"
    Write-Output "  Install the supported lint targets first:"
    Write-Output "    rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-msvc"
    Write-Output ""
    Write-Output "  Missing targets:"
    foreach ($target in $missingTargets) {
        Write-Output "    - $target"
    }
    exit 1
}

if ($targets -contains $hostTarget) {
    Write-Output "Detected supported host target: $hostTarget"
} else {
    Write-Output "Host target '$hostTarget' is not one of the supported release targets."
}

foreach ($target in $targets) {
    if ($target -eq $hostTarget) {
        Write-Output "-> Running native clippy for $target (all-targets)..."
        cargo clippy --locked --all-targets --target $target -- -D warnings
    } else {
        Write-Output "-> Running cross-target clippy for $target (lib only)..."

        cargo clippy --locked --lib --target $target -- -D warnings

    }

    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    Write-Output "  OK $target"
}
