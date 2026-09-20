# `xiao-bytecode/src/research/encode`

## 目录职责

这里承载 09R2 研究编码器的内部实现。父级 `encode.rs` 是研究公开门面，保留公开
类型、错误和编码/解码入口；本目录按职责拆开字节读写、标签映射、编码、解码、输入
校验和回归测试，避免单个 Rust 文件承担整个编码协议。

## 工程期

对应工程期 09R2。研究编码只在内存中的 `Vec<u8>` 上工作，不生成或承诺正式的
`.xiaoc` 文件格式；正式分段容器仍属于后续阶段。

## 模块边界

- `codec.rs`：LEB128、定宽操作数、长度/字符串和可选字段的读写器。
- `tags.rs`：opcode 及外部枚举的显式双向标签表，标签数值属于编码格式的一部分。
- `encoder.rs`：按冻结字段顺序把 TAC/ABI 事实写入字节流，并建立函数与基本块目录。
- `decoder.rs`：严格按格式读取字节流，并负责解码后的结构、handler pc 与可复用语义
  不变量校验；拒绝未知 opcode、版本、截断和尾部数据。
- `validate.rs`：编码输入的版本、签名和跨表引用校验；不负责 decoder 专属的结构或
  handler pc 校验，也不重新推断类型或生命周期。
- `tests.rs`：布局版本 2、38 个 opcode、两种操作数宽度、源码映射和损坏输入的往返/拒绝
  测试，并用显式数值钉住集合算子标签；布局版本 1 会在读取函数体前被拒绝。

子模块只通过 `pub(super)` 暴露给门面和同级实现，外部调用方应使用父级
`research::encode` 的公开入口。

## 依赖方向

实现模块的依赖图固定如下；箭头表示“可以依赖”，未列出的同级依赖一律禁止：

```text
research::encode（公开门面）
├── codec（叶子）
├── tags（叶子）
├── validate ──> codec, tags
├── encoder  ──> codec, tags, validate
└── decoder  ──> codec, tags, validate

tests（仅 cfg(test)） ──> 门面及测试所需的内部辅助接口
```

具体约束：

- `codec`、`tags` 只能依赖父模块提供的格式常量/数据类型和外部基础 crate，不能依赖
  `validate`、`encoder`、`decoder` 或 `tests`。
- `validate` 可以复用 `codec` 的宽度检查和 `tags` 的标签映射，但不能反向依赖编码器、
  解码器或测试模块。
- `encoder` 与 `decoder` 都可以使用前三个实现模块，但二者互不依赖，不能通过对方
  共享实现；公共协作逻辑应下沉到 `codec`、`tags` 或 `validate`。
- 门面只负责装配、调用和公开 API；生产模块不能依赖 `tests`，测试模块也不能成为
  任何生产路径的依赖。
- 禁止循环依赖、反向依赖和从目录外绕过 `research::encode` 访问这些私有模块。源码级
  架构回归测试 `module_dependency_direction_is_acyclic` 会锁住上述禁止边。
