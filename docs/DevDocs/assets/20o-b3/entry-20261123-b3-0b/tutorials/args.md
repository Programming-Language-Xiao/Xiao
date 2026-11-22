# Args 程序参数教程草案

状态为 `planned`。`std.args` 和 `stdlib.args` 提供运行快照中的程序名和参数。读取需要 `process` 能力，示例还使用 `stdout`。标准库接口目标为 1。[冻结契约](../contract-draft.md)

## 完整示例

将示例保存为 `demo.xiao`。产品接入后可向运行命令传入程序参数，例如 `xiao run demo.xiao -- "" "two words" -- again again`。精确测试由运行入口注入相同的程序名和参数，以确定测试输入中的空项。

```xiao
from std.args import program_name, argv

print("program=" + program_name())
copy = argv()
array_push(copy, "local")
for value in argv()
    print("[" + value + "]")
```

```text
program=demo.xiao
[]
[two words]
[--]
[again]
[again]
```

示例改动第一次返回的数组，再读取快照。第二次读取保留原顺序、空项和重复值。后续在 VM、原生 O0/O3 逐字核对输出和退出码 0。[源码夹具](../candidates/tests/fixtures/docs/args/b3-0b-candidate.xiao) · [运行输入](../candidates/tests/fixtures/docs/args/b3-0b-candidate.input.json) · [独立黄金](../candidates/tests/fixtures/docs/args/b3-0b-candidate.stdout)

## 获取程序名与参数

`program_name() -> str` 返回程序名，CLI 用运行目标的输入拼写，直接启动原生产物时读取宿主拆分出的 `argv[0]`。嵌入请求省略快照时得到空程序名与空参数数组。`argv() -> [str]` 返回拥有的数组，程序名通过独立入口获取。

最多 1,024 个参数，程序名和每项各最多 4,096 个 UTF-8 字节，合计至多 65,536 字节。读取前缺少 `process` 报 026。错误的原生编码、嵌入输入中的 NUL 或超预算分别沿执行前的 `X09-DRIVER-004` 和 `X09-DRIVER-005` 拒绝。Windows 使用宿主拆分后的原参数，应用启动边界在有损转换之前校验。[输入与错误表](../contract-draft.md)
