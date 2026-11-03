#Requires -Version 7.0
<#
.SYNOPSIS
复跑真实归档、两个受控反例及“源码回显假命中”回归。
.DESCRIPTION
仅在新建临时目录复制和变造归档，原始证据不动。子进程catch真实ErrorRecord，
按Exception.Message与异常类型核对原因；格式化错误中的源码行不参与通过判定。
SourceRoot透传到每次审计；未指定时仍使用RepositoryRoot，历史归档须显式传入对应快照。
#>
param([string]$RepositoryRoot, [string]$OutputDirectory, [string]$SourceRoot)
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
if (-not $RepositoryRoot) { $RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path }
if (-not $SourceRoot) { $SourceRoot = $RepositoryRoot }
$SourceRoot = [IO.Path]::GetFullPath($SourceRoot)
if (-not $OutputDirectory) { $OutputDirectory = Join-Path ([IO.Path]::GetTempPath()) ('xiao-b1-close-validation-' + [Guid]::NewGuid().ToString('N')) }
$outputRoot = [IO.Path]::GetFullPath($OutputDirectory)
if ((Test-Path -LiteralPath $outputRoot) -and @(Get-ChildItem -LiteralPath $outputRoot -Force).Count -gt 0) { throw '验证输出目录必须为空，防止覆盖既有证据' }
[void][IO.Directory]::CreateDirectory($outputRoot)
$workRoot = Join-Path ([IO.Path]::GetTempPath()) ('xiao-b1-close-cases-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($workRoot)
$audit = Join-Path $PSScriptRoot 'audit.ps1'
$reports = Join-Path $PSScriptRoot '41498029910'
$utf8 = [Text.UTF8Encoding]::new($false, $true)

function Save([string]$name, [string]$value) {
    $clean = [regex]::Replace($value.Replace("`r`n", "`n"), '\x1B\[[0-?]*[ -/]*[@-~]', '').TrimEnd() + "`n"
    [IO.File]::WriteAllText((Join-Path $outputRoot $name), $clean, $utf8)
}
function Save-Json([string]$name, $value) { Save $name ($value | ConvertTo-Json -Depth 30) }
function File-Digest([string]$path) { return (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() }
function Expected-Rejection($result, [string]$message) {
    return $result.exit_code -eq 1 -and -not $result.error.success -and
        $result.error.exception_type -ceq 'System.Management.Automation.RuntimeException' -and
        $result.error.exception_message -ceq $message
}

# 子进程只将实际异常消息写入stderr；完整格式化诊断另存，避免源码回显混入判定。
$runner = Join-Path $workRoot 'capture-error.ps1'
$runnerSource = @'
param([string]$Audit, [string]$RepositoryRoot, [string]$SourceRoot, [string]$ReportDirectory, [string]$Outcome)
$ErrorActionPreference = 'Stop'
$encoding = [Text.UTF8Encoding]::new($false)
try {
    $text = (& $Audit -RepositoryRoot $RepositoryRoot -SourceRoot $SourceRoot -ReportDirectory $ReportDirectory) -join "`n"
    $result = [ordered]@{success=$true;output=($text | ConvertFrom-Json)}
    [IO.File]::WriteAllText($Outcome,(($result | ConvertTo-Json -Depth 20).Replace("`r`n","`n")+"`n"),$encoding)
    exit 0
} catch {
    $record = $_
    $result = [ordered]@{
        success=$false
        exception_message=$record.Exception.Message
        exception_type=$record.Exception.GetType().FullName
        fully_qualified_error_id=$record.FullyQualifiedErrorId
        category=$record.CategoryInfo.Category.ToString()
        formatted_error=($record | Out-String -Width 4096)
    }
    [IO.File]::WriteAllText($Outcome,(($result | ConvertTo-Json -Depth 20).Replace("`r`n","`n")+"`n"),$encoding)
    [Console]::Error.WriteLine($record.Exception.Message)
    exit 1
}
'@
[IO.File]::WriteAllText($runner, $runnerSource, $utf8)
function Invoke-Audit([string]$script, [string]$directory, [string]$name) {
    $outcome = Join-Path $workRoot ($name + '-outcome.json')
    $priorPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $nativeOutput = & pwsh -NoProfile -File $runner -Audit $script -RepositoryRoot $RepositoryRoot -SourceRoot $SourceRoot -ReportDirectory $directory -Outcome $outcome 2>&1
        $nativeExit = $LASTEXITCODE
    } finally { $ErrorActionPreference = $priorPreference }
    if (-not (Test-Path -LiteralPath $outcome)) { throw "子进程未返回结构化结果：$name" }
    $record = [IO.File]::ReadAllText($outcome) | ConvertFrom-Json
    Save-Json ($name + '.error.json') $record
    Save ($name + '.log') (($nativeOutput | ForEach-Object { $_.ToString() }) -join "`n")
    return [pscustomobject]@{exit_code=$nativeExit;error=$record}
}

$positive = Invoke-Audit $audit $reports 'positive'
if ($positive.exit_code -ne 0 -or -not $positive.error.success -or -not $positive.error.output.passed) { throw '真实归档审计未通过' }
Save-Json 'audit-positive.json' $positive.error.output
$cases = @()
foreach ($case in @('aggregate-job-id', 'missing-csv-o3')) {
    $caseRoot = [IO.Path]::GetFullPath((Join-Path $workRoot $case))
    if (-not $caseRoot.StartsWith($workRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw '受控归档路径越界' }
    Copy-Item -LiteralPath $reports -Destination $caseRoot -Recurse
    $manifestPath = Join-Path $caseRoot 'evidence.json'
    $manifest = [IO.File]::ReadAllText($manifestPath) | ConvertFrom-Json
    if ($case -eq 'aggregate-job-id') {
        $vm = $manifest.logs | Where-Object label -EQ 'vm-linux'
        $aggregate = $manifest.jobs | Where-Object name -EQ 'Rust workspace default tests (Linux)'
        if ($vm.actual_job_id -eq $aggregate.id) { throw '没有建立实际ID与聚合ID不同的反例' }
        $vm.actual_job_id = $aggregate.id
        $expected = '实际日志作业被聚合作业ID替代或缺失'
    } else {
        $from = '20O-B1-IMP-PROBE case=csv backend=vm/native optimization=3 result=success'
        $to = '20O-B1-IMP-PROBE case=csv backend=vm/native optimization=0 result=success'
        $zipPath = Join-Path $caseRoot 'ci-logs.zip'
        $zip = [IO.Compression.ZipFile]::Open($zipPath, [IO.Compression.ZipArchiveMode]::Update)
        try {
            $entry = $zip.GetEntry('native-macos-full.log')
            $stream = $entry.Open(); $buffer = [IO.MemoryStream]::new()
            try { $stream.CopyTo($buffer); $raw = $buffer.ToArray() }
            finally { $stream.Dispose(); $buffer.Dispose() }
            $text = $utf8.GetString($raw)
            if ([regex]::Matches($text, [regex]::Escape($from)).Count -ne 1) { throw '没有唯一CSV O3标记' }
            $changed = $utf8.GetBytes($text.Replace($from, $to))
            $entry.Delete()
            $entry = $zip.CreateEntry('native-macos-full.log', [IO.Compression.CompressionLevel]::Optimal)
            $stream = $entry.Open()
            try { $stream.Write($changed, 0, $changed.Length) } finally { $stream.Dispose() }
        } finally { $zip.Dispose() }
        $algorithm = [Security.Cryptography.SHA256]::Create()
        try { $changedSha = [BitConverter]::ToString($algorithm.ComputeHash($changed)).Replace('-', '').ToLowerInvariant() }
        finally { $algorithm.Dispose() }
        $mac = $manifest.logs | Where-Object label -EQ 'native-macos'
        $mac.source_sha256 = $changedSha; $mac.source_bytes = $changed.Length
        $manifest.raw_log_archive_sha256 = File-Digest $zipPath
        $excerptPath = Join-Path $caseRoot 'native-macos-exact.log'
        [IO.File]::WriteAllText($excerptPath, ([IO.File]::ReadAllText($excerptPath).Replace($from, $to)), $utf8)
        ($manifest.excerpts | Where-Object file -EQ 'native-macos-exact.log').sha256 = File-Digest $excerptPath
        $expected = '产品模块或优化级别缺失/重复'
    }
    [IO.File]::WriteAllText($manifestPath, (($manifest | ConvertTo-Json -Depth 40).Replace("`r`n", "`n") + "`n"), $utf8)
    $result = Invoke-Audit $audit $caseRoot $case
    if (-not (Expected-Rejection $result $expected)) { throw "反例未按实际异常原因拒绝：$case" }
    $cases += [ordered]@{case=$case;synthetic=$true;exit_code=$result.exit_code;expected_rejection=$expected;actual_exception_message=$result.error.exception_message;actual_exception_type=$result.error.exception_type;fully_qualified_error_id=$result.error.fully_qualified_error_id;passed=$true}
}

# 独立复现旧误判：格式化诊断含预期源码文字，真实异常却是.Count缺失。
$faultScript = Join-Path $workRoot 'source-line-false-hit.ps1'
$faultSource = @'
param([string]$RepositoryRoot, [string]$SourceRoot, [string]$ReportDirectory)
Set-StrictMode -Version Latest
function Require([bool]$condition, [string]$message) { if (-not $condition) { throw $message } }
$origin = $null
Require ($origin.Count -eq 1) '实际日志作业被聚合作业ID替代或缺失'
'@
[IO.File]::WriteAllText($faultScript, $faultSource, $utf8)
$control = Invoke-Audit $faultScript $reports 'source-line-false-hit'
$expected = '实际日志作业被聚合作业ID替代或缺失'
$formatContains = $control.error.formatted_error.Contains($expected)
$accepted = Expected-Rejection $control $expected
if ($control.exit_code -ne 1 -or $control.error.success -or -not $formatContains -or $accepted) { throw '源码回显假命中的回归控制未通过' }
Save 'source-line-false-hit-formatted.log' $control.error.formatted_error
Save-Json 'negative-cases.json' $cases
$summary = [ordered]@{
    schema_version=1
    passed=$true
    scope='受控验证，不是CI或产品运行；原始归档不修改'
    audit_script_sha256=(File-Digest $audit)
    validator_script_sha256=(File-Digest $PSCommandPath)
    powershell=$PSVersionTable.PSVersion.ToString()
    positive_audit_passed=$true
    synthetic_negative_cases=2
    rejection_check='exit=1 + Exception.GetType().FullName + Exception.Message精确相等；不搜索格式化源码'
    source_line_false_hit=[ordered]@{synthetic=$true;exit_code=$control.exit_code;formatted_contains_expected=$formatContains;actual_exception_message=$control.error.exception_message;actual_exception_type=$control.error.exception_type;accepted_by_validator=$accepted;passed=$true}
}
Save-Json 'validation.json' $summary
[ordered]@{passed=$true;output_directory=$outputRoot;synthetic_negative_cases=2;false_source_hit_rejected=$true} | ConvertTo-Json
