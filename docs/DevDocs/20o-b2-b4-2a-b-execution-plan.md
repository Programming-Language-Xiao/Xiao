# 20O-B2-B4-2A/B. cmath TAC/字节码与 VM 执行规划

> **B4-2A 已完成 Windows 本机退出：TAC/字节码 65/65，FIX1 恢复 22c 29/29。B4-2B 已在 `246bea46` 完成 Windows 本机内部验收（见 §3.4），B4-2 内部退出；B4-3 起、B5～B7 与 B2-C 仍未开始。**
> 入口为 B4-1 产品代码 `c7a45e10`、B4-2 边界规划 `162d94ea`、Runtime 入口裁定 `8b8d5e3e` 和完整签名记录 `a5d7c866`。
> B4-2A 验证 TAC/字节码 34～48 往返；若发现缺口，只修对应 lowering/校验。B4-2B 接入 15 个 Runtime 门面与 VM 分派、错误及清理。
> 30 条 cmath 导入后调用探测仍拒绝，FIX1 已恢复未使用导入的子 VM/归档拒绝；`language_available=false`，B4-4 才接双路径。
> 2026-11-12 实施及自审：`09fa912b` 补齐 34～48 往返、完整 TAC/目录、精确拒绝与解码反例；只修改测试。原 `fe752363` 开工检查中，`c7a45e10` 至该入口无产品差异；当前已新增 TAC 测试。证据见[自审记录](assets/20o-b2-b/entry-20261112-b4-2a/review-20261112/README.md)。
> 22c 归属已由直接复跑确定：`7d8b85e4` 为 29/29，`c7a45e10` 为 27/29；修复 `c4610211` 为 29/29。B4-2A 退出回执 `b4-2a-exit.json` 见 [FIX1 证据](assets/20o-b2-b/entry-20261112-b4-2a-fix1/README.md)；旧 blocked 记录保留，当时另批安排的B4-2B现已按§3.4完成。

## 一、入口与冻结接口

| 项 | 当前状态 |
| --- | --- |
| 类型/IR | 15 项 `IntrinsicCall` 已保留 ID/参数/span |
| TAC lowering | 现有 `lower_intrinsic_call` 已用 `CallSig::intrinsic(id)`、`FuncId(0)`、`TacOp::Call` |
| VM | 15 项直接原语分派已接共享 Runtime；002/009/span/清理及挂起槽隔离已验证 |
| Runtime | crate 根 15 个 `cmath_*` Rust-only 门面已按冻结接口实现，直接复用私有内核 |
| 导入 | 30 条导入后调用仍拒绝；FIX1 恢复子 VM 未使用导入/归档写前拒绝，22c 29/29 |

已冻结的 Runtime API（B4-2B 使用）：

```text
xiao_runtime::cmath_sqrt(value: f64) -> RuntimeResult<f64>
xiao_runtime::cmath_floor(value: f64) -> RuntimeResult<f64>
xiao_runtime::cmath_ceil(value: f64) -> RuntimeResult<f64>
xiao_runtime::cmath_round(value: f64) -> RuntimeResult<f64>
xiao_runtime::cmath_trunc(value: f64) -> RuntimeResult<f64>
xiao_runtime::cmath_abs(value: f64) -> RuntimeResult<f64>
xiao_runtime::cmath_min(left: f64, right: f64) -> RuntimeResult<f64>
xiao_runtime::cmath_max(left: f64, right: f64) -> RuntimeResult<f64>
xiao_runtime::cmath_pow(base: f64, exponent: f64) -> RuntimeResult<f64>
xiao_runtime::cmath_sin(value: f64) -> RuntimeResult<f64>
xiao_runtime::cmath_cos(value: f64) -> RuntimeResult<f64>
xiao_runtime::cmath_tan(value: f64) -> RuntimeResult<f64>
xiao_runtime::cmath_exp(value: f64) -> RuntimeResult<f64>
xiao_runtime::cmath_ln(value: f64) -> RuntimeResult<f64>
xiao_runtime::cmath_log10(value: f64) -> RuntimeResult<f64>
```

内部 API，不承诺语言级 semver；不读写 Runtime ABI 挂起错误槽。

### 1.1 要做

1. 验证/补齐 34～48 的 TAC 编码、解码、校验和 PC→span 回填；
2. 增加 15 项 TAC/字节码往返测试，确认格式版本未变；
3. 在 `xiao-runtime` 增加 15 个 Rust-only 函数，内部复用私有 `cmath`；
4. VM 按 `VmBinding::Cmath*` 分派到上述函数；
5. 保留 002/009、source span、catch 路由和清理；
6. 验证 `Pure` 不删除结果未使用的可失败调用；
7. 保持 30 条导入路径拒绝；
8. 保存 TAC/VM/错误/清理/回归证据。

### 1.2 明确不做

- 不接 LLVM 动态/静态（B4-3）；
- 不接导入/导出/移除/归档/序列化（B4-4）；
- 不改 30 条 planned 导入路径；
- 不新增 intrinsic 49+/C 符号/语言公开 API；
- 不升 ABI 1.19/21，不改 registry/教程/锁；
- 不跑旧 721，不关闭历史债务。

## 二、B4-2A：TAC/字节码

### 2.1 现状核对

1. `lower_intrinsic_call` 已把 `IntrinsicCall` 降为 `TacOp::Call{callee:FuncId(0), signature:CallSig::intrinsic(id), arguments}`；
2. `CallSig` 用两个 u16 槽承载稳定 u32 ID；
3. `TAC_VERSION`/字节码格式版本已有；不得新增 opcode 或版本；
4. source span 由 `TacInstr.span` + PC→span map 承载，不加指令载荷；
5. 34～48 在 intrinsics 表 active，TAC 校验应接受；49/未知 ID 应拒绝。

### 2.2 执行

1. 对 15 个 ID 各构造一元/二元调用，覆盖位置/命名/逆序命名；
2. 编码→解码后逐项比较 TAC 调用 ID、参数顺序和 source span；VM 执行留在 B4-2B；
3. 验证 `TAC_VERSION`/`TAC_BYTECODE_ABI_VERSION`/容器格式未变；
4. 验证编码器和生产验证器均拒绝 49/未知 ID；
5. 若现有 lowering 已足够，不加生产代码，只补测试/证据；若发现缺失，只改字节码 lowering/校验，不接 VM。
6. 保存 15 项往返样本、span、版本和负例。

退出：34～48 TAC/字节码往返通过；格式版本未变；VM 未开始。

## 三、B4-2B：Runtime 门面与 VM

### 3.0 开工单（2026-11-13，记录开工时点）

1. 入口：B4-2A 退出产品 `c4610211`、TAC 测试 `09fa912b`；[FIX1 退出记录](assets/20o-b2-b/entry-20261112-b4-2a-fix1/b4-2a-exit.json)记录 22c 29/29、字节码 65/65、前端 3/3、wrapper 8/8。开工前复核 `c4610211` 到当前 HEAD 的 `core/`、`stdlib/` 无产品差异；如有差异，按实际差异重新登记回归，不能沿用旧回执。
2. 开工时：`VmBinding::Cmath*` 声明键已存在，15 项 VM 分派与 Rust-only Runtime 门面尚未实现；后续已按 §3.4 完成，`language_available=false` 仍保持。
3. 必须保留：FIX1 的 planned 门控（`DriverModuleLoader::compile`、归档写前导入扫描）、30 条导入后调用拒绝、22c 29/29 与子 VM/归档边界 3 项；不得靠放宽、删除或忽略测试推进。
4. 顺序：3.1 Runtime 门面 → 3.2 VM 分派 → 3.3 错误/span/清理/Pure → 证据与退出。任一步发现必须改 checker 或计划外组件时，先停下另立裁定。
5. 证据：新建 `assets/20o-b2-b/entry-20261113-b4-2b/`；原始日志按 SHA-256 内容寻址，同一内容只存一个对象；每次运行单独保存 PID、时间、argv、cwd、环境、源码/DevDocs 摘要和退出码。文档修改合并到定稿后只跑一次最终 docs 门禁，中间轮次不重复全量门禁；FIX1 历史目录与日志不改写。
6. 不做：LLVM（B4-3）、双路径/归档接线（B4-4）、C ABI、ABI 1.19/21、registry/教程、旧 721；Linux/macOS/远端 CI 不具备时如实登记。

### 3.1 Runtime 门面

1. 在 `xiao-runtime` crate 根增加 15 个 `pub fn cmath_*`；
2. 每个函数只调用现有私有 `cmath` 内核，不复制数学/域/ULP；
3. 返回 `RuntimeResult<f64>`；错误 009 原样返回；
4. 不读写 Runtime ABI 挂起错误槽；不调用 C ABI 包装；
5. 标记内部 API，文档说明非语言公开 API、无 semver 承诺；
6. 单元测试覆盖 15 项正常/域/NaN/Inf/符号/下溢。

退出：15 个 Rust-only 函数可用；无 C ABI/挂起槽副作用。

### 3.2 VM 分派

1. `execute_intrinsic` 增加 `VmBinding::Cmath*` 分支，按 15 个 ID 分派；
2. 一元/二元参数数量与 `cmath_parameters(id)` 一致；
3. 位置/命名参数按规范顺序绑定；表达式仍按源码顺序求值；
4. 参数形状/类型错映射 X06-RUNTIME-002；
5. 数学域/非有限映射 X06-RUNTIME-009，进入 `Fault::Error`/catch；
6. source span 附到 fault；成功不清 Runtime ABI 挂起错误；
7. 不因 `Pure` 删除结果未使用调用；
8. 30 条导入路径仍拒绝；VM 只执行直接原语/内部 IR；
9. 若分派改动触及模块加载或归档路径，必须重跑 22c 29/29 与 FIX1 子 VM/归档边界 3 项，不得以本批专项测试替代。

退出：VM 15 项可执行；错误/span/清理一致；导入仍 planned。

### 3.3 测试与证据

1. 15 项 VM 正常/特殊/域/NaN/Inf；
2. 位置/命名/逆序命名参数绑定；
3. 编码→解码后的 009、span、catch；
4. 未使用结果的可失败调用在 catch 中观察到 009（覆盖启用的优化配置）；
5. finally/release 计划和临时值清理；
6. 002/009 错误顺序；unknown ID 拒绝；
7. 30 条 planned 导入路径仍拒绝；
8. B4-0 向量、B4-1 类型/IR、B3-4 wrapper/22c、B1/B2/B3 历史回归；
9. 保存源码、工具链、目标、PID、日志、前后摘要；FIX1 后的新证据目录按原始日志字节 SHA-256 内容寻址，重复内容只存一个对象；每次运行仍单独保存 label、PID、时间、argv、cwd、环境/源码摘要、退出码，并引用日志对象的路径、字节数和摘要；
10. 记录 Windows 本机与未执行平台；
11. 文档修改合并到定稿后再跑最终 docs 门禁，中间轮次不重复全量门禁；FIX1 历史目录不改写。

退出：B4-2 内部证据完整；B4-3 未开始；`language_available=false`。

### 3.4 本机执行与退出（2026-11-13）

初实现为 `57ca957c`；首次完整docs门禁以A0-SIZE-001发现exec.rs达2514行。随后结构修复 `246bea46` 按声明类别委派数值适配器，未放宽2500行门禁或测试。最终产品 `246bea46` 只改 Runtime 根门面、VM 分派/参数绑定、专项测试及相邻代码说明共8个允许路径。入口回归43/43；实施后驱动50/50（含VM专项7、22c29、wrapper8、前端3、FIX1边界3），Runtime272/272、VM35/35、字节码65/65；数值回归15项通过，3项既有原生测试仍按原规则忽略。入口、首轮及修订验证分用全新baseline/fixed/revised target；首轮失败门禁与通过测试保留历史原样，最终退出只引用revised；开发target另列，不充作正式证据。

1,440项三载体对照验证15项逐位/错误码、位置与逆序命名；45项堆值失败验证强引用恢复与无部分结果。生产源码和`.xiaoc`可执行直接原语；decode后009保持原span，O0～O3未使用Pure结果仍进入catch并保持finally及完整释放序列。参数表达式009/013顺序与Runtime ABI挂起槽隔离有独立测试。

Clippy、fmt及B1/B2/B3/B3-4/B4-0/1/FIX1固定历史审计均通过。证据、最终docs门禁、独立Runtime归档摘要及只读反例见[本机记录](assets/20o-b2-b/entry-20261113-b4-2b/README.md)。30条模块导入仍planned拒绝，模块加载器/归档/LLVM/版本/锁/registry/教程未改；`language_available=false`。B4-2内部退出不启动B4-3，不代表Linux/macOS、远端CI、性能或旧721已执行。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| TAC/字节码 | 34～48 往返、ID/参数/span、版本不变、49 拒绝 |
| Runtime 门面 | 15 函数、RuntimeResult、无挂起槽/C ABI |
| VM | 15 项、位置/命名/逆序、002/009 |
| 错误 | catch、span、decode 后、挂起槽隔离 |
| 清理 | finally/release、成功/失败临时值 |
| Pure | 未使用结果仍可报错 |
| 导入 | 30 条 planned 仍拒绝 |
| 回归 | B4-0/1、B3-4、B1/B2/B3 |
| 边界 | 无 LLVM/导入/归档/ABI/registry/721 |

## 五、退出条件

1. 34～48 TAC/字节码往返通过，格式版本未变；
2. 15 个 Rust-only Runtime 函数按冻结签名可用；
3. VM 15 项绑定共享内核，无第二实现；
4. 002/009、span、catch、finally/release/临时值证据完整；
5. 30 条 planned 导入路径仍拒绝，`language_available=false`；
6. 无 LLVM/双路径/归档/ABI/registry 改动；
7. B4-0/1、B3-4、B1/B2/B3 回归通过；旧 721 未跑；
8. B4-3 未开始；三平台/性能仍未声称。

## 六、风险与禁止

- 在 VM/Runtime 门面里复制数学/域/ULP；
- 仅改 TAC 不绑 VM，或仅绑 VM 不解码；
- 把 34～48 当无条件 Pure 删除；
- 让 VM 调用 C ABI 从而写 Runtime 挂起槽；
- 把 30 条 planned 导入改成可执行；
- 提前接 LLVM/归档/ABI/registry；
- 用 B4-2 证据冒充 B4-4 双路径或 B2-C 发布。

## 七、交接

B4-2 完成后进入 B4-3：LLVM 动态/静态、静态 pow 共核、O0/O3。B4-4 接双路径/归档/序列化；B4-5 收口。

新 Agent 先读：

1. [B4-2 TAC/VM 规划](20o-b2-b4-2-tac-vm-plan.md) §1.3；
2. [B4-0/B4-1 记录](assets/20o-b2-b/entry-20261110-b4-0-1/README.md)；
3. [B3-4 收口](assets/20o-b2-b/entry-20261110-b3-wrapper-final/README.md)；
4. [F-5 原语/C ABI](20o-b2-freeze-b1-closeout-plan.md)；
5. [F-12 错误表](20o-b2-freeze-b3-error-table-plan.md)。

## 八、落点

```text
core/rust/crates/xiao-bytecode/src/lower/expr.rs       34～48 TAC lowering
core/rust/crates/xiao-bytecode/src/encode/              往返/校验/span
core/rust/crates/xiao-runtime/src/lib.rs                15 个 cmath_* Rust-only 门面
core/rust/crates/xiao-runtime/src/value/arithmetic/cmath.rs  私有内核
core/rust/crates/xiao-vm/src/semantics/exec.rs          VmBinding::Cmath* 分派
core/rust/crates/xiao-vm/src/semantics/numeric_intrinsics.rs  VM 绑定
core/rust/crates/xiao-driver/tests/                     TAC/VM/错误/清理测试
docs/DevDocs/assets/20o-b2-b/entry-<date>-b4/           B4-2 证据
```

本页只做 B4-2A/B；B4-3 起、B5～B7 与 B2-C 仍后置。

FIX1 之后的新证据目录统一用 `logs/sha256/<前两位>/<完整摘要>.log` 保存原始日志对象；执行回执各自记录 `(label, pid, started/finished, argv, cwd, source_sha, docs_commit/docs_commit_after, docs_tree_before/after, docs_tree_sha256, docs_tree_canonical_before/after, docs_tree_commit_raw/canonical_sha256, exit_code, log_ref)`，审计确认每个内容对象唯一且被回执引用。最终 docs 门禁须绑定已提交 DevDocs 快照：摘要覆盖所有 `docs/DevDocs` 静态文件的相对路径与工作树原始字节，包含本批 README、工具和反例源码，只排除运行期间生成的日志对象、回执与收口清单；原始和规范摘要须在门禁前后相同、Git HEAD 不变，UTF-8/LF 规范摘要须与固定 Git commit tree 的规范摘要相等，并单列 Git blob 原始摘要。相同日志的多次运行共享对象，不共享回执；历史日志不迁移。
