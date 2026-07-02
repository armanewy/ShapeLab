param(
    [ValidateSet("release", "debug")]
    [string]$Profile = "release",
    [switch]$PreviewCatalog,
    [switch]$NoBuild,
    [switch]$NoStopExisting
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")
$profileDir = if ($Profile -eq "release") { "release" } else { "debug" }
$binaryPath = Join-Path $repoRoot "target\$profileDir\orchard-app.exe"

Push-Location $repoRoot
try {
    if (-not $NoBuild) {
        $cargoArgs = @("build", "-p", "orchard-app")
        if ($Profile -eq "release") {
            $cargoArgs += "--release"
        }

        Write-Host "Building Object Orchard $Profile binary..."
        & cargo @cargoArgs
        if ($LASTEXITCODE -ne 0) {
            throw "cargo exited with code $LASTEXITCODE while building Object Orchard"
        }
    }

    if (-not (Test-Path -LiteralPath $binaryPath)) {
        throw "Object Orchard binary was not found after build: $binaryPath"
    }

    if (-not $NoStopExisting) {
        $targetRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target"))
        Get-Process orchard-app -ErrorAction SilentlyContinue |
            Where-Object {
                $_.Path -and [System.IO.Path]::GetFullPath($_.Path).StartsWith(
                    $targetRoot,
                    [System.StringComparison]::OrdinalIgnoreCase
                )
            } |
            Stop-Process
    }

    if ($PreviewCatalog) {
        $env:OBJECT_ORCHARD_PREVIEW_CATALOG = "1"
    }

    Write-Host "Launching $binaryPath"
    Start-Process -FilePath $binaryPath -WorkingDirectory $repoRoot
} finally {
    Pop-Location
}
