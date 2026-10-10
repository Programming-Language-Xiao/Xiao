# tests/benchmarks/src/bin

工程期 10Z 性能对照驱动器的独立二进制目录。职责是执行三侧语义互校与受控计时；边界是
`performance_driver.rs` 从冻结的
`manifest.json` 与 `baseline.json` 读取基准和计时协议，先做 Java、LLVM 原生、VM
三侧语义互校，再以独立子进程记录整进程挂钟样本、主机负载、后台进程和确定性
percentile-bootstrap 统计。它不修改 09R3 的 `src/main.rs` 或既有报告；受控性能结论
必须由受控主机运行后另行回填。
