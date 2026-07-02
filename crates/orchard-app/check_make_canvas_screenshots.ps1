param(
    [Parameter(Mandatory = $true)]
    [string]$ScreenshotDir,

    [int]$MinWidth = 1000,
    [int]$MinHeight = 700
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

$required = @(
    "choose_box_primitive.png",
    "make_ready_box_primitive.png",
    "generating_box_ideas.png",
    "generated_box_ideas.png",
    "selected_box_idea.png",
    "adjusted_box_control.png",
    "pack_drawer.png",
    "export_drawer.png"
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
    @("generating_box_ideas.png", "make_ready_box_primitive.png"),
    @("generated_box_ideas.png", "generating_box_ideas.png"),
    @("selected_box_idea.png", "generated_box_ideas.png"),
    @("adjusted_box_control.png", "make_ready_box_primitive.png"),
    @("pack_drawer.png", "adjusted_box_control.png"),
    @("export_drawer.png", "pack_drawer.png")
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
