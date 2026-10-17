# 20D-进程 API. `spawn` / `wait` 与子进程关闭语义

> **背景**：[20D-冻结](20d-freeze-resource-contract.md) §2.1 把 `process` 排除在首批之外，
> 理由写在纸面上：**它平台差异最大**（启动、退出码、子进程清理），
> 等 `file`/`directory` 把契约走通再单独立项。
> 前两种已在 [20D-首批 API](20d-first-api.md) 验收通过，本批补齐 20D 出口条件 1 的最后一项。
>
> **它不是「加两个 intrinsic」**：process 是第一个带**子进程生命周期**的资源——
> `wait` 会阻塞、`close` 与回收的关系、僵尸语义都必须先定死，否则三平台行为会分叉。

状态：**已实施（2026-10-18）**：五道裁定及补充类型裁定已落地；VM、ABI 与 Windows 受控原生专门夹具通过。完整门禁与三平台 push 取证见 §十一（实施中持续回填）。

## 一、为什么是这一批

1. **出口条件**：[冻结](20d-freeze-resource-contract.md) §四 第 1 条明确
   「进程 API 后置到 20D 的后续批次，**但仍计入 20D 的阶段出口**」——不交付就不能关闭 20D；
2. **排除理由必须逐项消解**：启动方式、退出码口径、子进程清理，本批各有一条裁定；
3. **复用已经走通的承载**：统一 `ExternalResource`、类型层种类、能力位、ABI 标签 17
   都已在 `file`/`directory` 上验证；process 不新增 `RuntimeValue` 变体。

## 二、冻结已定死的（**输入，不许重新论证**）

| 项 | 冻结内容 | 出处 |
| --- | --- | --- |
| 承载 | 统一 `ExternalResource` + 稳定种类标签；**语言类型区分种类** | 方向稿 §4.1 |
| 能力 | **bit 2 = `process`**；缺位报 `X06-RUNTIME-026` | [冻结](20d-freeze-resource-contract.md) §2.7 |
| 关闭 | 幂等；关闭后任何资源操作报 `X06-RUNTIME-025` | 同上 §2.2 |
| 容器/产物 | 集合元素/字典键拒绝；`const`/常量池/`.xiaoc`/`.xar` 拒绝 | 同上 §2.3 |
| ABI | `XiaoValueTag::ExternalResource = 17`；**不暴露宿主裸指针** | 同上 §2.4 |
| 平台 | Windows、Linux、macOS | 同上 §2.5 |
| 本批边界 | `process` 单独立项；**首批 process API 由本批冻结** | 同上 §2.1 |

## 三、本批冻结的契约（五道裁定之后）

| API | 签名 | 能力 | 失败身份 |
| --- | --- | --- | --- |
| `os.spawn` | `(str program) -> process` | `process` | 能力缺失 `X06-RUNTIME-026`；宿主启动失败 `X06-RUNTIME-012` 并带 `reason`；目标不支持 `X20-TARGET-001` |
| `os.wait` | `(process value) -> int` | `process` | 已关闭 → `X06-RUNTIME-025`；能力缺失 → `026`；等待失败 → `012` 并带 `reason` |
| `os.close` | `(file\|directory resource) -> none` | `filesystem` | 保持冻结不变；**process 参数在类型层拒绝** |
| **`os.close_process`** | **`(process value) -> none`** | **`process`** | 已关闭幂等成功；能力缺失 → `026` |

**为什么 process 单独一个关闭入口**：intrinsic 契约每个入口只有一个**静态**能力位，
没法按运行时种类切换；冻结的 `os.close` 只覆盖 file/directory，
所以 process 用独立的 `os.close_process`（能力位 `process`）。
`close(process)` 必须**在类型层拒绝**，否则它只能走 `filesystem` 位，语义错误。

**语义写死：**

- **`spawn`**：不经过 shell，`program` 作为**一个路径/程序名**原样交给宿主；
  由宿主按 PATH 解析；**继承父进程的 stdin/stdout/stderr、环境变量和 cwd**；
  创建成功后立即返回，不等待子进程；
- **`wait`**：**阻塞**到子进程结束，返回退出码；
  Unix 被信号终止返回 `128 + signal`；Windows 异常码按既有
  139/132/136/134 的故障类别映射（复用 `crash.rs` 的口径）；
  子进程结束前重复 `wait` 返回同一结果；**关闭后再 `wait` 报 `X06-RUNTIME-025`**；
- **`close_process(process)`**：**只释放句柄**，不终止、不阻塞、不等待；
  子进程继续运行。**未 `wait` 的子进程在其退出后会成为僵尸，直到父进程结束**
  ——这一句必须写进用户文档，不能用「close 会清理」掩盖；
- **`close(process)` 是类型错误**：通用 `close` 只服务 file/directory；
  唯一的进程关闭入口是 `close_process`。
- **不做**：命令行参数、显式 env/cwd、stdout/stderr 捕获、管道、shell 解释。

## 四、实施前核出的现状

### 4.1 类型与关键字是显式枚举

- `ResourceKind`（`xiao-syntax/src/ast.rs`）目前是 `File`/`Directory`/`Resource`，
  `KeywordKind`（`token.rs`）也只列了这三个；`parser.rs` 的 `is_type_keyword_token`
  是**显式匹配**，不是通配；
- `IrType::Resource { kind: String }`；`restore_type` 与 `validate_type`
  （`xiao-ir/src/tables.rs`、`validate.rs`）也都显式匹配 `file`/`directory`/`resource`；
- Runtime 的 `ExternalResourceKind` 只有 `File`/`Directory`，
  `runtime_value_matches`（`xiao-runtime/src/tables/mod.rs`）按种类显式匹配。

⇒ 加 `Process` 要同步 **六个地方**：关键字、类型、IR 复原/校验、Runtime 种类、
表值匹配、`type_name`/`display_text`（后者走共享的 `resource<kind>` 规则）。

### 4.2 intrinsic 契约只支持单参数

- `Signature` 是 `{ arity, parameter, return_type }`，`Arity` 没有二元；
  `ValueType` 没有数组类别（`Set` 有，`Array` 没有）；
- 语言的 `DeclaredType`/`FunctionTypeAnnotation` 也**没有数组形式**。

⇒ 这正是裁定一取「只 `spawn(str)` + `wait(process)`」的原因：**带 args 的签名今天写不出来**，
强行做要么新增语言特性，要么把参数编码成字符串，两条都不该混进这一批。

### 4.3 Runtime 承载可以复用

- `ExternalResourcePayload` 已有 `Empty`/`File`/`Directory`；
  process 需要第四种：`Child` + **缓存的退出码**（保证重复 `wait` 一致）；
- `on_drop`/`close` 现在都是 `payload.take()`；process 的 `Child` 被 drop 即**分离**，
  正好对应「只释放句柄」的裁定；
- `xiao-runtime/src/resources.rs` 已经是 VM 与 ABI 共用的宿主操作层，process 继续走它。

### 4.4 ABI 与新符号

- 标签 17 已存在，process 只是内部 kind，**不新增值标签**；
- `spawn`/`wait`/`close_process` 需要新 ABI 符号，按 10R 的规则**升次版本到 1.15**；
- 旧 Runtime 拒绝新产物的回归沿用 `is_abi_compatible` 的参数化用例。

### 4.5 三平台差分需要确定性的子进程

- `spawn` 不带参数，不能用 `cmd /c exit 7` / `sh -c 'exit 7'` 这类平台命令；
- 计划在 `xiao-driver` 包内加**测试支持二进制**：
  `process_child_exit7`（正常退出 7）与 `process_child_abort`（Unix 上 raise SIGTERM，
  用于信号退出码），集成测试用 `CARGO_BIN_EXE_*` 取路径并嵌进 Xiao 源码；
- 这样三平台跑的是**同一个子程序**，差分只比较 Xiao 侧的输出与错误码。

## 五、本批要做的事（顺序有依赖）

1. **类型与种类**（依赖：无）
   - `ResourceKind`/`KeywordKind`/解析器加 `process`；
   - `Type::Resource(Process)`、IR 的 `"process"`、`restore_type`/`validate_type`、
     Runtime `ExternalResourceKind::Process`、`runtime_value_matches`、`type_name`/`display_text`；
   - 集合/字典键与 `const` 拒绝沿用 `Type::Resource` 的既有规则，补 process 用例；
2. **intrinsic 契约**（依赖 1）
   - 新增 `os_spawn`（`str -> process`）、`os_wait`（`process -> int`）与
     **`os_close_process`（`process -> none`）**，ID 26/27/28；
   - **`os_close` 的类型检查要显式拒绝 `Process`**（它仍是 `filesystem` 位，
     不能关闭进程）；`os_close_process` 只接受 `Process`；
   - `ValueType` 加 `Process`；`Capability` 加 `Process`（复用 bit 2）；
     `Effects` 加 spawn/wait 分类；`IntrinsicKind`/`VmBinding`/`AbiBinding` 各加对应变体；
   - `build.rs` 断言：能力必须是 `process`、参数/返回类别正确、VM/ABI 绑定非空；
3. **Runtime 宿主实现**（依赖 2）
   - `resources.rs` 加 `spawn`/`wait`：先查 `process` 位，再触碰宿主；
   - `spawn` 用 `std::process::Command::new(program)`，不设 shell、不重定向 stdio；
   - `wait` 阻塞；Unix 用 `ExitStatusExt::signal()` 做 `128 + signal`；
     Windows 异常码复用 `crash.rs` 的类别映射；退出码缓存在 payload；
   - `close_process`/`on_drop` 只 `take()` 载荷（分离），不 kill、不 wait；
4. **两条路径**（依赖 2、3）
   - VM：`exec.rs` 分派 → `resources::spawn`/`wait`；能力检查仍在分派前；
   - 原生：新增带能力掩码的 ABI 入口，LLVM 只发射新符号；
   - ABI 次版本 14 → 15，补 `(1.15, 1.14) == false` 的旧 Runtime 拒绝用例；
5. **纯 Xiao 模块与文档**（依赖 2–4）
   - `stdlib/os.xiao` 加 `spawn`/`wait`/`close_process`（位置专用 `/`），
     内部名 `os_spawn`/`os_wait`/`os_close_process`；
   - `docs/UseDocs/library/os.md`、`stdlib/os.md` 写清继承、退出码、close 与僵尸语义；
   - `docs/module-registry.json` 补模块条目；
6. **回归**（依赖 3–5）
   - 正常：spawn helper → wait → 7；
   - 错误：不存在的程序 → `012`；缺 `process` 位 → `026`（两条路径）；
   - 生命周期：`close_process` 后 wait → `025`；重复 wait 返回同一结果；`close_process` 幂等；
   - 类型：`wait(file)`、`read_text(process)`、`spawn(1)` 都是编译期错误；
   - 产物：process 值不能进 `const`/表常量；
   - Unix 信号：abort helper → wait 返回 `128 + signal`（Linux/macOS 作业）；
7. **三平台差分**（依赖 5、6）
   - 在维护回归的三平台作业里跑 process 夹具；VM 夹具需要新增显式测试步骤
     （或把 `--test 20d_process_api` 加进现有 20D 步骤）；
   - 原生差分沿用 ignored 原生用例，比较 VM/LLVM 输出与错误码；
8. **20D 出口对账**（依赖全部）
   - 逐条回答六条出口条件；第 1 条随 process 关闭，其余五条引用既有证据；
   - 对账结果写进本文 §八 或单独的收口节，**不能只改索引行**。

## 六、不许

1. **不许**在本批加命令行参数、env/cwd 参数、stdout/stderr 捕获或 shell 解释——
   它们各自需要重新冻结签名；
2. **不许**把 `close(process)` 做成 kill 或阻塞等待；裁定一/三已经选「只释放句柄」；
3. **不许**让重复 `wait` 报错：退出码必须缓存；
4. **不许**用 `X20-TARGET-001` 冒充宿主启动失败，也不许用 `012` 冒充能力缺失；
5. **不许**在能力检查之前启动或等待子进程；
6. **不许**跳过 1.15 升版与「旧 Runtime 拒绝」回归；
7. **不许**只跑 Windows + Linux 就声称三平台差分；
8. **不许**让内部 intrinsic 名与 `stdlib/os.xiao` 的导出同名；
9. **不许**用「文档写了僵尸语义」代替用例：至少验证 `close_process` 不阻塞、
   `close_process` 后 wait 报 `025`、重复 wait 返回缓存；
10. **不许**让通用 `close`/`os_close` 接受 process——唯一的进程关闭入口是 `close_process`。

## 七、五道裁决（已落定，均甲）

| # | 问题 | 裁定 |
| --- | --- | --- |
| 1 | 首批 process API 范围 | **甲：只 `spawn(str)` + `wait(process)`**，复用资源承载；参数/环境/cwd 后置 |
| 2 | `wait` 退出码 | **甲：OS 码 + 信号归一**；Unix `128 + signal`，Windows 异常按 139/132/136/134 类别 |
| 3 | `close(process)` | **甲：只释放句柄**，不终止、不阻塞；`wait` 负责回收 |
| 4 | stdio/环境/cwd | **甲：全部继承**；`program` 由宿主按 PATH 解析 |
| 5 | process 的关闭入口与能力位 | **甲：单独的 `os.close_process`（`process` 位）**；通用 `close` 在类型层拒绝 process |

**来源**：星崽直接裁定（AskUserQuestion，2026-10-18）；五条均取甲。
第五条是在写文档时核出「一个 intrinsic 只有一个静态能力位」之后补发的裁决。

**补充类型裁定（星崽，2026-10-18）**：`resource` 继续只接受 `file`、`directory`；
`process` 是独立名义类型，不可隐式赋给 `resource`。这是实施时针对父类型边界的直接选择，
保留 `close(resource value)` 现有签名，同时阻断经父类型变量绕过进程关闭限制。
按 [20C](20c-official-xiao-library.md) §2.4 **G-2**，以直接裁定记。

**裁定一的连带，写在纸面上**：冻结文档里的 `os.spawn` 是**模块限定后的文档名**，
真实源码写法是 `from stdlib.os import spawn` 后调用 `spawn(...)`；
内部 intrinsic 名（`os_spawn`/`os_wait`）不是公开契约。

## 八、验收

### 8.1 本批验收

1. **契约**：四条签名（`spawn`/`wait`/`close_process`，以及保持不变的 `close`）、
   能力位、错误身份与本文 §三 一致；
2. **退出码**：正常退出码原样返回；Unix 信号为 `128 + signal`；重复 `wait` 一致；
3. **生命周期**：`close_process` 幂等且不阻塞；`close_process` 后 `wait` 报 `025`；
4. **能力**：缺 `process` 位时两条路径都报 `026`，且不启动子进程；
5. **类型**：`wait(file)`、`read_text(process)`、`spawn(1)`、**`close(process)`**、
   **`close_process(file)`** 都是编译期错误；process 不可哈希、不能进 `const`；
6. **ABI**：1.15 断言、旧 Runtime 拒绝回归；
7. **三平台**：Windows/Linux/macOS 同一子程序、同一输出与错误码；
8. **通用**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、
   `bunx tsc --noEmit` 全绿；提交标题带类型前缀、正文非空。

### 8.2 20D 出口条件对账（本批后必须逐条回答）

| # | 出口条件 | 本批后的预期 |
| --- | --- | --- |
| 1 | 交付物：文件/目录/进程首批 API + 能力上下文 + 三平台差分 | **满足**（process 由本批补齐） |
| 2 | 契约先于实现：A–G 全部冻结 | 满足（冻结批） |
| 3 | 能力路径存在且有负例 | 满足（能力上下文批；本批补 process 负例） |
| 4 | 三平台差分 | 满足（首批 API 批 + 本批 process） |
| 5 | 容器与产物边界可执行拒绝 | 满足（资源承载批 + 首批 API 批；本批补 process） |
| 6 | 移除验证 | 满足（首批 API 批 §10.3；本批补 process） |

## 九、落点

```text
docs/DevDocs/20d-process-api.md                       本文
core/rust/crates/xiao-syntax/src/{token.rs,ast.rs,parser.rs}    process 关键字与类型
core/rust/crates/xiao-types/src/{types.rs,conversion.rs,checker/**}  类型与检查
core/rust/crates/xiao-ir/src/{model.rs,lower.rs,tables.rs,validate.rs}  IR 种类
core/rust/crates/xiao-intrinsics/{intrinsics.json,src/lib.rs,build.rs}  三条进程契约
core/rust/crates/xiao-runtime/src/{value/mod.rs,resources.rs,abi_resource.rs,crash.rs}
core/rust/crates/xiao-runtime-abi/src/lib.rs           ABI 1.15 与新符号
core/rust/crates/xiao-vm/src/semantics/exec.rs         VM 分派
core/rust/crates/xiao-codegen-llvm/src/dynamic/{expression.rs,runtime_abi.rs}
core/rust/crates/xiao-driver/src/bin/process_child_exit7.rs     测试支持子程序
core/rust/crates/xiao-driver/src/bin/process_child_abort.rs     Unix 信号夹具
core/rust/crates/xiao-driver/tests/20d_process_api.rs           回归
stdlib/os.xiao / stdlib/os.md / docs/UseDocs/library/os.md      公开面
.github/workflows/maintenance-regression.yml           三平台显式步骤
docs/DevDocs/README.md                                 阶段索引
```

## 十、裁定结果（已执行）

见 §七 的五号表格；五条均取甲，来源为星崽直接裁定（AskUserQuestion，2026-10-18）。

## 十一、实施与验收记录（2026-10-18）

### 11.1 实际落点

| 交付 | 实际文件 | 已验证行为 |
| --- | --- | --- |
| 语言与类型 | `xiao-syntax/src/{token.rs,ast.rs,parser.rs}`、`xiao-types/src/{conversion.rs,checker/expression.rs}` | `process` 独立名义类型；`resource` 保留文件/目录边界；直接、别名、限定导入均拒绝错误种类和参数形状 |
| IR 与容器/产物 | `xiao-ir/src/{tables.rs,validate.rs}`、`xiao-runtime/src/tables/mod.rs`、`xiao-bytecode/src/lower/stmt.rs` | 进程表字段保持静态种类；集合/字典键不可哈希；源级 `const`/表常量报 `X02-TYPE-008`，产物边界报 `X07-FATAL-002` |
| intrinsic 契约 | `xiao-intrinsics/{intrinsics.json,src/lib.rs,build.rs}` | ID 26/27/28；静态 process 能力及 spawn/wait/release 效果；参数、返回、VM/ABI 绑定均验证 |
| 共用宿主实现 | `xiao-runtime/src/{resources.rs,value/mod.rs,crash.rs}` | 无 shell 启动、继承上下文、阻塞等待与缓存、Unix `128+signal`、Windows 故障类别；关闭与最终释放只分离 |
| VM / LLVM / ABI | `xiao-vm/src/semantics/exec.rs`、`xiao-codegen-llvm/src/dynamic/{expression.rs,runtime_abi.rs}`、`src/artifact.rs`、`xiao-runtime/src/abi_resource.rs` | 能力先于参数解码和宿主访问；三条带能力入口；新符号进入既有 value 组件映射；不新增值标签 |
| ABI 版本 | `xiao-runtime-abi/src/lib.rs` | 1.15；直接比较 required 1.15 / actual 1.14 拒绝，旧产物配新 Runtime 通过 |
| 公开面 | `stdlib/os.xiao`、`stdlib/os.md`、`docs/UseDocs/library/os.md`、`docs/module-registry.json` | 位置专用薄包装；单独 `close_process`；明确继承、退出码与 Unix 僵尸语义，条目保持 draft |
| 确定性夹具 | `xiao-driver/src/bin/{process_child_exit7.rs,process_child_abort.rs}`、`tests/20d_process_api.rs` | 三平台相同子程序；控制文件握手验证关闭不等待、不终止及最终引用释放；继承测试在隔离进程执行 |
| 三平台门禁 | `.github/workflows/maintenance-regression.yml` | 三平台显式运行文件/目录与进程 VM 夹具；两条 process 原生 ignored 用例由受控 workspace 作业执行 |

上表 crate 路径均以 `core/rust/crates/` 为前缀；`tests/20d_process_api.rs` 属于 `xiao-driver`。
不新增命令行参数、env/cwd 参数、捕获、管道或 shell 解释；没有新增 RuntimeValue 或 ABI 值标签。

### 11.2 定向验证

`20d_process_api` 默认 **9 passed / 3 ignored**；其中一条 ignored 是父用例驱动的隔离子进程，
其余两条是受控原生用例。Windows 本机已显式运行两条原生用例，**2 passed / 0 failed**。
`xiao-runtime --lib abi_resource` 为 **6 passed / 0 failed**，含 process 位先于参数解码、
ABI 标签往返、错误关闭入口不改变状态、关闭幂等与关闭后 wait 的错误身份。

原生差分记录为 cached（`resource<process>\n7\n7\n`）、closed（`025`）、missing（`012`）、
denied（`026`）、signal（Windows `139`、Unix `143`）。信号一项按冻结的平台口径分别断言，
其余四条逐字比较；两条路径都在能力缺位时拒绝，握手用例另证明未启动子进程。
本机单路径记录不代替三平台 CI；完整门禁及 push 证据在后续小节回填。

本批移除验证逐次修改并恢复源文件，均实际执行用例且退出码为 101：

| 临时移除点 | 必须失败的专门用例 | 实际失败 |
| --- | --- | --- |
| VM 的 `ProcessSpawn` 宿主操作 | `process_vm_wait_caches_exit_code_and_preserves_kind` | VM 成功断言失败 |
| `process_close` ABI 的共用关闭操作 | `process_abi_roundtrip_and_close_preserve_capability_and_alias_state` | 共享句柄未进入关闭状态，断言失败 |
| `stdlib/os.xiao` 的 `spawn` 导出 | `process_vm_wait_caches_exit_code_and_preserves_kind` | `X05-MODULE-005`，标准库缺少导出 |

三份源文件均按原始字节恢复，变异不提交；恢复后的完整工作区已重新通过。

本地完整验证：`cargo test --workspace --no-fail-fast -j 2` 为
**1197 passed / 0 failed / 39 ignored**；`tools/gates/run.sh` exit 0，包含 fmt、Clippy、
文档/覆盖率、锁文件、TypeScript、基准自检、驱动器测试和 Bun 测试。
Bun 为 **295 pass / 5 skip / 0 fail**；DevDocs 的逐行粗体配对检查通过。
ABI 升版同时更新既有兼容断言：当前 Runtime 接受 1.15、拒绝未来 1.16；
参数化旧 Runtime 门禁仍直接验证 `(required 1.15, actual 1.14) == false`。

### 11.3 20D 出口实际对账

| # | 实际状态与证据 |
| --- | --- |
| 1 | 实现已补齐文件/目录/进程 API 与能力上下文；三平台验收待本批 push 完成后关闭 |
| 2 | 满足：七项冻结见冻结批；进程五道裁定及补充类型关系见本文 §三/§七，均先于对应实现 |
| 3 | 本地通过：进程位负例和 ABI 参数前置校验；三平台由 process 专门夹具复跑 |
| 4 | 待本批 Windows/Linux/macOS 完整绿色 push，文件/目录既有证据仍见首批 API §13.1 |
| 5 | 满足：资源承载及首批 API 证据沿用；本批新增进程集合拒绝、表常量拒绝及手工越过前端的 IR 常量生产/编码拒绝 |
| 6 | 满足：本批 VM 绑定、ABI 关闭操作、模块导出三次移除均使专门用例失败，见 §11.2；既有三次验证仍见首批 API §10.3 |

## 相关页面

- [20D-冻结](20d-freeze-resource-contract.md) §2.1/§2.7——process 排除理由、能力位、关闭与产物边界
- [20D-首批 API](20d-first-api.md)——**上一批**：类型承载、ABI 1.14、三平台差分与移除验证的模板
- [20D-资源承载](20d-resource-carrier.md)——统一 `ExternalResource` 与 `is_hashable` 正向表
- [20D-规划](20d-platform-resources-stage-plan.md) §4.2——批次顺序与 20D 出口条件
- [17. `.xar` 字节码归档与启动](17-xar-archive.md)——20E 的前置；本批不做归档接线
