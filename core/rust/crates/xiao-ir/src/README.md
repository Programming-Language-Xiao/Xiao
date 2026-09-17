# `xiao-ir/src`

放置类型化 IR、效果/所有权信息、源码映射和验证实现。对应工程期 08、13；IR 节点必须可快照和版本化。

08A/U0 当前包含 `model.rs`、`lower.rs`、`validate.rs` 和 `snapshot.rs`；模块之间通过公开
值对象通信，禁止引入 CLI、Runtime、VM 或平台依赖。
