# `path` 模块

工程期：20C 首批、20M 文本路径扩展；规范源码为 [`path.xiao`](path.xiao)。

## 接口

`join(str base, str leaf) -> str` 按已探测的字符串拼接形状，用 `/` 连接两个路径片段并返回新文本。
不去掉已有分隔符或合并重复分隔符。

20M 兼容新增：

| 接口 | 冻结语义 |
| --- | --- |
| `basename(str path) -> str` | 最后一个 `/` 后的文本；没有 `/` 返回原串；尾随 `/` 返回空串 |
| `dirname(str path) -> str` | 最后一个 `/` 前的文本；没有 `/` 返回空串；不补 `.` 或根分隔符 |
| `extension(str path) -> str` | 只取末段最后一个 `.` 及之后文本，含点；没有点返回空串 |

三个接口的空输入都返回空串。`basename("/")`、`dirname("/")`、`dirname("/name")` 和
`extension("a/")` 均为空串；`dirname("a//b")` 为 `a/`，`extension(".env")` 为 `.env`，
`extension("file.")`、`extension(".")`、`extension("..")` 均为 `.`。

只识别 `/`，反斜杠和盘符都是普通文本；不合并重复 `/`，不解析 `.`/`..`，不访问文件系统。
完整 18 行边界表见 [公共文档](../docs/UseDocs/library/path.md)，冻结及验证见
[20M](../docs/DevDocs/20m-text-and-path-expansion.md)。本批不新增错误编号。

参数必须是 `str`；其它类型由现有类型层拒绝。模块是纯 Xiao 实现，不新增 intrinsic。

## 兼容身份

这是标准库接口版本 1 的新增接口，属于兼容扩展，不提升 `STDLIB_INTERFACE_VERSION`。模块由工具链专用
标准库命名空间提供，不进入普通包依赖。归档通过既有 `stdlib_modules` 记录 `stdlib.path`，
运行时由宿主加载规范源码，不复制库函数体，也不新增归档字段。
