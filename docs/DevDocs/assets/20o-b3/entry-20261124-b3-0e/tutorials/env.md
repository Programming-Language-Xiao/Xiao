# 运行环境变量教程草案

状态为 `planned`。`std.env` 使用 `environment` 授权读取和修改本次运行的环境视图。本例在授权的运行入口注入 `XIAO_B3_TUTORIAL_VALUE=initial`，并用 `stdout` 显示结果。别名 `stdlib.env` 使用同一官方身份。标准库接口目标为 1。[冻结契约](../../entry-20261123-b3-0c/contract-draft.md)

## 快速开始

从 `std.env` 导入环境操作函数。测试入口为本次运行提供合成环境快照。

## 完整示例

```xiao
from std.env import get as get_env, set as set_env, unset as unset_env

name = "XIAO_B3_TUTORIAL_VALUE"
for value in get_env(name)
    print("snapshot=[" + value + "]")

set_env(name = name, value = "")
for value in get_env(name)
    print("empty=[" + value + "]")

set_env(name, "local")
for value in get_env(name)
    print("updated=[" + value + "]")

print(unset_env(name))
print(unset_env(name))
for value in get_env(name)
    print("unexpected=[" + value + "]")
```

```text
snapshot=[initial]
empty=[]
updated=[local]
true
false
```

`get` 返回零项或一项数组。记录 `empty=[]` 来自一个空字符串，删除后返回零项，循环没有输出。`unset` 在删除存在项时返回 true。[源码夹具](../candidates/tests/fixtures/docs/env/tutorial.xiao) · [合成输入](../candidates/tests/fixtures/docs/env/tutorial.input.json) · [独立黄金](../candidates/tests/fixtures/docs/env/tutorial.stdout)

## 快照与限额

同一运行的 Xiao 并发任务共享局部写删，运行结束后释放。`std.os.spawn` 继续继承宿主环境。嵌入请求省略环境时使用空快照。Windows 使用宿主的序号不区分大小写规则，Linux 和 macOS 精确比较名称。

名称与值严格解码。名称最多 256 UTF-8 字节，值最多 65,536 字节。每次运行至多 1,024 对，名称与值合计最多 1 MiB。快照非法或超限在应用入口分别报告 `X09-DRIVER-006/007`，运行中的超限更新报告 `ENV_LIMIT_EXCEEDED` 并保留旧值。[资源与错误表](../../entry-20261123-b3-0c/contract-draft.md)
