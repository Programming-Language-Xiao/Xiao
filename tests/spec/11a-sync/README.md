# 11A-E2B：同步与安装规格

`valid.json` 与 `errors.json` 由 `xiao-package/tests/e2b_sync.rs` 逐例执行。
每条用例使用独立项目及全局缓存，锁文件/映射由实际 Rust 实现生成。

本批只覆盖本地路径依赖；远程源、版本求解和预编译产物不在 E2B 范围内。
