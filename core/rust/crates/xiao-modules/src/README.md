# `xiao-modules/src`

放置文件模块发现（`discovery.rs`）、导入目标/作用域绑定/依赖图解析（`resolver.rs`）、公开
模型（`model.rs`）和稳定诊断编号（`diagnostics.rs`）。对应工程期 05-A/05-B、08、11A；
不负责配置读取、下载依赖、Runtime 初始化或 CLI。

`lib.rs` 只装配并重导出公开 API。实现模块之间不得循环依赖；跨模块名称必须携带当前源码
模块上下文读取 `Name` 的 UTF-8 文本，不能从任意首个源码文件猜测名称。
