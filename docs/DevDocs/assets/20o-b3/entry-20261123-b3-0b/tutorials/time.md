# Time 时间教程草案

状态为 `planned`。B3 的 `std.time` 提供 UTC 墙钟、单调钟、UTC 格式化和可取消等待。别名 `stdlib.time` 使用同一官方身份。三个宿主时间入口需要 `clock`，示例还用到 `stdout`。标准库接口目标为 1。[冻结契约](../contract-draft.md)

## 完整示例

```xiao
from std.time import utc_now, monotonic, format_utc, sleep
from std.text import length as text_length

print(format_utc(0, "ms"))
print(format_utc(unix_time = -1, unit = "ns"))
print(text_length(format_utc(utc_now(unit = "ms"), "ms")))
before = monotonic("ns")
sleep(duration = 0, unit = "ms")
print(monotonic(unit = "ns") >= before)

try
    sleep(-1, "ms")
catch failure as TypeError
    print(error_code(failure))
```

```text
1970-01-01T00:00:00.000Z
1969-12-31T23:59:59.999999999Z
24
true
X06-RUNTIME-012
```

示例只输出当前 UTC 字符串的长度，并比较两次单调钟读数。后续精确目标使用处于格式域内的时钟输入，逐字核对输出和退出码 0。[源码夹具](../candidates/tests/fixtures/docs/time/b3-0b-candidate.xiao) · [独立黄金](../candidates/tests/fixtures/docs/time/b3-0b-candidate.stdout)

## 选择单位与处理错误

- `utc_now(str unit) -> int` 返回 Unix 纪元起算的 UTC 时间。系统校时可能改变读数方向。
- `monotonic(str unit) -> int` 返回本次运行的经过时间。子任务共享运行起点，测量用时应取两次同单位读数的差。
- `format_utc(int unix_time, str unit) -> str` 构造拥有的 UTC 字符串。单位为 `"ms"` 或 `"ns"` 时分别输出三位或九位小数，末尾固定为 `Z`。
- `sleep(int duration, str unit) -> none` 最多等待 60 秒。零时长完成安全点检查后返回，取消或运行截止沿运行控制通道交付。

显式单位是必填参数。时间表示的单位与宿主调度粒度分别观察。遇到非法单位、负时长或格式域越界可捕获 012，能力缺失为 026，表示溢出为 009。超出等待上限报告 `TIME_SLEEP_LIMIT_EXCEEDED`。[错误与边界表](../contract-draft.md#错误与优先级)
