---
id: language.collections.errors
title: 容器与集合错误诊断
status: verified
audience: learner
module: rust.xiao-types-sets
stage: "03E"
version: "0.1.0"
related:
  - README.md
  - indexing.md
  - advanced-selection.md
  - random-selection.md
  - broadcast-assignment.md
  - sets.md
  - ../../troubleshooting/README.md
---

# 容器与集合错误诊断

C0/C1/C2-A/C2-B 的静态容器错误在语法或类型检查阶段产生稳定编号。具体文案会随
`[language]` 国际化设置变化，编号、`message_id`、结构化参数和源码位置不变。09R2 研究
VM 另有选择器执行错误，仍使用同一结构化错误核心并沿 handler/finally 路由。

| 编号 | 含义 | 常见原因 |
| --- | --- | --- |
| `X03-TYPE-001` | 容器元素类型不匹配 | 显式 `int` 数组含字符串 |
| `X03-TYPE-002` | 字典键重复 | `name = 1` 后再次写 `"name" = 2` |
| `X03-TYPE-003` | 路径段种类错误 | 对数组使用键名路径 |
| `X03-TYPE-004` | 静态索引越界 | 固定数组读取不存在的位置 |
| `X03-TYPE-005` | 字典键不存在 | 读取未声明的键 |
| `X03-TYPE-006` | 路径格式错误 | 负索引或无法解析的数字 |
| `X03-TYPE-008` | 无序容器高级选择 | 对字典表使用范围、多选、步长、随机，或范围穿过字典表 |
| `X03-TYPE-009` | 非法步长 | 步长不是整数或静态值为零 |
| `X03-TYPE-010` | 非法随机数量 | 数量不是非负整数或超出可表示范围 |
| `X03-TYPE-011` | 随机候选不足 | 无放回超量，或从空来源抽取正数 |
| `X03-TYPE-012` | 选择器广播赋值错误 | 右值非标量、目标随机/不可变或使用不支持的赋值形式 |
| `X03-TYPE-013` | 随机种子错误 | `random.seed` 参数不是合法非负整数语义值 |
| `X03-TYPE-014` | 随机种子参数数量错误 | `random.seed` 不是恰好一个参数 |
| `X03-TYPE-015` | 集合元素类型不匹配 | 显式 `set<T | U>`、旧式同构前缀或集合成员判断发生静态类型冲突 |
| `X03-TYPE-016` | 集合元素不可哈希 | 数组、字典、元组（当前尚未实现递归证明）或其他已知不可哈希类型作为元素/成员值 |
| `X03-TYPE-017` | 静态集合元素重复 | 同一静态类型下的常量值在集合字面量中出现多次 |
| `X03-TYPE-018` | `set()` 参数数量错误 | 空集合构造式接收了参数；当前 C2-A/C2-B 只接受零参数 |
| `X03-TYPE-019` | 集合成员判断类型错误 | `in`/`not in` 右侧不是集合，或左侧不符合集合元素类型约束 |
| `X03-TYPE-020` | 集合不支持索引 | 对集合使用数字、键名、范围、多选、步长或随机选择 |
| `X03-TYPE-021` | 集合运算操作数错误 | 集合代数两侧不是可确认的集合形状，或集合与标量混用 |
| `X03-TYPE-022` | 集合比较操作数错误 | 集合比较两侧不是可确认的集合形状 |

## 研究 VM 运行时错误

以下错误由研究 VM 执行动态选择或种子计划时产生，均为可恢复 `XiaoError`：

| 编号 | 含义 |
| --- | --- |
| `X06-RUNTIME-017` | 选择器边界、路径或无序容器约束失败 |
| `X06-RUNTIME-018` | 步长不是整数或为零 |
| `X06-RUNTIME-019` | 随机数量为负、超量或来源为空 |
| `X06-RUNTIME-020` | 随机种子不是合法非负整数 |

语法阶段还会用 `X03-PARSE-002` 拒绝同一花括号中混用集合值和字典键值条目，并用
`X03-PARSE-004` 拒绝 `const name[path]`。C2-B 额外使用以下编号拒绝未开放的集合
类型注解形态：

| 编号 | 含义 | 常见原因 |
| --- | --- | --- |
| `X03-PARSE-005` | 集合类型注解结构非法 | `set<>`、非法类型项、尾部 `|` 或缺少 `>` |
| `X03-PARSE-006` | 集合类型注解不能带路径 | 写成 `set<int> values[0]` |
| `X03-PARSE-007` | `const` 集合类型尚未开放 | 写成 `const set<int> values = set()` |

`const` 的编译期常量语义与容器位置锁定不是同一件事，后者尚未进入 C0/C2-B。

集合诊断的 `message_id` 使用 `x03.type.set_*` 命名空间；集合类型冲突和成员类型冲突
携带 `actual_type`/`expected_type`，不可哈希诊断携带 `actual_type`，重复元素携带
`element`，构造器参数数量携带 `actual_count`/`expected_count`。程序和测试应匹配
`code`、`message_id` 与参数，而不是匹配中文或英语译文。

未知长度数组和字符串的边界无法在静态阶段证明；这类值会登记相应 Runtime 检查，
不应通过改写源码为范围或随机选择来绕过诊断。动态集合元素和动态成员判断分别登记
`SetHashability`/`SetMembership` 检查；动态集合赋给受限集合时也登记 `SetMembership`。
C2-C 的集合代数动态边界登记 `SetOperation`，集合比较动态边界登记 `SetComparison`。
C1/C2-A/C2-B/C2-C 的静态检查仍只生成计划；09R2 研究 VM 已执行有序容器读写、选择器
随机抽样和广播提交，但集合哈希、集合代数及自动扩容仍未开放。

更完整的生产运行时堆栈、日志和调试窗口属于第 07/11 阶段；本页同时列出研究 VM 的四个
选择器运行时码，但不承诺正式 `xiao run` 或 `.xiaoc` 产物已经开放。交接边界见 [03A](../../../DevDocs/03a-c0-containers.md)、
[03B](../../../DevDocs/03b-c1-ordered-selectors.md)、[03C](../../../DevDocs/03c-c2a-sets.md)
、[03D](../../../DevDocs/03d-c2b-heterogeneous-sets.md) 和
[03E](../../../DevDocs/03e-c2c-set-operations.md)。
