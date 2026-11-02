# B1 F10离线复核：强制四报告矩阵，重算中位数/翻倍比并对照20入口与产品摘要。
param([string]$ReportDirectory = $PSScriptRoot, [string]$RepositoryRoot)
$ErrorActionPreference = 'Stop'
if (-not $RepositoryRoot) { $RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../../../..')).Path }
$expectedCases = @('math.abs','math.min','math.max','math.clamp','math.gcd','math.pow','statistics.sum','statistics.min','statistics.max','fmt.format_int','fmt.pad_left','fmt.pad_right','fmt.scan','hex.encode','hex.decode','base64.encode','base64.encode_url','base64.decode','base64.decode_url','url.encode_ascii','url.encode_unicode','url.decode_ascii','url.decode_unicode','csv.parse_ascii','csv.parse_quotes','csv.parse_multiline','csv.parse_fields','csv.format_ascii','csv.format_quotes','csv.format_multiline','csv.format_fields')
$expectedExports = @('math.abs','math.min','math.max','math.clamp','math.gcd','math.pow','hex.encode','hex.decode','base64.encode','base64.decode','url.encode_component','url.decode_component','csv.parse_record','csv.format_record','fmt.pad_left','fmt.pad_right','fmt.format_int','statistics.sum','statistics.min','statistics.max') | Sort-Object
$reports = @(Get-ChildItem -LiteralPath $ReportDirectory -Filter 'b1-f10-*.json' -File -Recurse)
if ($reports.Count -ne 4) { throw '完整矩阵必须恰好四份报告：VM/Linux与三平台原生' }
$summary = foreach ($file in $reports) {
    $report = Get-Content -LiteralPath $file.FullName -Raw | ConvertFrom-Json
    if (-not $report.complete -or $report.failed -or $report.phase -ne 'all' -or $report.selected -ne 'all') { throw ('报告未完成或失败: '+$file.FullName) }
    if ($report.codegen_version -ne 20 -or $report.runtime_abi_encoded -ne 65554 -or $report.rust_debug_assertions) { throw '版本或构建配置不符' }
    $expectedKeys = [Collections.Generic.HashSet[string]]::new()
    $levels = if ($report.backend -eq 'native') { @(0,3) } elseif ($report.backend -eq 'vm') { @('vm') } else { throw '未知后端' }
    foreach ($case in $expectedCases) {
        $scales = if ($case.StartsWith('math.') -or $case -eq 'fmt.format_int') { @('maximum') } else { @('maximum','4096','8192','16384') }
        foreach ($level in $levels) { foreach ($scale in $scales) { [void]$expectedKeys.Add("$case|$level|$scale") } }
    }
    if ($report.records.Count -ne $expectedKeys.Count) { throw '测量记录数不符' }
    $actualKeys = [Collections.Generic.HashSet[string]]::new()
    $actualExports = @($report.records | ForEach-Object { $_.module+'.'+$_.function } | Sort-Object -Unique)
    if (@(Compare-Object $actualExports $expectedExports).Count) { throw '20入口覆盖不符' }
    foreach ($row in $report.records) {
        $level = if ($report.backend -eq 'vm') { 'vm' } else { [string]$row.optimization }
        $scale = if ($null -eq $row.scale) { 'maximum' } else { [string]$row.scale }
        $key = "$($row.case)|$level|$scale"
        if (-not $expectedKeys.Contains($key) -or -not $actualKeys.Add($key)) { throw ('重复或多余记录: '+$key) }
        if (-not $row.passed -or -not $row.batch_duration_passed -or $row.samples.Count -ne 5 -or $row.repeats -lt 1) { throw ('采样未通过: '+$key) }
        $budget = if ($report.backend -eq 'native') { 2.0 } else { 10.0 }
        $times = @($row.samples | ForEach-Object { if ($_.seconds -lt 0.05) { throw '采样批次不足50ms' }; [double]$_.seconds/[double]$row.repeats } | Sort-Object)
        if ([Math]::Abs($times[2]-$row.median_seconds) -gt 1e-10 -or $row.median_seconds -gt $budget) { throw ('中位数/时间预算不符: '+$key) }
        $peak = ($row.samples | Measure-Object resident_increment_upper_bytes -Maximum).Maximum
        if ($peak -ne $row.resident_increment_upper_bytes -or $peak -gt 67108864) { throw ('内存预算不符: '+$key) }
        if ($null -ne $row.doubling_ratio -and $row.doubling_ratio -gt 3) { throw ('翻倍预算不符: '+$key) }
        if ($row.calibration[-1].sample.seconds -lt 0.1) { throw '校准未达到100ms' }
        for ($i=1; $i -lt $row.calibration.Count; $i++) { if ($row.calibration[$i].repeats -gt 16*$row.calibration[$i-1].repeats) { throw '校准增长超过16倍' } }
        $productPath = Join-Path $RepositoryRoot ('stdlib/'+$row.module+'.xiao')
        if ((Get-FileHash -LiteralPath $productPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $row.product_sha256) { throw ('产品摘要不符: '+$key) }
    }
    foreach ($case in $expectedCases | Where-Object { -not $_.StartsWith('math.') -and $_ -ne 'fmt.format_int' }) {
        foreach ($level in $levels) {
            $rows = @($report.records | Where-Object { $_.case -eq $case -and $null -ne $_.scale -and (($report.backend -eq 'vm') -or $_.optimization -eq $level) } | Sort-Object scale)
            if ($rows.Count -ne 3 -or $null -ne $rows[0].doubling_ratio) { throw '三档规模不完整' }
            for ($i=1; $i -lt 3; $i++) { if ([Math]::Abs($rows[$i].doubling_ratio - $rows[$i].median_seconds/$rows[$i-1].median_seconds) -gt 1e-10) { throw '翻倍比重算不符' } }
        }
    }
    [ordered]@{file=$file.Name; target=$report.target; backend=$report.backend; records=$report.records.Count; functions=$actualExports.Count; maximum_seconds=($report.records|Measure-Object median_seconds -Maximum).Maximum; maximum_doubling_ratio=($report.records|Measure-Object doubling_ratio -Maximum).Maximum; maximum_resident_increment_upper_bytes=($report.records|Measure-Object resident_increment_upper_bytes -Maximum).Maximum; passed=$true}
}
$expectedMatrix=@('native|x86_64-unknown-linux-gnu','native|x86_64-pc-windows-msvc','native|aarch64-apple-darwin','vm|x86_64-unknown-linux-gnu') | Sort-Object
$actualMatrix=@($summary | ForEach-Object { $_.backend+'|'+$_.target } | Sort-Object -Unique)
if (@(Compare-Object $actualMatrix $expectedMatrix).Count) { throw '三平台后端矩阵不完整或重复' }
$summary | ConvertTo-Json -Depth 8
