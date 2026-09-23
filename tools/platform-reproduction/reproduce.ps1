param(
    [ValidateSet('native', 'docker')]
    [string]$Mode = 'native',
    [ValidateSet('linux/amd64', 'linux/arm64')]
    [string]$Platform = 'linux/amd64'
)

$ErrorActionPreference = 'Stop'
$scriptDirectory = Split-Path -Parent $MyInvocation.MyCommand.Path
$repositoryRoot = (Resolve-Path (Join-Path $scriptDirectory '..\..')).Path

function Invoke-Checked {
    param(
        [string]$Command,
        [string[]]$Arguments
    )
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "命令失败（$LASTEXITCODE）：$Command $($Arguments -join ' ')"
    }
}

if ($Mode -eq 'docker') {
    $image = "xiao-platform-reproduction:$($Platform.Replace('/', '-'))"
    Invoke-Checked 'docker' @('build', '--platform', $Platform, '-f', (Join-Path $repositoryRoot 'tools/platform-reproduction/Dockerfile'), '-t', $image, $repositoryRoot)
    Invoke-Checked 'docker' @('run', '--rm', '--platform', $Platform, '-e', 'XIAO_USE_XVFB=1', '-v', "$repositoryRoot`:/workspace", '-w', '/workspace', $image, 'bash', 'tools/platform-reproduction/reproduce.sh', 'native')
    exit 0
}

$cargoManifest = Join-Path $repositoryRoot 'core/rust/Cargo.toml'
$benchmarkManifest = Join-Path $repositoryRoot 'tests/benchmarks/Cargo.toml'
$hostTriple = (& rustc -vV | Select-String '^host:').ToString().Split(':', 2)[1].Trim()
$env:XIAO_TARGET_TRIPLE = $hostTriple
$env:XIAO_CLANG = (Get-Command clang).Source
$env:XIAO_LLVM_AS = (Get-Command llvm-as).Source
$env:XIAO_LLC = (Get-Command llc).Source

Write-Output "平台：Windows/$env:PROCESSOR_ARCHITECTURE"
Write-Output "Rust：$(& rustc --version)"
Write-Output "Bun：$(& bun --version)"
Write-Output "目标：$hostTriple"
Write-Output "clang：$((& $env:XIAO_CLANG --version) | Select-Object -First 1)"
Write-Output "llvm-as：$((& $env:XIAO_LLVM_AS --version) | Select-Object -First 1)"
Write-Output "llc：$((& $env:XIAO_LLC --version) | Select-Object -First 1)"

Push-Location $repositoryRoot
try {
    Invoke-Checked 'cargo' @('build', '--manifest-path', $cargoManifest, '-p', 'xiao-runtime', '--release')
    Invoke-Checked 'cargo' @('build', '--manifest-path', $cargoManifest, '-p', 'xiao-driver', '-p', 'xiao-diagnostics')
    $runtime = Join-Path $repositoryRoot 'core/rust/target/release/xiao_runtime.lib'
    if (-not (Test-Path -LiteralPath $runtime -PathType Leaf)) {
        throw "找不到 Runtime staticlib：$runtime"
    }
    $env:XIAO_RUNTIME_LIBRARY = $runtime

    Invoke-Checked 'cargo' @('test', '--manifest-path', $cargoManifest, '--workspace', '--', '--ignored')
    Invoke-Checked 'cargo' @('test', '--manifest-path', $cargoManifest, '-p', 'xiao-driver')
    Invoke-Checked 'bun' @('install', '--frozen-lockfile')
    Invoke-Checked 'bun' @('test')
    Invoke-Checked 'bunx' @('tsc', '--noEmit', '-p', 'tsconfig.json')
    Invoke-Checked 'cargo' @('check', '--manifest-path', $benchmarkManifest)
    Invoke-Checked 'bun' @('run', 'check')
    Invoke-Checked 'bun' @('run', 'check:coverage')
    Invoke-Checked 'cargo' @('fmt', '--all', '--manifest-path', $cargoManifest, '--', '--check')

    $temporaryRoot = Join-Path ([System.IO.Path]::GetTempPath()) "xiao-platform-reproduction-$PID"
    New-Item -ItemType Directory -Force -Path $temporaryRoot | Out-Null
    $packageDirectory = Join-Path $temporaryRoot 'package'
    Push-Location (Join-Path $repositoryRoot 'cli/ts')
    try {
        Invoke-Checked 'bun' @('run', 'src/platform/packaging.ts', '--outdir', $packageDirectory)
    } finally {
        Pop-Location
    }

    $cli = Join-Path $packageDirectory 'xiao.exe'
    $sourceDirectory = Join-Path $temporaryRoot 'source'
    New-Item -ItemType Directory -Force -Path (Join-Path $sourceDirectory 'tests') | Out-Null
    Set-Content -LiteralPath (Join-Path $sourceDirectory 'main.xiao') -Value "value = 1 + 2" -Encoding utf8NoBOM
    Set-Content -LiteralPath (Join-Path $sourceDirectory 'tests/smoke.xiao') -Value "value = 1 + 2" -Encoding utf8NoBOM

    $nativeOutput = Join-Path $temporaryRoot 'native.exe'
    $buildJson = & $cli '--json' 'build' '-o' $nativeOutput (Join-Path $sourceDirectory 'main.xiao') '-O0'
    if ($LASTEXITCODE -ne 0) { throw 'xiao build 失败' }
    $buildResponse = ($buildJson -join "`n") | ConvertFrom-Json
    if ($buildResponse.type -ne 'result' -or $buildResponse.exit_code -ne 0) { throw 'xiao build 协议字段不正确' }
    if (-not (Test-Path -LiteralPath $nativeOutput -PathType Leaf)) { throw 'xiao build 未生成产物' }

    $outside = Join-Path $temporaryRoot 'outside'
    New-Item -ItemType Directory -Force -Path $outside | Out-Null
    Copy-Item (Join-Path $sourceDirectory 'main.xiao') (Join-Path $outside 'main.xiao')
    $oldCorePath = $env:XIAO_CORE_PATH
    $oldPath = $env:PATH
    try {
        $env:XIAO_CORE_PATH = $null
        $env:PATH = "$env:SystemRoot\System32;$env:SystemRoot"
        $runJson = & $cli '--json' 'run' (Join-Path $outside 'main.xiao')
        if ($LASTEXITCODE -ne 0) { throw '独立产物核心发现或 xiao run 失败' }
        $runResponse = ($runJson -join "`n") | ConvertFrom-Json
        if ($runResponse.type -ne 'result' -or $runResponse.exit_code -ne 0 -or $runResponse.exit_name -ne 'success') { throw 'xiao run 协议字段不正确' }
        $testJson = & $cli '--json' 'test' $sourceDirectory
        if ($LASTEXITCODE -ne 0) { throw 'xiao test 失败' }
        $testResponse = ($testJson -join "`n") | ConvertFrom-Json
        if ($testResponse.type -ne 'test_result' -or $testResponse.total -ne 1 -or $testResponse.passed -ne 1 -or $testResponse.exit_code -ne 0) { throw 'xiao test 协议字段不正确' }
    } finally {
        $env:XIAO_CORE_PATH = $oldCorePath
        $env:PATH = $oldPath
    }
} finally {
    Pop-Location
}

Write-Output "平台复现脚本完成：$hostTriple"
