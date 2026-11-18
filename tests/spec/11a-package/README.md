# 11A-D1 包契约规格快照

本目录验证 D1 的本地路径依赖声明、包身份分层、完整内存依赖图，以及包身份冲突、缺失
依赖和依赖环三类稳定诊断。快照只描述 `config.xiao` 文件和期望结果，不包含缓存、锁文件、
远程源或 CLI 行为。

执行入口是 `core/rust/crates/xiao-package/tests/d1_package.rs`。入口会把每个用例写入隔离
临时项目，再调用 `xiao-package::resolve_project`；删除真实加载入口不会被视为完成。
