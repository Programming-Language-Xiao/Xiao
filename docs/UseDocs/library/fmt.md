---
id: library.fmt
title: 文本宽度与整数格式（B1 教程草案）
status: planned
audience: learner
module: stdlib.fmt
stage: 20O-B0/20P
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
---

# 文本宽度与整数格式（教程草案）

## 适用范围与状态

为填充、对齐和整数进制提供有语义增量的便利 API，不再建立等价于 str(value) 的占位函数。

本页为 `planned`：官方 `std.fmt` 尚未交付。完整示例只使用现有语言和已交付模块，
演示 B0 探测，不提供可导入的产品函数。适用基线：Xiao 0.1.0、Runtime ABI 1.17、
标准库接口版本 1。逐平台结果见 [B0 探测记录](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。

## 导入与准备

官方库随工具链提供，无需普通包安装；`std.fmt` 是拟定用户路径，
`stdlib.fmt` 是拟定内部身份。本轮不能导入该模块。已安装 Xiao 且检出仓库时，
可以直接运行下面的探测文件；源码中的 `probe_` 名称仅在这个示例中有效。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/docs/fmt/probe.xiao
```

[完整源码](../../../tests/fixtures/docs/fmt/probe.xiao) 与下列代码由测试逐字核对。

<!-- b0-executable-example -->
```xiao
# B0 探测码点宽度、填充与整数进制所需操作；不是通用格式化器。
from stdlib.text import length, repeat

def probe_padding(str value, int width, str fill) -> str
    if length(fill) != 1
        raise Error(code = "B0_FILL_WIDTH")
    missing = width - length(value)
    if missing <= 0
        return value
    return repeat(fill, missing) + value

print(probe_padding("雪😀", 4, "."))
print(probe_padding("long", 2, "."))
print(probe_padding("", 3, "0"))
digits = "0123456789abcdef"
number = 255
high = number // 16
low = number % 16
print(digits[high] + digits[low])
negative = -9223372036854775807 - 1
print(str(negative))
try
    print(probe_padding("雪", 3, "ab"))
catch invalid_fill as Error
    print("caught-fill-width")
```

示例输出 ..雪😀、long、000、ff、-9223372036854775808，再输出错误恢复标签。

## 核心概念与所有权

建议宽度按 Unicode 码点数计算，不能等同终端显示列宽或字素簇。填充字符必须恰好一个码点；不提供 locale 规则。输入只读，返回新字符串。

## 候选 API 参考

**以下是供 B1 评审的签名草案，不是可调用接口。** 所有候选均不消费输入句柄，
不改写输入数组；返回字符串或数组为新结果。最终签名、错误身份、上限须经确认。

| 候选签名 | 参数与返回 | 语义或待定边界 |
| --- | --- | --- |
| `pad_left(str text, int width, str fill) -> str；pad_right(str text, int width, str fill) -> str` | 按码点宽度补齐 | 建议宽度不超过原文则原样返回；负宽度策略待冻 |
| `format_int(int value, int radix) -> str` | 整数转指定进制 | 建议 2～36、小写数字、无前缀、负号在最前 |

## 错误与恢复

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| `B0_FILL_WIDTH` | 探测多码点填充被拒绝，非公开错误码 | 使用单码点填充 |
| 待冻结 | 非法进制、宽度或结果超限 | 使用冻结范围内参数 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。当前 `catch` 绑定的 `error.code` 成员读取尚不可用；
专用探测从驱动器机器结果读取真实 code，不匹配本地化消息。

## 资源、性能与安全边界

应避免对最小 int 先取正绝对值。B0 的最小负数示例只验证既有十进制转换，不证明任意进制极值安全；正式算法仍需探测。

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

码点宽度、进制和大小上限是草案；浮点格式化、模板插值与通用对象格式化不在此次探测中。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [整数计算](math.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。
