---
id: library.url
title: URL 组件转义（B1 教程草案）
status: planned
audience: learner
module: stdlib.url
stage: 20O-B0/B1-FIX-A/20P
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
---

# URL 组件转义（教程草案）

## 适用范围与状态

首批契约只处理组件的百分号编码/解码。完整 URL 分解、路径规范化、网络请求与表单编码不在此已确认 API 范围中。

本页为 `planned`：官方 `std.url` 尚未交付。完整示例只使用现有语言和已交付模块，
演示 B0 探测，不提供可导入的产品函数。适用基线：Xiao 0.1.0、Runtime ABI 1.18、
标准库接口版本 1。逐平台结果见 [B0 探测记录](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。

## 导入与准备

官方库随工具链提供，无需普通包安装；`std.url` 是拟定用户路径，
`stdlib.url` 是拟定内部身份。本轮不能导入该模块。已安装 Xiao 且检出仓库时，
可以直接运行下面的探测文件；源码中的 `probe_` 名称仅在这个示例中有效。

## 五分钟运行与完整示例

从仓库根执行：

```powershell
xiao run tests/fixtures/docs/url/probe.xiao
```

[完整源码](../../../tests/fixtures/docs/url/probe.xiao) 与下列代码由测试逐字核对。

<!-- b0-executable-example -->
```xiao
# B0 探测 UTF-8 字节编码所需的码点运算，不是完整 URL 解析器。
from stdlib.text import find

def probe_percent_byte(int byte) -> str
    digits = "0123456789ABCDEF"
    high = byte // 16
    low = byte % 16
    return "%" + digits[high] + digits[low]

snow = int("雪")
print(probe_percent_byte(224 + snow // 4096) + probe_percent_byte(128 + snow // 64 % 64) + probe_percent_byte(128 + snow % 64))
face = int("😀")
print(probe_percent_byte(240 + face // 262144) + probe_percent_byte(128 + face // 4096 % 64) + probe_percent_byte(128 + face // 64 % 64) + probe_percent_byte(128 + face % 64))
print(probe_percent_byte(32) + probe_percent_byte(43) + probe_percent_byte(47))
print(chr((233 - 224) * 4096 + (155 - 128) * 64 + (170 - 128)))
print(chr((240 - 240) * 262144 + (159 - 128) * 4096 + (152 - 128) * 64 + (128 - 128)))
print(find("0123456789ABCDEF", "G"))
print(int("雪"))
```

示例输出 %E9%9B%AA、%F0%9F%98%80、%20%2B%2F，并恢复 雪、😀；最后输出 -1 与码点 38634。

## 核心概念与所有权

百分号后是 UTF-8 字节的两位十六进制。一个雪字需要三个字节，一个 😀 需要四个；str 的索引单位仍是码点。函数返回新字符串，不修改输入。

## 已确认 API 契约（尚未实现）

七项一起冻结已确认；下表是 [B1 逐项契约](../../DevDocs/20o-b1-freeze-draft.md#34-url两项) 的概览，
F1～F13 已由星崽在 FIX-A 逐条确认；所有库函数参数必填，按现有位置/命名规则调用；
输入只读、不消费，失败不改调用方数组。**这些入口尚未实现，不能替换上面的 probe 示例调用。**

| 已确认签名 | 已确认边界 |
| --- | --- |
| `encode_component(str text) -> str` | UTF-8；仅保留 ASCII 字母数字及 -._~；大写 %HH |
| `decode_component(str text) -> str` | %HH 接受十六进制大小写；+ 与普通 Unicode 原样保留；拒绝坏转义/非法 UTF-8 |

## 错误与恢复

B1 已确认的稳定 code、类别、触发条件与检查顺序统一见 [错误表](../../DevDocs/20o-b1-freeze-draft.md#六统一错误表f11已确认)，
不在教程另立错误身份。下面保留的是**当前 B0 探测**的错误与边界，不能当作已交付产品契约。

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| B0 未覆盖 | 残缺百分号、非法十六进制、非法/非最短 UTF-8、代理码点 | 修正输入，不用替换字符吞错 |
| B0 未覆盖 | 输入/输出超限 | 调整调用预算 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。已确认采用自由函数 `error_code`，不开放错误成员访问；
其[精确位置/签名/错误表](../../DevDocs/20o-b1-freeze-draft.md#二error_code-的名称位置与签名)已冻结为独立 FIX-B 前置，仍未实现。
当前示例从驱动器机器结果核对 code，未调用尚未实现的 error_code。

## 资源、性能与安全边界

**F9 的限额与 F10 的验收目标已确认；当前探测没有实现或证明全部产品资源/性能要求。**

encode 输入最多 65,536 UTF-8 字节、输出最多 196,608 ASCII 字节；decode 输入最多 196,608 UTF-8 字节、结果最多 65,536 字节。大小不是码点数。

精确计量、编码膨胀和超限顺序见 [资源限额](../../DevDocs/20o-b1-freeze-draft.md#四资源限额f9已确认产品未验证)。
文本/编码目标为顺序扫描加至多 O(n log n) 的均衡拼接，禁止隐藏的二次复制/扫描；
翻倍比≤3、最大输入 VM≤10s/原生≤2s、增量内存≤64 MiB 是已确认、仍待实测的目标，
计时排除启动/编译/打印，具体方法见 [性能预算](../../DevDocs/20o-b1-freeze-draft.md#五性能预算与测量协议f10)。

本例的十秒看门狗只保护测试；不代表库 API 承诺运行时超时。纯计算不新增宿主能力，打印沿用 stdout。

## 示例如何进入 CI

完整示例仍在 `tests/fixtures/docs/url/probe.xiao`，本页完整代码与文件逐字核对。
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
error_code 已确认在独立 FIX-B 升级至 ABI 1.18，再交付七库；不能把读取前置混在纯库提交中。探测函数不属于公开 API，不进入官方加载器、
普通依赖求解或归档身份。只有产品实现、完整教程、示例、测试与证据均通过，才可升级
为 `verified`；不能仅因本页探测成功就宣称模块完成。

## 故障排查与下一步

严格组件契约已确认；FIX-A 新增 UTF-8 临界/非法序列与百分号对探测，完整组件算法和大小上限仍待实现批。
补充夹具和证据见 [FIX-A 记录](../../DevDocs/20o-b1-fix-a-contract-closeout.md)。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [CSV 字段与记录](csv.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b1-freeze-draft.md)。
