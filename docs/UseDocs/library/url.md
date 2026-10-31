---
id: library.url
title: URL 组件转义（B1 教程草案）
status: planned
audience: learner
module: stdlib.url
stage: 20O-B0/20P
version: "0.1.0"
related:
  - README.md
  - ../../DevDocs/20o-b0-baseline-and-b1-draft.md
---

# URL 组件转义（教程草案）

## 适用范围与状态

首批草案只处理组件的百分号编码/解码。完整 URL 分解、路径规范化、网络请求与表单编码不在此候选 API 中。

本页为 `planned`：官方 `std.url` 尚未交付。完整示例只使用现有语言和已交付模块，
演示 B0 探测，不提供可导入的产品函数。适用基线：Xiao 0.1.0、Runtime ABI 1.17、
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

## 候选 API 参考

**以下是供 B1 评审的签名草案，不是可调用接口。** 所有候选均不消费输入句柄，
不改写输入数组；返回字符串或数组为新结果。最终签名、错误身份、上限须经确认。

| 候选签名 | 参数与返回 | 语义或待定边界 |
| --- | --- | --- |
| `encode_component(str text) -> str` | UTF-8 百分号编码 | 建议只保留 ASCII 字母、数字及 -._~；使用大写十六进制 |
| `decode_component(str text) -> str` | 百分号字节转 UTF-8 文本 | 建议严格拒绝损坏转义/非法 UTF-8；+ 保持加号 |

## 错误与恢复

| 身份 | 触发条件 | 恢复方式 |
| --- | --- | --- |
| 待冻结 | 残缺百分号、非法十六进制、非法/非最短 UTF-8、代理码点 | 修正输入，不用替换字符吞错 |
| 待冻结 | 输入/输出超限 | 调整调用预算 |

`B0_*` 仅是夹具局部错误，不注册为标准库错误身份。示例中的 `caught-*` 是恢复分支标签，
不能作为稳定错误码证据。当前 `catch` 绑定的 `error.code` 成员读取尚不可用；
专用探测从驱动器机器结果读取真实 code，不匹配本地化消息。

## 资源、性能与安全边界

UTF-8 的一至四字节边界都须验证。B0 只演示三/四字节运算，尚不能证明严格 UTF-8 验证与大输入性能。

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

上面的保留字符、加号语义和严格解码都是待确认草案；不是自动采用某个浏览器或表单行为。

若无法找到完整示例，先确认位于仓库根；原生测试缺少 LLVM/Runtime 配置时应明确失败，
不能把环境跳过当成功。继续阅读 [CSV 字段与记录](csv.md)、
[官方库索引](README.md) 或 [B1 冻结草案](../../DevDocs/20o-b0-baseline-and-b1-draft.md)。
