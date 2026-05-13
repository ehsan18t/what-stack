#!/usr/bin/env pwsh

$ErrorActionPreference = "Stop"

$repoRoot = Join-Path $PSScriptRoot ".."
$hooksDir = Join-Path $repoRoot "hooks"
$hookPath = (git -C $repoRoot rev-parse --path-format=absolute --git-path hooks).Trim()

if (-not (Test-Path $hookPath)) {
    New-Item -ItemType Directory -Path $hookPath -Force | Out-Null
}

foreach ($hook in @("pre-commit", "pre-push", "commit-msg")) {
    $src = Join-Path $hooksDir $hook
    $dst = Join-Path $hookPath $hook
    Copy-Item $src $dst -Force
    Write-Output "  ${hook}: installed"
}

Write-Output ""
Write-Output "Git hooks installed."
Write-Output "Install lint targets: rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-msvc"
