# CLI 参数分隔教程草案

状态为 `planned`。`std.cli` 和 `stdlib.cli` 提供 `split_terminator([str] values) -> [[str]]`。分组函数使用调用者给出的值，示例的输出使用 `stdout`。标准库接口目标为 1。[冻结契约](../contract-draft.md)

`[[str]]` 在签名表中描述两组字符串数组的返回形状。源码返回声明方式见 B3-0B3 的补充裁定。[现有编译器探测](../cli-shape.zip)

## 完整示例

```xiao
from std.cli import split_terminator

groups = split_terminator(values = ["go", "", "--", "--", "next", "next"])
for group in groups
    print("group")
    for value in group
        print("[" + value + "]")

for group in split_terminator(["only"])
    print("group")
    for value in group
        print("[" + value + "]")
```

```text
group
[go]
[]
group
[--]
[next]
[next]
group
[only]
group
```

第一组是首个 `--` 前的值，第二组是其后的值。没有分隔符时第二组为空。每次调用产生两个新数组，保留组内顺序和空项。后续精确目标逐字核对输出和退出码 0。[源码夹具](../../../../../../tests/fixtures/docs/cli/b3-0b-candidate.xiao) · [独立黄金](../../../../../../tests/fixtures/docs/cli/b3-0b-candidate.stdout)

## 资源与恢复

参数可按位置或名称传入。输入限 1,024 项，每项 4,096 个 UTF-8 字节，合计 65,536 字节。超限报告 `ResourceError/CLI_LIMIT_EXCEEDED`，两组作为一个结果交付。三平台共用这套分组语义。工具启动器先分出应用参数，需要应用自行选择分组时再调用本函数。
