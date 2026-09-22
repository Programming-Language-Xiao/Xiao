# `cli/ts/src/protocol`

放置 TypeScript 到 Rust 核心的稳定调用适配、版本协商、取消和结构化结果转换。工程期 09、11；不得镜像 Rust 内部布局。

X0-A 冻结 8 字节大端长度前缀、16 MiB 上限和公共消息窄类型；`codec.ts`、`messages.ts`
与 `tests/spec/11x0-protocol` 共享夹具完成双向契约测试。X0-B 的 `client.ts` 在此基础上
负责握手、源码请求、取消和关闭；命令路由、终端渲染在相邻目录，独立打包和完整平台发现
已由 X0-C 接入，`xiao build` 与主机工具链发现仍归 X0-E。
