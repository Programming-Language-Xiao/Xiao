# 10R 后续：静态溢出统一错误路径立项

状态：**静态数值失败路径已实现（2026-10-12）**；动态错误参数与静态异常清理链仍是后续缺口。
来源：[10R §2.5](10r-release-accounting-and-native-ci-gate.md)；实现与证据见
[10R-实现](10r-static-overflow-implementation.md) §六。

实现前沿用 [10L §11.4](10l-native-dynamic-alignment.md) 的量化：静态 ir.rs 的
llvm.trap 有 **7 个发射点模板**，不是每个程序固定七个块。范围包括 checked 整数算术、
整数除零/最小值除负一、整数窄化和浮点有限性/范围检查。Windows 历史观测退出码
0xC000001D，未产生 Xiao 错误摘要。动态路径已经通过 Runtime 算子返回
X06-RUNTIME-009，不能将动态验证结论外推到静态路径。

已实现部分：编译器能预判的数值检查失败走 Runtime 可恢复错误通道，携带源码 span，
输出标准报告并以 Runtime 退出码结束。溢出使用 X06-RUNTIME-009；
除零应复用 Runtime 的专用除零身份，不能将所有七处都改为溢出码。
通过 checked intrinsic 的成功路径保留现有计算结果，不用删除检查换取性能。

未实现部分：静态 `catch`/`finally` 清理链传播仍按 10R D7 另立批次；10Z-D 的动态
`code`/`message` 参数仍只接受原生字符串字面量。

与 10E 协调：真正外部硬件非法指令仍为平台 Fatal；编译器主动发射的语言级数值
失败不再借非法指令模拟。不能将平台非法指令捕获后一律翻译成可恢复 ArithmeticError。

影响面：ir.rs 的失败块、静态函数错误传播约定、Runtime ABI 及所需符号版本、
纯静态产物是否需链接 Runtime 的裁剪/产物验证、错误位置和优化后的差分证据。
ABI 次版本当前为 **1.11**，由 `xiao_runtime_error_raise_numeric` 及静态入口兼容检查使用；
扩展须与工具链指纹和旧 Runtime 拒绝策略一致，不隐式链接完整诊断窗口。

已完成验收：Windows 受控工具链上逐模板边界用例、错误身份、span、退出码和外部 Fatal 分界；
普通报告产物无 `WS2_32.dll`，Runtime 单元测试覆盖 ABI 1.10/1.11 兼容规则。
尚未完成：Linux/macOS 裸机 O0–O3、静态 `catch`/`finally` 释放顺序、19D 受控性能基线判定和 F-3。
这些缺口仍登记为后续输入，不使用开发机计时宣称性能达标。
