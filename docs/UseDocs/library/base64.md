---
id: library.base64
title: Base64 编码（B1 教程草案）
status: planned
audience: learner
module: stdlib.base64
stage: 20O-B1-IMP-REM-A/20P
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
---

# Base64 编码（教程草案）

## 适用范围与状态

将字节表示为 ASCII 文本。首批已确认标准和 URL-safe 两种字母表，必须显式选择 url_safe；两种模式均严格补位，MIME 换行后置。

本页为 `planned`：纯 Xiao 产品草稿已提供标准与 URL-safe 两种模式。完整 F9/F10、三平台及生命周期验收仍按 B1 收口，不提前标 verified。
适用基线：Xiao 0.1.0、Runtime ABI 1.18、标准库接口版本 1。

## 导入与准备

官方库随工具链提供，无需普通包安装。公共路径为 std.base64，固定身份为 stdlib.base64，模式必须显式提供：

```xiao
from std.base64 import encode, decode
print(encode([102], false))
```

输出 Zg==。历史 B0 探测独立保留，下面另有完整产品示例。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/20o-b1/imp/base64/product.xiao
# 历史探测另行保留：
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

输入使用范围为 0～255 的 [int]；不是直接对 Unicode 码点编码。三个字节组成 24 位，再分成四个 6 位索引。解码应产生新数组，不消费输入。

## 完整产品示例（planned）

以下源与 [产品夹具](../../../tests/fixtures/20o-b1/imp/base64/product.xiao) 逐字一致，通过公开导入运行。RFC 与全字节黄金独立于产品算法，错误通过已验收的 error_code 核对。

<!-- b1-imp-executable-example -->
```xiao
from std.base64 import encode, decode

for sample in ["", "f", "fo", "foo", "foob", "fooba", "foobar"]
    sample_bytes = []
    for character in sample
        array_push(sample_bytes, int(character))
    print(encode(sample_bytes, false))
    print(encode(sample_bytes, true))
for encoded_text in ["", "Zg==", "Zm8=", "Zm9v", "Zm9vYg==", "Zm9vYmE=", "Zm9vYmFy"]
    restored_text = ""
    for decoded_byte in decode(encoded_text, false)
        restored_text = restored_text + chr(decoded_byte)
    print(restored_text)
values = [251, 255, 255]
alias = values
print(encode(values, false))
print(encode(values, true))
print(encode(url_safe = true, bytes = [251, 255]))
print(alias[0])
for standard_byte in decode(text = "+///", url_safe = false)
    print(standard_byte)
for safe_byte in decode(url_safe = true, text = "-___")
    print(safe_byte)
for zero_byte in decode("AA==", false)
    print(zero_byte)
empty_count = 0
for empty_byte in decode("", true)
    empty_count = empty_count + 1
print(empty_count)
all_values = []
next_byte = 0
while next_byte < 256
    array_push(all_values, next_byte)
    next_byte = next_byte + 1
print(encode(all_values, false))
print(encode(all_values, true))
expected_byte = 0
for whole_byte in decode("AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn+AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6ChoqOkpaanqKmqq6ytrq+wsbKztLW2t7i5uru8vb6/wMHCw8TFxsfIycrLzM3Oz9DR0tPU1dbX2Nna29zd3t/g4eLj5OXm5+jp6uvs7e7v8PHy8/T19vf4+fr7/P3+/w==", false)
    if whole_byte != expected_byte
        raise Error(code = "BASE64_GOLDEN_MISMATCH")
    expected_byte = expected_byte + 1
print(expected_byte)
safe_count = 0
for whole_safe_byte in decode("AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0-P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn-AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6ChoqOkpaanqKmqq6ytrq-wsbKztLW2t7i5uru8vb6_wMHCw8TFxsfIycrLzM3Oz9DR0tPU1dbX2Nna29zd3t_g4eLj5OXm5-jp6uvs7e7v8PHy8_T19vf4-fr7_P3-_w==", true)
    if whole_safe_byte != safe_count
        raise Error(code = "BASE64_SAFE_GOLDEN_MISMATCH")
    safe_count = safe_count + 1
print(safe_count)
extra_count = 0
while extra_count < 129
    array_push(all_values, 0)
    extra_count = extra_count + 1
print(encode(all_values, false) == "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn+AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6ChoqOkpaanqKmqq6ytrq+wsbKztLW2t7i5uru8vb6/wMHCw8TFxsfIycrLzM3Oz9DR0tPU1dbX2Nna29zd3t/g4eLj5OXm5+jp6uvs7e7v8PHy8/T19vf4+fr7/P3+/wAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==")
first = decode("AA==", false)
second = decode("AA==", false)
array_push(first, 255)
second_count = 0
for second_byte in second
    second_count = second_count + 1
print(second_count)
bad = [7, 256]
try
    print(encode(bad, false))
catch bad_byte as Error
    print(error_code(bad_byte))
print(bad[1])
try
    print(encode([-1], true))
catch negative_byte as Error
    print(error_code(negative_byte))
for invalid_text in ["Zg", "Zg=", "Zg===", " Zg==", "Zg==\n", "A===", "=AAA", "AA=A", "Zg==AAAA", "Zh==", "Zm9=", "-___", "AA_/", "éAAA", "AA A", "!!!!", "AB=="]
    try
        print(decode(invalid_text, false))
    catch invalid_encoding as Error
        print(error_code(invalid_encoding))
for invalid_safe_text in ["+///", "+___", "/w==", "AA+/", "Zh=="]
    try
        print(decode(invalid_safe_text, true))
    catch invalid_safe_encoding as Error
        print(error_code(invalid_safe_encoding))
over = [256]
over_count = 0
while over_count < 65536
    array_push(over, 0)
    over_count = over_count + 1
try
    print(encode(over, true))
catch encode_limit as ResourceError
    print(error_code(encode_limit))
print(over[0])
large = ""
block = "A"
remaining = 87385
while remaining > 0
    if remaining % 2 == 1
        large = large + block
    remaining = remaining // 2
    if remaining > 0
        block = block + block
try
    print(decode(large, false))
catch decode_limit as ResourceError
    print(error_code(decode_limit))
for recovered_byte in decode("AQ==", true)
    print(recovered_byte)
```

## 已确认 API 契约（产品草稿）

七项一起冻结已确认；下表是 [B1 逐项契约](../../DevDocs/20o-b1-freeze-draft.md#33-base64两项) 的概览，
F1～F13 已由星崽在 FIX-A 逐条确认；所有库函数参数必填，按现有位置/命名规则调用；
输入只读、不消费，失败不改调用方数组。产品草稿已提供下列入口，资源与整体验收继续按 B1 收口。

| 已确认签名 | 已确认边界 |
| --- | --- |
| `encode([int] bytes, bool url_safe) -> str` | 标准/URL-safe 由必填参数选择；强制补位、不换行；字节 0～255 |
| `decode(str text, bool url_safe) -> [int]` | 严格检查所选模式/字母表、长度、补位和尾位；拒绝混用和空白；空输入互转 |

## 错误与恢复

B1 已确认的稳定 code、类别、触发条件与检查顺序统一见 [错误表](../../DevDocs/20o-b1-freeze-draft.md#六统一错误表f11已确认)，
不在教程另立错误身份。产品示例使用 error_code 核对以下稳定身份。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| Error / BASE64_INVALID_BYTE | 字节不在 0～255 | 修正输入，原数组不变 |
| Error / BASE64_INVALID_ENCODING | 非规范长度/所选字母表/补位/尾位，包含空白 | 使用匹配模式的规范编码，不自动纠正 |
| ResourceError / BASE64_LIMIT_EXCEEDED | 字节数组、编码文本或真实结果超限 | 按冻结预算拆分，整次失败无部分结果 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。全局自由函数 `error_code` 已由独立 FIX-B 实现并验收，
其[精确位置/签名/错误表](../../DevDocs/20o-b1-freeze-draft.md#二error_code-的名称位置与签名)与
[三平台验收记录](../../DevDocs/20o-b1-fix-b-closeout.md)已交付；错误成员访问仍未开放。
FIX-B 已实现并验收；stdlib.base64 已有产品草稿。历史 B0 示例保留，完整产品示例调用公开函数并核对错误身份。

## 资源、性能与安全边界

**F9 的限额与 F10 的验收目标已确认；当前探测没有实现或证明全部产品资源/性能要求。**

encode 最多 65,536 字节整数、输出最多 87,384 ASCII 字节；decode 输入最多 87,384 字节且真实结果最多 65,536 项。仅检查编码长度不够。

精确计量、编码膨胀和超限顺序见 [资源限额](../../DevDocs/20o-b1-freeze-draft.md#四资源限额f9已确认产品未验证)。
文本/编码目标为顺序扫描加至多 O(n log n) 的均衡拼接，禁止隐藏的二次复制/扫描；
翻倍比≤3、最大输入 VM≤10s/原生≤2s、增量内存≤64 MiB 是已确认、仍待实测的目标，
计时排除启动/编译/打印，具体方法见 [性能预算](../../DevDocs/20o-b1-freeze-draft.md#五性能预算与测量协议f10)。

本例的十秒看门狗只保护测试；不代表库 API 承诺运行时超时。纯计算不新增宿主能力，打印沿用 stdout。

## 示例如何进入 CI

历史探测在 tests/fixtures/docs/base64/probe.xiao，产品示例在 tests/fixtures/20o-b1/imp/base64/product.xiao；两个完整代码块分别与文件逐字核对。
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
error_code 已随独立 FIX-B 在 ABI 1.18 实现并验收；Base64 产品草稿已接入 IMP，整体验收留待 CLOSE。
探测函数不属于公开 API，不进入官方加载器、
普通依赖求解或归档身份。只有产品实现、完整教程、示例、测试与证据均通过，才可升级
为 `verified`；不能仅因本页探测成功就宣称模块完成。

## 故障排查与下一步

标准和 URL-safe 的完整多组产品草稿已落地，url_safe 必填；RFC/全字节、严格补位/尾位与
资源边界进入实际产品回归。MIME 换行仍后置，完整 F10 与生命周期验收另行收口。
补充夹具和证据见 [FIX-A 记录](../../DevDocs/20o-b1-fix-a-contract-closeout.md)。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [URL 组件转义](url.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b1-freeze-draft.md)。
