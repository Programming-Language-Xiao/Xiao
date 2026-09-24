# `xiao-package/tests`

这里放置 11A-D1 包契约与本地路径依赖图的规格测试。测试通过 `xiao-package` 的公开
解析入口读取隔离临时项目，并验证包身份、确定性拓扑顺序和 `X05-PACKAGE-*` 诊断。

本目录不执行 Xiao 包代码，不写入缓存或锁文件，也不访问远程源。跨阶段共享的正反例
位于 `tests/spec/11a-package/`，由 `d1_package.rs` 真实加载。
