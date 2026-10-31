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

七项一起冻结已确认；下表是 [B1 逐项契约](../../DevDocs/20o-b1-freeze-draft.md#32-hex两项) 的概览，
细项已提交星崽询问，是否确认以该页 F 编号状态为准。所有参数必填，候选规则允许既有位置/命名调用；
输入只读、不消费，失败不改调用方数组。**这些入口尚未实现，不能替换上面的 probe 示例调用。**

| 精确候选签名 | 边界提案 |
| --- | --- |
| `encode([int] bytes) -> str` | 字节 0～255；每字节两位小写、无前缀；空数组变空串 |
| `decode(str text) -> [int]` | 大小写均接受；拒绝奇数长度、空白、0x 和坏字符；空串变空数组 |

## 错误与恢复

B1 候选稳定 code、类别、触发条件与检查顺序统一见 [错误表](../../DevDocs/20o-b1-freeze-draft.md#六统一错误表f11逐码待审)，
不在教程另立错误身份。下面保留的是**当前 B0 探测**的错误与边界，不能当作已交付产品契约。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| `B0_BYTE_RANGE` | 仅探测 256 超出字节范围 | 修正为 0～255 |
| 待冻结 | 奇数长度、非法字符、输入/结果超限 | 不忽略无效字符，不返回半结果 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。已确认采用自由函数 `error_code`，不开放错误成员访问；
其[精确位置/签名/错误表](../../DevDocs/20o-b1-freeze-draft.md#二error_code-的名称位置与签名)仍是待审前置。
当前示例从驱动器机器结果核对 code，未调用尚未实现的 error_code。

## 资源、性能与安全边界

**以下为 B1-F9/F10 的限额与验收提案，当前短探测没有实现或证明这些限制。**

encode 最多 65,536 项、输出最多 131,072 ASCII 字节；decode 输入最多 131,072 UTF-8 字节、结果最多 65,536 项。超限整次失败。

精确计量、编码膨胀和超限顺序见 [资源限额](../../DevDocs/20o-b1-freeze-draft.md#四资源限额f9数字提案未实测)。
文本/编码目标为顺序扫描加至多 O(n log n) 的均衡拼接，禁止隐藏的二次复制/扫描；
翻倍比≤3、最大输入 VM≤10s/原生≤2s、增量内存≤64 MiB 均是待实测目标，
计时排除启动/编译/打印，具体方法见 [性能预算](../../DevDocs/20o-b1-freeze-draft.md#五性能预算与测量协议f10)。

本例的十秒看门狗只保护测试；不代表库 API 承诺运行时超时。纯计算不新增宿主能力，打印沿用 stdout。

## 示例如何进入 CI

完整示例仍在 `tests/fixtures/docs/hex/probe.xiao`，本页完整代码与文件逐字核对。
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

七个库拟作接口版本 1 的兼容扩展，当前 ABI 仍为 1.17；error_code 的后端通路尚未冻结，
不能由纯 Xiao 库的计划推断该前置无需 ABI 变化。探测函数不属于公开 API，不进入官方加载器、
普通依赖求解或归档身份。只有产品实现、完整教程、示例、测试与证据均通过，才可升级
为 `verified`；不能仅因本页探测成功就宣称模块完成。

## 故障排查与下一步

完整解码器、全部 256 字节向量、奇数/坏字符与上限+1 拒绝尚未实现；不能只靠小样本查表认定通过。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [Base64 编码](base64.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b1-freeze-draft.md)。
