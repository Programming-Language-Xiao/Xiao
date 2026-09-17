# `xiao-driver`

## 目录职责

编排源码读取、前端、优化、字节码/LLVM 后端、Runtime、缓存和归档请求，提供给 TypeScript CLI 的稳定结构化服务接口。

## 工程期

08 建立前端驱动；09/10 接入运行和构建；11–18 接入配置、包、优化、缓存和 `.xar`。

## 模块放置

请求模型、流水线编排、版本协商和服务边界放在 `src/`；终端命令路由放在 `cli/ts/src/commands`。

## 边界

不保存终端编辑状态、不生成本地化文案、不暴露 Rust 内部布局；请求/结果必须带版本、目标、优化和诊断字段。

## 08A/U0 交付

`src/frontend.rs` 提供 `FrontendRequest`、`FrontendContext`、`FrontendCompiler` 和
`FrontendArtifact`。流水线固定调用解析、模块、类型、生命周期和 IR 验证；错误诊断会
累积，错误时不返回 IR。定向规格位于 `tests/u0_frontend.rs`，对应 UseDocs 为
`docs/UseDocs/language/compiler/frontend/README.md`。
