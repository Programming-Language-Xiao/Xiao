# `protocol/`

这里是 `xiao-driver` 的协议实现子模块。上层 `protocol.rs` 只保留稳定门面、公开重导出
和 `cfg(test)` 下的旧测试辅助导入；协议公开路径因此不随内部文件移动而改变。

## 工程期

11X0-F（协议职责解耦）。本目录记录 X0-A 协议的十个实现模块及其固定依赖方向；拆分不
新增字段、命令或语义，Windows 原生验证结果以父级交接文档为准。

职责与允许依赖固定如下：

| 模块 | 职责 | 允许依赖 |
| --- | --- | --- |
| `frame.rs` | 8 字节大端长度前缀、16 MiB 上限、JSON 帧编解码 | `serde`、`std::io` |
| `message.rs` | 版本、响应和跨进程消息类型 | 纯消息/已有核心版本常量 |
| `request.rs` | 请求配置、请求枚举和协议错误 | 目标/VM 类型 |
| `mapping.rs` | 内部诊断、事件、指标和值到协议类型的映射 | `message`、`request`、内部运行时类型 |
| `validate.rs` | 版本、源码身份和目标边界校验 | `message`、`request` |
| `run.rs` | 前端到 VM 的请求组装、诊断会话和运行响应 | `mapping`、`message`、`request`、驱动器 |
| `build.rs` | 工具链准备、原生构建及附属产物整理 | `config`、`mapping`、`message`、`request`、`run` |
| `config.rs` | `config.xiao` 静态固化与旁置 JSON 写入 | `message`、`request`、配置 crate |
| `service.rs` | 分发、环境元数据请求、取消登记、worker、stdin/stdout 服务和崩溃响应 | 以上所有协议子模块、`xiao-package` 环境接口 |

依赖只能沿 `frame/message/request → mapping/validate → run/build/config → service` 向上流动；
环境请求由 `service` 消费 `xiao-package` 的纯元数据接口，不把包解析或用户代码执行带入协议叶子。
叶子不得依赖服务层或门面，`run` 与 `build` 不得横向依赖彼此，只有 `service` 可以持有
`std::thread`、`Arc` 和共享输出锁。`protocol_architecture_tests.rs` 以源码级断言锁住
这些边界；`protocol_tests.rs` 与 `tests/spec/11x0-protocol/` 仍是协议行为和跨语言夹具的
单一来源，拆分不新增字段、命令或语义。
