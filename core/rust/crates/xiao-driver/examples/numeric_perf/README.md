# B2-A6受控测量支持

工程期：20O-B2-A6。职责：固定独立输入和黄金，生成完整Xiao调用，按F-3在release VM与原生O0/O3的新进程中测量。

cases读取tests/fixtures/20o-b2-a6/catalog.json；identity固定源码/锁/Runtime/工具内容；worker处理VM的窗口标记；native只在测量程序链接时替换print，C shim不进入生产ABI；process管理新PID、一次预热、有界校准、OS读数及原始stdout/stderr。

五批时间按repeat归一后取中位数，RSS在输入/预热前取同PID当前值，以OS高水位差的五批最大值验收。缺测和失败不得写为通过；最大输入不进入默认debug基准。B1原测量工具、721条及阈值不变。
