---
id: library.base64
title: Base64 编码（B1 教程草案）
status: planned
audience: learner
module: stdlib.base64
stage: 20O-B0/20P
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
---

# Base64 编码（教程草案）

## 适用范围与状态

将任意字节表示为 ASCII 文本。首批建议标准字母表与等号补位；URL-safe、MIME 换行另行评审。

本页为 `planned`：官方 `std.base64` 尚未交付。完整示例只使用现有语言和已交付模块，
演示 B0 探测，不提供可导入的产品函数。适用基线：Xiao 0.1.0、Runtime ABI 1.17、
标准库接口版本 1。逐平台结果见 [B0 探测记录](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。

## 导入与准备

官方库随工具链提供，无需普通包安装；`std.base64` 是拟定用户路径，
`stdlib.base64` 是拟定内部身份。本轮不能导入该模块。已安装 Xiao 且检出仓库时，
可以直接运行下面的探测文件；源码中的 `probe_` 名称仅在这个示例中有效。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/docs/base64/probe.xiao
```

[完整源码](../../../tests/fixtures/docs/base64/probe.xiao) 与下列代码由测试逐字核对。

<!-- b0-executable-example -->
```xiao
# B0 探测 24 位分组、余数补位及反向查表；不是完整 Base64 API。
from stdlib.text import find

alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
bytes = [102, 111, 111]
packed = bytes[0] * 65536 + bytes[1] * 256 + bytes[2]
a = packed // 262144
b = packed // 4096 % 64
c = packed // 64 % 64
d = packed % 64
print(alphabet[a] + alphabet[b] + alphabet[c] + alphabet[d])
one = bytes[0]
one_a = one // 4
one_b = one % 4 * 16
print(alphabet[one_a] + alphabet[one_b] + "==")
two = bytes[0] * 256 + bytes[1]
two_a = two // 1024
two_b = two // 16 % 64
two_c = two % 16 * 4
print(alphabet[two_a] + alphabet[two_b] + alphabet[two_c] + "=")
decoded = find(alphabet, "Z") * 262144 + find(alphabet, "m") * 4096 + find(alphabet, "9") * 64 + find(alphabet, "v")
print(decoded // 65536)
print(decoded // 256 % 256)
print(decoded % 256)
print(find(alphabet, "!"))
print(find(alphabet, "g") % 16 == 0)
print(find(alphabet, "h") % 16 == 0)
```

独立黄金值为 Zm9v、Zg==、Zm8=、102、111、111、-1、true、false；后三项验证非法字符和尾位判据的原语。

## 核心概念与所有权

输入建议使用范围为 0～255 的 [int]；不是直接对 Unicode 码点编码。三个字节组成 24 位，再分成四个 6 位索引。解码应产生新数组，不消费输入。

## 候选 API 参考

**以下是供 B1 评审的签名草案，不是可调用接口。** 所有候选均不消费输入句柄，
不改写输入数组；返回字符串或数组为新结果。最终签名、错误身份、上限须经确认。

| 候选签名 | 参数与返回 | 语义或待定边界 |
| --- | --- | --- |
| `encode([int] bytes) -> str` | 标准 Base64 编码 | 建议强制补位，无自动换行 |
| `decode(str text) -> [int]` | 严格解码 | 建议校验字母表、补位位置、长度与末尾未使用位 |

## 错误与恢复

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| 待冻结 | 无效字节、非法字符、长度/补位/非规范尾位 | 修正输入；不静默忽略空白 |
| 待冻结 | 输入或解码结果超过上限 | 拒绝整次操作 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。当前 `catch` 绑定的 `error.code` 成员读取尚不可用；
专用探测从驱动器机器结果读取真实 code，不匹配本地化消息。

## 资源、性能与安全边界

24 位分组落在 int 安全范围内。完整数组构造、长度限制与字符串累积成本仍须验证；编码不是保密手段。

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

探测不是完整解码器，没有把非法填充/空白/超限行为自动冻结为产品语义。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [URL 组件转义](url.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。
