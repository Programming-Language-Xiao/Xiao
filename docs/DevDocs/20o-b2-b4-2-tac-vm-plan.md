# 20O-B2-B4-2. cmath TAC/字节码与 VM 实施规划

> **B4-0/B4-1 已完成；B4-2A 已开工，TAC/字节码专项通过，22c 两项回归失败，尚未退出；B4-2B、B4-3 起、B5～B7 与 B2-C 仍未开始。**
> 入口为 B4-1 产品代码 `c7a45e10` 与证据；类型/IR 已能表达 34～48，声明表已有 `VmBinding::Cmath*` 键，但 VM 分派实现和 LLVM lowering 尚未接通。
> B4-2 只做 TAC/字节码编码、VM 执行和必要的 Rust-only Runtime 调用桥；不接 LLVM、双路径导入/归档、C ABI、ABI 1.19/21 或 registry。
> 30 条 `std.cmath`/`stdlib.cmath` 导入后调用探测仍拒绝；22c 的未使用导入子 VM 和归档拒绝存在失败，不能宣称完整保持 B3-4 边界。B4-4 才接双路径。本轮状态与原始运行见[自审记录](assets/20o-b2-b/entry-20261112-b4-2a/review-20261112/README.md)。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B4-0 | 正式向量已迁入 `tests/fixtures/docs/cmath/`，逐字节一致 |
| B4-1 | 15 项类型/IR 接通；`IntrinsicCall` 保留 ID/参数/span |
| VM/LLVM | 34～48 已登记声明层 VM 键；VM 分派实现与 LLVM lowering 缺失，直接原语可生成 IR 但不能执行 |
| 导入边界 | 30 条导入后调用探测仍拒绝；22c 27/29，子 VM 未使用导入/归档阻塞；`language_available=false` |
| 本批目标 | TAC/字节码 34～48 编码、VM 15 项执行、错误/span/清理 |
| 未做 | B4-3 LLVM、B4-4 双路径/归档、B5 版本、B6 性能、B2-C 发布 |

B4-2 是 VM 内部可执行批，不是 `std.cmath` 模块导入可用批。B4-4 才接真实导入/归档；B4-2 的测试主要经直接原语或内部 IR。

### 1.1 要做

1. TAC/字节码按 34～48 编码/解码，保持既有容器版本；
2. VM 按 `VmBinding::Cmath*` 或等价 ID 分派到共享 Runtime 内核；
3. 一元/二元参数与 F-5 表一致；不复制数学实现；
4. 数学域/非有限错误保留 009 与 source span，并经 VM `Fault::Error` 进入可 catch 路径；
5. 调用成功/失败清理、临时值释放与既有规则一致；
6. 不把 34～48 仅因 `Pure` 从“结果未使用”删除；
7. 保存字节码往返、VM 15 项、错误/span/清理证据；
8. 保持 30 条导入路径拒绝，B4-4 才改变。
9. VM 不读写 Runtime C ABI 的挂起错误槽，错误通过 Rust `Result` 转为 VM `Fault::Error`。

### 1.2 明确不做

- 不接 LLVM 动态/静态路径（B4-3）；
- 不接 `std.cmath`/`stdlib.cmath` 导入、导出、移除、归档（B4-4）；
- 不把 30 条 planned 导入改成可执行；
- 不新增 intrinsic 49+/C 符号，不改 1～48/ID 100；
- 不升 ABI 1.19/21，不改 registry/教程/锁；
- 不跑旧 721，不关闭历史债务。

### 1.3 已确认的 Runtime 入口

星崽已确认以下口径；B4-2A 可先开工，B4-2B 按此冻结签名接线：

1. 在 `xiao-runtime`  crate 根暴露 15 个类型化自由函数，与 F-5 原语一一对应；
2. 返回 `RuntimeResult<f64>`，错误直接复用 Runtime 的 009 等身份；不读写 Runtime ABI 挂起错误槽；
3. 作为 Runtime 内部 API，不承诺语言级 semver 稳定性；文档注明仅供 VM/后端调用，不是语言公开 API；
4. 一元函数均为 `fn name(value: f64) -> RuntimeResult<f64>`：`cmath_sqrt`、`cmath_floor`、`cmath_ceil`、`cmath_round`、`cmath_trunc`、`cmath_abs`、`cmath_sin`、`cmath_cos`、`cmath_tan`、`cmath_exp`、`cmath_ln`、`cmath_log10`；
5. 二元函数均为 `fn name(left: f64, right: f64) -> RuntimeResult<f64>`：`cmath_min`、`cmath_max`；`cmath_pow(base: f64, exponent: f64) -> RuntimeResult<f64>`；
6. 函数内部复用现有私有 `cmath` 内核；不复制数学/域/ULP 逻辑；
7. VM 通过这 15 个函数调用，不跨 crate 访问私有模块，也不借 C ABI 状态槽转运 VM 错误；
8. 本批不把该入口加入 intrinsic/C ABI/语言公开面；B4-3/B4-4 再接入各自路径。

## 二、开工闸门

以下为批准的要求。B4-2A 复跑发现第 5 项中的当前 22c 不满足 29/0；专项测试虽通过，仍保留阻塞，B4-2B 不启动。固定历史审计通过不替代当前运行。

1. B4-0/B4-1 审计通过，代码 `c7a45e10` 未漂移；
2. 15 项 `IntrinsicCall` 的 ID/参数/span 和 30 条 planned 拒绝可复现；
3. intrinsic 34～48 声明、15 C 符号、共享 Runtime 内核可复现；
4. ABI 1.18/生成器 20/接口 1/cmath planned 未变；
5. B3-4 wrapper 8/0、22c 29/0、B1/B2/B3 历史审计通过；
6. 唯一任务根、独立 target/Runtime/缓存已建立。
7. B4-2B 的调用边界已明确、精确 API 已确认：现有 `value::arithmetic::cmath` 函数是 crate 私有；C ABI 适配器通过状态码报告错误并写 Runtime 挂起错误槽。VM 不得直接跨 crate 调用私有函数，也不得借 C ABI 状态槽转运 VM 错误。星崽已确认采用 15 个类型化 Rust-only 函数、`RuntimeResult<f64>` 返回和内部 API 口径，精确签名见 §1.3；B4-2A 可先开工。

## 三、分批执行

### B4-2A：TAC/字节码编码

执行细化见[B4-2A/B TAC与VM执行规划](20o-b2-b4-2a-b-execution-plan.md)；B4-2A只验TAC往返。

1. 34～48 的 IntrinsicCall 使用既有 TAC 指令承载；
2. 复用 `TacOp::Call`、`FuncId(0)` 与 `CallSig::intrinsic(id)`；ID 仍由既有调用签名字段承载；
3. 保持现有 opcode/标签/`TAC_VERSION`/字节码格式版本，不新增版本；
4. 一元/二元参数数量、ID、返回 `float` 与 IR 一致；
5. 编码/解码往返保留 ID 和实参顺序；source span 通过现有 PC→span map 回填到 `TacInstr.span`，不得另加指令载荷；
6. 不把 ID 写死到用户源码字符串；仍按稳定数字身份；
7. 保存 34～48 的编码/解码样本、source span 和格式版本证据。

退出：TAC/字节码可往返 15 项；无容器版本变化。

### B4-2B：VM 绑定与执行

执行细化见[B4-2A/B TAC与VM执行规划](20o-b2-b4-2a-b-execution-plan.md)；B4-2B按§1.3冻结入口接VM。

1. VM 按 34～48 ID 分派到共享 Runtime `cmath` 内核；
2. 一元：sqrt/floor/ceil/round/trunc/abs/sin/cos/tan/exp/ln/log10；
3. 二元：min/max/pow；
4. 通过 §1.3 已确认的 15 个 Rust-only 函数调用现有私有内核；该入口返回 `RuntimeResult`，不得调用会写 Runtime 挂起错误槽的 `xiao_runtime_cmath_*` C ABI 包装；
5. 按 `cmath_parameters(id)` 绑定位置/命名参数到规范顺序；表达式仍按源码顺序求值，形状/类型不符映射 X06-RUNTIME-002；
6. 数学域与非有限结果按 X06-RUNTIME-009，作为 VM `Fault::Error` 进入既有 handler/catch 路由；指令 fault 注记附上 source span；成功调用不清除 Runtime ABI 的既有挂起错误；
7. 不复制数学/域/ULP 逻辑，不调用宿主 libm 形成第二套；
8. 成功/失败清理、临时值释放按既有 VM 规则；
9. 不因 `Pure` 删除结果未使用调用；错误仍可发生；
10. 保持 30 条导入路径拒绝，不改变模块可用性。

退出：VM 可执行 15 项直接原语；错误/span/清理一致；导入仍 planned。

### B4-2C：测试与证据

1. 15 项 VM 正常值/特殊值/域/NaN/Inf，并逐 ID 对照共享 Runtime 内核结果；
2. 一元/二元参数数量、参数类型、ID、返回类型及未知 ID 的拒绝路径；
3. 从编码后再解码的 TAC 执行一个 009 域错误，断言错误码、原始 source span 和可 catch 路由；
4. 验证命名参数逆序传入仍按 `cmath_parameters(id)` 绑定，并验证实参表达式的错误仍按源码顺序先后；VM 错误不污染 Runtime ABI 挂起错误槽；
5. 加入一个未使用结果的可失败 cmath 调用并在 `catch` 中观察 009，覆盖启用的字节码优化配置，证明 `Pure` 不抹除可观察错误；
6. 字节码往返、VM 执行、失败后的 finally/release 计划和临时值清理；
7. 30 条 planned 导入路径仍拒绝；
8. B4-0 向量、B4-1 类型/IR、B3-4 wrapper/22c、B1/B2/B3 历史回归；
9. 保存源码、工具链、目标、PID、日志和前后摘要；
10. 记录 Windows 本机与未执行平台。

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
| 错误 | 002/009、decode 后 span、`Fault::Error` catch 路由；Runtime ABI 挂起槽不变 |
| 清理 | 成功/失败 finally 与 release 计划、无部分结果 |
| 导入 | 30 条 planned 路径仍拒绝 |
| 回归 | B4-0/1、B3-4、B1/B2/B3 |
| 边界 | 无 LLVM/导入/归档/ABI/registry/721 |

## 五、退出条件

1. 34～48 的 TAC/字节码可往返，容器版本未变；
2. VM 15 项经 §1.3 已确认的 Rust-only Runtime 错误返回入口绑定共享内核，无第二实现；
3. 002/009、encode/decode 后 span、catch/清理及 ABI 挂起槽隔离证据完整；
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
