# `xiao-standard-library` 源码

本 crate 当前保留标准库身份常量和源码根解析；`text`、`path` 与 20D 首批 `os` 的规范实现位于顶层 `stdlib/`，不复制到 Rust 源码。

工程期 20C/20D/20E；此目录提供源码根与唯一的标准库身份定义，驱动器保留原重导出。

归档层复用身份做接口/语言版本检查，归档字段及编解码由 `xiao-artifacts`/`xiao-xar` 管理。
本目录不承载 Xiao 模块实现、普通包依赖、归档对象或 Runtime 实现。
