---
id: library.statistics
title: 整数统计（B1 教程草案）
status: planned
audience: learner
module: stdlib.statistics
stage: 20O-B0/20P
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
---

# 整数统计（教程草案）

## 适用范围与状态

D-5 已确认 B1 只做 int 数组的 sum/min/max。均值、中位数、方差、标准差以及浮点/NaN/精度策略留到 B5。

本页为 `planned`：官方 `std.statistics` 尚未交付。完整示例只使用现有语言和已交付模块，
演示 B0 探测，不提供可导入的产品函数。适用基线：Xiao 0.1.0、Runtime ABI 1.17、
标准库接口版本 1。逐平台结果见 [B0 探测记录](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。

## 导入与准备

官方库随工具链提供，无需普通包安装；`std.statistics` 是拟定用户路径，
`stdlib.statistics` 是拟定内部身份。本轮不能导入该模块。已安装 Xiao 且检出仓库时，
可以直接运行下面的探测文件；源码中的 `probe_` 名称仅在这个示例中有效。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/docs/statistics/probe.xiao
```

[完整源码](../../../tests/fixtures/docs/statistics/probe.xiao) 与下列代码由测试逐字核对。

<!-- b0-executable-example -->
```xiao
# B0 探测整数数组归约；D-5 已确认范围，公开签名和错误码尚未冻结。
def probe_sum([int] values) -> int
    total = 0
    for value in values
        total = total + value
    return total

def probe_min([int] values) -> int
    seen = false
    result = 0
    for candidate in values
        if not seen or candidate < result
            result = candidate
        seen = true
    if not seen
        raise Error(code = "B0_EMPTY_STATISTICS")
    return result

def probe_max([int] values) -> int
    seen = false
    result = 0
    for candidate in values
        if not seen or candidate > result
            result = candidate
        seen = true
    if not seen
        raise Error(code = "B0_EMPTY_STATISTICS")
    return result

values = [5, -2, 9, -2]
print(probe_sum(values))
print(probe_min(values))
print(probe_max(values))
print(values[0])
print(probe_sum([]))
try
    print(probe_min([]))
catch empty_min as Error
    print("caught-empty-min")
try
    print(probe_max([]))
catch empty_max as Error
    print("caught-empty-max")
try
    print(probe_sum([9223372036854775807, 1]))
catch overflow as Error
    print("caught-overflow")
```

示例输出 10、-2、9、5、0，然后捕获空 min、空 max 和溢出；5 验证原数组首项未改。

## 核心概念与所有权

sum([])=0；min/max 的空数组报错。按数组顺序累加，中间值溢出仍报错。仅读取输入数组，不排序、不消费或改写调用方数组。

## 候选 API 参考

**以下是供 B1 评审的签名草案，不是可调用接口。** 所有候选均不消费输入句柄，
不改写输入数组；返回字符串或数组为新结果。最终签名、错误身份、上限须经确认。

| 候选签名 | 参数与返回 | 语义或待定边界 |
| --- | --- | --- |
| `sum([int] values) -> int` | 整数和 | 空数组为 0；溢出沿用已有错误 |
| `min([int] values) -> int；max([int] values) -> int` | 最小/最大值 | 空数组报错，公开错误身份待冻结 |

## 错误与恢复

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| `X06-RUNTIME-009` | 既有 int 溢出；专用夹具核对机器身份 | 限制输入，不能自动改为浮点 |
| `B0_EMPTY_STATISTICS` | 本例的空数组探测错误，不是产品错误码 | 先确认输入非空 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。当前 `catch` 绑定的 `error.code` 成员读取尚不可用；
专用探测从驱动器机器结果读取真实 code，不匹配本地化消息。

## 资源、性能与安全边界

归约可一次遍历，额外状态为常量大小。产品数组元素上限、输入别名与大数组性能仍需 B1 验证。

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

D-5 是范围裁定，正式签名、所有权证据和空数组错误身份仍须 B1 冻结。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [整数计算](math.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。
