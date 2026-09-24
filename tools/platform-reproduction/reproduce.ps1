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
        throw "Command failed ($LASTEXITCODE): $Command $($Arguments -join ' ')"
    }
}

function Import-EnvironmentFromBatch {
    param(
        [string]$BatchFile
    )

    $command = "`"$BatchFile`" >nul && set"
    $environment = & cmd.exe /d /s /c $command
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to import batch environment ($LASTEXITCODE): $BatchFile"
    }

    foreach ($line in $environment) {
        $separator = $line.IndexOf('=')
        if ($separator -le 0) {
            continue
        }
        $name = $line.Substring(0, $separator)
        $value = $line.Substring($separator + 1)
        Set-Item -Path "Env:$name" -Value $value
    }
}

function Initialize-MsvcEnvironment {
    $vswhereCandidates = @(
        (Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'),
        (Join-Path $env:ProgramFiles 'Microsoft Visual Studio\Installer\vswhere.exe')
    )
    $vswhere = $vswhereCandidates | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
    if ($null -eq $vswhere) {
        throw 'Visual Studio vswhere.exe was not found; cannot prepare the MSVC linker environment'
    }

    $installationPath = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Select-Object -First 1).Trim()
    if ([string]::IsNullOrWhiteSpace($installationPath)) {
        throw 'Visual Studio installation with C++ x64 tools was not found'
    }

    $vcvars = Join-Path $installationPath 'VC\Auxiliary\Build\vcvars64.bat'
    if (-not (Test-Path -LiteralPath $vcvars -PathType Leaf)) {
        throw "MSVC environment script was not found: $vcvars"
    }
    Import-EnvironmentFromBatch $vcvars
}

function Write-Utf8NoBom {
    param(
        [string]$Path,
        [string]$Content
    )

    $encoding = New-Object -TypeName System.Text.UTF8Encoding -ArgumentList $false
    [System.IO.File]::WriteAllText($Path, $Content, $encoding)
}

function Write-ArtifactFormat {
    param(
        [string[]]$Paths
    )

    $fileCommand = Get-Command file -ErrorAction SilentlyContinue
    if ($null -ne $fileCommand) {
        & $fileCommand.Source @Paths
        if ($LASTEXITCODE -ne 0) {
            throw "file command failed ($LASTEXITCODE)"
        }
        return
    }

    foreach ($path in $Paths) {
        $bytes = [System.IO.File]::ReadAllBytes($path)
        if ($bytes.Length -ge 2 -and $bytes[0] -eq 0x4d -and $bytes[1] -eq 0x5a) {
            Write-Output "PE/COFF: $path"
        } else {
            Write-Output "Unknown Windows artifact format: $path"
        }
    }
}

if ($Mode -eq 'docker') {
    $image = "xiao-platform-reproduction:$($Platform.Replace('/', '-'))"
    Invoke-Checked 'docker' @('build', '--platform', $Platform, '-f', (Join-Path $repositoryRoot 'tools/platform-reproduction/Dockerfile'), '-t', $image, $repositoryRoot)
    Invoke-Checked 'docker' @('run', '--rm', '--platform', $Platform, '-e', 'XIAO_USE_XVFB=1', '-v', "$repositoryRoot`:/workspace", '-w', '/workspace', $image, 'bash', 'tools/platform-reproduction/reproduce.sh', 'native')
    exit 0
}

Initialize-MsvcEnvironment

$cargoManifest = Join-Path $repositoryRoot 'core/rust/Cargo.toml'
$benchmarkManifest = Join-Path $repositoryRoot 'tests/benchmarks/Cargo.toml'
$bunPath = (Get-Command bun).Source
$hostTriple = (& rustc -vV | Select-String '^host:').ToString().Split(':', 2)[1].Trim()
$env:XIAO_TARGET_TRIPLE = $hostTriple
$env:XIAO_CLANG = (Get-Command clang).Source
$env:XIAO_LLVM_AS = (Get-Command llvm-as).Source
$env:XIAO_LLC = (Get-Command llc).Source

Write-Output "Platform: Windows/$env:PROCESSOR_ARCHITECTURE"
Write-Output "Rust: $(& rustc --version)"
Write-Output "Bun: $(& bun --version)"
Write-Output "Target: $hostTriple"
Write-Output "clang: $((& $env:XIAO_CLANG --version) | Select-Object -First 1)"
Write-Output "llvm-as: $((& $env:XIAO_LLVM_AS --version) | Select-Object -First 1)"
Write-Output "llc: $((& $env:XIAO_LLC --version) | Select-Object -First 1)"

Push-Location $repositoryRoot
try {
    Invoke-Checked 'cargo' @('build', '--manifest-path', $cargoManifest, '-p', 'xiao-runtime', '--release')
    Invoke-Checked 'cargo' @('build', '--manifest-path', $cargoManifest, '-p', 'xiao-driver', '-p', 'xiao-diagnostics')
    $runtime = Join-Path $repositoryRoot 'core/rust/target/release/xiao_runtime.lib'
    if (-not (Test-Path -LiteralPath $runtime -PathType Leaf)) {
        throw "Runtime staticlib was not found: $runtime"
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
        $packageOutput = & $bunPath 'run' 'src/platform/packaging.ts' '--outdir' $packageDirectory
        if ($LASTEXITCODE -ne 0) { throw "Package build failed ($LASTEXITCODE)" }
        $packageOutput | Write-Output
        if (($packageOutput -join "`n") -notmatch 'development') { throw 'Package did not use development core discovery' }
    } finally {
        Pop-Location
    }

    $cli = Join-Path $packageDirectory 'xiao.exe'
    $sourceDirectory = Join-Path $temporaryRoot 'source'
    New-Item -ItemType Directory -Force -Path (Join-Path $sourceDirectory 'tests') | Out-Null
    $mainSource = Join-Path $sourceDirectory 'main.xiao'
    $testSource = Join-Path $sourceDirectory 'tests/smoke.xiao'
    Write-Utf8NoBom $mainSource "value = 1 + 2`n"
    Write-Utf8NoBom $testSource "value = 1 + 2`n"

    $nativeOutput = Join-Path $temporaryRoot 'native.exe'
    $llvmOutput = Join-Path $temporaryRoot 'native.ll'
    $buildJson = & $cli '--json' 'build' '-o' $nativeOutput $mainSource '--emit-llvm' $llvmOutput '-O0'
    if ($LASTEXITCODE -ne 0) { throw 'xiao build failed' }
    $buildResponse = ($buildJson -join "`n") | ConvertFrom-Json
    if ($buildResponse.type -ne 'result' -or $buildResponse.exit_code -ne 0) { throw 'xiao build protocol fields are invalid' }
    if (-not (Test-Path -LiteralPath $nativeOutput -PathType Leaf) -or (Get-Item -LiteralPath $nativeOutput).Length -eq 0) { throw 'xiao build did not produce an artifact' }
    if (-not (Test-Path -LiteralPath $llvmOutput -PathType Leaf) -or (Get-Item -LiteralPath $llvmOutput).Length -eq 0) { throw 'xiao build did not produce LLVM output' }

    $debugOutput = Join-Path $temporaryRoot 'debug-native.exe'
    $debugJson = & $cli '--json' 'build' '-debug' '-o' $debugOutput $mainSource '-O0'
    if ($LASTEXITCODE -ne 0) { throw 'xiao -debug build failed' }
    $debugResponse = ($debugJson -join "`n") | ConvertFrom-Json
    if ($debugResponse.type -ne 'result' -or $debugResponse.exit_code -ne 0) { throw 'xiao -debug build protocol fields are invalid' }
    if (-not (Test-Path -LiteralPath $debugOutput -PathType Leaf) -or (Get-Item -LiteralPath $debugOutput).Length -eq 0) { throw 'xiao -debug build did not produce an artifact' }

    Write-ArtifactFormat @($nativeOutput, $debugOutput, $cli)

    $outside = Join-Path $temporaryRoot 'outside'
    New-Item -ItemType Directory -Force -Path $outside | Out-Null
    Copy-Item $mainSource (Join-Path $outside 'main.xiao')
    $pathCliDirectory = Join-Path $temporaryRoot 'path-cli'
    $pathCoreDirectory = Join-Path $temporaryRoot 'path-bin'
    New-Item -ItemType Directory -Force -Path $pathCliDirectory, $pathCoreDirectory | Out-Null
    Copy-Item $cli (Join-Path $pathCliDirectory 'xiao.exe')
    Copy-Item (Join-Path $packageDirectory 'xiao-core.exe') (Join-Path $pathCoreDirectory 'xiao-core.exe')

    $oldCorePath = $env:XIAO_CORE_PATH
    $oldPath = $env:PATH
    try {
        $env:XIAO_CORE_PATH = $null
        $env:PATH = "$env:SystemRoot\System32;$env:SystemRoot"
        Write-Output '== Adjacent core discovery =='
        $runJson = & $cli '--json' 'run' (Join-Path $outside 'main.xiao')
        if ($LASTEXITCODE -ne 0) { throw 'independent artifact core discovery or xiao run failed' }
        $runResponse = ($runJson -join "`n") | ConvertFrom-Json
        if ($runResponse.type -ne 'result' -or $runResponse.exit_code -ne 0 -or $runResponse.exit_name -ne 'success') { throw 'xiao run protocol fields are invalid' }
        $testJson = & $cli '--json' 'test' $sourceDirectory
        if ($LASTEXITCODE -ne 0) { throw 'xiao test failed' }
        $testResponse = ($testJson -join "`n") | ConvertFrom-Json
        if ($testResponse.type -ne 'test_result' -or $testResponse.total -ne 1 -or $testResponse.passed -ne 1 -or $testResponse.exit_code -ne 0) { throw 'xiao test protocol fields are invalid' }

        Write-Output '== PATH core discovery =='
        $env:PATH = "$pathCoreDirectory;$env:SystemRoot\System32;$env:SystemRoot"
        $pathRunJson = & (Join-Path $pathCliDirectory 'xiao.exe') '--json' 'run' (Join-Path $outside 'main.xiao')
        if ($LASTEXITCODE -ne 0) { throw 'PATH core discovery or xiao run failed' }
        $pathRunResponse = ($pathRunJson -join "`n") | ConvertFrom-Json
        if ($pathRunResponse.type -ne 'result' -or $pathRunResponse.exit_code -ne 0 -or $pathRunResponse.exit_name -ne 'success') { throw 'PATH xiao run protocol fields are invalid' }

        Write-Output '== Development core discovery =='
        $developmentRunJson = & $bunPath (Join-Path $repositoryRoot 'cli/ts/src/main.ts') '--json' 'run' $mainSource
        if ($LASTEXITCODE -ne 0) { throw 'development core discovery or xiao run failed' }
        $developmentRunResponse = ($developmentRunJson -join "`n") | ConvertFrom-Json
        if ($developmentRunResponse.type -ne 'result' -or $developmentRunResponse.exit_code -ne 0 -or $developmentRunResponse.exit_name -ne 'success') { throw 'development xiao run protocol fields are invalid' }
    } finally {
        $env:XIAO_CORE_PATH = $oldCorePath
        $env:PATH = $oldPath
    }

    Write-Output '== Version negotiation mismatch =='
    $mismatchJson = & $bunPath (Join-Path $scriptDirectory 'check-protocol.ts') (Join-Path $repositoryRoot 'core/rust/target/debug/xiao-core.exe')
    if ($LASTEXITCODE -ne 0) { throw 'version mismatch probe failed' }
    $mismatchResponse = ($mismatchJson -join "`n") | ConvertFrom-Json
    $mismatchResponse | ConvertTo-Json -Compress | Write-Output
    if ($mismatchResponse.type -ne 'hello' -or $mismatchResponse.accepted -ne $false -or $mismatchResponse.error_code -ne 'X11-PROTOCOL-004') { throw 'version mismatch was not rejected by structured error code' }
} finally {
    Pop-Location
}

Write-Output "Platform reproduction completed: $hostTriple"
