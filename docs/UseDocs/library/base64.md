---
id: library.base64
title: Base64 编码（B1 教程草案）
status: planned
audience: learner
module: stdlib.base64
stage: 20O-B0/B1-FIX-A/20P
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
---

# Base64 编码（教程草案）

## 适用范围与状态

将字节表示为 ASCII 文本。首批已确认标准和 URL-safe 两种字母表，必须显式选择 url_safe；两种模式均严格补位，MIME 换行后置。

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

输入使用范围为 0～255 的 [int]；不是直接对 Unicode 码点编码。三个字节组成 24 位，再分成四个 6 位索引。解码应产生新数组，不消费输入。

## 已确认 API 契约（尚未实现）

七项一起冻结已确认；下表是 [B1 逐项契约](../../DevDocs/20o-b1-freeze-draft.md#33-base64两项) 的概览，
F1～F13 已由星崽在 FIX-A 逐条确认；所有库函数参数必填，按现有位置/命名规则调用；
输入只读、不消费，失败不改调用方数组。**这些入口尚未实现，不能替换上面的 probe 示例调用。**

| 已确认签名 | 已确认边界 |
| --- | --- |
| `encode([int] bytes, bool url_safe) -> str` | 标准/URL-safe 由必填参数选择；强制补位、不换行；字节 0～255 |
| `decode(str text, bool url_safe) -> [int]` | 严格检查所选模式/字母表、长度、补位和尾位；拒绝混用和空白；空输入互转 |

## 错误与恢复

B1 已确认的稳定 code、类别、触发条件与检查顺序统一见 [错误表](../../DevDocs/20o-b1-freeze-draft.md#六统一错误表f11已确认)，
不在教程另立错误身份。下面保留的是**当前 B0 探测**的错误与边界，不能当作已交付产品契约。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| B0 未覆盖 | 无效字节、非法字符、长度/补位/非规范尾位 | 修正输入；不静默忽略空白 |
| B0 未覆盖 | 输入或解码结果超过上限 | 拒绝整次操作 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。已确认采用自由函数 `error_code`，不开放错误成员访问；
其[精确位置/签名/错误表](../../DevDocs/20o-b1-freeze-draft.md#二error_code-的名称位置与签名)已冻结为独立 FIX-B 前置，仍未实现。
当前示例从驱动器机器结果核对 code，未调用尚未实现的 error_code。

## 资源、性能与安全边界

**F9 的限额与 F10 的验收目标已确认；当前探测没有实现或证明全部产品资源/性能要求。**

encode 最多 65,536 字节整数、输出最多 87,384 ASCII 字节；decode 输入最多 87,384 字节且真实结果最多 65,536 项。仅检查编码长度不够。

精确计量、编码膨胀和超限顺序见 [资源限额](../../DevDocs/20o-b1-freeze-draft.md#四资源限额f9已确认产品未验证)。
文本/编码目标为顺序扫描加至多 O(n log n) 的均衡拼接，禁止隐藏的二次复制/扫描；
翻倍比≤3、最大输入 VM≤10s/原生≤2s、增量内存≤64 MiB 是已确认、仍待实测的目标，
计时排除启动/编译/打印，具体方法见 [性能预算](../../DevDocs/20o-b1-freeze-draft.md#五性能预算与测量协议f10)。

本例的十秒看门狗只保护测试；不代表库 API 承诺运行时超时。纯计算不新增宿主能力，打印沿用 stdout。

## 示例如何进入 CI

完整示例仍在 `tests/fixtures/docs/base64/probe.xiao`，本页完整代码与文件逐字核对。
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

七个库按接口版本 1 兼容新增；B0/FIX-A 探测当前仍在 ABI 1.17 上运行。
error_code 已确认在独立 FIX-B 升级至 ABI 1.18，再交付七库；不能把读取前置混在纯库提交中。探测函数不属于公开 API，不进入官方加载器、
普通依赖求解或归档身份。只有产品实现、完整教程、示例、测试与证据均通过，才可升级
为 `verified`；不能仅因本页探测成功就宣称模块完成。

## 故障排查与下一步

标准和 URL-safe 均已确认首批交付，url_safe 必填；FIX-A 验证模式、四字符补位与尾位，不是完整多组解码器。MIME 换行仍后置。
补充夹具和证据见 [FIX-A 记录](../../DevDocs/20o-b1-fix-a-contract-closeout.md)。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [URL 组件转义](url.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b1-freeze-draft.md)。
