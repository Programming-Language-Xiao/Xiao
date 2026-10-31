# 22D-FIX.1. 并发差分矩阵收尾

> **这是 22D 的收尾补丁批，不是 22E。**
> 22D 的 LLVM lowering、Runtime ABI、三平台原生门禁和五项修补已落地；本批只补齐
> [22D](22d-llvm-concurrency-differential.md) §五黄金矩阵的缺口、补齐对应证据，并把
> 阶段退出条件从“大部分有证据”推进到“逐条可复核”。本批不改 API、ABI、Send/Sync 表、
> 32B/16B 布局、诊断编号或原生产物边界，不写 `stdlib/concurrent.xiao` 函数体。
>
> 状态：**实施中（2026-10-31，本地矩阵验证与门禁收尾）**。基线代码提交 `f4d31b0`；maintenance
> [`41176081348`](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41176081348)
> 与 workspace gates
> [`41176081391`](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41176081391)
> 均通过；证据补录提交 `0ec3ef4`。本地 2026-10-31 的 `cargo test --workspace --no-fail-fast`
> 为 `EXIT=0`（167 个测试组）。本批完成前 22D 保持**实施中**，22E 不开工。

## 一、入口条件与冻结输入

### 1.1 前置

1. [22A. 并发入口冻结](22a-concurrency-freeze.md)：17 个 `std.concurrent` 签名、
   32B 共享头、ABI 1.17、Send/Sync 表和 027–032；
2. [22A-FIX. 契约冲突收口](22a-fix-concurrency-freeze.md)：Runtime/pool 别名、
   shared 句柄、关闭语义和 `.xar` feature 位；
3. [22B. 并发类型、生命周期与 IR](22b-concurrency-types-lifetime-ir.md)：候选 A、
   `TaskBoundary`/`ThreadBoundary`、facts/transfers/cancellation；
4. [22C-FIX.1. 并发 VM 调用与端到端](22c-fix1-concurrency-vm-wiring.md)：26 条 VM
   回归和已关闭的 VM 行为；
5. [22D. LLVM 并发执行、差分与跨平台](22d-llvm-concurrency-differential.md)：
   D-1～D-9、当前差分矩阵和 §9.2 证据。

### 1.2 本批不得改变

| 项 | 冻结内容 |
| --- | --- |
| API | `std.concurrent` 的 17 个签名与消费规则 |
| 类型/能力 | 候选 A、`Runtime`/`Thread`/`Task`/`Pool`/`Shared`/`SharedMut`/`LockGuard` |
| ABI | 32B 共享头、16B 共享包装、`XiaoValueTag::Concurrency = 18`、ABI 1.17 |
| 诊断 | 027–032 编号、类别和“027 不进入 Xiao catch” |
| 产物 | 22D 只生成内部差分产物；原生产物元数据、required-feature 和 stdlib 入口仍属 22E |
| 差分口径 | [22D](22d-llvm-concurrency-differential.md) §5.2：同一份 IR、同一组判据，不用“结果碰巧相同”代替事件比较 |

实现约束：不得为夹具在生产路径加入测试专用分支；需要注入失败时只能使用
`#[cfg(test)]` 代码、已有内部选项或 Runtime 层单测，并在文档写明层次。

### 1.3 当前差分矩阵

现有 `22d_llvm_concurrency` 差分覆盖：

- 成功场景 5 个：`spawn-join`、`pool-submit-wait`、`shared-lock`、
  `shared-read-boundary`、`finally-return-overrides-child-error`；
- 运行时错误场景 9 个：三个 031、两个 finally 主/次错误、两个 027、
  `child-primary-and-finally-suppressed`、`native-stack-frames`；
- detached 场景 1 个：嵌套 detach 失败，退出码不变。

以上场景三平台远端通过；本批保留既有场景并扩展矩阵。新增夹具暴露的 LLVM 释放及循环
栈空间缺陷随本批修复，生成器版本升至 13；实现与证据登记到 22D §9.3，冻结输入不变。

## 二、缺口矩阵与夹具

| 编号 | §5.1 缺口 | 现有证据 | 本批夹具 | 主判据 |
| --- | --- | --- | --- | --- |
| C-1 | `spawn/join` 的 `str`、tuple、多参数 | 仅 int 单参数 | `spawn-join-str-multiargument`、`spawn-join-tuple` | stdout、退出码、释放投影 |
| C-2 | `detach` 成功路径 | 仅失败路径 | `detach-success` | stdout 集合、退出码 0、释放投影 |
| C-3 | pool 背压/容量 032 | 仅 VM 层 `pool_create(rt, 0)` | `pool-capacity-zero-032` | 029 之外的 032 身份和退出码 |
| C-4 | pool 关闭后 030 | 仅 VM 层 | `pool-submit-after-shutdown-030` | 030 身份、退出码 |
| C-5 | 多 Runtime 并行和 Runtime ID | 仅跨 Runtime 031 负例 | `multi-runtime-parallel` | 两 Runtime 输出、退出码、释放投影 |
| C-6 | shared 弱升级、guard 作用域释放 | lock/read/write 成功场景 | `lock-guard-scope-release`（语言级）；弱升级归 Runtime 层 | guard 恰好一次释放；弱升级不伪造语言夹具 |
| C-7 | LLVM 端排队/等待两位置取消 | 运行中、完成态两个 027 | `native-queued-cancellation`、`native-waiting-cancellation` | 027、finally 恰好一次、释放存在性 |
| C-8 | `wait`/`lock_wait` 的 027、外部取消通道 | join 027、完成态 027 | `native-wait-027`、`native-lock-wait-027`；外部取消归 Runtime 层 | 027 身份、主/suppressed |
| C-9 | XCON 深度/大小负例的同 IR 差分 | Runtime 层 `malformed_xcon_*`、序列化单测 | `xcon-oversized-029`、`xcon-bounded-string-success`；64 层 tuple 归 Runtime trampoline 回归 | 029 在启动前拒绝；成功场景逐字往返；递归深度按层次登记 |
| C-10 | 真实入口 close drain 失败差分 | 仅 helper 级 `native_finish_close_error_is_suppressed_under_primary` | `entry-close-drain-failure` | 主错误保留、close 错误 suppressed、退出码 |
| C-11 | 32B/16B 布局的 22D 产物级检查 | 22A 四平台测量、`abi` crate 的 `size_of/align_of` 断言 | 三平台 C 探针或书面顶账说明 | 尺寸/对齐/偏移一致，或明确引用 22A 证据 |

### 2.1 成功矩阵（C-1、C-2、C-5、C-6）

- **C-1**：复用 [22C-FIX.1](22c-fix1-concurrency-vm-wiring.md) 已验证的
  `work(int, str) -> str` 形状，覆盖多参数和 `str` 往返；tuple 先确认当前语言表面能构造
  并返回递归安全 tuple，能表达才加入源码夹具，否则在 §9.3 记录“tuple 只有 IR/Runtime
  证据”，不得用不存在的语法写假用例。
- **C-2**：detached 线程只做可观察输出，主线程输出后正常退出；比较 stdout 时按 §5.2
  允许 detached 完成顺序的集合差异，但退出码、主错误和释放投影必须一致。
- **C-5**：`runtime_create` 两个 Runtime，各自 `spawn/join` 一个标量任务；核对两个输出、
  退出码和 Runtime 归属事件。Runtime ID 不要求出现在 stdout，由释放投影和 031 负例共同
  证明身份。
- **C-6**：guard 在函数作用域结束时释放，释放投影必须恰好一次。弱升级不在冻结的
  `std.concurrent` 表面内，只能作为 Runtime 层回归，不得为它新增语言 API。

### 2.2 错误矩阵（C-3、C-4、C-8）

- **C-3/C-4**：直接复用 [22C-FIX.1](22c-fix1-concurrency-vm-wiring.md) 的
  `pool_create(rt, 0)` 和 shutdown 后 submit 形状，确认 VM/native 的 032/030 身份、
  类别和退出码一致；native 侧必须在启动线程/任务前失败。
- **C-8**：`wait` 和 `lock_wait` 的 027 必须与 join 027 同权：进入 Xiao `catch` 不可见、
  主执行单元退出码稳定。外部宿主取消没有公开的 Xiao 入口，只能由 Runtime C ABI 单测
  覆盖；在 §9.3 明确标为 Runtime 层证据，不冒充 LLVM 差分。

### 2.3 取消两位置（C-7）

- **排队**：容量 1 的池先提交一个用 `shared_mut` 握手且持续运行的任务，再提交第二个并
  在它开始前 `cancel(task)`；`wait(task)` 必须返回 027，任务体不得执行。
- **等待**：线程 A 持有 `shared_mut` 的锁，线程 B 在 `lock` 上等待；取消 B 后 join，
  必须返回 027，B 的 finally 恰好执行一次。
- 两个场景都不得对轮询握手的临时 guard 序列作逐事件断言；按 §5.2 只核对错误身份、
  清理存在性和稳定对象的释放投影。

### 2.4 XCON（C-9）

- **超限**：`16 MiB + 1` 的参数在启动线程前以 029 拒绝，native 与 VM 的主错误一致；
  不得出现“父线程已启动、子线程未运行”的半启动状态。
- **深度/节点**：使用边界内但足够深的递归 tuple 或字符串做往返，核对结果值、退出码和
  释放投影。
- **恶意 tag 13/14**：源码无法构造，只能保留 Runtime 层
  `malformed_xcon_never_enters_native_trampoline` 证据，并在 §9.3 按层次登记。

### 2.5 真实入口 close drain 失败（C-10）

先确认是否存在语言可表达的确定性触发路径；若不存在，允许改由 Runtime 层单测直接驱动
`xiao_runtime_concurrency_finish` 的失败分支，但必须满足：

1. 主错误已存在时，close 错误只能进入 suppressed，不能替换主错误；
2. 返回状态、退出码和 suppressed 顺序与 VM 的关闭口径一致；
3. 生产代码不为测试新增分支，注入只存在于 `#[cfg(test)]`；
4. §9.3 明确标注这是 Runtime 层证据，不写成“真实入口差分”。

### 2.6 布局（C-11）

§5.4 要求 32B 头/C ABI 的三平台产物检查。先复核 `xiao-runtime-abi` 的编译期
`size_of/align_of` 断言与 22A 四平台测量能否满足退出条件；若按 §5.4 字面要求仍需
22D 产物级探针，则在三平台 native job 中用同一 C 探针检查 `XiaoSharedHeader`（32B、
对齐 8）和 `XiaoSharedHandle`（16B）的尺寸/对齐/关键偏移。两种路径都必须在 §9.3 写明
采用哪一种，不能含糊默认。

## 三、差分判据

新增场景沿用 [22D](22d-llvm-concurrency-differential.md) §5.2，不允许降级：

- stdout 统一换行后逐字比较；允许的集合差异必须在夹具中显式声明；
- 主错误身份、错误类别、退出码、主/suppressed 链逐项比较；
- release trace 按 §5.2 的投影规则比较稳定对象的事件顺序；
- 运行中取消的轮询临时 guard 不作逐事件序列断言，只核对存在性和稳定对象；
- native 错误报告的用户帧顺序必须与 VM 的内到外顺序一致；
- 新增夹具必须同时运行 VM 与 NativeBuild，不得只跑 native。

## 四、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 复核现有 5/9/1 场景和 §5.1 对应关系；确认 tuple、弱升级、外部取消、close drain 的语言可表达性 | 条目表与层次登记 |
| 1 | 补成功矩阵 C-1、C-2、C-5、C-6 | 新夹具 VM/native stdout、退出码、释放投影 |
| 2 | 补错误矩阵 C-3、C-4、C-8 | 032/030/027 身份与退出码 |
| 3 | 补取消两位置 C-7 | 027、finally 恰好一次、无任务体执行 |
| 4 | 补 XCON C-9 | 029 启动前拒绝、边界内往返 |
| 5 | 补 close drain C-10 | 主/suppressed、退出码 |
| 6 | 复核或新增布局探针 C-11 | 三平台尺寸/对齐/偏移 |
| 7 | 更新 CI 精确执行列表和断言 | 三平台 maintenance 全绿 |
| 8 | 文档收口：22D §9.3、阶段状态和索引 | docs/usedocs/layout 通过 |

新增 ignored 测试时，工作流必须像现在的 22D 两个用例一样显式 `--exact` 执行，并断言
恰好 1 条通过；不允许靠“测试文件里存在”冒充执行证据。

## 五、验收与证据格式

- 每个 C 条目至少有一个稳定用例名，并在 `22d_llvm_concurrency.rs` 或 Runtime 单测中
  可单独定位；
- 三平台证据使用实际运行号和 job 链接，格式沿用 22D §9.2 的两张表；
- 本地证据记录 `cargo test --workspace`、clippy、fmt、docs/usedocs/layout 和
  `git diff --check`；不把本地通过写成远端通过；
- 证据必须对应明确的代码提交；后续文档提交不计作重新执行代码的证据。

## 六、退出条件

1. C-1～C-11 全部关闭，或按 §2 记录的层次降级并说明理由；
2. 既有 5/9/1 场景无退化，22C 的 26 条 VM 回归保持通过；
3. Windows/Linux/macOS 原生作业和完整 workspace gates 全部通过；
4. 22D §六 九条退出条件逐条可复核；
5. 完成以上后，22D 可标记完成，22E 才进入实施。

## 七、风险与禁止

1. **不得削弱判据**：新增场景不能为了避开发散而删掉 stdout、错误身份或释放投影比较；
2. **不得伪造语言表面**：tuple、弱升级、外部取消不可表达时按层次登记，不写编译不过或
   根本不执行的夹具；
3. **不得污染生产路径**：close drain 注入只能位于测试代码；
4. **detached 与取消时序**：只比较声明的集合/最终顺序，但主错误、退出码和清理次数必须
   确定；
5. **范围蔓延**：本批不写 stdlib 函数体、不加原生产物元数据、不改 ABI/诊断/冻结 API；
6. **平台证据**：不以单平台成功替代其他平台，不用本地日志替代远端运行号。

## 八、落点

```text
docs/DevDocs/22d-fix1-concurrency-differential-closure.md  本计划
docs/DevDocs/22d-llvm-concurrency-differential.md         22D 计划与 §9 证据
docs/DevDocs/22-concurrency-execution-and-shared-handles.md 22D 阶段说明
core/rust/crates/xiao-driver/tests/22d_llvm_concurrency.rs 差分夹具
core/rust/crates/xiao-driver/tests/concurrency_closure/   收尾源码与 C 布局探针
core/rust/crates/xiao-runtime/src/abi_concurrency_tests.rs 并发 ABI 单测
core/rust/crates/xiao-runtime/src/abi_tests.rs             Runtime ABI 单测
core/rust/crates/xiao-runtime/src/scheduler_tests.rs      Runtime 四位置回归
.github/workflows/maintenance-regression.yml              三平台精确执行
```

## 九、实施记录

2026-10-31 已加入七个成功场景和七个错误场景，原有 5/9/1 场景保留：

- 成功：str/多参数、纯标量 tuple、detached 成功、多 Runtime、函数 guard 释放、
  十万次 worker 循环及 64 KiB XCON 字符串逐字往返。
- 错误：032、030、排队取消、锁等待取消、wait 027、lock_wait 027、
  `16 MiB + 1` 在启动目标前的 029。
- Runtime 层：真实 `xiao_runtime_concurrency_finish` 失败分支的主/suppressed 顺序；
  宿主令牌对三个 C ABI 安全点的取消；等待位屏障证明真实阻塞的 lock C ABI 可取消；
  64 层 tuple 的 native trampoline 输入/结果信封逐字节相同；恶意 tag、超深、
  超节点数和超大单值拒绝，入口调用数保持 0。
- C-11 使用三平台同源 C 编译探针，对照 Rust ABI 类型的尺寸、对齐和偏移，并生成
  host 目标对象；工作流精确执行汇编、C 布局和 VM/native 差分三个用例。

新增矩阵复现并修正既有降低缺陷：纯标量 tuple 虽被静态计划归为 stack，动态 ABI
仍分配盒子，原生退出曾漏释放；边界复制源归还早于普通绑定，造成释放次序差异。
另以十万次有限循环稳定复现了 134：循环内固定 ABI scratch `alloca` 不断累积，现统一
移至函数入口。stdout/stderr 改为同时读取，64 KiB 输出用于验证不会因管道填满挂起。

`native-wait-027` 同时验证 `finally` 和自定义表 `drop` 各打印一次。该场景复现了 native
析构回调重复消费取消、漏执行 drop 主体的问题；现以同一错误 checkpoint 包住 drop，
清理中的普通 helper 不重复投递取消，恢复后仍保留 027 主错误和清理错误的 suppressed
身份/顺序。`native_cleanup_checkpoint_defers_repeated_cancellation` 覆盖该 Runtime 规则。

源码中的 32 层 tuple 会遇到现有优化快照 JSON 递归限制；C-9 按 §2.4 选择字符串作为
LLVM 源码往返夹具，64 层深度在 Runtime trampoline 层验证，未计作 LLVM 源码深度证据。
语言锁等待场景验证锁不可用、已进入 try 和 finally 恰好一次；具体已阻塞位置另由
Runtime C ABI 单测用 `#[cfg(test)]` 等待位观察证明，不为产物增加观察入口。

本批最终源码提交、本地门禁和远端三平台运行完成后，统一补录到 22D §9.3；
补录完成前保持实施中。
