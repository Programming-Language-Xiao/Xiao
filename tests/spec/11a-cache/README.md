# 11A-E1 本地依赖与共享缓存规格快照

本目录描述 E1 的最小静态场景：`XIAO_HOME` 注入、源码对象去重、只读逻辑映射、v1 元数据兼容
和损坏对象隔离。快照只提供阶段标识与场景名称，真实文件树、SHA-256 摘要和环境物化由 Rust
集成测试创建，不执行包代码，不写入用户真实的 `~/.xiao`。

执行入口是 `core/rust/crates/xiao-package/tests/e1_cache.rs`，其中的
`cache_spec_snapshot_is_executed` 会通过 `include_str!` 读取本目录的 `valid.json`；删除入口后，
规格目录不应被视为已覆盖。
