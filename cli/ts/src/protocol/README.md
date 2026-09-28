# `cli/ts/src/protocol`

放置 TypeScript 到 Rust 核心的稳定调用适配、版本协商、取消和结构化结果转换。工程期 09、11；不得镜像 Rust 内部布局。

X0-A 冻结 8 字节大端长度前缀、16 MiB 上限和公共消息窄类型；`codec.ts`、`messages.ts`
与 `tests/spec/11x0-protocol` 共享夹具完成双向契约测试。X0-B 的 `client.ts` 在此基础上
负责握手、源码请求、取消和关闭；命令路由、终端渲染在相邻目录，独立打包和核心发现
由 X0-C 接入。X0-E 在同一客户端增加真实源码 `build` 请求，X0-T 增加批量 `test` 请求、
逐用例结果和结构化渲染，不改变帧与版本协商契约。

11B-I4a 增加 `repl_packages` 消息镜像及协议客户端的只读查询入口；旧核心不提供
`repl_packages` 能力时返回空视图并正常关闭子进程，不将包管理 `package` 误判为视图能力。
REPL 的命名空间注册与长驻会话仍留给 I4b。

11C 的 `run` 请求可以附加规范语言 `locale`，Rust 响应的诊断、报告和错误体可以附加
本地化 `text`。两字段均可缺省以兼容旧客户端/旧核心；`code`、`message_id`、参数和
退出码不因语言变化。TypeScript 不复制核心翻译表，只呈现核心提供的 `text`。
