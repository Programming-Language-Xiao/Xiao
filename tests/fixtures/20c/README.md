# 20C 能力探测夹具

工程期：20C。

本目录保存 20C §3.1 的首批夹具与第二批七项候选探测夹具，不是官方标准库，也不进入普通包依赖求解。

| 文件 | 探测范围 | 预期观察 |
| --- | --- | --- |
| `strings-path.xiao` | 字符串拼接形成相对路径 | VM 与 LLVM 原生输出 `lib/core` |
| `arithmetic-error.xiao` | 函数算术、运行期整数溢出与错误路径 | VM 保留 `X06-RUNTIME-009`；原生结果按实际后端/链接/进程状态记录 |
| `strings-path-representative.xiao` | 路径字符串的 `for c in s` 逐字符扫描与条件判断 | VM 与 LLVM 原生输出 `/` |
| `strings-index-boundary.xiao` | 非字面量字符串下标边界 | VM/LLVM 在类型层拒绝 `X03-TYPE-003`（迭代器式语言的既定边界） |
| `arithmetic-representative.xiao` | 除法、取模、数字转文本和字符串拼接 | VM/LLVM 在类型层拒绝 `X02-TYPE-006`（当前不支持 `int` 到 `str` 转换） |
| `second-string-comparison.xiao` | 字符串相等与字典序比较 | VM/LLVM 均在类型层拒绝 `X02-TYPE-005` |
| `second-string-comparison-parameters.xiao` | 字符串参数的相等与字典序比较 | VM/LLVM 输出 `ok`；运行时序关系可用 |
| `second-count-length.xiao` | `for` 逐字符计数 | 第二批 `text.length` 的双路径依据，VM/LLVM 输出 `2` |
| `second-string-accumulate.xiao` | 循环内字符串累积拼接 | 第二批候选：循环赋值与字符串拼接 |
| `second-substring-lookahead.xiao` | 变量下标前瞻 | 复合表达式最先拒绝 `X01-PARSE-007`；根边界为单下标的 `X03-TYPE-003` |
| `second-split-array.xiao` | 累积并构造数组结果 | VM/LLVM 在类型层拒绝 `X02-TYPE-005` |
| `second-case-conversion.xiao` | ASCII 小写转换的数字/文本转换尝试 | VM/LLVM 在类型层拒绝 `X02-TYPE-006` |
| `second-case-conversion-logic.xiao` | 字符分类所需的 `and` 逻辑组合 | 语言边界批单独登记；当前 VM 为 `X09-BYTECODE-001`，原生为后端 Unsupported |
| `second-set-membership.xiao` | 集合构造与 `in`/`not in` | VM/LLVM 输出 `ok`，仅保留探测结论 |

夹具由 `core/rust/crates/xiao-driver/tests/c20_capability_probe.rs` 读取。这里不添加
intrinsic、不绕过类型检查，也不把探测结果当作官方库接口承诺。
