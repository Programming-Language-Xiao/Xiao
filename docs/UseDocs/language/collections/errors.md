---
id: language.collections.errors
title: 容器错误与诊断
status: verified
audience: learner
module: rust.xiao-types
stage: "03B"
version: "0.1.0"
related:
  - README.md
  - indexing.md
  - advanced-selection.md
  - random-selection.md
  - broadcast-assignment.md
  - ../../troubleshooting/README.md
---

# 容器错误与诊断

C0/C1 的容器错误在类型检查阶段产生稳定编号。具体文案会随 `[language]` 国际化设置变化，
编号和源码位置不变；C1 的选择器错误仍只表示静态计划阶段的诊断。

| 编号 | 含义 | 常见原因 |
| --- | --- | --- |
| `X03-TYPE-001` | 容器元素类型不匹配 | 显式 `int` 数组含字符串 |
| `X03-TYPE-002` | 字典键重复 | `name = 1` 后再次写 `"name" = 2` |
| `X03-TYPE-003` | 路径段种类错误 | 对数组使用键名路径 |
| `X03-TYPE-004` | 静态索引越界 | 固定数组读取不存在的位置 |
| `X03-TYPE-005` | 字典键不存在 | 读取未声明的键 |
| `X03-TYPE-006` | 路径格式错误 | 负索引或无法解析的数字 |
| `X03-TYPE-007` | 选择器形态未实现 | 仅适用于未进入 C1 的旧调用边界 |
| `X03-TYPE-008` | 无序容器高级选择 | 对字典表使用范围、多选、步长、随机，或范围穿过字典表 |
| `X03-TYPE-009` | 非法步长 | 步长不是整数或静态值为零 |
| `X03-TYPE-010` | 非法随机数量 | 数量不是非负整数或超出可表示范围 |
| `X03-TYPE-011` | 随机候选不足 | 无放回超量，或从空来源抽取正数 |
| `X03-TYPE-012` | 选择器广播赋值错误 | 右值非标量、目标随机/不可变或使用不支持的赋值形式 |
| `X03-TYPE-013` | 随机种子错误 | `random.seed` 参数不是合法非负整数语义值 |
| `X03-TYPE-014` | 随机种子参数数量错误 | `random.seed` 不是恰好一个参数 |

语法阶段还会用 `X03-PARSE-004` 拒绝 `const name[path]`。`const` 的编译期常量语义与
容器位置锁定不是同一件事，后者尚未进入 C0。

未知长度数组和字符串的边界无法在静态阶段证明；这类值会登记相应 Runtime 检查，
不应通过改写源码为范围或随机选择来绕过诊断。C1 的静态检查只生成计划，不执行真实
容器读写或随机抽样。

更完整的运行时堆栈、日志和调试窗口属于第 07/11 阶段；当前页面覆盖 C0/C1 类型阶段诊断，
不承诺 Runtime 执行结果。交接边界见 [03A](../../../DevDocs/03a-c0-containers.md) 和
[03B](../../../DevDocs/03b-c1-ordered-selectors.md)。
