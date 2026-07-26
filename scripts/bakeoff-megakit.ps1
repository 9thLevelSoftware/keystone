#Requires -Version 5.1
<#
.SYNOPSIS
  Local MegaKit bake-off: measure → analyze → vibe-ready → propose → resolve + metrics.

.DESCRIPTION
  Does NOT commit commercial assets. Default pack:
    $env:KEYSTONE_BAKEOFF_PACK or
    C:\Users\dasbl\Downloads\Modular SciFi MegaKit[Source]\glTF

  Writes report JSON + markdown under target/bakeoff/ (gitignored via target/).
#>
param(
    [string]$Pack = $env:KEYSTONE_BAKEOFF_PACK,
    [string]$Cli = "",
    [int]$MaxPieces = 12,
    [switch]$NoExclude
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not $Pack) {
    $Pack = "C:\Users\dasbl\Downloads\Modular SciFi MegaKit[Source]\glTF"
}
if (-not (Test-Path -LiteralPath $Pack)) {
    Write-Error "Pack not found: $Pack (set KEYSTONE_BAKEOFF_PACK)"
}

if (-not $Cli) {
    $release = Join-Path $Root "target\release\asset-mapper.exe"
    $debug = Join-Path $Root "target\debug\asset-mapper.exe"
    if (Test-Path $release) { $Cli = $release }
    elseif (Test-Path $debug) { $Cli = $debug }
    else {
        Push-Location $Root
        cargo build -p asset-mapper-cli 2>&1 | Out-Host
        Pop-Location
        $Cli = $debug
    }
}

$OutDir = Join-Path $Root "target\bakeoff"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$reportPath = Join-Path $OutDir "megakit-$stamp.json"
$mdPath = Join-Path $OutDir "megakit-$stamp.md"
$planPath = Join-Path $OutDir "megakit-$stamp-plan.json"
$proposePath = Join-Path $OutDir "megakit-$stamp-propose.json"

Write-Host "CLI:  $Cli"
Write-Host "Pack: $Pack"
Write-Host "Out:  $OutDir"

# Ensure sidecar exists
$sidecar = Join-Path $Pack ".asset-mapper\pack.assetmap.json"
if (-not (Test-Path -LiteralPath $sidecar)) {
    & $Cli init $Pack --name "MegaKit glTF bakeoff" --license "See parent License_Source.txt" --author "MegaKit"
}

Write-Host "=== measure-bounds ==="
& $Cli measure-bounds $Pack | Out-File -Encoding utf8 (Join-Path $OutDir "measure-$stamp.json")

Write-Host "=== analyze --replace ==="
$analyzeArgs = @("analyze", $Pack, "--replace")
if (-not $NoExclude) {
    $analyzeArgs += @("--exclude-glob", "Decals/**", "--exclude-glob", "Aliens/**", "--exclude-glob", "*.png")
}
$analyzeJson = & $Cli @analyzeArgs
$analyzeJson | Out-File -Encoding utf8 (Join-Path $OutDir "analyze-$stamp.json")
$analyze = $analyzeJson | ConvertFrom-Json

Write-Host "=== vibe-ready ==="
$vibeJson = & $Cli vibe-ready $Pack 2>&1 | Out-String
$vibeExit = $LASTEXITCODE
$vibeJson | Out-File -Encoding utf8 (Join-Path $OutDir "vibe-$stamp.json")
try { $vibe = $vibeJson | ConvertFrom-Json } catch { $vibe = $null }

Write-Host "=== propose-assembly ==="
& $Cli propose-assembly $Pack --max-pieces $MaxPieces -o $proposePath | Out-Null
if (-not (Test-Path -LiteralPath $proposePath) -or (Get-Item -LiteralPath $proposePath).Length -lt 10) {
    Write-Error "propose-assembly did not write $proposePath"
}
$propose = Get-Content -LiteralPath $proposePath -Raw -Encoding utf8 | ConvertFrom-Json
# Write plan JSON without BOM (serde rejects UTF-8 BOM).
$planJson = $propose.plan | ConvertTo-Json -Depth 20 -Compress
[System.IO.File]::WriteAllText($planPath, $planJson)

Write-Host "=== resolve ==="
$resolveOk = $true
$resolveOut = Join-Path $OutDir "resolve-$stamp.json"
$resolveText = & $Cli resolve $Pack $planPath 2>&1 | Out-String
if ($LASTEXITCODE -ne 0) { $resolveOk = $false }
[System.IO.File]::WriteAllText($resolveOut, $resolveText)
if ($resolveText.Length -lt 10 -or $resolveText -match 'expected value') { $resolveOk = $false }

# Metrics from sidecar
$packObj = Get-Content -LiteralPath $sidecar -Raw | ConvertFrom-Json
$classHist = @{}
$roleHist = @{}
$byCat = @{}
$tier = @{}
$totalConn = 0
foreach ($a in $packObj.assets) {
    $cat = if ($a.source_path -match '/') { ($a.source_path -split '/')[0] } else { 'root' }
    if (-not $byCat.ContainsKey($cat)) { $byCat[$cat] = @{ assets = 0; with_conn = 0 } }
    $byCat[$cat].assets++
    if ($a.connectors.Count -gt 0) { $byCat[$cat].with_conn++ }
    foreach ($c in $a.connectors) {
        $totalConn++
        $cl = $c.class
        if (-not $classHist.ContainsKey($cl)) { $classHist[$cl] = 0 }
        $classHist[$cl]++
        $r = $c.role
        if (-not $roleHist.ContainsKey($r)) { $roleHist[$r] = 0 }
        $roleHist[$r]++
    }
}

$doorwayFrac = 0.0
if ($totalConn -gt 0 -and $classHist.ContainsKey('doorway')) {
    $doorwayFrac = [double]$classHist['doorway'] / $totalConn
}

# Stratified sample expectations
$samples = @(
    @{ path = 'Walls/WallAstra_Straight.gltf'; expect = 'wall_edge'; tier = 'A' },
    @{ path = 'Walls/WallBand_Straight.gltf'; expect = 'wall_edge'; tier = 'A' },
    @{ path = 'Walls/WallAstra_Straight_Window.gltf'; expect = 'window_frame'; tier = 'B' },
    @{ path = 'Walls/WallWindow_Straight.gltf'; expect = 'window_frame'; tier = 'B' },
    @{ path = 'Platforms/Door_Frame_Square.gltf'; expect = 'doorway'; tier = 'C' },
    @{ path = 'Platforms/Door_Simple.gltf'; expect = 'doorway'; tier = 'C' },
    @{ path = 'Platforms/Platform_Simple.gltf'; expect = 'floor_edge'; tier = 'D' },
    @{ path = 'Platforms/Platform_Metal.gltf'; expect = 'floor_edge'; tier = 'D' },
    @{ path = 'Walls/WallAstra_Corner_Square_Outer.gltf'; expect = 'wall_edge'; tier = 'E' }
)

$sampleResults = @()
$plausible = 0
$sampleN = 0
foreach ($s in $samples) {
    $asset = $packObj.assets | Where-Object { $_.source_path -eq $s.path } | Select-Object -First 1
    if (-not $asset) {
        $sampleResults += @{ path = $s.path; found = $false }
        continue
    }
    $sampleN++
    $hist = @{}
    foreach ($c in $asset.connectors) {
        if (-not $hist.ContainsKey($c.class)) { $hist[$c.class] = 0 }
        $hist[$c.class]++
    }
    $maj = $null
    $majN = -1
    foreach ($k in $hist.Keys) {
        if ($hist[$k] -gt $majN) { $maj = $k; $majN = $hist[$k] }
    }
    $ok = ($maj -eq $s.expect) -and ($asset.connectors.Count -ge 1) -and ($asset.connectors.Count -le 8)
    if ($ok) { $plausible++ }
    $sampleResults += @{
        path = $s.path
        tier = $s.tier
        expect = $s.expect
        majority = $maj
        n_connectors = $asset.connectors.Count
        plausible = $ok
        classes = $hist
    }
}

$report = [ordered]@{
    stamp = $stamp
    pack = $Pack
    analyze = $analyze
    vibe = $vibe
    vibe_exit = $vibeExit
    doorway_fraction = $doorwayFrac
    total_connectors = $totalConn
    class_histogram = $classHist
    role_histogram = $roleHist
    category_breakdown = $byCat
    propose_placed = @($propose.placed_asset_ids)
    propose_ops = @($propose.plan.operations).Count
    resolve_ok = $resolveOk
    sample_plausible = if ($sampleN -gt 0) { $plausible / $sampleN } else { 0 }
    samples = $sampleResults
    targets = @{
        doorway_fraction_max = 0.25
        sample_plausible_min = 0.6
        resolve_ok = $true
    }
}

$report | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $reportPath -Encoding utf8

$md = @"
# MegaKit bake-off $stamp

Pack: ``$Pack``

## Pipeline

| Step | Result |
| --- | --- |
| Analyze connectors | $($analyze.connectors_added) (mesh $($analyze.mesh_socket_assets), AABB $($analyze.bounds_fallback_assets)) |
| Doorway fraction | $([math]::Round($doorwayFrac*100,1))% (target < 25%) |
| Vibe-ready score | $(if ($vibe) { $vibe.score } else { 'n/a' }) ready=$(if ($vibe) { $vibe.ready } else { 'n/a' }) |
| Propose placed | $(@($propose.placed_asset_ids).Count) ops=$(@($propose.plan.operations).Count) |
| Resolve | $resolveOk |
| Sample plausible | $plausible / $sampleN ($([math]::Round(100*$plausible/[math]::Max(1,$sampleN),0))%) |

## Class histogram

$($classHist.GetEnumerator() | Sort-Object Value -Descending | ForEach-Object { "- **$($_.Key)**: $($_.Value)" } | Out-String)

## Stratified samples

$($sampleResults | ForEach-Object {
  if (-not $_.found -and $null -eq $_.tier) { "- $($_.path): **missing**" }
  else { "- Tier $($_.tier) ``$($_.path)``: n=$($_.n_connectors) maj=$($_.majority) expect=$($_.expect) plausible=$($_.plausible)" }
} | Out-String)

## Pass?

- doorway_fraction < 0.25: **$(if ($doorwayFrac -lt 0.25) { 'PASS' } else { 'FAIL' })**
- sample plausible ≥ 60%: **$(if ($sampleN -gt 0 -and ($plausible / $sampleN) -ge 0.6) { 'PASS' } else { 'FAIL' })**
- resolve_ok: **$(if ($resolveOk) { 'PASS' } else { 'FAIL' })**
"@
Set-Content -LiteralPath $mdPath -Value $md -Encoding utf8

Write-Host ""
Write-Host "=== SUMMARY ==="
Write-Host "doorway_fraction = $([math]::Round($doorwayFrac,3))"
Write-Host "sample_plausible = $plausible / $sampleN"
Write-Host "resolve_ok = $resolveOk"
Write-Host "Report: $reportPath"
Write-Host "Markdown: $mdPath"

if ($doorwayFrac -ge 0.25 -or -not $resolveOk) { exit 1 }
exit 0
