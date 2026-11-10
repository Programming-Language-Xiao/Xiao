# 20O-B2-B4-2. cmath TAC/字节码与 VM 实施规划

> **B4-0/B4-1 已完成；B4-2 尚未开工，B4-3 起、B5～B7 与 B2-C 仍未开始。**
> 入口为 B4-1 产品代码 `c7a45e10` 与证据；类型/IR 已能表达 34～48，VM/LLVM 尚未绑定。
> B4-2 只做 TAC/字节码编码与 VM 执行；不接 LLVM、双路径导入/归档、ABI 1.19/21 或 registry。
> 30 条 `std.cmath`/`stdlib.cmath` 导入路径仍按 B3-4 边界拒绝；B4-4 才接双路径。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B4-0 | 正式向量已迁入 `tests/fixtures/docs/cmath/`，逐字节一致 |
| B4-1 | 15 项类型/IR 接通；`IntrinsicCall` 保留 ID/参数/span |
| VM/LLVM | 34～48 尚无 VM/LLVM 绑定；直接原语可生成 IR，但不能执行 |
| 导入边界 | 30 条 cmath 导入路径仍 planned 拒绝；`language_available=false` |
| 本批目标 | TAC/字节码 34～48 编码、VM 15 项执行、错误/span/清理 |
| 未做 | B4-3 LLVM、B4-4 双路径/归档、B5 版本、B6 性能、B2-C 发布 |

B4-2 是 VM 内部可执行批，不是 `std.cmath` 模块导入可用批。B4-4 才接真实导入/归档；B4-2 的测试主要经直接原语或内部 IR。

### 1.1 要做

1. TAC/字节码按 34～48 编码/解码，保持既有容器版本；
2. VM 按 `VmBinding::Cmath*` 或等价 ID 分派到共享 Runtime 内核；
3. 一元/二元参数与 F-5 表一致；不复制数学实现；
4. 错误保留 009、source span、挂起错误和可 catch 路径；
5. 调用成功/失败清理、临时值释放与既有规则一致；
6. 不把 34～48 仅因 `Pure` 从“结果未使用”删除；
7. 保存字节码往返、VM 15 项、错误/span/清理证据；
8. 保持 30 条导入路径拒绝，B4-4 才改变。

### 1.2 明确不做

- 不接 LLVM 动态/静态路径（B4-3）；
- 不接 `std.cmath`/`stdlib.cmath` 导入、导出、移除、归档（B4-4）；
- 不把 30 条 planned 导入改成可执行；
- 不新增 intrinsic 49+/C 符号，不改 1～48/ID 100；
- 不升 ABI 1.19/21，不改 registry/教程/锁；
- 不跑旧 721，不关闭历史债务。

## 二、开工闸门

1. B4-0/B4-1 审计通过，代码 `c7a45e10` 未漂移；
2. 15 项 `IntrinsicCall` 的 ID/参数/span 和 30 条 planned 拒绝可复现；
3. intrinsic 34～48 声明、15 C 符号、共享 Runtime 内核可复现；
4. ABI 1.18/生成器 20/接口 1/cmath planned 未变；
5. B3-4 wrapper 8/0、22c 29/0、B1/B2/B3 历史审计通过；
6. 唯一任务根、独立 target/Runtime/缓存已建立。

## 三、分批执行

### B4-2A：TAC/字节码编码

1. 34～48 的 IntrinsicCall 使用既有 TAC 指令承载；
2. 保持现有 opcode/标签/容器版本，不新增容器版本；
3. 一元/二元参数数量、ID、返回 `float` 与 IR 一致；
4. 编码/解码往返保留 ID、实参顺序和 source span；
5. 不把 ID 写死到用户源码字符串；仍按稳定数字身份；
6. 保存 34～48 的编码/解码样本和容器版本证据。

退出：TAC/字节码可往返 15 项；无容器版本变化。

### B4-2B：VM 绑定与执行

1. VM 按 34～48 ID 分派到共享 Runtime `cmath` 内核；
2. 一元：sqrt/floor/ceil/round/trunc/abs/sin/cos/tan/exp/ln/log10；
3. 二元：min/max/pow；
4. 错误 009 保留可 catch 身份、source span 和挂起错误；
5. 不复制数学/域/ULP 逻辑，不调用宿主 libm 形成第二套；
6. 成功/失败清理、临时值释放按既有 VM 规则；
7. 不因 `Pure` 删除结果未使用调用；错误仍可发生；
8. 保持 30 条导入路径拒绝，不改变模块可用性。

退出：VM 可执行 15 项直接原语；错误/span/清理一致；导入仍 planned。

### B4-2C：测试与证据

1. 15 项 VM 正常值/特殊值/域/NaN/Inf；
2. 一元/二元参数数量、ID、返回类型；
3. 009 错误、source span、可 catch、挂起错误；
4. 字节码往返、VM 执行、临时值/清理；
5. 30 条 planned 导入路径仍拒绝；
6. B4-0 向量、B4-1 类型/IR、B3-4 wrapper/22c、B1/B2/B3 历史回归；
7. 保存源码、工具链、目标、PID、日志和前后摘要；
8. 记录 Windows 本机与未执行平台。

退出：15 项 VM 证据完整；B4-3 未开始；语言可用性仍 false。

### B4-2D：收口与 B4-3 交接

1. 复核 34～48 编码/VM/错误边界；
2. 复核没有 LLVM/导入/归档/ABI/registry 改动；
3. 复核 30 条 planned 路径仍拒绝，`language_available=false`；
4. 形成 B4-3 入口：LLVM 动态/静态、静态 pow 共核。

退出：B4-2 内部收口；B4-3 未开始。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| TAC/字节码 | 34～48 编码/解码、ID/参数/span、容器版本不变 |
| VM | 15 项正常/特殊/域/NaN/Inf |
| 错误 | 009、span、可 catch、挂起错误 |
| 清理 | 成功/失败临时值释放、无部分结果 |
| 导入 | 30 条 planned 路径仍拒绝 |
| 回归 | B4-0/1、B3-4、B1/B2/B3 |
| 边界 | 无 LLVM/导入/归档/ABI/registry/721 |

## 五、退出条件

1. 34～48 的 TAC/字节码可往返，容器版本未变；
2. VM 15 项绑定共享内核，无第二实现；
3. 009/span/挂起/清理证据完整；
4. 30 条 planned 导入路径仍拒绝，`language_available=false`；
5. 无 LLVM/双路径/归档/ABI/registry 改动；
6. B4-0/1、B3-4、B1/B2/B3 回归通过；旧 721 未跑；
7. B4-3 未开始；三平台/性能仍未声称。

## 六、风险与禁止

- 在 VM 里重写 cmath 数学/域/ULP；
- 只加 TAC/字节码但不绑 VM，或只绑 VM 不解码；
- 把 34～48 当无条件 Pure 删除；
- 把 30 条 planned 导入改成可执行来“证明”VM 接通；
- 提前接 LLVM/归档/ABI/registry；
- 用 B4-2 VM 证据冒充 B4-4 双路径或 B2-C 发布。

## 七、交接

B4-2 完成后进入 B4-3：LLVM 动态/静态路径、静态 pow 与 cmath 共核、O0/O3 证据。B4-4 接双路径/归档/序列化；B4-5 收口。

新 Agent 先读：

1. [B4 语言全链路规划](20o-b2-b4-language-integration-plan.md)；
2. [B4-0/B4-1 记录](assets/20o-b2-b/entry-20261110-b4-0-1/README.md)；
3. [B3-4 收口证据](assets/20o-b2-b/entry-20261110-b3-wrapper-final/README.md)；
4. [F-5 原语/C ABI](20o-b2-freeze-b1-closeout-plan.md)；
5. [F-12 错误表](20o-b2-freeze-b3-error-table-plan.md)。

## 八、落点

```text
core/rust/crates/xiao-bytecode/                      34～48 TAC/字节码
core/rust/crates/xiao-vm/src/semantics/              VM cmath 分派
core/rust/crates/xiao-runtime/src/value/arithmetic/  共享 cmath 内核
core/rust/crates/xiao-driver/tests/                  VM/字节码测试
docs/DevDocs/assets/20o-b2-b/entry-<date>-b4/        B4-2 证据
```

本页只做 B4-2；B4-3 起、B5～B7 与 B2-C 仍后置。
