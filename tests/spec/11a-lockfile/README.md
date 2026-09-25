# 11A-E2A 锁文件与环境映射规格快照

`valid.json` 逐例描述可写入隔离项目的真实文件树、完整传递依赖图、分类及锁文件首次创建与
二次复用的预期；`errors.json` 包含前后文件树与独立稳定诊断，覆盖配置、源码内容、来源路径
变化和未来锁文件版本。JSON 不包含真实用户主目录，所有 `XIAO_HOME` 均由测试注入临时路径。

执行入口是 `core/rust/crates/xiao-package/tests/e2a_lockfile.rs`，通过两个 `include_str!`
真实加载并逐例运行本目录的 `valid.json` 和 `errors.json`；删除执行入口即不算覆盖。
