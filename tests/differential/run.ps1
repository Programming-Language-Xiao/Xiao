param(
    [switch]$Release = $true
)

$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$manifest = Join-Path $repo "tests/benchmarks/Cargo.toml"

if ($Release) {
    cargo run --release --manifest-path $manifest
} else {
    cargo run --manifest-path $manifest
}

$reportPath = Join-Path $repo "tests/benchmarks/reports/windows-native-semantic-differential.json"
$report = Get-Content -Raw $reportPath | ConvertFrom-Json
if (-not $report.passed) {
    throw "09R3 语义差分未通过：$reportPath"
}
Write-Host "09R3 三机型语义差分通过。"
