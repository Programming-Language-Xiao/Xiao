# `xiao-driver/src`

放置编译/运行/构建请求编排、版本协商和结构化服务接口。对应工程期 08–18；不放终端 UI 或本地化文案。

08A 只实现统一前端静态编排；09-B0-C 的 `run.rs` 接入生产字节码 VM 内部驱动器，11 以后
再接入 CLI、配置和平台。驱动器只消费前端产物，不重新解析源码或推断语义。

X0-A 的 `protocol.rs` 提供长度前缀 JSON、统一 `core_version` 协商、结构化错误/退出码/事件
映射和可取消的 stdin/stdout 服务；`protocol_main.rs` 是 `xiao-core` 进程入口。协议只调用
`FrontendVmDriver`/`FrontendNativeDriver`，不在边界层复制语言语义。

X0-E 在同一协议中接入真实源码原生构建、运行时配置固化和诊断组件携带；对应的协议边界
测试拆在 `protocol_tests.rs`，避免传输与构建编排文件超过仓库单文件尺寸门禁。
