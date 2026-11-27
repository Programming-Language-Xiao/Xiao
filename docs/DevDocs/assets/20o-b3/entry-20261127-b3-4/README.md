# B3-4-ENV-1A 实施证据

B3-4A 固定产品 `3506455ee5839214082df15af4c760f8d0361e66`，Git 树为
`6c5b1177d7ca7faa2d7237c80cb83d970cc630b6`，ABI 1.20、生成器 26、标准库接口 1。
开工工作区的产品路径差异仅涉及三份已提交的回归测试说明与辅助代码。入口 intrinsic
1～59 为 `active`，60～63 为 `reserved`，100 为 `tombstone`。两个锁文件的工作区字节
与固定提交一致。[入口基线](baseline.json)

基线引用 22D/FIX.2 的固定产品和三平台 fs 原始回执。Windows 六份 fs 产物的二进制和链接图
逐份绑定到 `Complete` 回执。B1 的七库 20 入口、18 项资源及 VM、原生精确入口沿用
B3-0A 归档，28 项守卫由 4 份实际 Cargo 输出和 24 项派生负例组成。
[22D/FIX.2 证据](../../22d-fix2/entry-20261126/README.md) · [B1 原始归档](../entry-20261122-b3-0/observations.zip)

本批验证使用独立任务目录 `core/rust/target/b3-4-env-1a`，原始输出见 `logs/`。
B3-4B 的实现、实测退出码和后续接线位置见 [实施报告](REPORT.md)。
