# `xiao-runtime/src/value`

这里放 Runtime 标量、`str` 堆对象和统一 `RuntimeValue`。固定宽度标量保持内联，
字符串通过不透明强句柄管理；本阶段只实现最小布尔奇偶加减、字符串拼接和同宽度
数值安全检查，复杂容器值留给后续 Runtime 阶段。
