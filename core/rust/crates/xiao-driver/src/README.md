# `xiao-driver/src`

放置编译/运行/构建请求编排、版本协商和结构化服务接口。对应工程期 08–18；不放终端 UI 或本地化文案。

08A 只实现统一前端静态编排；09-B0-C 的 `run.rs` 接入生产字节码 VM 内部驱动器，11 以后
再接入 CLI、配置和平台。驱动器只消费前端产物，不重新解析源码或推断语义。

X0-A 的 `protocol.rs` 是稳定门面，只负责公开重导出和协议常量；实现按职责放在
`protocol/` 下的 `frame`、`message`、`request`、`mapping`、`validate`、`run`、`build`、
`config`、`service` 九个模块。依赖方向固定为叶子类型/帧 → 映射与校验 → 运行/构建/配置
→ 服务分发，只有 `service` 持有线程和共享输出锁。协议仍提供长度前缀 JSON、统一
`core_version` 协商、结构化错误/退出码/事件映射和可取消的 stdin/stdout 服务；
`protocol_main.rs` 是 `xiao-core` 进程入口。协议只调用 `FrontendVmDriver`/`FrontendNativeDriver`，
不在边界层复制语言语义。

X0-E 在同一协议中接入真实源码原生构建、运行时配置固化和诊断组件携带；对应的协议边界
测试拆在 `protocol_tests.rs`，避免传输与构建编排文件超过仓库单文件尺寸门禁。
协议拆分的依赖方向由 `protocol_architecture_tests.rs` 锁定；共享协议 fixture 和既有
`protocol_tests.rs` 保持为单一来源证据，拆分不改变公开路径或响应语义。
