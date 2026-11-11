# 20O-B2-B4-3. cmath LLVM 动态/静态与共享 pow 实施规划

> **B4-2A/B 尚未开工；B4-3 仍未开始，B4-4、B5～B7 与 B2-C 后置。**
> 入口为 B4-1 类型/IR、B4-2 的 VM 执行与已冻结的 15 个 Rust-only Runtime 门面；当前 LLVM 尚无 cmath lowering。
> B4-3 只做 LLVM 动态/静态路径与 float `**` 共核；不接导入/归档/序列化（B4-4），不升 ABI 1.19/21。
> 30 条 cmath 导入路径仍 planned 拒绝；`language_available=false`。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| 类型/IR | 15 项 `IntrinsicCall` 保留 ID/参数/span（B4-1） |
| VM | B4-2 提供 VM 分派与错误/清理；本批不复用 VM 状态机 |
| Runtime | 15 个 `xiao_runtime::cmath_*` Rust 门面与 15 个 C ABI 符号 |
| 静态 LLVM | Float 的 `**` 仍用 `llvm.pow.f64`；sfloat 的 `**` 仍用 `llvm.pow.f32`；cmath 34～48 未 lowering |
| 动态 LLVM | 有通用 intrinsic/dynamic Runtime 机制，但 cmath 分支未接 |
| 版本 | ABI 1.18、生成器 20、接口 1；本批不升 |

B4-3 是 **LLVM 后端内部验收**，不是 `std.cmath` 导入可用。B4-4 才接双路径/归档/序列化，B5 才切 1.19/21。

### 1.1 要做

1. LLVM 动态路径为 34～48 生成 Runtime C ABI 调用；
2. 静态路径为 15 项 cmath 生成同一 Runtime 内核调用；
3. 静态 Float `**` 从 `llvm.pow.f64` 切到共享 cmath_pow 内核；sfloat 的 `llvm.pow.f32` 保持原路径；
4. 正确处理 C ABI 的借读输入、拥有 out、i32 状态和挂起错误；
5. 保留 009/002 错误身份、source span 和既有静态/动态错误通道；
6. 保持 sfloat `**` 现有路径，不扩大范围；
7. 保存动态/静态 O0/O3、符号、错误和回归证据；
8. 保持 30 条 cmath 导入路径拒绝。

### 1.2 明确不做

- 不接 `std.cmath`/`stdlib.cmath` 导入、导出、移除、归档（B4-4）；
- 不接序列化/并发承载（B4-4）；
- 不新增 intrinsic 49+/C 符号，不改 1～48/ID 100；
- 不升 ABI 1.19/生成器 21，不改 registry/教程/锁；
- 不实现 sfloat cmath；
- 不跑旧 721，不关闭历史债务。

## 二、开工闸门

1. B4-2A/B 审计通过，VM 15 项、TAC 往返、30 条 planned 拒绝可复现；
2. 15 个 C ABI 符号、17,243 条参考和共享 Runtime 内核可复现；
3. 当前静态 Float `**` 仍指向 `llvm.pow.f64`、sfloat `**` 仍指向 `llvm.pow.f32`，均未半接线；
4. ABI 1.18/生成器 20/接口 1/cmath planned 未变；
5. B4-0/B4-1、B3-4、B1/B2/B3 回归可复现；
6. 唯一任务根、独立 target/Runtime/缓存已建立。

## 三、分批执行

### B4-3A：动态 LLVM 路径

1. 为 34～48 生成 `xiao_runtime_cmath_*` 调用；
2. 一元：一个借读输入 + 拥有 out；二元：min/max/pow 两个输入 + out；
3. 参数按源码顺序求值，out 槽初始化且不复用输入；
4. 状态 0 读取 out；状态 6 保留 out 并沿既有错误通道；
5. 1/2/3 结构错误不碰挂起槽，按既有 Fatal/状态处理；
6. 不因 `Pure` 把调用当可删除；
7. 保存动态 IR、C 符号、状态/错误和 O0/O3 证据。

退出：动态 LLVM 可执行 15 项 cmath；错误/所有权与 C ABI 一致。

### B4-3B：静态 LLVM 路径

1. 为 15 项 cmath 生成 `xiao_runtime_cmath_*` 调用；静态产物设置 `uses_runtime=true`；
2. 静态 float `**` 从 `llvm.pow` 改为同一 cmath_pow 调用；
3. 保留 `sfloat **` 现有 `llvm.pow.f32` 或既有路径，不扩大；
4. 状态非零时沿既有静态错误通道（raise/report/exit）处理；不伪造可恢复 Fatal；
5. 输入/out 复用与释放按既有静态 ABI 规则；
6. 不改 `.xiaoc`/`.xar`/原生元数据版本；
7. 保存静态 IR、符号、O0/O3、错误/清理证据。

退出：静态/动态 float `**` 与 cmath 共用内核；无 `llvm.pow.f64` 第二实现。

### B4-3C：测试与证据

1. 15 项 cmath 动态/静态 O0/O3；
2. float `**` 与 cmath_pow 同输入位模式/错误；
3. 002/009、状态 1/2/3/6、挂起/错误通道；
4. 15 个 C 符号存在、无 cmath 第二内核；
5. 30 条 planned 导入路径仍拒绝；
6. B4-0/1/2、B3-4、B1/B2/B3 回归；
7. 保存源码、工具链、目标、PID、日志、前后摘要；
8. 记录 Windows 本机与未执行平台。

退出：LLVM 动态/静态证据完整；B4-4 未开始；language false。

### B4-3D：收口与 B4-4 交接

1. 复核 15 项 cmath/float pow 共核；
2. 复核无导入/归档/序列化/ABI/registry 改动；
3. 复核 30 条 planned 仍拒绝；
4. 形成 B4-4 入口：`std.cmath`/`stdlib.cmath` 双路径、导出/移除/归档/序列化。

退出：B4-3 内部收口；B4-4 未开始。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 动态 | 15 项、C 符号、状态/out、009/002 |
| 静态 | 15 项、uses_runtime、O0/O3、错误/清理 |
| 共享 pow | float `**`/cmath_pow 同输入位模式；无 `llvm.pow.f64` |
| sfloat | 保持既有 `llvm.pow.f32` 路径 |
| 边界 | 无导入/归档/序列化/ABI/registry/721 |
| 回归 | B4-0/1/2、B3-4、B1/B2/B3 |
| 导入 | 30 条 planned 仍拒绝 |

## 五、退出条件

1. 动态/静态 LLVM 均执行 15 项 cmath；
2. float `**` 与 cmath_pow 共用内核，静态无 `llvm.pow.f64` 第二实现；
3. C ABI 状态/out/错误通道正确；
4. sfloat `**`、30 条 planned 导入、registry/cmath 状态未变；
5. 无导入/归档/序列化/ABI/registry 改动；
6. B4-0/1/2、B3-4、B1/B2/B3 回归通过；旧 721 未跑；
7. B4-4 未开始；三平台/性能仍未声称。

## 六、风险与禁止

- 静态路径继续保留 `llvm.pow`，形成第二内核；
- 动态/静态各自实现 cmath，绕过 Runtime；
- 状态 6 被当成功，或 1/2/3 写挂起槽；
- 把 `Pure` 当可删除；
- 提前接导入/归档/序列化或改 ABI/registry；
- 用 B4-3 的 Windows 本机证据冒充三平台或 B2-C。

## 七、交接

B4-3 完成后进入 B4-4：双路径、导出/移除、归档、序列化、普通包隔离。B4-5 收口，B5 切 ABI 1.19/21。

新 Agent 先读：

1. [B4 语言全链路规划](20o-b2-b4-language-integration-plan.md)；
2. [B4-2A/B 执行规划](20o-b2-b4-2a-b-execution-plan.md)；
3. [B4-0/B4-1 记录](assets/20o-b2-b/entry-20261110-b4-0-1/README.md)；
4. [F-5 原语/C ABI](20o-b2-freeze-b1-closeout-plan.md)；
5. [F-9 共享内核](20o-b2-freeze-draft.md#七cmath行为误差与二进制转换f-2f-8f-9已确认)。

## 八、落点

```text
core/rust/crates/xiao-codegen-llvm/src/dynamic/       动态 cmath 调用
core/rust/crates/xiao-codegen-llvm/src/ir.rs          静态 cmath/float pow
core/rust/crates/xiao-runtime/src/                    15 个 Rust/C ABI 内核
core/rust/crates/xiao-codegen-llvm/tests/             动态/静态 O0/O3
docs/DevDocs/assets/20o-b2-b/entry-<date>-b4/         B4-3 证据
```

本页只做 B4-3；B4-4 起、B5～B7 与 B2-C 仍后置。
