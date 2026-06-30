param(
    [Parameter(Mandatory = $true)]
    [string]$ScreenshotDir,

    [int]$MinWidth = 1000,
    [int]$MinHeight = 700
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

$required = @(
    "01_choose.png",
    "02_make_ready.png",
    "03_generating_ideas.png",
    "04_generated_ideas.png",
    "05_selected_comparison.png",
    "06_focus_body.png",
    "07_generating_body_ideas.png",
    "08_body_ideas.png",
    "09_pack_drawer.png",
    "10_export_drawer.png"
)

$records = @{}

foreach ($name in $required) {
    $path = Join-Path $ScreenshotDir $name
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Missing screenshot: $name"
    }

    $image = [System.Drawing.Image]::FromFile($path)
    try {
        if ($image.Width -lt $MinWidth -or $image.Height -lt $MinHeight) {
            throw "Screenshot is too small: $name is $($image.Width)x$($image.Height)"
        }
        $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
        $records[$name] = [pscustomobject]@{
            Name = $name
            Width = $image.Width
            Height = $image.Height
            Sha256 = $hash
        }
    }
    finally {
        $image.Dispose()
    }
}

$differentPairs = @(
    @("03_generating_ideas.png", "02_make_ready.png"),
    @("04_generated_ideas.png", "03_generating_ideas.png"),
    @("05_selected_comparison.png", "04_generated_ideas.png"),
    @("06_focus_body.png", "05_selected_comparison.png"),
    @("07_generating_body_ideas.png", "06_focus_body.png"),
    @("08_body_ideas.png", "06_focus_body.png"),
    @("09_pack_drawer.png", "08_body_ideas.png"),
    @("10_export_drawer.png", "09_pack_drawer.png")
)

foreach ($pair in $differentPairs) {
    $left = $pair[0]
    $right = $pair[1]
    if ($records[$left].Sha256 -eq $records[$right].Sha256) {
        throw "Screenshots should differ but are identical: $left and $right"
    }
}

$records.GetEnumerator() |
    Sort-Object Name |
    ForEach-Object { $_.Value } |
    Format-Table -AutoSize

Write-Host "Make Canvas screenshot sanity passed."
