---
id: library.hex
title: 十六进制编码（B1 教程草案）
status: planned
audience: learner
module: stdlib.hex
stage: 20O-B1-IMP-REM-A/20P
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
---

# 十六进制编码（教程草案）

## 适用范围与状态

把字节序列转换为十六进制文本，以及严格解码。适合表示二进制摘要和协议字段；不承担加密。

本页为 `planned`：`stdlib/hex.xiao` 已提供纯 Xiao 产品草稿。完整 F9/F10、三平台功能与生命周期验收仍需按 B1 收口；本页不提前升级 verified。
适用基线：Xiao 0.1.0、Runtime ABI 1.18、标准库接口版本 1。

## 导入与准备

官方库随工具链提供，无需普通包安装。公共路径为 `std.hex`，固定身份为 `stdlib.hex`：

```xiao
from std.hex import encode, decode
print(encode([0, 255]))
```

输出 `00ff`。下面先保留历史 B0 探测，再给出通过实际模块导入的完整产品示例。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/20o-b1/imp/hex/product.xiao
# 历史探测另行保留：
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

## 完整产品示例（planned）

以下源码与 [产品夹具](../../../tests/fixtures/20o-b1/imp/hex/product.xiao) 逐字一致，通过公开 `std.hex` 路径运行。全字节黄金独立于产品算法；错误读取使用已验收的 `error_code`。

<!-- b1-imp-executable-example -->
```xiao
from std.hex import encode, decode

values = [0, 1, 15, 16, 127, 128, 255]
alias = values
print(encode(values))
print(encode(bytes = [0, 255]))
print(alias[0])
print(encode([]) == "")
for byte in decode(text = "00aAFF")
    print(byte)
empty_count = 0
for empty_byte in decode("")
    empty_count = empty_count + 1
print(empty_count)
all_bytes = []
next_byte = 0
while next_byte < 256
    array_push(all_bytes, next_byte)
    next_byte = next_byte + 1
print(encode(all_bytes))
array_push(all_bytes, 0)
print(encode(all_bytes) == "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff00")
expected_byte = 0
for decoded_byte in decode("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff")
    if decoded_byte != expected_byte
        raise Error(code = "HEX_GOLDEN_MISMATCH")
    expected_byte = expected_byte + 1
print(expected_byte)
first = decode("00")
second = decode("00")
array_push(first, 1)
second_count = 0
for second_byte in second
    second_count = second_count + 1
print(second_count)
bad = [7, 256]
try
    print(encode(bad))
catch invalid_byte as Error
    print(error_code(invalid_byte))
print(bad[1])
try
    print(encode([-1]))
catch negative_byte as Error
    print(error_code(negative_byte))
try
    print(decode("g"))
catch invalid_length as Error
    print(error_code(invalid_length))
try
    print(decode("é"))
catch odd_codepoints as Error
    print(error_code(odd_codepoints))
try
    print(decode("é0"))
catch invalid_unicode as Error
    print(error_code(invalid_unicode))
for invalid_text in ["0x", "00 ", "00\n0", "gg"]
    try
        print(decode(invalid_text))
    catch invalid_character as Error
        print(error_code(invalid_character))
over = [256]
over_count = 0
while over_count < 65536
    array_push(over, 0)
    over_count = over_count + 1
try
    print(encode(over))
catch encode_limit as ResourceError
    print(error_code(encode_limit))
print(over[0])
large = "0"
doublings = 0
while doublings < 17
    large = large + large
    doublings = doublings + 1
try
    print(decode("g" + large))
catch decode_limit as ResourceError
    print(error_code(decode_limit))
for recovered_byte in decode("01")
    print(recovered_byte)
```

## 已确认 API 契约（产品草稿）

七项一起冻结已确认；下表是 [B1 逐项契约](../../DevDocs/20o-b1-freeze-draft.md#32-hex两项) 的概览，
F1～F13 已由星崽在 FIX-A 逐条确认；所有库函数参数必填，按现有位置/命名规则调用；
输入只读、不消费，失败不改调用方数组。产品草稿已提供下列入口；完整资源/性能与三平台验收继续按 B1 收口。

| 已确认签名 | 已确认边界 |
| --- | --- |
| `encode([int] bytes) -> str` | 字节 0～255；每字节两位小写、无前缀；空数组变空串 |
| `decode(str text) -> [int]` | 大小写均接受；拒绝奇数长度、空白、0x 和坏字符；空串变空数组 |

## 错误与恢复

B1 已确认的稳定 code、类别、触发条件与检查顺序统一见 [错误表](../../DevDocs/20o-b1-freeze-draft.md#六统一错误表f11已确认)，
不在教程另立错误身份。产品示例逐项使用 `error_code` 核对下列身份。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| Error / HEX_INVALID_BYTE | encode 元素不在 0～255 | 修正字节值，输入数组不变 |
| Error / HEX_INVALID_LENGTH | decode 码点数为奇数，先于坏字符检查 | 补齐完整的两位字节 |
| Error / HEX_INVALID_CHARACTER | 非十六进制字符，包含前缀、空白和 Unicode | 使用无分隔 ASCII 十六进制 |
| ResourceError / HEX_LIMIT_EXCEEDED | 字节数组>65,536 或文本>131,072 UTF-8 字节 | 按预算拆分输入，整次失败无部分结果 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。全局自由函数 `error_code` 已由独立 FIX-B 实现并验收，
其[精确位置/签名/错误表](../../DevDocs/20o-b1-freeze-draft.md#二error_code-的名称位置与签名)与
[三平台验收记录](../../DevDocs/20o-b1-fix-b-closeout.md)已交付；错误成员访问仍未开放。
FIX-B 已实现并验收；`stdlib.hex` 已有产品草稿。上面的历史 B0 示例继续保留，下面的产品示例通过 `error_code` 核对真实错误身份。

## 资源、性能与安全边界

**F9 的限额与 F10 的验收目标已确认；当前探测没有实现或证明全部产品资源/性能要求。**

encode 最多 65,536 项、输出最多 131,072 ASCII 字节；decode 输入最多 131,072 UTF-8 字节、结果最多 65,536 项。超限整次失败。

精确计量、编码膨胀和超限顺序见 [资源限额](../../DevDocs/20o-b1-freeze-draft.md#四资源限额f9已确认产品未验证)。
文本/编码目标为顺序扫描加至多 O(n log n) 的均衡拼接，禁止隐藏的二次复制/扫描；
翻倍比≤3、最大输入 VM≤10s/原生≤2s、增量内存≤64 MiB 是已确认、仍待实测的目标，
计时排除启动/编译/打印，具体方法见 [性能预算](../../DevDocs/20o-b1-freeze-draft.md#五性能预算与测量协议f10)。

本例的十秒看门狗只保护测试；不代表库 API 承诺运行时超时。纯计算不新增宿主能力，打印沿用 stdout。

## 示例如何进入 CI

历史探测在 `tests/fixtures/docs/hex/probe.xiao`；产品示例在 `tests/fixtures/20o-b1/imp/hex/product.xiao`，本页两个完整代码块分别与文件逐字核对。
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
error_code 已随独立 FIX-B 在 ABI 1.18 实现并验收；hex 产品草稿已接入 IMP，整批验收仍待 CLOSE。
探测函数不属于公开 API，不进入官方加载器、
普通依赖求解或归档身份。只有产品实现、完整教程、示例、测试与证据均通过，才可升级
为 `verified`；不能仅因本页探测成功就宣称模块完成。

## 故障排查与下一步

产品草稿覆盖全部 256 字节的独立黄金、三块归并、大小写、空输入、输入不变和错误优先级；最大合法输入与 F10 按剩余实现规划另测。
补充夹具和证据见 [FIX-A 记录](../../DevDocs/20o-b1-fix-a-contract-closeout.md)。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [Base64 编码](base64.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b1-freeze-draft.md)。
