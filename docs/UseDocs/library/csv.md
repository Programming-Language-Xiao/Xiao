---
id: library.csv
title: CSV 字段与记录（B1 教程草案）
status: planned
audience: learner
module: stdlib.csv
stage: 20O-B1-IMP-REM-B/20P
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
---

# CSV 字段与记录（教程草案）

## 适用范围与状态

已确认首批仅做 parse_record/format_record 记录子集，全文 CSV 与 [[str]] 后置。本页完整示例仍是 B0 对逗号、双引号、空字段和字段内换行的能力探测。

本页为 planned：纯 Xiao 的逻辑单记录产品草稿已落地。全文 CSV、流式和表头后置；完整资源/性能、三平台和生命周期验收仍按 B1 收口。
适用 Xiao 0.1.0、Runtime ABI 1.18、标准库接口版本 1。

## 导入与准备

官方库随工具链提供，无需普通包安装。公共路径 std.csv，固定身份 stdlib.csv：

```xiao
from std.csv import parse_record, format_record
print(format_record(["a", "b,c"]))
```

输出 a,"b,c"，不追加记录结束符。历史 B0 探测保留，完整产品示例另列。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/20o-b1/imp/csv/product.xiao
# 历史探测另行保留：
xiao run tests/fixtures/docs/csv/probe.xiao
```

[完整源码](../../../tests/fixtures/docs/csv/probe.xiao) 与下列代码由测试逐字核对。

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

示例保留 b,c、d"e、尾空字段与 雪/😀 之间的字段内换行，并捕获未闭合引号。另一个真实夹具记录嵌套数组注解的前端拒绝。

## 核心概念与所有权

一条逻辑记录可以在带引号字段中含换行；不能先按换行切文件。字段是字符串，首版不隐式识别数字或日期。输入只读，结果数组新建，普通容器不隐式跨线程。

## 完整产品示例（planned）

以下源与 [产品夹具](../../../tests/fixtures/20o-b1/imp/csv/product.xiao) 逐字一致，使用实际公开模块。字段/记录黄金独立于产品算法，错误通过 error_code 核对；最后一个合法往返不替代独立向量。

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

## 已确认 API 契约（产品草稿）

七项一起冻结已确认；下表是 [B1 逐项契约](../../DevDocs/20o-b1-freeze-draft.md#35-csv两项已确认记录子集) 的概览，
F1～F13 已由星崽在 FIX-A 逐条确认；所有库函数参数必填，按现有位置/命名规则调用；
输入只读、不消费，失败不改调用方数组。产品草稿已提供下列入口；整体验收仍按 B1 收口。

| 已确认签名 | 已确认边界 |
| --- | --- |
| `parse_record(str record) -> [str]` | 无记录结束符；拒绝引号外 CR/LF；严格引号位置；空文本为一个空字段 |
| `format_record([str] fields) -> str` | 有逗号/引号/CR/LF 才加引号，内部引号双写；不追加结束符，[] 报错 |

## 错误与恢复

B1 已确认的稳定 code、类别、触发条件与检查顺序统一见 [错误表](../../DevDocs/20o-b1-freeze-draft.md#六统一错误表f11已确认)，
不在教程另立错误身份。产品示例核对以下身份，历史 B0 错误标签另保留。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| Error / CSV_INVALID_RECORD | 错误引号位置、未闭引号、闭引号后多余文字、引号外 CR/LF | 使用严格逻辑单记录，不静默补引号 |
| Error / CSV_EMPTY_FIELDS | format_record([]) | 一个空字段应显式使用 [""] |
| ResourceError / CSV_LIMIT_EXCEEDED | 记录、字段数、单字段、S 或 E 超限 | 按预算拆分，失败不返回部分结果 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。全局自由函数 `error_code` 已由独立 FIX-B 实现并验收，
其[精确位置/签名/错误表](../../DevDocs/20o-b1-freeze-draft.md#二error_code-的名称位置与签名)与
[三平台验收记录](../../DevDocs/20o-b1-fix-b-closeout.md)已交付；错误成员访问仍未开放。
FIX-B 已实现并验收；stdlib.csv 已提供单记录产品草稿，完整产品示例调用公开函数和 error_code。

## 资源、性能与安全边界

**F9 的限额与 F10 的验收目标已确认；当前探测没有实现或证明全部产品资源/性能要求。**

记录/字段数/单字段的已确认限额分别为 65,536 字节、1,024 字段、16,384 字节。
P3-1 已确认：format 的原始字段总量≤65,536 **只是预检**；还须校验精确编码长度
E=S+Q+2W+(字段数−1)≤65,536（S 为原始字节总量、Q 为双引号数、W 为需外引号的字段数）。
先查类型/空字段数组及原始大小，再查编码预算；任何预算超限统一报 CSV_LIMIT_EXCEEDED，
不返回部分结果。完整规则见 [CSV 编码预算](../../DevDocs/20o-b1-freeze-draft.md#41-csv-编码预算)。
字段数/单字段数值与 BOM/空白/字段内换行规则也已在 F6/F9/F12 确认，产品尚未实现。

精确计量、编码膨胀和超限顺序见 [资源限额](../../DevDocs/20o-b1-freeze-draft.md#四资源限额f9已确认产品未验证)。
文本/编码目标为顺序扫描加至多 O(n log n) 的均衡拼接，禁止隐藏的二次复制/扫描；
翻倍比≤3、最大输入 VM≤10s/原生≤2s、增量内存≤64 MiB 是已确认、仍待实测的目标，
计时排除启动/编译/打印，具体方法见 [性能预算](../../DevDocs/20o-b1-freeze-draft.md#五性能预算与测量协议f10)。

本例的十秒看门狗只保护测试；不代表库 API 承诺运行时超时。纯计算不新增宿主能力，打印沿用 stdout。

## 示例如何进入 CI

历史探测位于 tests/fixtures/docs/csv/probe.xiao，产品位于 tests/fixtures/20o-b1/imp/csv/product.xiao；均与本页完整代码逐字核对。
维护回归默认作业独立精确执行 VM 探测，Windows/Linux/macOS 原生作业独立精确执行原生探测；
对应批量命令排除这两个名字，计数须恰好 1 passed、0 failed、0 ignored，命令失败不得吞掉。
文档一致性与错误/语言边界仍由默认 workspace 的其余测试执行。

```powershell
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- b1_capabilities_have_vm_golden_outputs --exact --nocapture
# 配置受控工具链后执行：
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- --ignored b1_capabilities_match_native_golden_outputs --exact --nocapture
```

仅修改本目录教程也会触发维护回归。源码/输出逐字比较和退出码要求不变；
接线与实际取得的证据见 [B1 CI 记录](../../DevDocs/20o-b1-freeze-draft.md#八本轮-ci-接线与验证)。
没有对应提交的三平台证据时仍为 planned。

## 兼容与废弃

七个库按接口版本 1 兼容新增；历史 B0/FIX-A 证据基于 ABI 1.17，当前仓库探测使用 1.18。
error_code 已随独立 FIX-B 在 ABI 1.18 实现并验收；CSV 记录产品草稿已进入 IMP，整批验收留待 CLOSE。
探测函数不属于公开 API，不进入官方加载器、
普通依赖求解或归档身份。只有产品实现、完整教程、示例、测试与证据均通过，才可升级
为 `verified`；不能仅因本页探测成功就宣称模块完成。

## 故障排查与下一步

记录子集产品草稿已实现；最大 ASCII/Unicode/全引号/多换行的真实解析和格式化、字段数、E
等号及超限均进入资源回归。完整 F10 和生命周期仍待收口，历史算术探测不代替产品性能。
补充夹具和证据见 [FIX-A 记录](../../DevDocs/20o-b1-fix-a-contract-closeout.md)。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [文本宽度与整数格式](fmt.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b1-freeze-draft.md)。
