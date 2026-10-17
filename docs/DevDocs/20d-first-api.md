# 20D-首批 API. `os` 四个入口、资源类型与三平台差分

> **背景**：[20D-冻结](20d-freeze-resource-contract.md) §2.5 已把首批 `os` 契约逐条冻死；
> [20D-资源承载](20d-resource-carrier.md) 已把 `RuntimeValue::ExternalResource` 与 ABI 标签 17 接好。
> 本批是 **20D 的第四步**（冻结 → 能力上下文 → 资源承载 → **首批 API**）。
>
> **它不是「加四个 intrinsic」那么简单**：这是 **资源第一次作为语言类型出现**，
> 也是 **第一次生成资源值**——两份前置（[阶段计划](20d-platform-resources-stage-plan.md) §4.2 的
> F-1 产物边界、ABI 1.14 门禁）都在本批到期。

状态：**规划完成（2026-10-17）**：六道裁定全部取甲；**未改任何代码**，实施由下一批 Agent 做。

## 一、为什么是这一批

1. **顺序不可倒**：[20D-规划](20d-platform-resources-stage-plan.md) §4.2 的裁定是
   冻结 → 能力上下文 → 资源承载 → 首批 API；前三批已完成并审核通过；
2. **冻结已经给全签名与错误身份**：本批只实现，**不重新论证**；
3. **两个「第一次」**：资源类型第一次进入 `xiao-types`；第一次有 intrinsic 生成资源值——
   因此 F-1（产物边界可执行拒绝）与 ABI 次版本门禁**必须在本批落地**，不能再后置；
4. **阶段出口**：20D 六条出口条件里，第 1、4、6 条要等本批才有证据，
   第 5 条的「产物半边」也在本批完成。

## 二、冻结已定死的（**输入，不许重新论证**）

| 项 | 冻结内容 | 出处 |
| --- | --- | --- |
| `os.open_file` | `(str path) -> file`，能力 `filesystem` | [冻结](20d-freeze-resource-contract.md) §2.5 |
| `os.open_directory` | `(str path) -> directory`，能力 `filesystem` | 同上 |
| `os.close` | `(file\|directory resource) -> none`，能力 `filesystem`；**已关闭别名仍成功**；其它资源类型**在类型层拒绝** | 同上 |
| `os.read_text` | `(file resource) -> str`，能力 `filesystem`；**读整个文件**，无偏移/容量/流式参数；非 UTF-8 一律失败 | 同上 |
| 失败身份 | 已关闭 → `X06-RUNTIME-025`；能力缺失 → `X06-RUNTIME-026`；目标不支持 → `X20-TARGET-001`；宿主路径/权限/IO 与非 UTF-8 → `X06-RUNTIME-012` 并保留原因参数 | 同上 |
| 平台 | **Windows、Linux、macOS 三者都支持** `file`/`directory` 的打开、关闭与 `read_text` | 同上 |
| ABI | `XiaoValueTag::ExternalResource = 17`（只追加），载荷 `XiaoHandle`，**不暴露宿主裸指针** | 同上 §2.4 |
| 产物边界 | 资源不进 `const`/常量池/`.xiaoc`/`.xar`；源级 `X02-TYPE-008`、产物级 `X07-FATAL-002`，**都要可执行** | 同上 §2.3 |
| 容器边界 | 数组/元组允许；集合元素/字典键拒绝；`type_name` 不落 `dynamic` | 同上 §2.3 |

**本批不做**：写文件、目录枚举、进程启动、时钟、网络；不做偏移/流式读；
不做符号链接或路径穿越的额外限制（首版按普通本机程序的进程权限运行，见方向稿 §5.1）；
`process` API 后置，但**仍计入 20D 阶段出口**。

## 三、实施前核出的现状

### 3.1 类型系统**没有**资源类型

- `Type` 枚举（`core/rust/crates/xiao-types/src/types.rs:36`）只有
  标量、`none`、变量、函数、数组、元组、字典、集合、表、`dynamic`——**没有资源**；
- 类型注解只认关键字：`parser.rs:401`、`parser/statements.rs:628` 都走
  `ScalarType::from_keyword`，而 `ScalarType::from_name`（`ast.rs:336`）只认八个标量名；
- ⇒ 冻结写的 `(str path) -> file` **今天连源码都写不出来**。这是本批最大的一块，
  不是「顺手加两个关键字」。

### 3.2 intrinsic 只认**裸名**

- 契约解析是 `by_name(Expression::Name)`（`xiao-types/src/checker/conversion.rs:38`），
  `intrinsics.json` 现有 21 个活动条目**全部是裸名**（`print`、`input`、`int`、`chr`…）；
- `ValueType`（`xiao-intrinsics/src/lib.rs:72`）只有 `None`/`Dynamic`/标量/`Str`/`Bool`/`Set`/`ErrorParameters`，
  **没有资源类别**；`IntrinsicKind`、`VmBinding`、`AbiBinding`（同文件 `:51`/`:178`/`:189`）也没有资源变体；
- `Capability` 枚举（同文件 `:97`）只有 `None`/`Stdout`/`Stdin`，但位常量
  `CAPABILITY_FILESYSTEM = 1 << 0`（`:104`）已经存在——**位有了，契约声明不了**。

### 3.3 Runtime 承载**没有宿主句柄**

- `ExternalResourceObject`（`xiao-runtime/src/value/mod.rs:193`）只有 `kind` 与 `closed`，
  **不持有任何文件或目录**；
- `ensure_open` 已经返回 `X06-RUNTIME-025`（`value/mod.rs:294` 附近），关闭语义可复用；
- `xiao-runtime` 目前没有资源类的宿主 IO 路径（`std::fs` 只在内存计量里出现）。

### 3.4 标准库与模块登记

- 官方模块是 `stdlib/*.xiao`，命名空间前缀 `stdlib.`（`xiao-standard-library/src/lib.rs:11/14`），
  用户写法是 `from stdlib.text import contains_char`；
- 新模块要进 `docs/module-registry.json`，并补 `docs/UseDocs/library/os.md`；
- **现有 intrinsic 名与模块函数名不能同名**：纯 Xiao 模块要调用裸名 intrinsic，
  若两边都叫 `open_file`，模块内会解析成递归调用。

### 3.5 三平台差分的执行点

- `maintenance-regression.yml` 的 push 作业只有 **Windows + Linux**；
- `platform-reproduction.yml` 是 **schedule + dispatch**，四平台但不随 push 跑；
- 阶段出口第 4 条要求三平台一致——**本批要在 push 上有 macOS 执行点**（裁定见 §六）。

### 3.6 两份前置在本批到期

- **产物边界**：承载批审核已确认本批之前**没有插入点**（`TacConstant` 是封闭标量枚举、
  `resource` 类型不存在、VM 不校验外部 `.xiaoc`）；类型一落地，源级拒绝就有地方可写；
- **ABI 门禁**：承载批沿用 1.13 的前提是「不生成资源值」；本批**第一次生成**，
  必须升次版本或加等价门禁（裁定见 §六）。

## 四、本批要做的事（顺序有依赖）

1. **资源类型进类型系统**（依赖：无）
   - 新增 `Type::Resource(ResourceKind)`；`ResourceKind` 至少区分 `File`/`Directory`，
     并提供**父类型 `resource`** 供 `os.close` 使用；
   - 可赋值关系：`file <: resource`、`directory <: resource`；
     `file` 与 `directory` 互不可赋值；`resource` **不得**隐式赋回 `file`/`directory`
     （首批没有种类收窄操作）；
   - **不要**把资源塞进 `ScalarType`：标量转换、算术矩阵、`scalar_type()` 的
     假设会全部被污染；
   - 关键字：按 §六 裁定，`file`/`directory`/`resource` 作为**保留关键字**，
     与 `str`/`int` 同路径进入 `KeywordKind`、词法、类型注解解析；
   - 同步点：`Type` 的相等/统一/打印、类型层不可哈希判定、`X02-TYPE-008` 的常量路径、
     IR 与两条后端的类型表示、错误消息（不能落回 `dynamic`）；
2. **intrinsic 契约**（依赖 1）
   - 新增四条契约，ID 取下一个空位（当前活动条目到 21，100 保留）；
   - **裸名用内部前缀**（建议 `os_open_file`/`os_open_directory`/`os_close`/`os_read_text`），
     公开面是纯 Xiao 模块导出的 `open_file`/`open_directory`/`close`/`read_text`
     ——避免与模块函数同名导致递归；
   - `ValueType` 增加 `File`/`Directory`/`Resource`；`IntrinsicKind`、`VmBinding`、
     `AbiBinding` 各加对应变体；`Capability` 增加 `Filesystem`（复用现有位 0）；
     `Effects` 增加文件系统读写分类；
   - `intrinsics.json` 的 `parameter_type`/`return_type` 要能表达 `file`/`directory`/`resource`；
     `os_close` 的参数是**父类型**，`os_read_text` 的参数是 **`file`**；
3. **VM 路径**（依赖 1、2）
   - 在 `exec.rs` 现有 intrinsic 分派前的能力检查（`declaration.capability.bit()`）之后
     调用 Runtime；**先查位、再触碰宿主**，缺位报 `X06-RUNTIME-026`；
   - `close` 对已关闭别名返回成功；`read_text` 对已关闭返回 `X06-RUNTIME-025`；
   - `directory` 传给 `read_text` 在**类型层**就被拒，运行时路径不存在；
4. **原生路径**（依赖 1、2）
   - 新增 Runtime ABI 入口（建议四条，与 intrinsic 一一对应；或者一个带操作码的入口），
     入口参数包含能力掩码，**与 `print`/`input` 的带能力入口同形**；
   - LLVM 生成器只发射带能力掩码的符号；旧无掩码形态不新增；
   - ABI 次版本按 §六 升到 **1.14**；
5. **Runtime 宿主承载**（依赖 2）
   - `ExternalResourceObject` 增加宿主载荷：`file` 持有打开的文件；`directory`
     只保存**已验证的路径**，不要求持有宿主目录句柄——Windows 打开目录需要
     `FILE_FLAG_BACKUP_SEMANTICS`，而首批目录只用于关闭，没必要把平台差异带进来；
   - `read_text` 读整个文件并要求合法 UTF-8；**不做有损解码**；
     非 UTF-8 与宿主 IO 失败都映射到 `X06-RUNTIME-012` 并带原因参数；
   - `close` 与最后一个强引用释放**幂等**，宿主句柄只能释放一次（`Option::take` 一类写法）；
   - 相对路径按 §六 裁定以进程 cwd 为基准，字符串原样交给宿主；
     符号链接与路径穿越按宿主默认语义，首版不额外限制；
   - `X20-TARGET-001` 在首批三平台**不可达**，不得用它冒充宿主 IO 错误；
6. **纯 Xiao 模块**（依赖 2、3、4）
   - `stdlib/os.xiao` 导出 `open_file`/`open_directory`/`close`/`read_text`，只做薄包装；
   - 补 `docs/UseDocs/library/os.md`、`docs/module-registry.json` 条目；
   - 文档要写明：**公开面是模块函数**，内部 intrinsic 名不是契约；
7. **产物边界的可执行拒绝**（依赖 1、6）
   - 源级：`const` 初始化遇到资源/外部调用走 `X02-TYPE-008`（既有 `NON_CONSTANT_CODE`），
     要有**可执行用例**；
   - 产物级：在常量编码/解码边界加防御性拒绝，遇到资源值报 `X07-FATAL-002`；
     `TacConstant` **不许**加资源变体；源级写不出资源常量时，用测试入口直接构造，
     断言编码器拒绝；
8. **ABI 门禁**（依赖 4）
   - 次版本 13 → 14，更新编码版本断言；
   - 回归：要求 1.14 的产物在 1.13 Runtime 上**启动即被拒**（`required_minor <= actual` 不成立），
     1.14 正常通过；
9. **三平台差分**（依赖 3–6）
   - 在 `maintenance-regression.yml` 增加 macOS 作业；push 路径已含 `core/rust/**` 与 `stdlib/**`；
   - 夹具至少覆盖：正常打开/读/关、文件不存在、非 UTF-8、关闭后再读、
     目录传给 `read_text`（编译期拒）、缺 `filesystem` 位的负例（两条路径）、
     相对的 `/` 路径；
   - 三平台输出与错误码**逐字一致**，不许有平台专属文本；
10. **移除验证**（依赖 6）
    - 分别删掉 VM 绑定、ABI 包装、模块导出之一，专门用例必须失败。

## 五、不许

1. **不许**重新论证冻结的签名、错误身份与平台清单；
2. **不许**把 `directory` 传给 `read_text` 留到运行时才拒——冻结要求**类型层拒绝**；
3. **不许**用 `dynamic` 承载资源以求省事——种类区分是冻结的硬要求；
4. **不许**让资源进入 `const`/常量池/`.xiaoc`/`.xar` 而只靠文档声明；
5. **不许**在能力检查之前触碰宿主（打开、读、关闭都不行）；
6. **不许**顺手实现写文件、枚举、进程、时钟或网络；它们必须各自重新冻结签名；
7. **不许**用 `X20-TARGET-001` 冒充宿主 IO 失败，也不许用 `X06-RUNTIME-012` 冒充能力缺失；
8. **不许**跳过 1.14 升版与「旧 Runtime 拒绝」回归；
9. **不许**只跑 Windows + Linux 就声称完成三平台差分；
10. **不许**让内部 intrinsic 名与 `stdlib/os.xiao` 的导出同名——那会变成递归调用。

## 六、六道裁决（已落定，均甲）

| # | 问题 | 裁定 | 落点 |
| --- | --- | --- | --- |
| 1 | 资源怎么进类型系统 | **甲：两个名义类型 + 父类型**：`file`/`directory` 独立，另设 `resource` 父类型供 `os.close` 使用 | §3.1、§4.1 |
| 2 | 用户怎么写四个 API | **甲：裸名 intrinsic + 纯 Xiao `stdlib/os.xiao`**，用户 `from stdlib.os import open_file` | §3.2、§4.2/4.6 |
| 3 | 相对路径基准 | **甲：进程 cwd 为基准，字符串原样交给宿主**；分隔符由宿主解释 | §4.5 |
| 4 | 三平台差分在哪取证 | **甲：`maintenance-regression.yml` 增加 macOS 作业**，push 三平台并跑 | §3.5、§4.9 |
| 5 | ABI 门禁 | **甲：升到 1.14**，旧 1.13 Runtime 在启动兼容检查处被拒 | §3.6、§4.8 |
| 6 | `file`/`directory` 的语法形态 | **甲：保留关键字**，与 `str`/`int` 同路径 | §3.1、§4.1 |

**来源**：星崽直接裁定（AskUserQuestion，2026-10-17）；六条均取甲。
按 [20C](20c-official-xiao-library.md) §2.4 **G-2**，与审核倾向一致也以**直接裁定**记。

**裁定二（裸名 + 模块）的连带，写在纸面上**：冻结文档里的 `os.open_file` 是
**模块限定后的文档名**，真实源码写法是 `from stdlib.os import open_file` 后调用 `open_file`。
内部 intrinsic 名（`os_open_file` 等）**不是**公开契约，不得写进用户文档。

**裁定六（保留关键字）的代价**：外部程序若把 `file`/`directory` 用作变量名会失效，
需要用语言已有的反引号转义；仓内 `.xiao` 源码目前没有这种用法，破坏面为零。

## 七、落点

```text
docs/DevDocs/20d-first-api.md                              本文
core/rust/crates/xiao-syntax/src/token.rs                  KeywordKind 加 file/directory/resource
core/rust/crates/xiao-syntax/src/ast.rs                    类型拼写与逆映射
core/rust/crates/xiao-syntax/src/parser.rs                 类型注解解析
core/rust/crates/xiao-types/src/types.rs                   Type::Resource / ResourceKind / 可赋值
core/rust/crates/xiao-types/src/checker/**                 类型检查、常量路径、错误消息
core/rust/crates/xiao-intrinsics/intrinsics.json           四条契约
core/rust/crates/xiao-intrinsics/src/lib.rs                ValueType/Kind/Binding/Capability/Effects
core/rust/crates/xiao-vm/src/semantics/exec.rs             VM 分派与能力检查
core/rust/crates/xiao-codegen-llvm/src/**                  原生发射与能力掩码
core/rust/crates/xiao-runtime/src/value/mod.rs             宿主句柄与关闭语义
core/rust/crates/xiao-runtime/src/abi*.rs                  四条带能力 ABI 入口
core/rust/crates/xiao-runtime-abi/src/lib.rs               ABI 次版本 1.14
core/rust/crates/xiao-bytecode/** 或产物入口               资源常量的防御性拒绝
stdlib/os.xiao                                            纯 Xiao 模块
docs/UseDocs/library/os.md                                用户文档
docs/module-registry.json                                 模块登记
.github/workflows/maintenance-regression.yml              macOS 作业
docs/DevDocs/README.md                                    阶段索引加本文一行
```

具体文件名以实施时的实际落点为准；**上面每一项都要在实施记录里指到真实文件与行号**。

## 八、验收

### 8.1 对 20D 阶段出口的映射

| 出口条件 | 本批要交的证据 |
| --- | --- |
| 1 交付物 | 四个 API 两条路径可用；纯 Xiao 模块；`process` 仍登记为后置但计入出口 |
| 2 契约先于实现 | 已完成（冻结批），本批只需引用，不重开 |
| 3 能力路径存在 | `filesystem` 缺位的**两条路径负例**，报 `X06-RUNTIME-026`，且不触碰宿主 |
| 4 三平台差分 | push 触发的 Windows/Linux/macOS 三作业，同一夹具输出与错误码逐字一致 |
| 5 容器与产物边界 | 集合/字典键拒绝（承载批已交）；本批交**产物边界可执行拒绝**与 `type_name` 检查 |
| 6 移除验证 | 删 VM 绑定／ABI 包装／模块导出之一，专门用例必须失败 |

### 8.2 本批自己的验收

1. **类型层拒绝**：`read_text(directory)` 与 `close(非资源)` 是编译期错误，不是运行时错误；
2. **错误身份**：已关闭 → `X06-RUNTIME-025`；能力缺失 → `X06-RUNTIME-026`；
   宿主 IO/非 UTF-8 → `X06-RUNTIME-012` 且带原因参数；`X20-TARGET-001` 不被误用；
3. **关闭幂等**：两次 `close` 成功；最后一个强引用释放只关一次宿主句柄；
4. **读语义**：整文件、合法 UTF-8 才成功，**没有**有损解码；
5. **不暴露裸指针**：ABI 仍只用 `XiaoHandle`；
6. **ABI**：1.14 断言、旧 Runtime 拒绝回归；
7. **产物边界**：源级与产物级各一条**可执行**拒绝；
8. **三平台**：macOS 作业在 push 上真实跑到，不是手动派发的孤证；
9. **通用**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、
   `bunx tsc --noEmit` 全绿；提交标题带类型前缀、正文非空；
   凡改动 `README.md` 的 20D 行或本文，逐行跑一次粗体配对检查。

## 九、裁定结果（已执行）

见 §六 的六行表格；六条均取甲，来源为星崽直接裁定（AskUserQuestion，2026-10-17）。

## 相关页面

- [20D-冻结](20d-freeze-resource-contract.md) §2.5——**本批的直接前置**：签名、错误身份、平台清单
- [20D-资源承载](20d-resource-carrier.md)——**上一批**：`ExternalResource` 变体、ABI 标签 17、关闭语义
- [20D-能力上下文](20d-capability-context.md)——**上上批**：能力位与两条路径的拦截点
- [20D-规划](20d-platform-resources-stage-plan.md) §4.2——批次顺序与 F-1/ABI 两道前置
- [20. 内置函数与标准库](20-builtins-and-standard-library.md) §4.1/§5.1/§5.2——资源句柄契约与平台顺序
