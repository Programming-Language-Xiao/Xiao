# 20M 文本与路径夹具

工程期：20M；保存冻结前的候选探测与冻结后的公开接口调用。

`probe-*.xiao` 对应 20M 的九项候选，使用现有语言与标准库能力，不定义产品 API。
其中 `starts-with`、`ends-with` 直接验证既有函数的一行组合；其余夹具覆盖 ASCII 空白、
Unicode 字符、替换与重复边界，以及路径的逐格文本语义。

无 `probe-` 前缀的七个接口夹具只导入产品实现，复用同一组独立黄金值。
`integration.xiao` 验证函数导入别名、跨模块组合、前后缀组合、目录中的点、尾随分隔符和非空白 NUL。
归档回归使用这些真实源码构建 `.xar`，逐项核对实际标准库要求、接口版本 1 和应用源码
移除后的独立重放。VM 加载器与 LLVM 函数体的移除验证分别覆盖七项新入口。

统一入口是
[`20m_text_and_path.rs`](../../../core/rust/crates/xiao-driver/tests/20m_text_and_path.rs)，
黄金输出由测试独立保存。VM 与受控 LLVM 原生复用同源 IR，分别逐字核对输出；原生还须
退出码为 0、stderr 为空。候选与产品原生测试均由三平台维护回归独立精确执行，校验恰好
一条通过。详细结果及已确认冻结表见
[20M](../../../docs/DevDocs/20m-text-and-path-expansion.md) §九。

```powershell
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20m_text_and_path
# 按 10D §4 配置工具链环境后执行原生探测和产品回归。
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20m_text_and_path -- --ignored --nocapture
```
