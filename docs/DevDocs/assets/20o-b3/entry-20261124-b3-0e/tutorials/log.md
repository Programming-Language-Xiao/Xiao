# 标准输出日志教程草案

状态为 `planned`。`std.log.write` 读取 UTC 墙钟并交付一条日志，需要 `clock` 与 `stdout`。首批目标为 `stdout`，级别为 `debug`、`info`、`warn`、`error`。别名 `stdlib.log` 使用同一官方身份。标准库接口目标为 1。[冻结契约](../../entry-20261123-b3-0c/contract-draft.md)

## 快速开始

从 `std.log` 导入 `write`，并传入级别、消息和目标 `stdout`。

## 完整示例

精确验收给运行时钟注入 `946684800000` Unix 毫秒，即 `2000-01-01T00:00:00.000Z`，并捕获运行输出。

```xiao
from std.log import write as write_log

write_log("info", "task started", "stdout")
try
    write_log("warn", "two\nlines", "stdout")
catch failure as TypeError
    print(error_code(failure))
write_log(level = "error", message = "task stopped", target = "stdout")
```

```text
2000-01-01T00:00:00.000Z INFO task started
X06-RUNTIME-012
2000-01-01T00:00:00.000Z ERROR task stopped
```

记录包含 UTC 毫秒时间戳、空格、大写级别、空格、消息和一个 LF。消息中的 CR、LF、NUL 在读钟前报 `X06-RUNTIME-012`。[源码夹具](../candidates/tests/fixtures/docs/log/tutorial.xiao) · [固定时钟](../candidates/tests/fixtures/docs/log/tutorial.input.json) · [独立黄金](../candidates/tests/fixtures/docs/log/tutorial.stdout)

## 配额与输出失败

单条消息最多 4,096 个 UTF-8 字节，每次运行最多尝试交付 1,024 条，完整编码总量至多 1 MiB。开始交付后出现的失败也占用配额。运行输出端写入或刷新失败报 `LOG_OUTPUT_FAILED`，已经写出的字节仍在输出端。CLI 在运行结果之后转发失败时，由 CLI 报告。操作期限错误为 `LOG_TIMEOUT`。[资源与错误表](../../entry-20261123-b3-0c/contract-draft.md)
