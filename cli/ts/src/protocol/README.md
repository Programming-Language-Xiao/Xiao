# `cli/ts/src/protocol`

放置 TypeScript 到 Rust 核心的稳定调用适配、版本协商、取消和结构化结果转换。工程期 09、11；不得镜像 Rust 内部布局。

X0-A 只冻结 8 字节大端长度前缀、16 MiB 上限和公共消息窄类型；`codec.ts`、`messages.ts`
与 `tests/spec/11x0-protocol` 共享夹具完成双向契约测试。完整命令路由、终端渲染、打包和
平台发现归 X0-B/C，不在本目录提前实现。
