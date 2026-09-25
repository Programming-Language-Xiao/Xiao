# 11A-E3B 缓存与快速路径规格

`valid.json` 的配置模板、两条实际源路径占位符、候选版本与期望结果由
`core/rust/crates/xiao-package/tests/e3b_cache.rs` 真实解析并执行。
`errors.json` 指定首源不可达的输入、目标包与稳定诊断；测试会移走对应目录，
确认不能跳到后一源。两份夹具都不触发网络读取。
