# `xiao-standard-library` 源码

本 crate 当前保留标准库身份常量和源码根解析；`text`、`path` 与 20D 首批 `os` 的规范实现位于顶层 `stdlib/`，不复制到 Rust 源码。

工程期 20C；此目录只保留标准库元数据 crate 的最小 Rust 入口。

职责：提供可编译的工程边界。禁止在本批加入标准库模块实现、普通包依赖或 `.xar` 兼容字段。
