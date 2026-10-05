# `tools/release`

编排 SHA-256 复核、`.xar` 发布报告、三平台产物和签名/公证流程。工程期 17–19；macOS 签名步骤必须在 macOS/Xcode 或受控 macOS CI 执行。

## 报告入口

发布报告不新增 CLI 顶层命令，统一通过 Rust 核心的 `verify --detail` 生成。CLI 只展示
`release_report` 字段，不在 TypeScript 或本脚本中重算摘要：

```text
xiao verify dist/main.xiaoc --detail --json
xiao verify dist/main.xar --detail --json
```

报告包含整体 SHA-256、`.xiaoc` 对象或 `.xar` 成员摘要、优化配置指纹、依赖锁摘要、工具链、
目标平台、Runtime ABI、可复现性状态和未签名警示。`SHA-256` 只保证完整性，不代表发布者可信。

## 重复构建记录

在同一受控环境对相同输入执行两次构建，分别保存两份产物并运行上述 `verify --detail`；逐字节
相同则白名单为空。任何差异都必须写入报告的 `reproducibility.allowed_differences` 并说明原因。
当前 `verify` 对单份输入返回 `status=not-measured`，没有第二份受控产物时不能把它写成通过。
