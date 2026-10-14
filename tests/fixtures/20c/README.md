# 20C 能力探测夹具

工程期：20C。

本目录保存 20C §3.1 的首批夹具、第二批七项候选探测夹具、第四批转换夹具和第五批复探夹具，不是官方标准库，也不进入普通包依赖求解。

| 文件 | 探测范围 | 预期观察 |
| --- | --- | --- |
| `strings-path.xiao` | 字符串拼接形成相对路径 | VM 与 LLVM 原生输出 `lib/core` |
| `arithmetic-error.xiao` | 函数算术、运行期整数溢出与错误路径 | VM 保留 `X06-RUNTIME-009`；原生结果按实际后端/链接/进程状态记录 |
| `strings-path-representative.xiao` | 路径字符串的 `for c in s` 逐字符扫描与条件判断 | VM 与 LLVM 原生输出 `/` |
| `strings-index-boundary.xiao` | 非字面量字符串下标边界 | VM/LLVM 在类型层拒绝 `X03-TYPE-003`（迭代器式语言的既定边界） |
| `arithmetic-representative.xiao` | 除法、取模、数字转文本和字符串拼接 | VM/LLVM 输出 `42`；本批 `int` → `str` 双路径依据 |
| `second-int-to-str.xiao` | `int`/`sint`/`lint` 各自的 `as str`、`str(value)` 与异构数组动态值转换 | VM/LLVM 输出 `42:42:42:42:42:42:42`；本批整数族转换的双路径依据 |
| `fifth-number-formatting.xiao` | 整数转文本、循环与字符串拼接 | VM/LLVM 输出 `n=42`；第五批数字格式化候选双路径依据 |
| `fifth-character-classification.xiao` | ASCII 大写区间比较与 `and` | VM/LLVM 输出 `true`；第五批字符分类候选双路径依据 |
| `fifth-case-conversion.xiao` | `str` → `int` 码点转换 | VM/LLVM 在前端拒绝 `X02-TYPE-006`；第五批大小写转换缺口依据 |
| `fifth-string-literal-comparison.xiao` | 字面量字符串相等与序关系 | VM/LLVM 在前端拒绝 `X02-TYPE-005`；常量折叠缺口依据 |
| `fifth-substring-split.xiao` | 非字面量字符串下标前瞻 | VM/LLVM 最先拒绝 `X01-PARSE-007`；子串候选缺口依据 |
| `fifth-split-array.xiao` | 循环累积数组切分 | VM/LLVM 在类型层拒绝 `X02-TYPE-005`；切分候选缺口依据 |
| `fifth-set-membership.xiao` | 集合构造与成员判断 | VM/LLVM 输出 `ok`；集合最小能力只保留探测结论 |
| `second-string-comparison.xiao` | 字面量常量折叠的相等与字典序比较 | VM/LLVM 均在类型层拒绝 `X02-TYPE-005` |
| `second-string-comparison-parameters.xiao` | 字符串参数的相等与字典序比较 | VM/LLVM 输出 `ok`；运行时序关系可用 |
| `second-count-length.xiao` | `for` 逐字符计数 | 第二批 `text.length` 的双路径依据，VM/LLVM 输出 `2` |
| `second-string-accumulate.xiao` | 循环内字符串累积拼接 | 第二批候选：循环赋值与字符串拼接 |
| `second-substring-lookahead.xiao` | 变量下标前瞻 | 复合表达式最先拒绝 `X01-PARSE-007`；根边界为单下标的 `X03-TYPE-003` |
| `second-split-array.xiao` | 累积并构造数组结果 | VM/LLVM 在类型层拒绝 `X02-TYPE-005` |
| `second-case-conversion.xiao` | ASCII 小写转换的数字/文本转换尝试 | VM/LLVM 在类型层拒绝 `X02-TYPE-006` |
| `second-case-conversion-logic.xiao` | 字符分类所需的 `and` 逻辑组合 | VM 与 LLVM 原生均输出 `true`（维护回归 `38493485483` 两平台各 14 行 `20C-PROBE`）；右侧按需短路 |
| `second-set-membership.xiao` | 集合构造与 `in`/`not in` | VM/LLVM 输出 `ok`，仅保留探测结论 |

夹具由 `core/rust/crates/xiao-driver/tests/c20_capability_probe.rs` 读取。这里不添加
intrinsic、不绕过类型检查，也不把探测结果当作官方库接口承诺。
