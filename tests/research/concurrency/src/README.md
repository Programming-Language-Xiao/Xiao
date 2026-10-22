# 07-D.6 原型源码

工程期：07-D.6 隔离研究原型。

职责：提供线程入口、深拷贝、边界事实、候选 A、原子句柄、取消清理和测量的最小 Rust
模型。这里的源码只供 `tests/research/concurrency` 独立 crate 使用，不属于 Xiao Runtime、
语言类型、IR、ABI 或标准库公开契约。

边界：不得从 `core/rust` workspace 或产品 crate 引用；不得把 `Arc`、`Weak`、`Mutex` 或
原型中的枚举和结构体当作正式语言能力。模拟场景和标准库参考模型的结果必须在文档中单独
标注，不能解释为 Runtime/IR 已实现行为。
