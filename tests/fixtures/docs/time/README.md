# time 教程夹具

## 工程期

20O-B3-1。来源是 B3-0E 冻结的 time 示例、独立输出和合成时钟。[冻结教程](../../../../docs/DevDocs/assets/20o-b3/entry-20261124-b3-0e/tutorials/time.md)

## 职责

`tutorial.xiao`、`tutorial.stdout` 和 `tutorial.input.json` 用于驱动器 `20o_b3_time_tutorial` 目标。VM 和原生入口覆盖 `std.time`、`stdlib.time`，原生分别执行 O0、O3。

## 边界

夹具供 B3-1 内部验证。公共教程验收归 B3-7。
