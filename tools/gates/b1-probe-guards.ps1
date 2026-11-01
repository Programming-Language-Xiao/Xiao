<#
.SYNOPSIS
从维护回归工作流提取 B1 计数判断，复跑四条路径的 28 个正反例。
.DESCRIPTION
只模拟 cargo 输出/退出码，不运行产品或外部服务；需要 PowerShell 7 和 Bash。
Windows 请显式传入 MSYS Bash。可选 ReportPath 保存带工作流摘要的机器报告。
#>
[CmdletBinding()]
param(
    [string] $BashPath,
    [string] $ReportPath
)

$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$workflowPath = Join-Path $repoRoot '.github/workflows/maintenance-regression.yml'
$workflow = [IO.File]::ReadAllText($workflowPath).Replace("`r`n", "`n")
$workflowLines = $workflow.Split([char]10)

if (-not $BashPath) {
    if ($IsWindows) { throw 'Windows 请用 -BashPath 显式指定 MSYS Bash，避免误选 WSL。' }
    $BashPath = (
        Get-Command bash -CommandType Application -ErrorAction Stop |
        Select-Object -First 1
    ).Source
}
$BashPath = (Get-Item -LiteralPath $BashPath -ErrorAction Stop).FullName
$pwshPath = Join-Path $PSHOME ('pwsh' + $(if ($IsWindows) { '.exe' } else { '' }))

function Get-WorkflowScript([string] $stepName) {
    $indices = @(for ($index = 0; $index -lt $workflowLines.Count; $index++) {
        if ($workflowLines[$index] -eq "      - name: $stepName") { $index }
    })
    if ($indices.Count -ne 1) { throw "工作流步骤必须唯一：$stepName" }
    $start = $indices[0] + 1
    while ($start -lt $workflowLines.Count -and $workflowLines[$start] -ne '        run: |') {
        if ($workflowLines[$start] -match '^      - name:|^  [A-Za-z0-9_-]+:') {
            throw "找不到 run 块：$stepName"
        }
        $start++
    }
    if ($start -eq $workflowLines.Count) { throw "找不到 run 块：$stepName" }
    $body = [Collections.Generic.List[string]]::new()
    for ($index = $start + 1; $index -lt $workflowLines.Count; $index++) {
        $line = $workflowLines[$index]
        if ($line.StartsWith('          ')) { $body.Add($line.Substring(10)) }
        elseif (-not $line.Trim()) { $body.Add('') }
        else { break }
    }
    return ($body -join "`n").TrimEnd() + "`n"
}

function Invoke-GuardProcess([string] $program, [string[]] $scriptArguments) {
    $startInfo = [Diagnostics.ProcessStartInfo]::new($program)
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    foreach ($argument in $scriptArguments) {
        if ($program -eq $BashPath) { $startInfo.ArgumentList.Add($argument.Replace([char]92, [char]47)) }
        else { $startInfo.ArgumentList.Add($argument) }
    }
    $startInfo.Environment['PATH'] = (Split-Path -Parent $BashPath) + [IO.Path]::PathSeparator + $env:PATH
    $process = [Diagnostics.Process]::Start($startInfo)
    try {
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit(15000)) {
            $process.Kill($true)
            $process.WaitForExit()
            throw '计数守卫脚本超过 15 秒，已终止。'
        }
        return [pscustomobject]@{
            ExitCode = $process.ExitCode
            Stdout = $stdout.GetAwaiter().GetResult()
            Stderr = $stderr.GetAwaiter().GetResult()
        }
    }
    finally { $process.Dispose() }
}

$default = Get-WorkflowScript 'Run B1 VM probe exactly once'
if ($default -notmatch '-- b1_capabilities_have_vm_golden_outputs --exact --nocapture') {
    throw '默认入口缺少目标或 --exact。'
}
$windows = Get-WorkflowScript 'Build runtime and run native gates'
$windowsMatch = [regex]::Match($windows, '(?ms)^\$b1Output = cargo test.*?(?=^exit \$LASTEXITCODE\s*$)')
if (-not $windowsMatch.Success -or $windowsMatch.Value -notmatch '--ignored b1_capabilities_match_native_golden_outputs --exact --nocapture') {
    throw 'Windows 精确入口缺失或边界改变。'
}
$scripts = [ordered]@{ default = $default }
foreach ($platform in @('Linux', 'macOS')) {
    $native = Get-WorkflowScript "Build runtime and run $platform native gates"
    $helper = [regex]::Match($native, '(?ms)^run_exact_native_test\(\) \{.*?^\}')
    if (-not $helper.Success -or $native -notmatch '(?m)^run_exact_native_test b1_capabilities_match_native_golden_outputs 20o_b1_probe$') {
        throw "$platform 原生入口缺失或边界改变。"
    }
    $scripts[$platform] = "set -euo pipefail`n" + $helper.Value + "`nrun_exact_native_test b1_capabilities_match_native_golden_outputs 20o_b1_probe`n"
}

$summary = 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.01s'
$cases = @(
    @{ Name = 'one'; Output = $summary; Exit = 0; Accept = $true },
    @{ Name = 'zero'; Output = $summary.Replace('1 passed', '0 passed'); Exit = 0; Accept = $false },
    @{ Name = 'two'; Output = $summary.Replace('1 passed', '2 passed'); Exit = 0; Accept = $false },
    @{ Name = 'ignored'; Output = $summary.Replace('1 passed', '0 passed').Replace('0 ignored', '1 ignored'); Exit = 0; Accept = $false },
    @{ Name = 'failed'; Output = $summary.Replace('ok.', 'FAILED.').Replace('1 passed', '0 passed').Replace('0 failed', '1 failed'); Exit = 101; Accept = $false },
    @{ Name = 'failed-exit'; Output = $summary; Exit = 101; Accept = $false },
    @{ Name = 'duplicate'; Output = $summary + "`n" + $summary; Exit = 0; Accept = $false }
)
$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ('xiao-b1-guards-' + [guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($tempRoot)
$encoding = [Text.UTF8Encoding]::new($false)
$results = [Collections.Generic.List[object]]::new()
try {
    foreach ($platform in $scripts.Keys) {
        $syntaxPath = Join-Path $tempRoot "$platform-syntax.sh"
        [IO.File]::WriteAllText($syntaxPath, $scripts[$platform], $encoding)
        $syntax = Invoke-GuardProcess $BashPath @('-n', $syntaxPath)
        if ($syntax.ExitCode -ne 0) { throw "$platform Bash 语法错误：$($syntax.Stderr)" }
        foreach ($case in $cases) {
            $escapedOutput = $case.Output.Replace("'", "'\''")
            $mock = "cargo() { printf '%s\n' '$escapedOutput'; return $($case.Exit); }`n"
            $path = Join-Path $tempRoot "$platform-$($case.Name).sh"
            [IO.File]::WriteAllText($path, $mock + $scripts[$platform], $encoding)
            $result = Invoke-GuardProcess $BashPath @($path)
            if (($result.ExitCode -eq 0) -ne $case.Accept) {
                throw "$platform/$($case.Name) 判断错误：$($result.Stdout) $($result.Stderr)"
            }
            $results.Add([pscustomobject]@{ Path = $platform; Case = $case.Name; Passed = $true; ExitCode = $result.ExitCode })
        }
        Write-Output "B1-GUARDS path=$platform passed=7 total=7"
    }
    $tokens = $null
    $parseErrors = $null
    [void][Management.Automation.Language.Parser]::ParseInput($windowsMatch.Value, [ref]$tokens, [ref]$parseErrors)
    if ($parseErrors.Count -ne 0) { throw "Windows PowerShell 语法错误：$parseErrors" }
    foreach ($case in $cases) {
        $escapedOutput = $case.Output.Replace("'", "''")
        $mock = "function cargo { '$escapedOutput' -split [char]10 | ForEach-Object { Write-Output `$_ }; `$global:LASTEXITCODE = $($case.Exit) }`n"
        $path = Join-Path $tempRoot "Windows-$($case.Name).ps1"
        [IO.File]::WriteAllText($path, "`$ErrorActionPreference = 'Stop'`n" + $mock + $windowsMatch.Value, $encoding)
        $result = Invoke-GuardProcess $pwshPath @('-NoProfile', '-File', $path)
        if (($result.ExitCode -eq 0) -ne $case.Accept) {
            throw "Windows/$($case.Name) 判断错误：$($result.Stdout) $($result.Stderr)"
        }
        $results.Add([pscustomobject]@{ Path = 'Windows'; Case = $case.Name; Passed = $true; ExitCode = $result.ExitCode })
    }
    Write-Output 'B1-GUARDS path=Windows passed=7 total=7'
    if ($results.Count -ne 28) { throw "应执行 28 个守卫场景，实际 $($results.Count)。" }
    if ($ReportPath) {
        $reportAbsolute = [IO.Path]::GetFullPath($ReportPath)
        $report = [ordered]@{
            schemaVersion = 1
            workflow = '.github/workflows/maintenance-regression.yml'
            workflowSha256 = (Get-FileHash -LiteralPath $workflowPath -Algorithm SHA256).Hash.ToLowerInvariant()
            scriptSha256 = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()
            scope = 'simulated-cargo-output'
            hostOS = $PSVersionTable.OS
            powershell = $PSVersionTable.PSVersion.ToString()
            passed = 28
            total = 28
            cases = $results.ToArray()
        }
        [IO.File]::WriteAllText($reportAbsolute, ($report | ConvertTo-Json -Depth 5) + "`n", $encoding)
    }
    Write-Output 'B1-GUARDS passed=28 total=28; 仅验证计数脚本，不是 VM/native 三平台执行证据。'
}
finally {
    $resolvedTemp = [IO.Path]::GetFullPath($tempRoot)
    $expectedParent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([char[]]@([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar))
    if ([IO.Directory]::GetParent($resolvedTemp).FullName -eq $expectedParent -and
        [IO.Path]::GetFileName($resolvedTemp).StartsWith('xiao-b1-guards-')) {
        Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
    }
}
