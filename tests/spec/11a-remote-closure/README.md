# 11A-E3D1 远程闭环规格

`valid.json` 驱动本机目录源的版本、传递依赖和锁定映射验收；
`errors.json` 驱动条件候选保守排除的稳定诊断。
`core/rust/crates/xiao-package/tests/e3d1_remote.rs` 逐例运行，不访问外部网络。
本机 HTTP 的源列表、凭据脱敏、摘要失配与副作用探针由 `e3c_remote.rs` 验证。
