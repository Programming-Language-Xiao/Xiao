# 22E. 标准库公开面、产物元数据与发布收口

> **这是 22 阶段的第五批：把 22D 已稳定的并发行为变成可公布的公共表面。**
> 22A 已冻结 17 个 `std.concurrent` 签名、32B/16B 布局、ABI 1.17、诊断编号和
> required-feature 位表；22B/22C/22D 已完成类型、VM、LLVM 和差分。本批只开放公共文档、
> 补齐产物元数据与发布矩阵，**不重新决定语言语义，不新增并发 API**。
>
> 状态：**已完成（2026-10-31）**。代码 `57b5d24` 的 maintenance `41209554809` 六个
> 作业与完整 workspace gates `41209554821` 全部成功；三平台真实元数据/strip/verify、
> 五个教程和 22D 12/16/1 差分通过，实际 job 与退出条件对账见 §9.4。D-1/D-3 均按
> 星崽确认的甲方案交付：保留虚拟模块，内嵌只读自描述元数据，复用既有错误身份。

## 一、入口条件与冻结输入

### 1.1 前置

1. [22A. 并发入口冻结](22a-concurrency-freeze.md)：17 个签名、ABI 1.17、32B/16B 布局、
   027–032、`.xar`/`.xiaoc` 位表和 UseDocs 入口；
2. [22A-FIX. 契约冲突收口](22a-fix-concurrency-freeze.md)：多 Runtime、shared 包装、
   关闭语义和扩展 `X17-XAR-011`；
3. [22C-FIX.1. 并发 VM 调用与端到端](22c-fix1-concurrency-vm-wiring.md)：26 条 VM 回归；
4. [22D. LLVM 并发执行、差分与跨平台](22d-llvm-concurrency-differential.md) §9.3：
   12/16/1 差分、C 布局探针、三平台证据和九条退出条件；
5. [22D-FIX.1. 并发差分矩阵收尾](22d-fix1-concurrency-differential-closure.md)：
   C-1～C-11 的验证层次与证据格式。

### 1.2 实施前基线（当前结果见 §9）

| 面 | 当前事实 | 对 22E 的影响 |
| --- | --- | --- |
| 模块身份 | `std.concurrent` 是编译器/VM 绑定的**虚拟原生模块**，没有 `stdlib/concurrent.xiao`；签名来自 `xiao-types` 的冻结表，VM 只在调用 span 上查 TAC 计划 | 不得新增会遮蔽虚拟模块的 `.xiao` 文件；22A 的“函数体”在首版指签名表 + Runtime 实现 |
| 公共执行面 | 17 个入口已能被 VM 和内部原生差分执行；公开原生构建仍结构化拒绝 | 22E 才决定并开启公开原生并发产物 |
| required-feature | `.xiaoc` 与 `.xar` 的并发位已在编码/解码路径实现，未知位和缺失位有稳定错误 | 22E 补最小位集、负例和发布说明，不重定位表 |
| 原生产物元数据 | **尚未实现**；22A 明确后置到 22E 冻结 | 本批必须定义段/符号、字段、校验入口和错误身份 |
| UseDocs | `docs/UseDocs/language/concurrency/README.md` 为 `planned` | 教程、示例、错误和兼容完成后才能改为 `verified` |
| 兼容矩阵 | `xiao-driver` 已有兼容轴/格子和 stdlib 身份块 | 22E 增加并发产物轴和发布报告 |

### 1.3 本批不得改变

| 项 | 冻结内容 |
| --- | --- |
| API | 22A 的 17 个签名、消费规则和 `std.concurrent` 唯一公开导入路径 |
| ABI | 32B 共享头、16B 包装、标签 18、ABI 1.17、普通 `StrongHandle`/`WeakHandle` 单线程规则 |
| 诊断 | 027–032、X03-TYPE-030–032、X06-LIFETIME-006–008 和 `X20-TARGET-001` |
| 位表 | `.xar` bit 0=`archive_requirements`；并发位从 bit 1 起；`.xiaoc` 独立 bit 0–5 表 |
| 语义 | Send/Sync、XCON v1、取消四位置、主/suppressed、detached 退出码规则 |

## 二、目标与不负责

### 2.1 目标

1. 把 17 个 `std.concurrent` 入口作为**已实现**的公共表面登记到 UseDocs、标准库索引和
   模块登记，提供可运行示例和错误说明；
2. 定义并实现**原生并发产物元数据**，使产物可被工具链自描述地校验 ABI、required-feature、
   目标和生成器版本；
3. 复核 `.xiaoc`/`.xar`/原生三类的 required-feature 发射、最小位集和拒绝路径；
4. 建立发布兼容矩阵和发布报告，覆盖旧/新 Runtime、三平台、32 位拒绝和未知位拒绝；
5. 保持 22C/22D 的 VM/native 行为和测试无退化；
6. 关闭或登记 22D 审核留下的 P3 欠账（见 §3.6）。

### 2.2 不负责

- 不新增并发 API、Actor/`async`/`await`、官方并发容器或跨 Runtime 堆岛生态；
- 不修改 32B 共享头、ABI 1.17、Send/Sync 表、XCON v1 格式或 027–032 编号；
- 不把 07-D 原型或 Rust 内部布局写成产品 ABI；
- 不做 JIT/LTO/PGO/性能目标；
- 不在没有三平台证据时宣称发布完成。

## 三、设计裁定点

### D-1（标准库交付形态）：虚拟模块，不新增遮蔽文件

**建议取甲**：`std.concurrent` 保持虚拟原生模块。签名表、类型检查、TAC 计划和 Runtime C ABI
就是首版实现；22E 只补公共文档、模块登记和兼容身份，不新增 `stdlib/concurrent.xiao`。
理由（当前代码已核对）：VM 的 `module_member` 对 `std.concurrent`/`stdlib.concurrent` 直接
返回虚拟函数句柄；归档闭包显式跳过着两类身份；增加同名 `.xiao` 不会成为执行体，只会让
“源码函数体”与签名表出现两套事实。

**乙**：新增 `stdlib/concurrent.xiao` 声明或薄包装；需要改加载器和类型检查的模块身份，
存在双绑定和遮蔽风险，不采用。

**已确认取甲**：22 阶段文档里“写 stdlib 函数体”的旧措辞解释为“公开签名 + Runtime 实现 +
文档”，同步 22 阶段文档和 stdlib README。

### D-2（公共文档）：一份权威页面，状态最后升级

- 以 `docs/UseDocs/language/concurrency/README.md` 为唯一权威页面，不另建重复的 library 页面；
- 内容补齐：17 个入口的所有权/消费规则、线程与任务池示例、shared/lock 作用域、
  取消与 027 不可 catch、detached 028、XCON 边界 029、关闭错误 030–032、跨 Runtime 031；
- 示例必须能通过 VM 和内部原生构建；文档引用真实运行号；
- 只有代码、示例、错误说明和兼容矩阵全部完成后，才把 `status: planned` 改为 `verified`；
- `stdlib/README.md` 和 `docs/module-registry.json` 增加虚拟模块的登记说明（代码路径指向
  签名表/Runtime，不指向不存在的 `.xiao`）。

### D-3（原生产物元数据）：内嵌自描述段

**建议取甲**：在原生可执行文件中内嵌固定格式的只读元数据段：

| 字段 | 内容 |
| --- | --- |
| magic | 固定 8 字节标识 |
| metadata version | 独立于 ABI 的元数据版本 |
| runtime ABI | major/minor（首版 1.17） |
| required features | 与 `.xar` 并发位同一编码，不含 `archive_requirements` |
| target | 规范化 triple + 指针宽度 |
| generator | `CODEGEN_VERSION` |
| stdlib identity | 语言版本、stdlib 接口版本、ABI 和 portable target |

平台载体：ELF `.note.xiao`、PE 自定义节、Mach-O `__XIAO,__meta`（名称在实施时按链接器
约束定稿）。元数据**不改变程序启动语义**，由工具链的校验入口读取；旧 Runtime 验收时按
ABI 和位集拒绝，未知位拒绝，不静默降级。

**乙**：旁边放 `.xiao.json` sidecar；容易与可执行文件分离或漂移，不采用。

**已确认取甲**：元数据内嵌，工具链读取校验且不执行用户代码；不使用 sidecar。

**错误身份**：优先复用语义相同的 `X17-XAR-007`（缺 requirements/版本不满足）、
`X17-XAR-011`（未知位）和 `X20-TARGET-001`（32 位）。若实施发现 native 需要独立错误族，
只提出 `X17-NATIVE-001..003` 候选，**必须经星崽确认后才冻结**，不在实现时临时编号。

### D-4（required-feature 最短位集）

- 从同一份 `IrProgram`/TAC 计划推导：线程→`threads`、shared→`shared_handles`、
  pool→`task_pool`、显式多 Runtime→`multi_runtime`、边界 XCON→`serialization_v1`、
  可取消等待→`cancel_wait`；
- `.xar` 始终带 bit 0；`.xiaoc` 用独立表；原生元数据用 `.xar` 并发位编码；
- 只发射实际使用的最小位集，不做“全置位”；无法静态确定时 fail-closed，不猜测；
- 缺失所需位、未知位和版本不满足分别走已冻结错误，测试需逐项覆盖。

### D-5（发布与兼容矩阵）

- 在 `xiao-driver` 兼容矩阵中增加并发产物格子：`.xiaoc`、`.xar`、原生 × 旧/新 Runtime，
  每格标明 `Direct`/`Reject{code}` 和 `Verified`/`Unverified`；
- 发布报告列出三平台产物、ABI 1.17、位集、元数据校验结果和实际运行号；
- 32 位目标在构建期以 `X20-TARGET-001` 拒绝，不进入发布矩阵；
- 旧产物不静默获得新并发语义；未来 ABI 次版本必须显式扩大范围。

### D-6（22D 审核欠账，先做）

1. **清理期取消语义**：native 的 `error_cleanup_is_active` 目前让 checkpoint 期间所有取消检查
   返回 Ok，而 VM 只在首次投递后抑制（`cancellation_handled`）。需要确认“首次取消在 finally
   内到达”的 VM 行为，补一个差分或明确登记为允许差异；
2. **帧存储启发式**：`finalize_frame_storage` 按文本判断 alloca 是否定长，元素类型不是 `iN`
   时把变量数量也当定长。当前产物只有字面量数量；收紧为“非字面量数量一律视为动态”，并加
   一个回归，防止未来动态聚合数组被挪到入口后违背 dominance；
3. **文档口径**：22D §9.3 的 `22D-CASE` 未包含 detached 场景（实际 28 条标记）；FIX.1 §二
   C-10 的夹具名与实现用例名不一致；FIX.1 §1.3 的 5/9/1 应标为“本批前基线”。

以上三项作为 E-0 前置，不改变冻结 API/ABI。

## 四、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 关闭或登记 E-0 三项；确认 D-1/D-3 的甲方案 | 定向测试 + 文档 |
| 1 | 标准库身份与文档：stdlib README、模块登记、UseDocs 教程/示例/错误 | docs/usedocs/layout；示例 VM 运行 |
| 2 | required-feature 复核：最小位集推导、缺失/未知位负例 | `.xiaoc`/`.xar` 产物夹具 |
| 3 | 原生元数据发射：格式、三平台段名、字段写入 | 三平台构建产物 |
| 4 | 元数据校验入口与负例：截断、坏 magic、版本、未知位、32 位 | 校验测试 + 三平台 |
| 5 | 兼容矩阵与发布报告：旧/新 Runtime 格子、三平台证据 | `compatibility` 测试 + 报告 |
| 6 | 打开公开原生并发构建的验收路径（仍不发布元数据之外的承诺） | 构建/运行黄金场景 |
| 7 | 全量回归：22C 26/26、22D 12/16/1、workspace/clippy/fmt/docs/coverage | 本地 + 三平台 CI |
| 8 | 文档收口：UseDocs 转 `verified`、22 阶段状态、12/README 索引 | docs/usedocs/layout |

## 五、测试与验收

### 5.1 公共表面

- 17 个入口逐个从 `std.concurrent` 导入并运行，VM 与内部原生构建都通过；
- `join`/`detach`/`wait` 的消费语义、`cancel` 不消费、guard 作用域释放；
- 普通句柄、动态值、资源、可变容器的跨线程编译期拒绝保持；
- 不存在的导入、错误参数数量和错误类型仍按冻结诊断拒绝。

### 5.2 产物位集

- 每个特性单独使用时只发射对应最小位；组合使用是并集；
- `.xar` 缺 bit 0 或缺失所需位 → `X17-XAR-007`；未知位 → `X17-XAR-011`；
- `.xiaoc` 版本/未知位 → `XIAOC-002/003/005/008`；
- 32 位目标 → `X20-TARGET-001`。

### 5.3 原生元数据

- 三平台产物都能被校验入口读出 ABI、位集、目标和生成器版本；
- 坏 magic、截断、版本不满足、未知位、目标不匹配分别拒绝；
- 元数据缺失的旧产物不被误判为满足并发要求；
- 校验入口不执行用户代码。

### 5.4 发布矩阵

- `.xiaoc`/`.xar`/原生 × 旧/新 Runtime 的每格都有实际编码器/解码器或校验路径证据；
- 发布报告列出平台、产物、位集、ABI 和运行号；
- 无单平台冻结，不用本地日志代替远端运行号。

### 5.5 回归

- 22C 26/26 和 22D 12/16/1 不退化；
- workspace、clippy、fmt、docs/usedocs/layout、coverage 和三个原生门禁全绿；
- 公开构建从“结构化拒绝”切到可验收路径时，拒绝用例同步更新，不留过期负例。

## 六、退出条件

1. 17 个入口的公共文档、示例和错误说明齐全，UseDocs 已转 `verified`；
2. `std.concurrent` 仍只有一种实现事实，没有同名 `.xiao` 遮蔽；
3. 三类产物的 required-feature 最小位集和拒绝路径全部有证据；
4. 原生元数据格式、发射和校验在三平台通过，错误身份已冻结；
5. 发布兼容矩阵和报告有实际运行号，32 位和未知位拒绝明确；
6. 22C/22D 回归与完整 gates 无退化；
7. 未修改冻结 API/ABI/Send-Sync/诊断编号（22E 新冻结的元数据错误除外）；
8. E-0 三项关闭或按层次登记。

## 七、风险与禁止

1. **遮蔽风险**：不得新增会让 `std.concurrent` 走到文件加载器的 `.xiao`；不得出现两套签名；
2. **元数据漂移**：段内字段必须由同一构建产生，不能手工维护 sidecar；
3. **错误编号**：未确认前不新增强行编号；复用已有错误时必须逐项核对语义；
4. **发布超前**：没有三平台元数据和矩阵证据前，不把公开原生并发产物写成已发布；
5. **位集过宽**：不得用全置位掩盖推导缺失；无法确定时 fail-closed；
6. **范围蔓延**：不新增 API、容器、Actor/async、性能或 JIT 目标。

## 八、落点

```text
docs/DevDocs/22e-standard-library-and-release.md           本计划
docs/DevDocs/22-concurrency-execution-and-shared-handles.md 22E 阶段说明
docs/DevDocs/22a-concurrency-freeze.md                    冻结 API/ABI/位表
docs/DevDocs/22d-llvm-concurrency-differential.md         22D 证据与 §9.3
docs/UseDocs/language/concurrency/README.md               公共文档（planned→verified）
stdlib/README.md                                          虚拟模块登记说明
docs/module-registry.json                                 模块身份登记
core/rust/crates/xiao-driver/src/compatibility.rs         兼容矩阵与报告
core/rust/crates/xiao-codegen-llvm/src/artifact.rs        原生元数据段发射
core/rust/crates/xiao-codegen-llvm/src/dynamic/release.rs E-0 帧存储修正
core/rust/crates/xiao-runtime/src/abi.rs                  E-0 清理期取消语义
core/rust/crates/xiao-bytecode/src/concurrency.rs         `.xiaoc` 位集
core/rust/crates/xiao-artifacts/src/lib.rs                `.xar` 位集
```

## 九、实施记录与验收规格

### 9.1 已确认的交付形态与 E-0

星崽已分别确认 D-1/D-3 采用甲方案。`std.concurrent` 保持虚拟模块，冻结签名表与
Runtime 只有一份事实，不创建 `stdlib/concurrent.xiao`。公共原生构建使用与内部差分
相同的元数据发射/验收路径，`with_internal_concurrency` 保留为调用方兼容接口，
不再成为绕过验收的开关。生成器版本升至 14，Runtime ABI 保持 1.17。

E-0 帧分配分类现在只移动隐式单元素或字面量数量的 alloca，聚合类型内部逗号不作为
数量分隔符，未知/SSA 数量留在原支配块；`only_literal_alloca_counts_can_move_to_entry`
覆盖聚合元素、非 i64 数量类型及动态数量。

清理期取消改为按执行单元记忆“自身取消已投递”，只抑制已投递令牌；trampoline 前后
与独立入口 reset 都重置该状态。`native_first_cancellation_inside_finally_is_delivered`
和既有清理 checkpoint 回归证明首次到达不会被无条件屏蔽。

实际 VM 核对另发现：`run_blocks` 按指令检查，finally 的 `run_subroutine` 不按指令
检查并发令牌，但其中调用普通 helper 会检查。新增真实 VM/native 的 E-0 握手例证明
finally 内 helper 首次收到取消后两侧均以 recoverable 027、退出码 3 结束；VM 的
路由多保留一层 suppressed 027，native 无此层。本批按 D-6 允许明确登记这一差异，
测试分别精确断言链形状，不把错误链写成相等；原有 22D 12/16/1 判据保持。

`22D-CASE` 现在包含 `detached-failure`，总共 29 条；FIX.1 C-10 的实际用例为
`entry_close_drain_failure_preserves_primary_and_suppressed`，旧 5/9/1 标注为本批前基线。

### 9.2 元数据格式 v1 与不执行校验

记录统一小端编码，固定头 60 字节，之后为三个 u32 字节长度前缀的 UTF-8 字符串。
总记录不超过 64 KiB，每个字符串 1～4096B；截断、尾部额外记录、重复节和非零保留位
均拒绝。对象节仅按目录名称定位，不在可执行文件全文搜索 magic。

| 偏移 | 字段 | 固定内容/校验 |
| --- | --- | --- |
| 0 | magic，8B | `XIAOMETA` |
| 8 | metadata version，u16 | 1 |
| 10 | flags，u16 | 0 |
| 12 | total length，u32 | 完整记录长度 |
| 16/24 | Runtime ABI min/max，u64 | 首版均为 1.17，未来次版本不隐式扩大 |
| 32 | required features，u64 | `.xar` 并发位，不含归档 bit 0 |
| 40 | generator，u32 | 当前 `CODEGEN_VERSION`，未知未来版本拒绝 |
| 44/46 | pointer width/reserved，u16 | 64 / 0 |
| 48 | stdlib interface，u32 | 权威接口版本 1 |
| 52 | stdlib ABI，u64 | 权威身份 ABI 1.17 |
| 60 起 | 三个字符串 | 规范 target triple、language version、stdlib portable target |

平台载体：ELF `.note.xiao` 包装为 owner `XIAO`、type 1 的对齐 note；PE `.xiaomta`
满足 8B 节名限制；Mach-O `__XIAO,__meta` 由链接器固定只读权限。`llvm.used` 加目标
链接器保留符号，strip 后仍须能读取。首版读取当前单架构 Mach-O，不将 fat 文件任意
切片冒充当前目标；需通用二进制时先显式选择目标切片。

`read_native_metadata` 只解析；`validate_native_metadata_file` 校验实际 Runtime ABI/位集、
目标/宽度、生成器和标准库身份。NativeBuild 链接后调用同一验收，失败删除刚产生的
无效文件；`xiao verify` 也使用此读取/校验路径，不运行产物。无元数据的旧普通原生文件
不因此失去运行能力，但不能通过并发元数据验收。

错误身份复用已确认方案：格式/缺失/版本/目标/标准库/缺位为 `X17-XAR-007`，未知位
或归档 bit 0 为 `X17-XAR-011`，32 位构建为 `X20-TARGET-001`，不新增编号。

### 9.3 最小位集与公共示例

`xiao-ir::required_concurrency_features` 使用 `xiao-runtime-abi` 的权威操作依赖表；
TAC/`.xiaoc` 将位集右移一位，`.xar` 与 native 使用同一并发编码。当前 Runtime 不要求
显式多 Runtime 位；空池只要求 task_pool；spawn/join/detach 要求 threads、XCON 和
可取消等待；共享值的实际存储编解码要求 shared_handles 与 serialization_v1；lock 再
加入 cancel_wait。位集取计划/类型的最小并集，未知操作拒绝，不发布全置位产物。

权威 UseDocs 的五个 `xiao` 代码块直接作为 VM/公开原生的测试输入，操作并集覆盖
全部 17 个冻结入口。公开测试还核对真实节、损坏 magic/版本/位、旧 Runtime、strip、
缺元数据旧产物、目标不匹配及 `verify` 发布报告。兼容矩阵新增三种载体 × 1.16/1.17
六个格子，真实编码器/校验器证明 Direct 或 Reject，公开构建没有内部绕行。

### 9.4 最终证据、发布矩阵与退出条件

代码提交：`57b5d24411196afdea6ca09ab6f47984854cff15`。maintenance
[`41209554809`](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41209554809)
与完整 workspace gates
[`41209554821`](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41209554821)
均为 `success`，head SHA 一致。后续公共文档状态/证据补录不计作重新执行代码。

| 平台/原生产物 | 节 | ABI / 元数据 / 生成器 | 代表产物位集 | 实际 job |
| --- | --- | --- | --- | --- |
| Windows `x86_64-pc-windows-msvc` / PE | `.xiaomta` | 1.17 / 1 / 14 | `0x62` | [124561486341](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41209554809/job/124561486341)，success |
| Linux `x86_64-unknown-linux-gnu` / ELF | `.note.xiao` | 1.17 / 1 / 14 | `0x62` | [124561486215](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41209554809/job/124561486215)，success |
| macOS `aarch64-apple-darwin` / Mach-O | `__XIAO,__meta` | 1.17 / 1 / 14 | `0x62` | [124561486216](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41209554809/job/124561486216)，success |

`0x62` 是代表性的 current Runtime + spawn/join 产物：threads、serialization_v1 和
cancel_wait 的最小并集。每个平台均精确执行 `public_native_concurrency_metadata_and_release_matrix`，
核对元数据读回与 strip 保留、旧 ABI/坏 magic/版本/未知位/目标/无元数据拒绝、
不执行入口的 verify 报告，以及全部五个教程的 VM/native 黄金输出。
日志包含五条 `22E-EXAMPLE`、一条登记差异的 `22E-E0` 和一条实际目标的 `22E-RELEASE`。

| 载体 × Runtime 校验输入 | 行为 | 真实证据 |
| --- | --- | --- |
| `.xiaoc` × 1.16 | Reject，`XIAOC-005` | 实际编码产物头部的 Runtime 要求预检 |
| `.xiaoc` × 1.17 + 所需位 | Direct | 正式编码/解码与要求预检 |
| `.xar` × 1.16 | Reject，`X17-XAR-007` | 实际归档 `run_archive` 的入口前预检 |
| `.xar` × 1.17 + 所需位 | Direct | 实际归档读取与 VM 执行 |
| native × 1.16 | Reject，`X17-XAR-007` | 三平台真实元数据要求校验 |
| native × 1.17 + 所需位 | Direct | 三平台公开构建、strip/verify 和程序运行 |

旧 Runtime 格子使用正式校验器注入 1.16 ABI/能力，不宣称下载或执行了旧 Runtime 二进制。
32 位构建拒绝有独立非 ignored 回归；未知位与缺位分别经过对应载体 codec/要求检查。
没有新增迁移行为，既有普通产物不被静默当成并发产物。

| 其他门禁 | 实际 job | 结果 |
| --- | --- | --- |
| Linux workspace 默认测试 | [124561486270](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41209554809/job/124561486270) | success |
| 解析器与产物安全 | [124561485966](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41209554809/job/124561485966) | success |
| Windows 产物尺寸基线 | [124561486283](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41209554809/job/124561486283) | success |
| 完整 workspace gates | [124561486060](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41209554821/job/124561486060) | success |

本地 workspace 168 个测试组退出 0，含 Runtime 176 项与 22C 26/26；Clippy、fmt、
docs/usedocs/layout、覆盖率、TypeScript、CLI 解析测试和两份锁文件检查通过。
原 22D 的 29 场景差分、汇编和 C 布局探针继续通过。覆盖率本机记录为 90.17%，
公共 API 100%，完整远端门禁再次确认仓库规则通过。

| §六退出条件 | 完成证据 |
| --- | --- |
| 1. 公共面与 UseDocs | 五个权威源码例并集覆盖 17 入口，三平台运行后页面升级 verified；错误/所有权/兼容说明齐全 |
| 2. 唯一虚拟模块 | stdlib README、registry 和不存在 `concurrent.xiao` 的定向回归；只沿用签名表/Runtime |
| 3. 最小位集 | 同源 IR 推导，`.xiaoc`/`.xar`/native 正反例与六格兼容矩阵 |
| 4. 原生元数据 | 三平台内嵌节发射、真实读取、strip 后读取、无执行验收与冻结错误身份 |
| 5. 发布报告 | 上列真实目标/位集/版本/job；verify 生成元数据与兼容矩阵报告；32 位和未知位拒绝 |
| 6. 基线无退化 | 22C 26/26、22D 12/16/1、workspace、三平台原生及完整 gates 全通过 |
| 7. 冻结输入不变 | 未改 API、ABI 1.17、32B/16B、Send/Sync 或诊断编号；生成器升级只隔离新 lowering 文本 |
| 8. E-0 | 首次/已投递取消与帧分配回归通过；VM 额外 suppressed 027 按 §9.1 精确登记，标记和夹具口径已更正 |

22E 及 22 阶段按上述范围完成。E-0 错误链差异、源码递归类型的既有 JSON 限制和
单架构 Mach-O 的读取范围保留在文档中，不扩大验证结论。
