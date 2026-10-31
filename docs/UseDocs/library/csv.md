---
id: library.csv
title: CSV 字段与记录（B1 教程草案）
status: planned
audience: learner
module: stdlib.csv
stage: 20O-B0/20P
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
---

# CSV 字段与记录（教程草案）

## 适用范围与状态

探测逗号、双引号转义、空字段和引号内换行。全文 CSV 所需的嵌套数组签名目前有语言阻塞，不能将记录片段误称完整解析器。

本页为 `planned`：官方 `std.csv` 尚未交付。完整示例只使用现有语言和已交付模块，
演示 B0 探测，不提供可导入的产品函数。适用基线：Xiao 0.1.0、Runtime ABI 1.17、
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

## 候选 API 参考

**以下是供 B1 评审的签名草案，不是可调用接口。** 所有候选均不消费输入句柄，
不改写输入数组；返回字符串或数组为新结果。最终签名、错误身份、上限须经确认。

| 候选签名 | 参数与返回 | 语义或待定边界 |
| --- | --- | --- |
| `parse_record(str record) -> [str]` | 候选：解析单条逻辑记录 | 引号位置、结束符、错误/大小限制均待冻 |
| `format_record([str] fields) -> str` | 候选：序列化单条逻辑记录 | 建议必要时加引号、内部引号双写；记录结束符待冻 |

## 错误与恢复

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| `B0_UNCLOSED_QUOTE` | 探测未闭合引号，非公开错误码 | 补齐引号 |
| `X04-PARSE-005` | 当前 [[str]] 函数注解被共同前端拒绝 | 等待签名方案裁定 |
| 待冻结 | 非法引号位置、额外记录、字段数/长度上限 | 不默默丢弃字段或记录 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。当前 `catch` 绑定的 `error.code` 成员读取尚不可用；
专用探测从驱动器机器结果读取真实 code，不匹配本地化消息。

## 资源、性能与安全边界

本例只验证状态转换，故意没有冒充严格 RFC 4180 解析器：未拒绝所有非法引号位置。产品还要明确 BOM、CRLF/LF、空文件和末尾空记录的区别。

探测每次执行限制为十秒；这是测试保护，不是产品 API 超时承诺。
本批没有新增能力需求，示例打印沿用 `stdout`；生产限额须在 B1 冻结表中逐项补齐。

## 示例如何进入 CI

`20o_b1_probe` 默认测试运行全部七份示例，并校验本页的完整代码与源码一致。
同名测试目标中的受控原生测试由已有三平台维护回归的 workspace ignored 命令发现；
未取得该提交的远端运行号前，不声明三平台验证完成。

```powershell
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe
```

VM 与原生各自逐字对比独立黄金 stdout；成功退出码为 0，原生 stderr 必须为空。
受控原生的环境配置、精确执行和计数命令见 [B0 记录](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。

## 兼容与废弃

B1 草案计划保持 ABI 1.17 与接口版本 1。探测函数不属于公开 API，不进入官方加载器、
普通依赖求解或归档身份。只有产品实现、完整教程、示例、测试与证据均通过，才可升级
为 `verified`；不能仅因本页探测成功就宣称模块完成。

## 故障排查与下一步

是否先交付记录 API、采用其他结果表示，或单立语言前置，必须请星崽裁定；B0 不擅自缩小已确认的 csv 模块范围。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [文本宽度与整数格式](fmt.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。
