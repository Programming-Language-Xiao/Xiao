#Requires -Version 7.0
<#
.SYNOPSIS
只读复核B1-CLOSE的日志、来源、源码文件、冻结摘要及沿用的721条F10。
.DESCRIPTION
不调用网络、Git或测量工具，不修改证据。SourceRoot须含被测快照的源码、教程和registry；
完整Git树的独立对照命令见本目录README。所有文本摘要明确采用UTF-8/LF，ZIP按原始字节核对。
#>
param([string]$RepositoryRoot, [string]$SourceRoot, [string]$ReportDirectory)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (-not $RepositoryRoot) { $RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../../..')).Path }
if (-not $SourceRoot) { $SourceRoot = $RepositoryRoot }
if (-not $ReportDirectory) { $ReportDirectory = Join-Path $PSScriptRoot '41498029910' }
$utf8 = [Text.UTF8Encoding]::new($false, $true)
$modules = @('math', 'hex', 'base64', 'url', 'csv', 'fmt', 'statistics')

function Require([bool]$condition, [string]$message) {
    if (-not $condition) { throw $message }
}
function Child([string]$base, [string]$relative) {
    $prefix = [IO.Path]::GetFullPath($base).TrimEnd([IO.Path]::DirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    $path = [IO.Path]::GetFullPath((Join-Path $base $relative))
    Require ($path.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) "路径越过证据/源码目录：$relative"
    return $path
}
function Digest([byte[]]$bytes) {
    $algorithm = [Security.Cryptography.SHA256]::Create()
    try { return [BitConverter]::ToString($algorithm.ComputeHash($bytes)).Replace('-', '').ToLowerInvariant() }
    finally { $algorithm.Dispose() }
}
function Read-Lf([string]$path) { return $utf8.GetString([IO.File]::ReadAllBytes($path)).Replace("`r`n", "`n") }
function Text-Digest([string]$value) { return Digest ($utf8.GetBytes($value)) }
function Blob-Digest([string]$value) {
    $body = $utf8.GetBytes($value)
    $header = $utf8.GetBytes(('blob ' + $body.Length + [char]0))
    $algorithm = [Security.Cryptography.SHA1]::Create()
    try { return [BitConverter]::ToString($algorithm.ComputeHash([byte[]]($header + $body))).Replace('-', '').ToLowerInvariant() }
    finally { $algorithm.Dispose() }
}
function Check-File([string]$base, [string]$relative, [string]$expected) {
    $bytes = [IO.File]::ReadAllBytes((Child $base $relative))
    Require ((Digest $bytes) -ceq $expected) "文件摘要不符：$relative"
}
function Zip-Bytes($zip, [string]$name) {
    $entry = $zip.GetEntry($name)
    Require ($null -ne $entry) "ZIP缺少原始日志：$name"
    $inputStream = $entry.Open()
    $buffer = [IO.MemoryStream]::new()
    try { $inputStream.CopyTo($buffer); return ,$buffer.ToArray() }
    finally { $inputStream.Dispose(); $buffer.Dispose() }
}
function Log-Lines([byte[]]$raw) { return ,([regex]::Replace($utf8.GetString($raw), '\x1B\[[0-?]*[ -/]*[@-~]', '').Replace("`r`n", "`n") -split "`n") }
function Excerpt($lines, $item) {
    if ($item.PSObject.Properties.Name -contains 'source_lines') {
        $indices = @($item.source_lines)
    } else {
        Require ($item.source_end_line -ge $item.source_start_line) '摘录行段倒置'
        $indices = @($item.source_start_line..$item.source_end_line)
    }
    Require ($indices.Count -gt 0) '摘录没有源行'
    $selected = foreach ($number in $indices) {
        Require ($number -ge 1 -and $number -le $lines.Count) '摘录源行越界'
        $lines[$number - 1].TrimEnd()
    }
    $value = $selected -join "`n"
    if (-not ($item.PSObject.Properties.Name -contains 'source_lines')) { $value = $value.TrimEnd() }
    return $value + "`n"
}
function Between([string]$text, [string]$start, [string]$end) {
    $first = $text.IndexOf($start, [StringComparison]::Ordinal)
    Require ($first -ge 0) "缺少冻结段起点：$start"
    $last = $text.IndexOf($end, $first, [StringComparison]::Ordinal)
    Require ($last -gt $first) "缺少冻结段终点：$end"
    return $text.Substring($first, $last - $first)
}

$manifest = Read-Lf (Child $ReportDirectory 'evidence.json') | ConvertFrom-Json
Require ($manifest.schema_version -eq 1 -and $manifest.runtime_abi -eq '1.18' -and $manifest.codegen_version -eq 20) 'CLOSE清单版本不符'
$snapshot = $manifest.status_snapshot_commit
foreach ($run in @($manifest.maintenance, $manifest.workspace_gates)) {
    Require ($run.status -eq 'completed' -and $run.conclusion -eq 'success' -and $run.head_sha -ceq $snapshot) '门禁不是同一快照的完整成功运行'
}
Require ($manifest.jobs.Count -eq 7) '维护回归六作业和工作区作业必须完整'
foreach ($job in $manifest.jobs) {
    Require ($job.status -eq 'completed' -and $job.conclusion -eq 'success' -and $job.head_sha -ceq $snapshot) '聚合作业状态不符'
}
Check-File $ReportDirectory 'macos-capacity.json' $manifest.first_attempt_capacity_cancellation.sha256
$capacity = Read-Lf (Child $ReportDirectory 'macos-capacity.json') | ConvertFrom-Json
Require ($capacity.run.head_sha -ceq $snapshot -and $capacity.run.conclusion -eq 'failure') '首轮取消的快照或结论不符'
Require ($capacity.macos_job.steps.Count -eq 0 -and $capacity.macos_job.conclusion -eq 'cancelled') '容量取消不应伪装为已执行测试'
Require (@($capacity.jobs | Where-Object conclusion -EQ 'success').Count -eq 5) '首轮其余五作业的成功状态丢失'

$labels = @('vm-linux', 'native-linux', 'native-windows', 'native-macos', 'workspace-gates')
Require ($manifest.logs.Count -eq 5) '必须恰好五份完整成功日志'
$seen = [Collections.Generic.HashSet[string]]::new()
$linesByLabel = @{}
Check-File $ReportDirectory 'ci-logs.zip' $manifest.raw_log_archive_sha256
$zip = [IO.Compression.ZipFile]::OpenRead((Child $ReportDirectory 'ci-logs.zip'))
try {
    Require ($zip.Entries.Count -eq 5) 'ZIP中的日志数不符'
    foreach ($log in $manifest.logs) {
        Require ($log.label -cin $labels -and $seen.Add($log.label)) '完整日志标签重复或未知'
        Require ($log.archive_entry -ceq ($log.label + '-full.log')) '完整日志的ZIP身份不符'
        $origin = @(if ($log.label -in @('vm-linux', 'native-linux', 'native-windows')) {
            $capacity.jobs | Where-Object id -EQ $log.actual_job_id
        } else { $manifest.jobs | Where-Object id -EQ $log.actual_job_id })
        Require ($origin.Count -eq 1) '实际日志作业被聚合作业ID替代或缺失'
        Require ($origin[0].run_attempt -eq $log.actual_attempt -and $origin[0].run_id -eq $log.run_id -and $origin[0].head_sha -ceq $snapshot) '实际作业的尝试/运行/源码不一致'
        $raw = Zip-Bytes $zip $log.archive_entry
        Require ($raw.Length -eq $log.source_bytes -and (Digest $raw) -ceq $log.source_sha256) '完整日志字节或摘要不符'
        $lines = Log-Lines $raw
        Require (($lines -join "`n").Contains($snapshot)) '原始日志缺少快照身份'
        $linesByLabel[$log.label] = $lines
    }
} finally { $zip.Dispose() }
Require ($manifest.excerpts.Count -eq 15) 'CLOSE摘录矩阵不完整'
$excerptsByFile = @{}
foreach ($item in $manifest.excerpts) {
    Require (-not $excerptsByFile.ContainsKey($item.file)) '摘录文件重复'
    $value = Excerpt $linesByLabel[$item.source_label] $item
    Require ((Read-Lf (Child $ReportDirectory $item.file)) -ceq $value -and (Text-Digest $value) -ceq $item.sha256) "摘录/源行/摘要不符：$($item.file)"
    $excerptsByFile[$item.file] = $value
}
foreach ($label in $labels | Where-Object { $_ -ne 'workspace-gates' }) {
    $text = $excerptsByFile[$label + '-exact.log']
    $target = if ($label -eq 'vm-linux') { 'b1_capabilities_have_vm_golden_outputs' } else { 'b1_capabilities_match_native_golden_outputs' }
    Require ([regex]::Matches($text, 'test result:').Count -eq 1 -and $text.Contains('test result: ok. 1 passed; 0 failed; 0 ignored;')) '精确目标不再恰好通过一项'
    Require ([regex]::Matches($text, [regex]::Escape("test $target ... ok")).Count -eq 1) '精确目标名称不符'
    foreach ($module in $modules) {
        Require ([regex]::Matches($text, [regex]::Escape("20O-B0-PROBE module=$module backend=")).Count -eq 1) '历史B0配置的执行次数不符'
        $levels = if ($label -eq 'vm-linux') { @('vm result=success') } else { @('vm/native optimization=0 result=success', 'vm/native optimization=3 result=success') }
        foreach ($level in $levels) {
            Require ([regex]::Matches($text, [regex]::Escape("20O-B1-IMP-PROBE case=$module backend=$level")).Count -eq 1) '产品模块或优化级别缺失/重复'
        }
    }
    if ($label -ne 'vm-linux') {
        $release = $excerptsByFile[$label + '-release.log']
        Require ([regex]::Matches($release, '20O-REM-RELEASE case=').Count -eq 12 -and [regex]::Matches($release, 'order=matched').Count -eq 12) '六组释放向量未完整匹配'
        foreach ($case in @('success', 'with-bounds', 'nested-jumps', 'temporary-return', 'selector-errors', 'loop-body-error')) {
            foreach ($level in @(0, 3)) {
                Require ([regex]::Matches($release, [regex]::Escape("20O-REM-RELEASE case=$case optimization=$level ")).Count -eq 1) '释放向量或优化级别缺失/重复'
            }
        }
    }
}
foreach ($pair in @(@('20o_b1_imp_contract', 9), @('20o_b1_imp_resources', 17), @('20o_b1_imp_vm_performance', 5))) {
    Require ($excerptsByFile['vm-linux-' + $pair[0] + '.log'].Contains("test result: ok. $($pair[1]) passed; 0 failed; 0 ignored;")) '默认契约/资源/性能回归不符'
}
$guards = $excerptsByFile['vm-linux-guards.log']
Require ([regex]::Matches($guards, 'B1-GUARDS ').Count -eq 5) '计数守卫输出重复或缺失'
foreach ($label in @('default', 'Linux', 'macOS', 'Windows')) {
    Require ([regex]::Matches($guards, [regex]::Escape("B1-GUARDS path=$label passed=7 total=7")).Count -eq 1) '计数守卫路径缺失/重复'
}
Require ($guards.Contains('B1-GUARDS passed=28 total=28;')) '计数守卫没有28/28'

Check-File $ReportDirectory 'macos-22c-attempt2-failure.json' $manifest.separate_22c_failure.sha256
$failure = Read-Lf (Child $ReportDirectory 'macos-22c-attempt2-failure.json') | ConvertFrom-Json
Require ($failure.head_sha -ceq $snapshot -and $failure.actual_attempt -eq 2 -and $failure.conclusion -eq 'failure' -and -not $failure.b1_native_exact_executed) '22C失败被核销或混入B1成功证据'
Require ((Blob-Digest (Read-Lf (Child $SourceRoot $failure.source))) -ceq $failure.unchanged_test_git_blob) '22C原断言源码已变'
Check-File $ReportDirectory $failure.raw_archive.file $failure.raw_archive.sha256
$zip = [IO.Compression.ZipFile]::OpenRead((Child $ReportDirectory $failure.raw_archive.file))
try {
    $raw = Zip-Bytes $zip $failure.raw_archive.entry
    Require ((Digest $raw) -ceq $failure.source_sha256) '22C原始失败日志不符'
    $value = Excerpt (Log-Lines $raw) $failure.excerpt
    Require ((Read-Lf (Child $ReportDirectory $failure.excerpt.file)) -ceq $value -and (Text-Digest $value) -ceq $failure.excerpt.sha256) '22C失败摘录不符'
} finally { $zip.Dispose() }

$expectedDocuments = @($modules | ForEach-Object { "docs/UseDocs/library/$_.md" }) + @('docs/module-registry.json')
Require ($manifest.status_documents.Count -eq 8) '七册教程与registry的快照不完整'
$seen.Clear()
foreach ($item in $manifest.status_documents) {
    Require ($item.path -cin $expectedDocuments -and $seen.Add($item.path)) '状态文件缺失、重复或越界'
    $text = Read-Lf (Child $SourceRoot $item.path)
    Require ((Text-Digest $text) -ceq $item.sha256 -and (Blob-Digest $text) -ceq $item.git_blob) "状态文件已偏离被测快照：$($item.path)"
}
$registry = Read-Lf (Child $SourceRoot 'docs/module-registry.json') | ConvertFrom-Json
foreach ($module in $modules) {
    $entry = @($registry.modules | Where-Object id -EQ "stdlib.$module")
    Require ($entry.Count -eq 1 -and $entry[0].status -eq 'verified' -and $entry[0].stage.Contains('20O-B1-CLOSE')) '七库状态不一致'
}
$preflightRoot = Join-Path $PSScriptRoot 'preflight-20261103'
$preflight = Read-Lf (Child $preflightRoot 'evidence.json') | ConvertFrom-Json
Check-File $preflightRoot 'tool-identity.log' $preflight.tool_identity_sha256
$toolEntries = (Read-Lf (Child $preflightRoot 'tool-identity.log')).TrimEnd() -split "`n"
Require ($toolEntries.Count -eq 9) '测量与诊断工具对象清单不完整'
foreach ($line in $toolEntries) {
    Require ($line -cmatch '^100644 blob ([0-9a-f]{40})\t(core/rust/crates/xiao-driver/examples/.+)$') '工具对象行格式不符'
    $blob = $Matches[1]; $path = $Matches[2]
    Require ((Blob-Digest (Read-Lf (Child $SourceRoot $path))) -ceq $blob) "工具源码与df27对象不同：$path"
}
$contract = Between (Read-Lf (Child $SourceRoot 'docs/DevDocs/20o-b1-freeze-draft.md')) '## 三、' '## 八、'
$sections = @{
    tables = (@([regex]::Matches($contract, '(?m)^\|.*$') | ForEach-Object Value) -join "`n")
    signatures_and_csv_formula = (@([regex]::Matches($contract, '```text\n(.*?)```', 'Singleline') | ForEach-Object { $_.Groups[1].Value }) -join "`n")
    measurement_protocol = (Between $contract '已确认的可复跑测量协议：' '## 六、')
    csv_budget_order = (Between $contract '已确认的预算检查顺序：' '若类型、')
    error_priority = (Between $contract '### 6.1 已确认检查优先级' '上表与顺序')
}
foreach ($name in $sections.Keys) {
    Require ((Text-Digest $sections[$name]) -ceq $manifest.f10.frozen_contract_sections.$name -and $manifest.f10.frozen_contract_sections.$name -ceq $preflight.frozen_contract_sections.$name) "冻结摘要不符：$name"
}
$f10Root = Join-Path $RepositoryRoot 'docs/DevDocs/assets/20o-b1-imp/rem-identity-final-41464439400'
Check-File $f10Root 'ci-logs.zip' $manifest.f10.original_log_archive_sha256
Require ($manifest.f10.mode -eq 'carried' -and $manifest.f10.records -eq 721) '沿用F10的边界被改写'
$f10 = ((& (Join-Path $f10Root 'audit.ps1') -ReportDirectory $f10Root -RepositoryRoot $SourceRoot) -join "`n") | ConvertFrom-Json
Require (($f10 | Measure-Object records -Sum).Sum -eq 721 -and @($f10 | Where-Object { -not $_.passed }).Count -eq 0) '原始721条F10未通过'
[ordered]@{ status_snapshot = $snapshot; passed = $true; raw_success_logs = 5; exact_targets = 4; excerpts = 15; release_records = 36; historical_attempts_preserved = 2; status_files = 8; tool_files = 9; frozen_sections = 5; f10_mode = 'carried'; f10_records = 721; performed_measurement = $false } | ConvertTo-Json
