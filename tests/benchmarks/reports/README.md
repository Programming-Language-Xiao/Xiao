# `tests/benchmarks/reports`

工程期 09R3；这里存放 Windows 原生生成的语义差分、性能、内存、编码体积和冻结 JSON。报告
必须带工具链、构建配置、统计协议、`FORMAT_VERSION = 3`、opcode `0..40` 与平台状态；Linux
和 macOS 在复现前只能写待复现清单，不能把数字并入本目录的验收结论。

`19d-performance.json` 是 19D 的独立三态报告，引用 `../baseline.json` 和 `../java/` 的
Temurin/OpenJDK 21 对照材料；它与 09R3 的四份冻结报告相互独立，旧报告不得重写。
