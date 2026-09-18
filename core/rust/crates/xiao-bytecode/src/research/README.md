# `xiao-bytecode/src/research`

放置 09R 特别研究工程的三地址模型、调用签名表、降低器与验证器。对应工程期为
09R2，研究代码**不是稳定语言接口**：试验性指令、寄存器类别和调用签名在 09R3
冻结前不得被 `xiao-driver`、CLI 或任何生产路径依赖，研究产物也不得使用
`.xiaoc` 扩展名落盘。

`tac.rs` 只描述算什么，`sig.rs` 补 `IrProgram` 缺失的调用 ABI 描述，
`lower/` 按关注点拆分降低规则，`verify.rs` 做自校验与释放序列对账，`encode.rs`
提供内存中的研究编码、解码、稳定 opcode、两种操作数宽度和 `pc -> IrSpan` 目录。
当前 R2C 还提供 `MakeError`/`Raise`/`Check`、`TacHandler` 和按子程序复用的 `finally`
降低；未支持的 RuntimeCheck 会保留在 `TacProgram.unsupported`，编码器对此明确拒绝。
禁止在这里重新推断类型、重算生命周期或重排释放顺序。编码器不是公开 `.xiaoc`
文件格式，研究产物只在内存中流转。
