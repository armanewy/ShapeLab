$ErrorActionPreference = "Stop"

$cacheRoot = if ($env:OBJECT_ORCHARD_CACHE_DIR) {
    $env:OBJECT_ORCHARD_CACHE_DIR
} else {
    Join-Path $HOME "AppData\Local\ObjectOrchard"
}

if (-not $env:CARGO_TARGET_DIR) {
    $env:CARGO_TARGET_DIR = Join-Path $cacheRoot "cargo-target"
}
if (-not $env:SCCACHE_DIR) {
    $env:SCCACHE_DIR = Join-Path $cacheRoot "sccache"
}
if (-not $env:SCCACHE_CACHE_SIZE) {
    $env:SCCACHE_CACHE_SIZE = "50G"
}

New-Item -ItemType Directory -Force -Path $env:CARGO_TARGET_DIR | Out-Null
New-Item -ItemType Directory -Force -Path $env:SCCACHE_DIR | Out-Null

$sccache = Get-Command sccache -ErrorAction SilentlyContinue
if ($sccache) {
    if (-not $env:RUSTC_WRAPPER) {
        $env:RUSTC_WRAPPER = "sccache"
    }
    & sccache --start-server | Out-Null
    $sccacheStatus = "found: using RUSTC_WRAPPER=$env:RUSTC_WRAPPER"
} else {
    $sccacheStatus = "not found: install sccache to enable compiler caching"
}

Write-Host "Object Orchard development environment"
Write-Host "  CARGO_TARGET_DIR=$env:CARGO_TARGET_DIR"
Write-Host "  SCCACHE_DIR=$env:SCCACHE_DIR"
Write-Host "  SCCACHE_CACHE_SIZE=$env:SCCACHE_CACHE_SIZE"
Write-Host "  sccache: $sccacheStatus"
Write-Host ""
Write-Host "Use this script with:"
Write-Host "  . .\scripts\dev_env.ps1"
Write-Host ""
Write-Host "To unset:"
Write-Host "  Remove-Item Env:CARGO_TARGET_DIR,Env:RUSTC_WRAPPER,Env:SCCACHE_DIR,Env:SCCACHE_CACHE_SIZE -ErrorAction SilentlyContinue"
Write-Host ""
Write-Host "Warning:"
Write-Host "  A shared CARGO_TARGET_DIR reduces disk and rebuild time, but parallel Cargo"
Write-Host "  builds may contend on target locks. For heavy parallel Codex work, prefer"
Write-Host "  per-worktree targets plus RUSTC_WRAPPER=sccache."
