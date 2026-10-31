# 20M 文本与路径探测夹具

工程期：20M；本目录仅保存候选能力探测，接口冻结前不修改标准库实现。

`probe-*.xiao` 对应 20M 的九项候选，使用现有语言与标准库能力，不定义产品 API。
其中 `starts-with`、`ends-with` 直接验证既有函数的一行组合；其余夹具覆盖 ASCII 空白、
Unicode 字符、替换与重复边界，以及路径的逐格文本语义。

统一入口是
[`20m_text_and_path.rs`](../../../core/rust/crates/xiao-driver/tests/20m_text_and_path.rs)，
黄金输出由测试独立保存。VM 与受控 LLVM 原生复用同源 IR，分别逐字核对输出；原生还须
退出码为 0、stderr 为空。详细结果及待确认冻结表见
[20M](../../../docs/DevDocs/20m-text-and-path-expansion.md) §九。

```powershell
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20m_text_and_path
# 按 10D §4 配置工具链环境后执行原生探测。
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20m_text_and_path -- --ignored --nocapture
```
