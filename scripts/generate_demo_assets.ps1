param(
    [string]$OutDir = "target/demo-assets",
    [string[]]$Preset = @("box-primitive"),
    [UInt64]$Seed = 42,
    [ValidateSet("explore", "refine")]
    [string]$Mode = "explore",
    [int]$ProposalCount = 24,
    [int]$ResultCount = 4,
    [int]$DescriptorResolution = 8,
    [int]$MeshResolution = 16,
    [int]$AcceptIndex = 0,
    [switch]$ReleaseCli,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

function Write-Usage {
    $usage = @"
Generate deterministic Shape Lab demo assets with the headless CLI.

Usage:
  pwsh -File scripts/generate_demo_assets.ps1 [options]

Options:
  -OutDir <path>                Output root. Default: target/demo-assets
  -Preset <id[]>                Presets to generate. Default: box-primitive
  -Seed <number>                Deterministic search seed. Default: 42
  -Mode <explore|refine>        Search mode. Default: explore
  -ProposalCount <number>       Raw proposals per preset. Default: 24
  -ResultCount <number>         Candidate count per preset. Default: 4
  -DescriptorResolution <num>   Descriptor sampling resolution. Default: 8
  -MeshResolution <number>      Mesh resolution. Default: 16
  -AcceptIndex <number>         Candidate accepted into project-after.json. Default: 0
  -ReleaseCli                   Run shape-cli in release mode.
  -Help                         Print this help.

Outputs per preset:
  project-before.json, current.obj, current.png, candidate OBJ/PNG files,
  contact-sheet.png, project-after.json, accepted.obj, accepted.png, summary.json.
"@
    Write-Host $usage
}

if ($Help) {
    Write-Usage
    exit 0
}

$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")
if ([System.IO.Path]::IsPathRooted($OutDir)) {
    $outRoot = [System.IO.Path]::GetFullPath($OutDir)
} else {
    $outRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $OutDir))
}

New-Item -ItemType Directory -Force -Path $outRoot | Out-Null

Push-Location $repoRoot
try {
    foreach ($presetId in $Preset) {
        $presetOut = Join-Path $outRoot $presetId
        New-Item -ItemType Directory -Force -Path $presetOut | Out-Null

        $cargoArgs = @("run", "-p", "shape-cli")
        if ($ReleaseCli) {
            $cargoArgs += "--release"
        }
        $cargoArgs += @(
            "--",
            "demo",
            "--preset", $presetId,
            "--seed", $Seed.ToString(),
            "--mode", $Mode,
            "--proposal-count", $ProposalCount.ToString(),
            "--result-count", $ResultCount.ToString(),
            "--descriptor-resolution", $DescriptorResolution.ToString(),
            "--mesh-resolution", $MeshResolution.ToString(),
            "--accept-index", $AcceptIndex.ToString(),
            "--out-dir", $presetOut
        )

        Write-Host "Generating $presetId demo assets in $presetOut"
        & cargo @cargoArgs
        if ($LASTEXITCODE -ne 0) {
            throw "cargo exited with code $LASTEXITCODE while generating $presetId"
        }

        foreach ($required in @("contact-sheet.png", "summary.json", "project-after.json")) {
            $requiredPath = Join-Path $presetOut $required
            if (-not (Test-Path -LiteralPath $requiredPath)) {
                throw "Expected output was not created: $requiredPath"
            }
            if ((Get-Item -LiteralPath $requiredPath).Length -le 0) {
                throw "Expected output is empty: $requiredPath"
            }
        }
    }
} finally {
    Pop-Location
}

Write-Host "Generated Shape Lab demo assets in $outRoot"
