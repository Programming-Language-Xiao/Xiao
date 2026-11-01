---
id: library.csv
title: CSV 字段与记录（B1 教程草案）
status: planned
audience: learner
module: stdlib.csv
stage: 20O-B0/B1-FIX-A/20P
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
---

# CSV 字段与记录（教程草案）

## 适用范围与状态

已确认首批仅做 parse_record/format_record 记录子集，全文 CSV 与 [[str]] 后置。本页完整示例仍是 B0 对逗号、双引号、空字段和字段内换行的能力探测。

本页为 `planned`：官方 `std.csv` 尚未交付。完整示例只使用现有语言和已交付模块，
演示 B0 探测，不提供可导入的产品函数。适用基线：Xiao 0.1.0、Runtime ABI 1.18、
标准库接口版本 1。逐平台结果见 [B0 探测记录](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。

## 导入与准备

官方库随工具链提供，无需普通包安装；`std.csv` 是拟定用户路径，
`stdlib.csv` 是拟定内部身份。本轮不能导入该模块。已安装 Xiao 且检出仓库时，
可以直接运行下面的探测文件；源码中的 `probe_` 名称仅在这个示例中有效。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
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

## 已确认 API 契约（尚未实现）

七项一起冻结已确认；下表是 [B1 逐项契约](../../DevDocs/20o-b1-freeze-draft.md#35-csv两项已确认记录子集) 的概览，
F1～F13 已由星崽在 FIX-A 逐条确认；所有库函数参数必填，按现有位置/命名规则调用；
输入只读、不消费，失败不改调用方数组。**这些入口尚未实现，不能替换上面的 probe 示例调用。**

| 已确认签名 | 已确认边界 |
| --- | --- |
| `parse_record(str record) -> [str]` | 无记录结束符；拒绝引号外 CR/LF；严格引号位置；空文本为一个空字段 |
| `format_record([str] fields) -> str` | 有逗号/引号/CR/LF 才加引号，内部引号双写；不追加结束符，[] 报错 |

## 错误与恢复

B1 已确认的稳定 code、类别、触发条件与检查顺序统一见 [错误表](../../DevDocs/20o-b1-freeze-draft.md#六统一错误表f11已确认)，
不在教程另立错误身份。下面保留的是**当前 B0 探测**的错误与边界，不能当作已交付产品契约。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| `B0_UNCLOSED_QUOTE` | 探测未闭合引号，非公开错误码 | 补齐引号 |
| `X04-PARSE-005` | 当前 [[str]] 函数注解被共同前端拒绝 | 全文 CSV 已确认后置 |
| B0 未覆盖 | 非法引号位置、额外记录、字段数/长度上限 | 不默默丢弃字段或记录 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。已确认采用自由函数 `error_code`，不开放错误成员访问；
其[精确位置/签名/错误表](../../DevDocs/20o-b1-freeze-draft.md#二error_code-的名称位置与签名)已冻结为独立 FIX-B 前置，仍未实现。
当前示例从驱动器机器结果核对 code，未调用尚未实现的 error_code。

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

完整示例仍在 `tests/fixtures/docs/csv/probe.xiao`，本页完整代码与文件逐字核对。
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
error_code 已确认在独立 FIX-B 升级至 ABI 1.18，再交付七库；不能把读取前置混在纯库提交中。探测函数不属于公开 API，不进入官方加载器、
普通依赖求解或归档身份。只有产品实现、完整教程、示例、测试与证据均通过，才可升级
为 `verified`；不能仅因本页探测成功就宣称模块完成。

## 故障排查与下一步

严格记录子集、字段/字节限额及 E 预算均已确认。FIX-A 验证短记录计数和大数值预算边界；全量扫描曾触发本机看门狗，不能以算术探测代替性能证据。
补充夹具和证据见 [FIX-A 记录](../../DevDocs/20o-b1-fix-a-contract-closeout.md)。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [文本宽度与整数格式](fmt.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b1-freeze-draft.md)。
