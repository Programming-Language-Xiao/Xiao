---
id: library.hex
title: 十六进制编码（B1 教程草案）
status: planned
audience: learner
module: stdlib.hex
stage: 20O-B0/20P
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
---

# 十六进制编码（教程草案）

## 适用范围与状态

把字节序列转换为十六进制文本，以及严格解码。适合表示二进制摘要和协议字段；不承担加密。

本页为 `planned`：官方 `std.hex` 尚未交付。完整示例只使用现有语言和已交付模块，
演示 B0 探测，不提供可导入的产品函数。适用基线：Xiao 0.1.0、Runtime ABI 1.17、
标准库接口版本 1。逐平台结果见 [B0 探测记录](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。

## 导入与准备

官方库随工具链提供，无需普通包安装；`std.hex` 是拟定用户路径，
`stdlib.hex` 是拟定内部身份。本轮不能导入该模块。已安装 Xiao 且检出仓库时，
可以直接运行下面的探测文件；源码中的 `probe_` 名称仅在这个示例中有效。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/docs/hex/probe.xiao
```

[完整源码](../../../tests/fixtures/docs/hex/probe.xiao) 与下列代码由测试逐字核对。

<!-- b0-executable-example -->
```xiao
# B0 字节整数、字符查表与错误传播探测，不是公开编码器。
from stdlib.text import find, lower_ascii

def probe_hex_byte(int value) -> str
    if value < 0 or value > 255
        raise Error(code = "B0_BYTE_RANGE")
    digits = "0123456789abcdef"
    high = value // 16
    low = value % 16
    return digits[high] + digits[low]

encoded = ""
for byte in [0, 1, 15, 16, 127, 128, 255]
    encoded = encoded + probe_hex_byte(byte)
print(encoded)
digits = "0123456789abcdef"
print(find(digits, "f") * 16 + find(digits, lower_ascii("F")))
print(find(digits, "g"))
try
    print(probe_hex_byte(256))
catch invalid_byte as Error
    print("caught-byte-range")
```

示例输出 00010f107f80ff、255、-1 与错误恢复标签，覆盖 0、127、128、255、大小写查表与超范围字节。

## 核心概念与所有权

草案用 [int] 表示字节，每项必须位于 0～255。文本字符是 Unicode 码点，不能把 int("雪") 当成 UTF-8 字节。输入数组只读，解码返回新数组。

## 候选 API 参考

**以下是供 B1 评审的签名草案，不是可调用接口。** 所有候选均不消费输入句柄，
不改写输入数组；返回字符串或数组为新结果。最终签名、错误身份、上限须经确认。

| 候选签名 | 参数与返回 | 语义或待定边界 |
| --- | --- | --- |
| `encode([int] bytes) -> str` | 字节转十六进制 | 建议小写、每字节两位、不加 0x |
| `decode(str text) -> [int]` | 文本转新字节数组 | 建议接受大小写；拒绝奇数长度、空白、非十六进制字符 |

## 错误与恢复

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| `B0_BYTE_RANGE` | 仅探测 256 超出字节范围 | 修正为 0～255 |
| 待冻结 | 奇数长度、非法字符、输入/结果超限 | 不忽略无效字符，不返回半结果 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。当前 `catch` 绑定的 `error.code` 成员读取尚不可用；
专用探测从驱动器机器结果读取真实 code，不匹配本地化消息。

## 资源、性能与安全边界

输入/结果上限和具体错误身份待 B1 冻结。重复拼接字符串可能反复复制；本例的小样本不能证明大输入是线性时间。

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

这里只探测单字节编码和查表原语，未验证完整解码器的奇数长度与失败原子性。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [Base64 编码](base64.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。
