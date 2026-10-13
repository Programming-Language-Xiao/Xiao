# `xiao-standard-library`

工程期 20C；这是官方标准库的 Rust 元数据 crate，不承载 `text` 或 `path` 的 Xiao 模块实现。

## 职责与边界

- 负责为工具链保留官方库的 crate 落点和后续元数据接口位置。
- 官方库身份与加载边界由 `xiao-driver` 的并列标准库身份块和工具链专用加载器管理。
- 本 crate 不进入 `PackageRegistry` 普通依赖图，不提供普通包配置，不写入 `.xar` 兼容字段，也不发布预编译对象。
- 后续模块实现必须同步本 README、`stdlib/` 下模块 README、UseDocs、DevDocs 阶段索引和兼容元数据。

首批模块源码位于顶层 `stdlib/text.xiao` 与 `stdlib/path.xiao`；本 crate 仍只负责工程落点、兼容身份和加载边界。
