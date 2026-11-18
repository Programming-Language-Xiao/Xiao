# 11A-E3C 网络源规格

`valid.json` 固定提交、标签和三态结果；`errors.json` 是 Git smart HTTP pkt-line 的
反例输入。`core/rust/crates/xiao-package/tests/e3c_remote.rs`
还在本地 TCP 服务内测索引、Git 不可变提交、HTTP Range、截断、不可用状态和远程内容不执行。
测试不访问真实 GitHub，也不依赖外部 DNS。
真正的 DNS 解析失败另设 `#[ignore]` 手动探针，不进入 CI。
