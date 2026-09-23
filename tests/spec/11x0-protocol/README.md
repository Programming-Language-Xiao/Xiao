# 11X0 协议共享夹具

本目录是 X0-A 的跨语言契约样本。Rust `xiao-driver::protocol` 和 TypeScript
`cli/ts/src/protocol` 都读取同一批 JSON，不复制 Rust 内部结构。

帧格式固定为 8 字节大端无符号长度字段加 UTF-8 JSON 负载；长度只计算负载，最大负载
为 16 MiB。夹具只覆盖协议字段和机器可读错误，不把人类可读文案作为断言依据。

## 文件

- `hello-request.json`：统一核心版本协商请求。
- `run-response.json`：包含退出码、诊断、事件和指标的运行响应。
- `test-request.json`：按稳定项目相对路径排列的测试源码请求。
- `test-response.json`：包含聚合退出码和逐用例结构化结果的测试响应。
- `debug-run-request.json`：`-debug` 与终端/文件等级配置的运行请求。

Rust 和 TypeScript 各自的测试都必须对这些文件做编码/解码回环；增加字段时先更新夹具，
再更新两侧的显式类型和测试。
