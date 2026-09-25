# `xiao-package/tests`

这里放置 11A-D1 包契约、11A-E0 环境物化、11A-E1 本地依赖缓存及 11A-E2A/E2B 锁与同步的规格测试。测试通过
`xiao-package` 的公开入口读取隔离临时项目，并验证包身份、确定性拓扑顺序、环境元数据版本、
SHA-256 对象、只读校验、损坏隔离和 `X05-*` 诊断。

工程期：11A。

本目录不执行 Xiao 包代码，不访问远程源，也不碰真实 `~/.xiao`；每个用例注入临时缓存根目录。
D1 夹具位于 `tests/spec/11a-package/`，由 `d1_package.rs` 真实加载；E1 夹具位于
`tests/spec/11a-cache/`，由 `e1_cache.rs` 真实加载；E2A 双夹具位于
`tests/spec/11a-lockfile/`，由 `e2a_lockfile.rs` 真实加载并执行；E2B 双夹具位于
`tests/spec/11a-sync/`，由 `e2b_sync.rs` 逐例执行。
E3A 的 `e3a_source.rs` 真实加载 `tests/spec/11a-jcs/vectors.json` 及
`tests/spec/11a-source/valid.json`/`errors.json`，另以隔离目录测快照、分片、正文和摘要校验。
E3B 的 `e3b_cache.rs` 真实加载 `tests/spec/11a-cache-policy/valid.json`/`errors.json`，
验证离线命中、有界并行、优先级、缓存损坏与基础设施故障、跨进程条目锁和中断后重试；
本地目录源不验证真实 HTTP Range 断点续传。
