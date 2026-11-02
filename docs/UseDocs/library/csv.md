---
id: library.csv
title: CSV 字段与记录
status: verified
audience: learner
module: stdlib.csv
stage: 20O-B1-IMP-REM-B/20P/20O-B1-CLOSE
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
  - ../../DevDocs/20o-b1-closeout.md
---

# CSV 字段与记录

## 适用范围与状态

严格的CSV逻辑单记录与字符串字段数组互转，不处理全文文件、表头、方言或流式输入。

本页与stdlib.csv登记为verified。适用Xiao 0.1.0、Runtime ABI 1.18、标准库接口版本1。
已验证范围覆盖VM及Windows/Linux/macOS原生O0/O3、资源边界与稳定错误；
[收口记录](../../DevDocs/20o-b1-closeout.md)列出运行号、源码身份与限制。

## 导入与准备

官方库随工具链提供，无需普通包安装。公共路径为std.csv，固定身份为stdlib.csv；
下面的完整示例使用选择导入，所有参数必填，支持既有位置与命名调用规则。
使用包含本批源码的工具链；从仓库根运行示例时，标准库加载根应指向本仓库的stdlib目录。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/20o-b1/imp/csv/product.xiao
```

以下代码与[实际产品夹具](../../../tests/fixtures/20o-b1/imp/csv/product.xiao)逐字一致；
输出由[独立黄金](../../../tests/fixtures/20o-b1/imp/csv/golden.json)核对。

<!-- b1-imp-executable-example -->
```xiao
from std.csv import parse_record, format_record

fields = ["a", "b,c", "d\"e", "", "\r\n", "雪😀", " é ", "﻿bom", "\0"]
alias = fields
print(format_record(fields = fields))
print(alias[1] == "b,c")
for parsed_field in parse_record(record = "a,\"b,c\",\"d\"\"e\",,\"\r\n\",雪😀, é ,﻿bom,\0")
    print("[" + parsed_field + "]")
for known_record in ["", ",", ",a,", "\"\"", "\"a,b\",c", "\"a\"\"b\",\"x\r\ny\"", "﻿ a , b ", "\0,\"é😀\""]
    for known_field in parse_record(known_record)
        print("[" + known_field + "]")
print(format_record([""]) == "")
print(format_record([" ", chr(65279)]))
first = parse_record("a,b")
second = parse_record("a,b")
array_push(first, "z")
second_count = 0
for second_field in second
    second_count = second_count + 1
print(second_count)
try
    print(format_record([]))
catch empty_fields as Error
    print(error_code(empty_fields))
for invalid_record in ["\"", "\"\"\"", "\"a", "a\"b", "\"a\"x", "\"a\" ", "a\nb", "a\rb", "\"a\",b\n", "﻿\"a\"", ",x\"y"]
    try
        print(parse_record(invalid_record))
    catch invalid_syntax as Error
        print(error_code(invalid_syntax))
long_field = "x"
doublings = 0
while doublings < 14
    long_field = long_field + long_field
    doublings = doublings + 1
try
    print(parse_record(long_field + "x"))
catch parse_field_limit as ResourceError
    print(error_code(parse_field_limit))
try
    print(format_record([long_field + "x"]))
catch format_field_limit as ResourceError
    print(error_code(format_field_limit))
quote_field = "\""
quote_doublings = 0
while quote_doublings < 14
    quote_field = quote_field + quote_field
    quote_doublings = quote_doublings + 1
try
    print(format_record([quote_field, quote_field, quote_field, quote_field]))
catch encoded_limit as ResourceError
    print(error_code(encoded_limit))
many_fields = []
field_count = 0
while field_count < 1025
    array_push(many_fields, "")
    field_count = field_count + 1
try
    print(format_record(many_fields))
catch format_count_limit as ResourceError
    print(error_code(format_count_limit))
commas = ","
comma_doublings = 0
while comma_doublings < 10
    commas = commas + commas
    comma_doublings = comma_doublings + 1
try
    print(parse_record(commas + "\""))
catch parse_count_before_syntax as ResourceError
    print(error_code(parse_count_before_syntax))
large = "x"
record_doublings = 0
while record_doublings < 16
    large = large + large
    record_doublings = record_doublings + 1
try
    print(parse_record("\"" + large))
catch input_before_syntax as ResourceError
    print(error_code(input_before_syntax))
print(format_record(parse_record("a,\"b,c\"")))
```

独立黄金输出（用JSON字符串显示CR和NUL等控制字符）：

```json
"a,\"b,c\",\"d\"\"e\",,\"\r\n\",雪😀, é ,﻿bom,\u0000\ntrue\n[a]\n[b,c]\n[d\"e]\n[]\n[\r\n]\n[雪😀]\n[ é ]\n[﻿bom]\n[\u0000]\n[]\n[]\n[]\n[]\n[a]\n[]\n[]\n[a,b]\n[c]\n[a\"b]\n[x\r\ny]\n[﻿ a ]\n[ b ]\n[\u0000]\n[é😀]\ntrue\n ,﻿\n2\nCSV_EMPTY_FIELDS\nCSV_INVALID_RECORD\nCSV_INVALID_RECORD\nCSV_INVALID_RECORD\nCSV_INVALID_RECORD\nCSV_INVALID_RECORD\nCSV_INVALID_RECORD\nCSV_INVALID_RECORD\nCSV_INVALID_RECORD\nCSV_INVALID_RECORD\nCSV_INVALID_RECORD\nCSV_INVALID_RECORD\nCSV_LIMIT_EXCEEDED\nCSV_LIMIT_EXCEEDED\nCSV_LIMIT_EXCEEDED\nCSV_LIMIT_EXCEEDED\nCSV_LIMIT_EXCEEDED\nCSV_LIMIT_EXCEEDED\na,\"b,c\"\n"
```

## 核心概念与所有权

记录不带结束符。引号只能在字段首出现，字段内双引号写成两个双引号；闭引号后只许逗号或结束。引号外CR/LF拒绝，引号内CR/LF/CRLF原样保留。空文本解析为一个空字段；零字段数组格式化报错。BOM、空白和字段内容不裁剪、不推断类型。

输入文本和字段数组只读、不消费；parse_record返回独立的新数组，format_record返回保持有效生命周期的不可变字符串。失败不修改调用方数据，也不返回部分结果；没有需手工关闭的资源。
打印示例沿用stdout能力。

## API

| 已确认签名 | 已确认边界 |
| --- | --- |
| `parse_record(str record) -> [str]` | 无记录结束符；拒绝引号外 CR/LF；严格引号位置；空文本为一个空字段 |
| `format_record([str] fields) -> str` | 有逗号/引号/CR/LF 才加引号，内部引号双写；不追加结束符，[] 报错 |

参数名属于契约。完整签名、边界与检查顺序以[B1冻结契约](../../DevDocs/20o-b1-freeze-draft.md#三七库已确认签名与调用边界f2f8f12)为准。

## 错误与恢复

示例在catch中使用全局自由函数error_code读取原错误码，读取不消费错误对象。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| Error / CSV_INVALID_RECORD | 引号位置错误、未闭引号、闭引号后多余文字或引号外CR/LF | 提供严格单记录，不静默补引号 |
| Error / CSV_EMPTY_FIELDS | format_record的fields为零字段数组 | 一个空字段应使用含空字符串的一项数组 |
| ResourceError / CSV_LIMIT_EXCEEDED | 记录、字段数、单字段、S或E超限 | 按完整编码预算拆分数据 |

调用或静态类型错误沿用X04-TYPE-002，内部动态类型边界沿用X06-RUNTIME-002。
先处理调用/类型和参数域，再按[统一错误顺序](../../DevDocs/20o-b1-freeze-draft.md#61-已确认检查优先级)检查输入、内容与结果预算。
可本地化的message不参与错误身份。error_code的[独立验收](../../DevDocs/20o-b1-fix-b-closeout.md)不开放error.code成员访问。

## 资源、性能与安全边界

记录最多65,536 UTF-8字节、最多1,024字段，每字段解码后最多16,384字节。format_record先检查类型、非空数组、字段数、单字段及原始总量S≤65,536，再检查精确编码预算E=S+Q+2W+(k-1)≤65,536；Q为双引号数，W为需加外引号的字段数，k为字段数。S仅为预检，不能保证编码后仍合规。CR/LF自身不双写；字段含逗号、引号、CR或LF时才加外引号。全部预算通过后才构造结果。

等于上限合法，超过上限整次失败，不静默截断。具体数值见[资源限额](../../DevDocs/20o-b1-freeze-draft.md#四资源限额f9已验收)。

固定源码身份工具的[完整F10](../../DevDocs/assets/20o-b1-imp/rem-identity-final-41464439400/README.md)
共721条，覆盖20入口/31负载：标量记最大规模，其余还测4,096→8,192→16,384；原生O0/O3分记。
受控平台测得翻倍比≤3、最大合法输入VM≤10秒/原生≤2秒、驻留增量上界≤64MiB。
一次预热、五批采样取中位数，编译、输入准备和终端I/O在窗口外；内存按OS历史驻留峰值减当前基线估计，
是包含暂存与输出的保守上界。这些是已测硬件上的验收结果，不是任意机器的时延保证或API超时语义。
详见[测量协议](../../DevDocs/20o-b1-freeze-draft.md#五性能预算与测量协议f10)。

## 示例如何进入 CI

产品示例与历史探测均由既有两个精确目标执行。VM与原生逐字核对stdout、退出码与stderr；
默认契约测试另检查真实导出、逐函数移除、真实.xar重放、普通包隔离、双路径、参数负例和教程源码一致性。

```powershell
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- b1_capabilities_have_vm_golden_outputs --exact --nocapture
# 配置受控LLVM/Runtime工具链后执行：
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- --ignored b1_capabilities_match_native_golden_outputs --exact --nocapture
```

维护回归的四个执行点各要求恰好1 passed、0 failed、0 ignored；批量路径排除这两个名字，计数守卫28/28。
仅修改教程也会触发维护回归，三平台记录与源码身份见[收口证据](../../DevDocs/20o-b1-closeout.md)。

## 兼容与废弃

本模块按标准库接口版本1兼容新增，固定身份不进入普通依赖求解，也不把官方函数体复制进应用归档。
当前Runtime ABI为1.18，生成器20；error_code前置已独立验收。
历史B0/FIX-A探测最初使用ABI1.17，仅用于解释当时能力，不作为本页产品或性能成绩。
后续新增入口、资源限额或错误规则应另立批次；本次状态升级没有修改产品函数、ABI、生成器或预算。

## 故障排查与下一步

导入失败时先检查工具链是否包含本批源码及标准库加载根；不要把官方库安装成普通包。
原生测试缺少LLVM、Runtime或目标平台配置时应明确失败，不能把环境跳过当成功。
遇到数据错误按上表修正参数，遇到资源超限按完整输入与输出预算拆分。
继续阅读[官方库索引](README.md)、[冻结契约](../../DevDocs/20o-b1-freeze-draft.md)和[收口记录](../../DevDocs/20o-b1-closeout.md)。

## 附：B0历史探测

下面保留[B0探测文件](../../../tests/fixtures/docs/csv/probe.xiao)的原文，仍进CI；
它的局部probe函数、B0_*错误与caught-*标签只描述历史，不属于当前公开接口。
正常使用本库应运行本页前面的产品示例。

<!-- b0-executable-example -->
```xiao
# B0 探测引号状态机与同质字段数组；不是完整 CSV 解析器。
from stdlib.text import length

def probe_fields(str record) -> [str]
    fields = []
    field = ""
    quoted = false
    cursor = 0
    count = length(record)
    while cursor < count
        character = record[cursor]
        if character == "\""
            if quoted and cursor + 1 < count and record[cursor + 1] == "\""
                field = field + "\""
                cursor = cursor + 1
            else
                quoted = not quoted
        else
            if character == "," and not quoted
                array_push(fields, field)
                field = ""
            else
                field = field + character
        cursor = cursor + 1
    if quoted
        raise Error(code = "B0_UNCLOSED_QUOTE")
    array_push(fields, field)
    return fields

for value in probe_fields("a,\"b,c\",\"d\"\"e\",")
    print("[" + value + "]")
for empty in probe_fields("")
    print("[" + empty + "]")
for multiline in probe_fields("\"雪\n😀\",x")
    print("[" + multiline + "]")
try
    print(probe_fields("\"unfinished"))
catch unclosed as Error
    print("caught-unclosed-quote")
```
