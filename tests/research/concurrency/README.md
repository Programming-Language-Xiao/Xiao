# 07-D.6 最小 OS 线程原型

这是独立的 Rust 研究 crate，只使用标准库，不是 `core/rust` workspace 成员，也不提供
Xiao Runtime、语言语法、IR、ABI 或标准库 API。`Arc`、`Weak` 和 `Mutex` 只作为原子计数、
弱升级和显式锁的参考模型，原型类型不能被产品 crate 引用。

在仓库根目录运行：

```text
cargo test --manifest-path tests/research/concurrency/Cargo.toml
cargo run --release --manifest-path tests/research/concurrency/Cargo.toml -- --iterations 100
```

测试覆盖八类场景和一个自定义 ABI 场景：线程入口和深拷贝隔离、失败复制的逆序清理、`CrossThread` 任务/线程
事实拆分、候选 A 递归接受/拒绝表、只读原子句柄、显式锁保护的共享可变句柄、四位置取消
清理，以及自定义 `#[repr(C)]` 对象头、强/弱计数、弱升级、普通/共享 tag、spawn/join、深拷贝、
原子操作、锁和状态尺寸测量。自定义 ABI 场景仍是隔离模型，不属于 Xiao Runtime。

测量输出记录操作系统、架构、重复次数、每项操作的平均纳秒数和原型包装/状态尺寸。线程调度
和锁竞争受机器影响，输出只用于研究比较，不能直接冻结 Runtime ABI 或实现阶段。
