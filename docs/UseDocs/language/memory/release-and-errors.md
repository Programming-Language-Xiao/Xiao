---
id: language.memory.release-and-errors
title: 释放、循环引用与诊断
status: verified
audience: learner
module: rust.xiao-lifetime
stage: "06A"
version: "0.1.0"
related:
  - README.md
  - static-analysis.md
  - ../control-flow/returns-and-control.md
  - ../../troubleshooting/README.md
  - ../../../DevDocs/06a-lifetime-static-closure.md
---

# 释放、循环引用与诊断

作用域正常结束、`return`、`break`、`continue`、一般错误、构造失败、动态检查失败和致命
退出分别拥有释放计划。存在拥有依赖时遵循拓扑顺序；没有依赖的值按声明逆序处理。同一份
计划中的值只出现一次，已经返回或转移到外层的值不会在当前作用域再次释放。

边 `A -> B` 的固定含义是 A 必须先于 B 释放。弱边不延长 B 的生命周期，也不参加强边
拓扑。当前阶段只提供 Strong/Weak 静态模型，尚未开放 Xiao 源码中的 Weak 构造和升级操作。

对象直接持有自身或两个容器对象相互强持有，都会产生 `X06-LIFETIME-001`：

```xiao
a = []
b = [a]
a[0] = b
```

普通变量共享同一对象不等于对象互相持有，因此正常别名不会被误报为循环引用。真正的强环
必须在后续可用的 Weak 边上打断；Xiao 不会声称引用计数可以自动回收强环。

| 编号 | 含义 |
| --- | --- |
| `X06-LIFETIME-001` | 检测到强引用对象环 |
| `X06-LIFETIME-002` | 所有权图边无效 |
| `X06-LIFETIME-003` | 引用了不存在的作用域或值 |
| `X06-LIFETIME-004` | 生命周期事实冲突（预留） |
| `X06-LIFETIME-005` | 动态边界需要 Runtime 检查 |

系统文案以后可以随 `[language]` 切换；程序应匹配稳定编号、`message_id`、结构化参数和
源码位置，不要匹配当前中文预览文本。完整运行时堆栈、日志和调试窗口属于后续阶段。
