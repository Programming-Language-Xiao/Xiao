# 20D-首批 API. `os` 四个入口、资源类型与三平台差分

> **背景**：[20D-冻结](20d-freeze-resource-contract.md) §2.5 已把首批 `os` 契约逐条冻死；
> [20D-资源承载](20d-resource-carrier.md) 已把 `RuntimeValue::ExternalResource` 与 ABI 标签 17 接好。
> 本批是 **20D 的第四步**（冻结 → 能力上下文 → 资源承载 → **首批 API**）。
>
> **它不是「加四个 intrinsic」那么简单**：这是 **资源第一次作为语言类型出现**，
> 也是 **第一次生成资源值**——两份前置（[阶段计划](20d-platform-resources-stage-plan.md) §4.2 的
> F-1 产物边界、ABI 1.14 门禁）都在本批到期。

状态：**已完成审核返工与三平台验收（2026-10-17）**：§十一 的 F-1～F-6 与 O-1/O-2 均已关闭；
维护回归 `39041556630` 六个作业全部成功，20D 出口第 4 条已取证。修复与绿色 push 证据见 §十二。

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

## 十、实施记录与证据

本批实际落点如下，`file`、`directory`、`resource` 仍是独立关键字和名义类型，资源没有退回 `dynamic`：

| 交付 | 实际落点 | 证据 |
| --- | --- | --- |
| 语法与类型 | `core/rust/crates/xiao-syntax/src/{token.rs,ast.rs,parser.rs,parser/statements.rs}`、`core/rust/crates/xiao-types/src/{types.rs,conversion.rs,checker/expression.rs,checker/statement.rs,function_checker.rs}` | `file`/`directory`/`resource` 注解可解析；`file`/`directory` 只可赋给 `resource`；`directory` 传给 `read_text` 和非资源传给 `close` 在类型/函数检查阶段拒绝 |
| intrinsic 契约 | `core/rust/crates/xiao-intrinsics/intrinsics.json`、`src/lib.rs`、`build.rs` | ID 22–25 使用 `os_` 内部名；参数/返回资源类别、filesystem 能力和文件系统效果进入生成表；`cargo test -p xiao-intrinsics` 通过 |
| Runtime/ABI | `core/rust/crates/xiao-runtime/src/{value/mod.rs,abi_resource.rs}`、`core/rust/crates/xiao-runtime-abi/src/lib.rs` | 文件打开、目录验证、整文件 UTF-8 读取、幂等关闭、共享别名和 `reason` 参数；四个入口只接受 `XiaoValue`/`XiaoHandle`，ABI 次版本为 1.14 |
| 两条执行路径 | `core/rust/crates/xiao-vm/src/semantics/exec.rs`、`core/rust/crates/xiao-codegen-llvm/src/dynamic/{expression.rs,runtime_abi.rs}`、`core/rust/crates/xiao-runtime/src/resources.rs` | VM 调用共用 Runtime 操作，LLVM 经带能力掩码的新 ABI 委托同一操作；两者均在宿主访问前检查 filesystem 位；旧无掩码资源符号没有新增 |
| 纯 Xiao 公开面 | `stdlib/os.xiao`、`stdlib/os.md`、`docs/UseDocs/library/os.md`、`docs/module-registry.json` | `from stdlib.os import open_file, open_directory, close, read_text` 走薄包装；公开函数名与内部 intrinsic 名分离 |
| 产物边界 | `core/rust/crates/xiao-types/src/checker/statement.rs`、`core/rust/crates/xiao-bytecode/src/{lower/stmt.rs,encode/validate.rs,encode/decoder.rs}`、`src/verify.rs`、`core/rust/crates/xiao-vm/src/run.rs` | 源级资源 `const` 报 `X02-TYPE-008`；降低、编码和解码边界对资源常量统一保留 `X07-FATAL-002`，生产验证映射为 `FatalError::corrupt_artifact`；`TacConstant` 没有资源变体 |
| 三平台门禁 | `.github/workflows/maintenance-regression.yml`、`core/rust/crates/xiao-driver/tests/20d_first_api.rs` | push 矩阵包含 Windows、Linux、macOS；VM 夹具在三作业显式运行，受控工具链下的 ignored 原生夹具同时比较 VM/LLVM 的 `open_file`→`read_text`→`close` 输出；夹具还覆盖关闭后读取、缺能力、非 UTF-8、目录→`read_text` 编译期拒绝和相对路径形态 |

返工后的定向验证：`cargo test -p xiao-runtime abi_resource`（4 项通过）、
`cargo test -p xiao-driver --test 20d_first_api`（9 项通过，2 项 ignored）、相关 workspace crate 的
`cargo check` 与既有语法/类型/字节码/VM 回归均通过。三平台 push 结果以维护回归工作流为最终取证，
本地单机运行不替代 macOS 作业证据。

### 10.1 ABI 旧 Runtime 门禁

`xiao-runtime-abi` 的 `ABI_MINOR_VERSION` 已从 13 升到 14；`xiao_runtime_abi_is_compatible`
继续允许旧次版本调用方读取兼容入口。比较规则已抽成
`core/rust/crates/xiao-runtime-abi/src/lib.rs:35` 的 `is_abi_compatible`，生产 ABI 调用同一函数；
`resource_api_requires_runtime_1_14` 直接断言 required 1.14 / actual 1.13 拒绝、
1.14 / 1.14 通过、旧产物配新 Runtime 通过、主版本不匹配拒绝。
这是参数化启动比较回归，没有声称另行安装并执行一份旧 1.13 Runtime 二进制。

### 10.2 自审核修正

审核时修正了三处容易把边界做松的实现风险：

1. `parse_declared_type` 改为显式识别资源关键字，不再把未知 token 伪装成 `int`；
2. 普通函数调用的资源父类型兼容只在 `Type::Resource` 分支使用 `can_assign`，没有放宽数字或其它类型统一；
3. `open_file` 和 `open_directory` 在能力检查后分别验证宿主目标种类，错误仍统一为 `X06-RUNTIME-012` 并保留 `reason`，没有引入平台专属错误码。

### 10.3 本批移除验证（返工实跑）

三次变异分别执行并逐次恢复源码；每次都真实执行一个专门用例，测试退出码均为 101。

| 临时移除点 | 必须失败的用例 | 实际失败 |
| --- | --- | --- |
| `exec.rs` 的 `VmBinding::ResourceOpenFile` 操作 | `20d_first_api::os_module_opens_reads_and_closes_file_in_vm` | VM 无法打开资源，成功断言失败 |
| `abi_resource.rs` 的文件打开 ABI 包装操作 | `abi::abi_resource::tests::open_read_close_and_closed_alias_are_stable` | 返回标签为 0，期望的资源标签为 17 |
| `stdlib/os.xiao` 的 `open_file` 导出 | `20d_first_api::os_module_opens_reads_and_closes_file_in_vm` | `X05-MODULE-005`，模块未导出 `open_file` |

恢复后专门夹具与 Runtime 测试重新通过。变异内容均未提交。

## 十一、审核附录（2026-10-17）

审核对象：`08a94c8`（实现）与 `a1a30b0`（测试加强）。方式是**自己跑**——读 diff、逐条核实现、
跑全量门禁与 workspace 测试、对关键路径做临时探针（跑完即删、`git status` 为空），
再核对两次 push 的维护回归与 workspace 门禁。

### 11.1 通过的项（证据）

| 项 | 证据 |
| --- | --- |
| 类型层 | `file`/`directory`/`resource` 关键字与名义类型；`directory`→`read_text`、非资源→`close` 编译期拒绝；`const file` → `X02-TYPE-008`；推断集合 `{f}` → `X03-TYPE-016` |
| 父类型 | `file`/`directory` → `resource` 可赋值；`resource` 不隐式收窄；`function_checker` 只对资源分支特判 |
| intrinsic 契约 | ID 22–25、`os_` 内部名、`filesystem` 能力与读写效果；`build.rs` 逐条断言 |
| Runtime | 整文件 UTF-8、非 UTF-8 → `012` 带 `reason`、关闭后 → `025`、关闭幂等、别名共享、最后强引用只释放一次 |
| 能力 | VM 分派前报 `026`；原生入口触碰宿主前报 `026`；ignored 原生负例断言退出码 3 且 stderr 含 `X06-RUNTIME-026` |
| 产物边界 | 源级 `X02-TYPE-008` 可执行用例；降低/验证/编码 `X07-FATAL-002`；解码拒绝常量标签 11/17；`TacConstant` 未加资源变体 |
| ABI | 1.14、标签 17、只用 `XiaoHandle` |
| 标准库 | 薄包装；内部名与导出名分离；`module-registry` 为 `draft` |
| 门禁 | `tools/gates/run.sh` exit 0；`cargo test --workspace` **1172 passed / 0 failed / 35 ignored**；`bun test` 295 pass / 5 skip / 0 fail |
| 原生差分 | `native_os_module_matches_vm_output ... ok` **三平台都有**：Windows 日志第 1176 行、Linux 第 953 行、macOS 第 983 行（运行 `39029110186` 与 `39031224866` 的作业日志） |

### 11.2 F-1（**阻断**）：macOS 作业红，20D 出口第 4 条未取证

两次 push 的维护回归都在 macOS 作业上失败：

- `08a94c8` → 运行 `39029110186`（macOS 作业 `117385821579`）
- `a1a30b0` → 运行 `39031224866`（macOS 作业 `117391904090`）

| 失败用例 | 现象 | 根因指向 |
| --- | --- | --- |
| `optional_external_illegal_instruction_remains_fatal`（`n0_a.rs:1037`） | 期望 `Some(132)`，macOS 实际 `None` | 进程被 SIGILL 终止时 `ExitStatus::code()` 为 `None`；退出码没有按 `128 + signal` 映射 |
| `real_terminal_session_is_environment_gated`（`diagnostics.rs:1159`） | 「诊断进程 10 秒内未连接」 | macOS runner 没有终端；Windows 作业**显式跳过**这两条，macOS 作业没加同样的 `--skip` |
| `native_table_methods_debug_execution_o0_o3`（`d19a_differential.rs:1145`） | 同类失败 | 同上：依赖终端/调试窗口，Windows 已跳过 |
| `diagnostic_session_forwards_events_and_closes`（仅第一次运行） | 握手读取 `Resource temporarily unavailable (os error 35)` | `abi_diagnostics.rs` 把监听 socket 设为非阻塞后，BSD/macOS 的 `accept` **继承** `O_NONBLOCK`；代码只设读超时，没把已接受的流改回阻塞 |

**必须说清楚**：20D 自己的三平台差分**没有失败**——`native_os_module_matches_vm_output` 在 macOS 作业日志第 983 行通过。
红的是新作业带出来的**既有可移植性缺口**。但「main 绿」是硬要求，阶段出口第 4 条要的是
**一次绿的维护回归**，不是「红运行里有一条绿子用例」。
⇒ **F-1 是本批的阻断项**：补 Windows 同款 `--skip`、按平台映射信号退出码、
在 `accept` 后显式 `set_nonblocking(false)`，然后重新取证。

### 11.3 F-2（中）：移除验证没有记录

阶段出口第 6 条要求「删掉 VM 绑定／ABI 包装／模块导出之一，专门用例必须失败」，§十 没有这条证据。
审核侧代做了一次：把 `VmBinding::ResourceOpenFile` 改成不可达后，
`os_module_opens_reads_and_closes_file_in_vm` **立即红**（`intrinsic ID 22 尚未绑定 VM 实现`）。
用例是有效的，但**批次自己的验收记录缺这一条**，应补进 §十。

### 11.4 F-3（中）：ABI 旧 Runtime 拒绝只测了一半

`abi_tests.rs:733` 断言的是**新 Runtime** 接受 required 1.14、拒绝 1.15；
「1.14 产物在 1.13 Runtime 上被拒」没有直接用例，只是由 `required_minor <= actual` 推出。
§10.1 的「实际 1.13 Runtime 上启动检查失败」说过头了：应改成「由比较规则推出」，
或把比较抽成参数化函数后补 `(1.14, 1.13) == false` 的用例。

### 11.5 F-4（低）：`set<file>` 半接线

`is_set_type_term_token` 已接受资源关键字，AST/降低/检查器也有 `TypeTerm::Resource`，
但 `parse_set_type_term`（`parser.rs:508`）只认标量和 `none`；实测 `set<file> values = set()`
得到 `X03-PARSE-005`。要么补全「先解析、再由类型层报不可哈希」，要么撤回死路径并让解析器成为唯一拒绝点。

### 11.6 F-5（低）：关键字实参类型检查放行

实测 `open_file(path = "x")` **编译通过**，VM 运行才报 `X06-RUNTIME-012`；原生 codegen 直接给 `Unsupported`。
检查器应在类型阶段拒绝关键字实参（VM/原生已经这么做）。

### 11.7 F-6（低）：相对路径没有常驻用例

§十 声称夹具覆盖「相对路径形态」，实际夹具全用绝对路径。审核探针验证了进程 cwd 语义**是对的**
（临时目录里 `open_file("relative.txt")` 输出 `relative-ok`），但这份证据没有入库。
要么补一条永久用例，要么把 §十 的措辞改成「由探针验证，未入库」。

### 11.8 观察

- **O-1**：VM 自己用 `std::fs` 再做一遍打开/校验，与 Runtime ABI 的实现重复；计划原本要求 VM 调 Runtime。
  两条路径目前语义一致，但存在漂移风险，建议后续合并到同一入口。
- **O-2**：`xiao-standard-library/src/README.md` 仍写「工程期 20C」，而它现在也覆盖 20D 的 `os`。
- **审核侧自账**：第一次 `tools/gates/run.sh` 的失败由审核侧临时探针文件触发 `cargo fmt --check`，
  探针已删除、重跑 exit 0；**不记为实现的问题**。

**审核结论**：实现主体通过；**F-1 未解决前，本批不能记为「验收通过」**，20D 出口第 4 条保持未满足。

## 十二、审核返工（2026-10-17）

§十一 保留针对旧提交的审核事实。当前修复与证据如下；F-1 已由 §12.1 的完整绿色 push 关闭。

| 项 | 修复落点 | 验证与状态 |
| --- | --- | --- |
| F-1：平台故障 | `core/rust/crates/xiao-runtime/src/crash.rs:56`；`tests/n0_c2_error_boundary.rs` | 按故障类别映射冻结退出码，兼容 BSD 的 SIGBUS 编号；新增 SIGTRAP 处理，真实子进程覆盖 SIGSEGV/SIGBUS/SIGTRAP；三平台 CI 均通过，macOS 的两条外部平台 Fatal 用例也通过 |
| F-1：握手 | `core/rust/crates/xiao-runtime/src/abi_diagnostics.rs:86`；`src/abi_tests.rs:740` | accept 后显式恢复阻塞模式，再设置超时；延迟 hello 的真实 socket 回归通过 |
| F-1：终端 | `.github/workflows/maintenance-regression.yml:247` | macOS 仅排除与 Windows 相同的两个终端依赖用例；Linux/Xvfb 保留完整终端验证，macOS 资源差分与平台 Fatal 用例继续执行；三平台作业全绿，已关闭 |
| F-2 | §10.3 | 本批实际完成 VM 绑定、ABI 包装、模块导出的三次变异验证，均如期失败并恢复 |
| F-3 | `core/rust/crates/xiao-runtime-abi/src/lib.rs:35`；`core/rust/crates/xiao-runtime/src/abi_version.rs:20` | 同一参数化比较函数明确覆盖 1.14 产物 / 1.13 Runtime 拒绝；版本入口独立为小模块，ABI 核心文件继续符合 2500 行门禁；已关闭 |
| F-4 | `core/rust/crates/xiao-syntax/src/parser.rs:508`；`core/rust/crates/xiao-types/src/set_checker.rs:140`；`src/table_checker.rs:348` | `set<file/directory/resource>` 先解析，再统一报 `X03-TYPE-016`；变量/表字段、空初始化/未初始化、混合并集均有常驻负例；已关闭 |
| F-5 | `stdlib/os.xiao:4`；`core/rust/crates/xiao-types/src/checker/expression.rs:589` | 公开函数使用已有 `/` 位置专用参数语法，内部 intrinsic 检查实参种类；关键字及展开实参在前端拒绝，两条后端无需承接无效调用；已关闭 |
| F-6 | `core/rust/crates/xiao-driver/tests/20d_first_api.rs:195`；同文件原生夹具 | VM 子进程在隔离 cwd 下读取 `data/relative.txt`；原生夹具以同样相对路径读取并输出 `relative-ok`；Windows 本机双路径通过，三平台 CI 使用同一用例；已关闭 |
| O-1/O-2 | `core/rust/crates/xiao-runtime/src/resources.rs:38`；`core/rust/crates/xiao-standard-library/src/README.md` | VM/ABI 共用打开、校验、读取、关闭与能力检查；移除 close 时无意义的目录路径复制，路径容量计入载荷内存；元数据 crate 阶段更新为 20C/20D |

对 F-1 根因补充：CI 的空 stderr 与 `ExitStatus::code() == None` 说明处理器没有运行。
本机 clang 的 AArch64/macOS 汇编探针确认 `llvm.trap` 降为 `brk #0x1`，对应 SIGTRAP，
原处理器只注册 SIGILL。故本次同时补注册 SIGTRAP 与故障类别退出映射，没有放宽 Fatal 断言。

返工后的 Windows 受控原生夹具已实跑通过，新增正常相对读取、目录重复关闭、文件不存在、
非法 UTF-8、关闭后别名读取的输出/错误码断言；缺 filesystem 能力仍报 `X06-RUNTIME-026`。
用户文档也已修正保留关键字 `file` 被误用为变量名的示例及 `os` 模块登记身份。

本地返工门禁：`tools/gates/run.sh` exit 0（含 fmt、Clippy、文档/覆盖率、TypeScript、基准自检、
驱动器测试和 Bun 测试）；`cargo test --workspace --no-fail-fast -j 2` 为
**1178 passed / 0 failed / 36 ignored**；`bun test` 为 295 pass / 5 skip / 0 fail。
两份 DevDocs 的逐行粗体配对检查通过。

### 12.1 绿色 push 与三平台日志对账

修复提交：`b956a0e60071f1fb7af501cf9755efa949a9c3ad`。
push 触发的 [维护回归 `39041556630`](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/39041556630)
六个作业全部 **success**；同一提交的 [工作区门禁 `39041556594`](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/39041556594)
也为 **success**（作业 `117421917454`）。

| 平台 | 原生回归作业 | 20D 专门夹具与修复回归 |
| --- | --- | --- |
| Windows | [117421917734](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/39041556630/job/117421917734) | 完整作业成功，原生专门夹具及默认 VM 专门夹具均通过 |
| Linux | [117421917826](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/39041556630/job/117421917826) | 完整作业成功，原生夹具日志第 963 行通过；延迟握手、信号映射、相对路径与类型负例均执行成功 |
| macOS | [117421917698](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/39041556630/job/117421917698) | 完整作业成功；延迟握手第 599 行、既有握手第 611 行、信号映射第 625 行、两条外部平台 Fatal 第 879/880 行、原生专门夹具第 973 行均通过；默认 VM 的相对路径及类型负例也通过 |

下载三平台作业日志后，对 `20D-NATIVE` 五条记录逐字比较，三个列表完全相等：

| 用例 | 标准输出 | 错误身份 |
| --- | --- | --- |
| 相对路径 `data/relative.txt` | `relative-ok\n` | 成功 |
| 目录重复关闭 | `resource<directory>\n` | 成功 |
| 文件不存在 | 空 | `X06-RUNTIME-012` |
| 非 UTF-8 | 空 | `X06-RUNTIME-012` |
| 关闭后别名读取 | 空 | `X06-RUNTIME-025` |

同一原生专门用例还验证正常 VM/LLVM 输出相等，以及缺 filesystem 能力时退出码 3、
错误码 `X06-RUNTIME-026`。因此 **F-1 阻断已关闭，20D 出口第 4 条已满足**。
`process` 交付仍按冻结规划在后置批次登记。

## 相关页面

- [20D-冻结](20d-freeze-resource-contract.md) §2.5——**本批的直接前置**：签名、错误身份、平台清单
- [20D-资源承载](20d-resource-carrier.md)——**上一批**：`ExternalResource` 变体、ABI 标签 17、关闭语义
- [20D-能力上下文](20d-capability-context.md)——**上上批**：能力位与两条路径的拦截点
- [20D-规划](20d-platform-resources-stage-plan.md) §4.2——批次顺序与 F-1/ABI 两道前置
- [20. 内置函数与标准库](20-builtins-and-standard-library.md) §4.1/§5.1/§5.2——资源句柄契约与平台顺序
