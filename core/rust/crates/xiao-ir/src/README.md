# `xiao-ir/src`

放置类型化 IR、效果/所有权信息、源码映射和验证实现。对应工程期 08、13；IR 节点必须可快照和版本化。

08A/U0 当前包含 `model.rs`、`lower.rs`、`validate.rs` 和 `snapshot.rs`；模块之间通过公开
值对象通信，禁止引入 CLI、Runtime 执行实现、VM 或平台依赖。

22E 的 `concurrency_features.rs` 只消费 `xiao-runtime-abi` 的冻结常量与已验证 IR 操作/类型，
供 `.xiaoc`、`.xar` 和原生元数据共享最小 feature 并集，不创建 Runtime 或执行用户代码。

09R2H 增加 `tables.rs`，保存前端表接口的可序列化镜像，并还原类型层成员签名供后端
复用。转换不做类型推断；方法的完整调用签名留在函数 IR，空表签名兼容既有快照。
