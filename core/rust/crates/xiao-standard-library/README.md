# `xiao-standard-library`

工程期 20C；这是官方标准库的 Rust 元数据 crate，不承载 `text` 或 `path` 的 Xiao 模块实现。

## 职责与边界

- 负责为工具链保留官方库的 crate 落点和后续元数据接口位置。
- `StandardLibraryIdentity`、接口版本常量及 `current_standard_library_identity()` 由本 crate 提供，驱动器保留原有重导出；归档层与驱动器共享同一身份。VM 通过 `stdlib.` 固定命名空间加载源码，LLVM 将纯 Xiao 导入函数静态并入原生入口。
- 随工具链分发的源码根可由 `XIAO_STDLIB_ROOT` 指定；未设置时定位仓库顶层 `stdlib/`。
- 本 crate 不进入 `PackageRegistry` 普通依赖图，不提供普通包配置，也不发布预编译对象；20E 由 `xiao-xar` 将身份的接口版本/语言版本写入归档要求。
- 后续模块实现必须同步本 README、`stdlib/` 下模块 README、UseDocs、DevDocs 阶段索引和兼容元数据；当前第二批 `text.length` 沿用既有 `text` 模块落点。

首批模块源码位于顶层 `stdlib/text.xiao`、`stdlib/path.xiao` 与 `stdlib/set.xiao`；
本 crate 仍只负责工程落点、兼容身份和加载边界。`set` 的通用参数在生成期擦除，
不需要新增 Runtime ABI 或 intrinsic。
