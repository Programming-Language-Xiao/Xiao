---
id: library.math
title: 整数计算（B1 教程草案）
status: planned
audience: learner
module: stdlib.math
stage: 20O-B0/20P
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
---

# 整数计算（教程草案）

## 适用范围与状态

整数边界、最大公约数和非负整数幂。首批不涉及浮点、高精度或数值重载。

本页为 `planned`：官方 `std.math` 尚未交付。完整示例只使用现有语言和已交付模块，
演示 B0 探测，不提供可导入的产品函数。适用基线：Xiao 0.1.0、Runtime ABI 1.17、
标准库接口版本 1。逐平台结果见 [B0 探测记录](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。

## 导入与准备

官方库随工具链提供，无需普通包安装；`std.math` 是拟定用户路径，
`stdlib.math` 是拟定内部身份。本轮不能导入该模块。已安装 Xiao 且检出仓库时，
可以直接运行下面的探测文件；源码中的 `probe_` 名称仅在这个示例中有效。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/docs/math/probe.xiao
```

[完整源码](../../../tests/fixtures/docs/math/probe.xiao) 与下列代码由测试逐字核对。

<!-- b0-executable-example -->
```xiao
# B0 能力探测；这些局部函数不是 std.math 的产品实现。
def probe_gcd(int left, int right) -> int
    if left < 0
        left = -left
    if right < 0
        right = -right
    while right != 0
        remainder = left % right
        left = right
        right = remainder
    return left

def probe_power(int base, int exponent) -> int
    if exponent < 0
        raise Error(code = "B0_NEGATIVE_EXPONENT")
    result = 1
    remaining = exponent
    while remaining > 0
        result = result * base
        remaining = remaining - 1
    return result

print(probe_gcd(54, 24))
print(probe_gcd(-54, 24))
print(probe_gcd(0, 0))
print(probe_power(2, 10))
print(probe_power(0, 0))
try
    print(probe_power(2, -1))
catch negative as Error
    print("caught-negative-exponent")
try
    print(probe_power(2, 63))
catch overflow as Error
    print("caught-overflow")
```

示例输出依次为 6、6、0、1024、1，再输出两个错误恢复标签；验证了负数、零、负指数和 2 的 63 次方溢出。

## 核心概念与所有权

int 是固定 64 位有符号整数。值按值传入，没有可关闭资源；结果溢出应报错而非回绕。

## 候选 API 参考

**以下是供 B1 评审的签名草案，不是可调用接口。** 所有候选均不消费输入句柄，
不改写输入数组；返回字符串或数组为新结果。最终签名、错误身份、上限须经确认。

| 候选签名 | 参数与返回 | 语义或待定边界 |
| --- | --- | --- |
| `abs(int value) -> int` | 取绝对值 | 最小负数的绝对值溢出 |
| `min(int left, int right) -> int；max(int left, int right) -> int` | 取较小/较大值 | 相等时数值不变 |
| `clamp(int value, int lower, int upper) -> int` | 限定到闭区间 | lower > upper 的错误身份待冻 |
| `gcd(int left, int right) -> int` | 非负最大公约数 | 零、负数、最小负数的策略待冻 |
| `pow(int base, int exponent) -> int` | 非负整数幂 | 负指数报错；0 的 0 次幂建议 1，待冻 |

## 错误与恢复

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| `X06-RUNTIME-009` | 已有整数溢出；机器身份由专用夹具核对 | 缩小输入，或等待任意精度批次 |
| `B0_NEGATIVE_EXPONENT` | 仅探测负指数拒绝，不是公开错误码 | 示例使用非负指数 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。当前 `catch` 绑定的 `error.code` 成员读取尚不可用；
专用探测从驱动器机器结果读取真实 code，不匹配本地化消息。

## 资源、性能与安全边界

gcd 可用余数迭代；正式 pow 应评估平方求幂，探测中的逐次乘法不作为产品性能承诺。最大负数与中间乘积溢出须在 B1 补逐格验证。

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

abs/clamp 的完整极值表与错误身份尚未冻结，不能把 gcd/pow 探测外推为六个接口均已交付。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [整数统计](statistics.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。
