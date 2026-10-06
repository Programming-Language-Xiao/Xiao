# 19D Java 参考实现

`Benchmark.java` 是 19D 的无依赖参考实现。它只输出一个稳定的制表符记录，便于驱动器在不引入
JSON 或基准框架依赖的情况下校验 `manifest.json` 的期望值：

```text
success<TAB><value>
error<TAB>X06-RUNTIME-009
```

在固定的 Temurin/OpenJDK 21 环境中编译和运行。`baseline.json` 固定 JVM 参数与统计协议；其中
`resolved_version` 和摘要必须由受控机器填写，不能用开发机的 Java 版本替代。

溢出用例保留用于语义记录，但 Java 的整数回绕和 Xiao 的 `X06-RUNTIME-009` 不具备可比语义，
因此只能记为「数据不足」，不参与性能通过或回归判定。
