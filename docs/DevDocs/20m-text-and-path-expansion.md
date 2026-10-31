# 20M. 标准库文本与路径扩展（纯 Xiao）

> **这是 20 系列的下一批，不是 19 发布收口。**
> [20C](20c-official-xiao-library.md) 首批、20F–20L 已交付 `text`/`path`/`set`/`dict`/`os`；
> 本批只在既有的 `stdlib/text.xiao` 与 `stdlib/path.xiao` 上增加**纯 Xiao**接口。
> 不新增 intrinsic、不改 Runtime ABI、不引入泛型约束或新容器参数，也不碰
> `std.concurrent` 的虚拟模块边界。
>
> 状态：**九项候选的本地 VM/native 探测通过，冻结草案待确认（2026-11-01）**。
> 基线为 22 阶段完成后的 `cff1499`；探测与冻结草案见 §九，冻结前不写产品函数体。
> 19 发布收口、11C L4、11B I5 和 22E 的登记差异
> 仍保持各自文档的未闭合状态，不被本批顺带宣称完成。

## 一、入口条件与边界

### 1.1 前置

1. [20. 内置函数与标准库路线](20-builtins-and-standard-library.md) §2.1、§六、§七：
   纯 Xiao 优先、无宿主必要性不新增 intrinsic；
2. [20C. 官方 Xiao 库](20c-official-xiao-library.md) §3.2.1、§3.4.1：接口兼容策略、
   最小增量规则和模块登记；
3. [20I. 表达式下标与切分](20i-nonliteral-indexing-and-splitting.md)：`text.find`/`slice`/`split`
   已证明字符迭代和切片可用；
4. [20J/20K/20L](20l-dict-methods-and-20k-closeout.md)：模块式 API、静态链接和标准库
   loader 的既有边界；
5. `stdlib/` 当前已交付：`text`（7 个函数）、`path`（1 个函数）、`set`、`dict`、`os`，
   以及虚拟模块 `std.concurrent`。

### 1.2 本批不改变

| 项 | 冻结内容 |
| --- | --- |
| 实现形态 | 候选接口全部是纯 Xiao；不得新增 intrinsic、C ABI 或 Runtime 变体 |
| 兼容身份 | 新增接口属于兼容扩展，`STDLIB_INTERFACE_VERSION` 保持 1；不删除、不改签名、不改已有错误身份 |
| 类型系统 | 不新增泛型约束、重载、方法值或动态接收者分派 |
| 平台 | `path` 只定义文本语义，不访问文件系统，不承诺 Windows 反斜杠/盘符语义 |
| 范围 | 不改 `std.concurrent`、产品 ABI、19 发布矩阵或原生产物元数据 |

### 1.3 当前缺口

| 模块 | 当前接口 | 明显缺口 |
| --- | --- | --- |
| `text` | `contains_char`、`length`、`is_ascii_upper`、`lower_ascii`、`find`、`slice`、`split` | 首尾空白、前后缀判断、替换、重复、ASCII 大写方向 |
| `path` | `join` | 取末段、取父段、取扩展名 |
| `set`/`dict` | 自由函数与方法式 API 已交付 | 本批不扩展 |
| `os` | 文件/目录/进程首批 API | 本批不扩展，环境/时间/随机需要新 intrinsic |

## 二、目标与不负责

### 2.1 目标

1. 让 `text` 覆盖常见的纯文本检查与变换，不依赖 Unicode 表或宿主能力；
2. 让 `path` 具备可组合的文本级路径操作，明确只处理 `/`；
3. 为每个新增接口建立 VM/native 双路径、移除验证、模块登记和 UseDocs；
4. 保持标准库接口版本 1 和既有错误身份不变；
5. 用探测结果裁剪候选，拒绝零增量占位。

### 2.2 不负责

- 不做 Unicode 大小写、全量 Unicode 空白、正规化或字节级索引；
- 不做平台路径规范化、绝对路径判断、盘符或反斜杠兼容；
- 不新增 `math`、`random`、`time`、`array` 或 `collections` 模块；理由见 §3.4；
- 不做方法式 API 扩展、动态接收者或方法值；
- 不修复 19/11/22 的发布欠账，也不宣称发布验收完成。

## 三、设计裁定点

### D-1（候选清单）：文本与路径，先探测后冻结

| 编号 | 候选接口 | 形参/返回 | 探测要点 |
| --- | --- | --- | --- |
| T-1 | `trim(str) -> str` | 去掉首尾 ASCII 空白 | 空串、全空白、单侧空白、内部空白保留 |
| T-2 | `starts_with(str, str) -> bool` | 空 needle、长 needle、字符边界 | 用切片/逐字符，不依赖字节偏移 |
| T-3 | `ends_with(str, str) -> bool` | 同上 | 同上 |
| T-4 | `replace_all(str, str, str) -> str` | 非重叠、从左到右 | 空 needle、替换串含 needle、连续匹配 |
| T-5 | `repeat(str, int) -> str` | 计数 ≤ 0 返回空串 | 0、负数、长串和错误边界 |
| T-6 | `upper_ascii(str) -> str` | `a`–`z` 转大写 | 非 ASCII 原样保留，和 `lower_ascii` 对称 |
| P-1 | `basename(str) -> str` | 最后一个 `/` 之后 | 无分隔符、尾随 `/`、根 `/` |
| P-2 | `dirname(str) -> str` | 最后一个 `/` 之前 | 无分隔符、根 `/`、连续 `/` |
| P-3 | `extension(str) -> str` | 末段最后一个 `.` 起（含点） | 无点、以点开头、多个点、目录中的点 |

以上九项是**待探测候选**，不是已冻结 API。每项必须先有 VM/native 双路径探测和一条
真实文本夹具，冻结后才写入 `stdlib/text.xiao` / `stdlib/path.xiao`。

### D-2（语义冻结）：边界先写死

冻结时必须逐项写清并在 UseDocs 复述：

- `trim` 的空白集合固定为 ASCII `space/tab/CR/LF/VT/FF`；其他字符不剥除；
- `starts_with`/`ends_with` 以字符为单位；空 needle 返回 `true`；
- `replace_all` 从左到右、非重叠；空 needle 返回原串，不执行替换；
- `repeat` 的计数 ≤ 0 返回空串；不新增错误编号；
- `upper_ascii` 只转换 ASCII `a`–`z`，与 `lower_ascii` 的规则对称；
- `path` 只识别 `/`；反斜杠是普通字符；不合并重复分隔符、不解析 `.`/`..`；
- `basename`/`dirname`/`extension` 对空串、尾随分隔符和根路径的行为必须在冻结表里
  逐格写明，不能只写“取最后一段”。

若探测发现某项无法在不新增语言能力的前提下稳定表达，**删除该项**并登记原因，不用
近似实现凑数。

### D-3（探测与增量门槛）

沿用 [20C](20c-official-xiao-library.md) §3.4.1 的“最小接口必须真有增量”：

1. 每项候选先在 `core/rust/crates/xiao-driver/tests/c20_capability_probe.rs` 或同族
   探测入口增加一条真实夹具，VM 与受控原生都要成功；
2. 探测只证明可表达，不冻结 API；结果成表后再做 D-1 冻结；
3. 只有“现有接口无法用一行组合得到”的项才保留；能用现有 `slice`/`find`/`contains_char`
   直接表达的，登记为“组合即可”，不入库；
4. 任何需要新增 intrinsic、Unicode 表、平台路径规则或泛型约束的项，直接移出本批。

### D-4（模块、文档与兼容身份）

- 只改 `stdlib/text.xiao`、`stdlib/text.md`、`stdlib/path.xiao`、`stdlib/path.md`；
- 更新 `stdlib/README.md` 的模块职责与状态，更新 `docs/module-registry.json` 对应条目；
- 新增 UseDocs 小节和示例；页面保持现有 `verified` 状态的前提是本批 CI 证据通过；
- 新接口属于兼容扩展，不提升 `STDLIB_INTERFACE_VERSION`；
- `.xar` 的 `stdlib_modules` 继续按实际导入记录 `stdlib.text`/`stdlib.path`，不新增字段；
- 不新增 `stdlib/concurrent.xiao`，不改 22E 的虚拟模块与原生元数据。

### D-5（测试与证据）

- 每个接口至少一条 VM 成功、一条原生成功，并逐字比较输出；
- 边界用例覆盖 D-2 的空白、空串、负计数、连续分隔符、尾随分隔符和点文件；
- 保留 `text_module_vm_result_and_removal_boundary_are_stable` 与
  `path_module_vm_result_and_removal_boundary_are_stable` 的移除验证，并扩展到新接口；
- 模块命名空间/普通包隔离和 `.xar` 重放继续按 20C/20E 的用例执行；
- 三平台维护回归和完整 workspace gates 全部通过；本地 docs/usedocs/layout/覆盖率通过。

### D-6（欠账与口径）

- 修正 `stdlib/README.md` 里“真实 push 运行号和 Linux/macOS 受控复跑仍待补齐”的过时
  表述：以本批实际运行号为准，不把旧状态带进新模块；
- 20C/20I/20L 的状态只在新证据后更新，不提前把本批接口写成已验证；
- 19 发布收口、11C L4、11B I5 和 22E 登记差异保持原文档状态，本批不认领。

## 四、实施步骤

| 步 | 内容 | 验证 |
| --- | --- | --- |
| 0 | 候选探测：九项各一条 VM/native 夹具，成表 | 双路径结果；不可表达项剪掉 |
| 1 | 冻结 D-1/D-2：接口、边界、错误和“组合即可”剔除表 | 冻结表 + 星崽确认 |
| 2 | 实现 `text` 通过项 | VM/native 黄金输出、边界用例 |
| 3 | 实现 `path` 通过项 | 同上；`/`-only 口径 |
| 4 | 模块文档、README、registry、UseDocs | docs/usedocs/layout；module-registry 校验 |
| 5 | 移除验证、普通包隔离、`.xar` 重放 | 既有 loader 用例扩展 |
| 6 | 三平台维护回归 + 完整 workspace gates | 实际运行号 |
| 7 | 文档收口：20M 状态、20C/20 路线索引、stdlib 状态 | docs/usedocs/layout |

## 五、验收与证据

1. 九项候选中保留项全部有 VM/native 双路径、边界和移除验证；被剪项有书面理由；
2. `text`/`path` 源码、模块文档、UseDocs、registry 和 stdlib README 同步；
3. 标准库接口版本仍为 1，没有删除或改签名；
4. 三平台维护回归与完整 workspace gates 通过，运行号写回本文；
5. 没有新增 intrinsic、C ABI、Runtime 变体或 `.xar` 字段；
6. 既有 20C/20I/20L 用例不退化，22D/22E 的并发用例不受影响。

## 六、退出条件

1. D-1 冻结表有结论，保留项与剪掉项都写明依据；
2. 保留项实现、文档、验证和 CI 证据齐全；
3. `text`/`path` 模块状态与 registry/UseDocs 一致，无虚报；
4. 本批只做兼容扩展，接口版本、错误身份和冻结 ABI 不变；
5. 发布收口（19）与 11/22 的未闭合项仍按各自文档登记，不因本批被误标完成。

## 七、风险与禁止

1. **Unicode 过度承诺**：不得把 `upper_ascii`/`trim` 写成 Unicode 语义；
2. **路径平台化**：本批只认 `/`；不得声称 Windows 兼容或文件系统行为；
3. **零增量占位**：能用现有接口组合的，不入库；
4. **错误身份漂移**：新接口不新增错误编号；边界行为用返回值定义；
5. **兼容身份误升**：新增接口不提升标准库接口版本；
6. **顺带发布**：不得在本批宣称 19 发布收口或 22E 之后的发布完成；
7. **范围蔓延**：不新增泛型约束、方法值、动态接收者、math/random/time/array 模块。

## 八、落点

```text
docs/DevDocs/20m-text-and-path-expansion.md          本计划
docs/DevDocs/20-builtins-and-standard-library.md     20 路线批次表
docs/DevDocs/20c-official-xiao-library.md            20C 冻结规则
docs/DevDocs/README.md                               阶段索引
stdlib/text.xiao / text.md                           文本模块
stdlib/path.xiao / path.md                           路径模块
stdlib/README.md                                     模块职责与状态
docs/module-registry.json                            模块登记
docs/UseDocs/library/text.md                         文本公共文档
docs/UseDocs/library/path.md                         路径公共文档
core/rust/crates/xiao-driver/tests/c20_capability_probe.rs  候选探测
core/rust/crates/xiao-driver/tests/20c_first_modules.rs     移除与双路径
```

## 九、探测记录与冻结草案（2026-11-01）

### 9.1 真实源码探测

九项夹具位于 `tests/fixtures/20m/probe-*.xiao`，统一入口为
`core/rust/crates/xiao-driver/tests/20m_text_and_path.rs`。每项从同一份前端 IR 分别运行
VM 和受控原生构建；两路逐字匹配独立黄金输出，原生退出码为 0、stderr 为空。
VM 与原生执行均限制为十秒；原生构建使用真实 `llvm-as` 校验，不以 LLVM 文本包含关系代替。

本机 Windows `x86_64-pc-windows-msvc` 实跑：2 个测试通过、9/9 候选双路径通过。
覆盖六种 ASCII 空白、非 ASCII 空白保留、空串、Unicode 码点、非正重复次数、非重叠替换、
替换结果不重扫，以及 §9.3 的 18 行路径边界。尚未产生本批三平台远端证据。

```powershell
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20m_text_and_path -- --include-ignored --nocapture
```

### 9.2 接口裁剪草案

以下是**待星崽确认的草案**，不是已交付接口。按本文 D-3 的一行组合门槛裁剪；
该门槛比 20C §3.4.1 仅要求冻结语义决定更严格。

| 编号 | 候选 | VM/native | 建议 | 冻结的决定或裁剪依据 |
| --- | --- | --- | --- | --- |
| T-1 | `trim(str text) -> str` | 通过 | 保留 | 只剥除首尾六种 ASCII 空白，内部和其他 Unicode 空白保留 |
| T-2 | `starts_with(str text, str needle) -> bool` | 通过 | 组合即可，剔除 | `find(text, needle) == 0`；空 needle 为 true |
| T-3 | `ends_with(str text, str needle) -> bool` | 通过 | 组合即可，剔除 | 下列长度守卫＋字符切片表达式；长 needle 短路，不触发负切片 |
| T-4 | `replace_all(str text, str needle, str replacement) -> str` | 通过 | 保留 | 从左到右非重叠匹配，空 needle 返回原串，替换结果不重新扫描 |
| T-5 | `repeat(str text, int count) -> str` | 通过 | 保留 | count ≤ 0 返回空串；不增加专用错误身份 |
| T-6 | `upper_ascii(str text) -> str` | 通过 | 保留 | 仅转换 ASCII a–z，不依赖 Unicode 表或 locale |
| P-1 | `basename(str path) -> str` | 通过 | 保留 | 最后一个 `/` 后的文本，保留空末段，不规范化 |
| P-2 | `dirname(str path) -> str` | 通过 | 保留 | 最后一个 `/` 前的文本；无 `/` 返回空串，不补 `.` 或根分隔符 |
| P-3 | `extension(str path) -> str` | 通过 | 保留 | 只取末段最后一个点及其后文本，含点；开头点和结尾点均参与，不特殊解析点文件 |

前后缀的既有组合（函数参数为 `str`）：

```xiao
find(text, needle) == 0
length(needle) <= length(text) and slice(text, length(text) - length(needle), length(text)) == needle
```

建议新增 text 4 项、path 3 项；没有不可表达项。保留项都需要扫描、循环或累积构造，
不能由现有接口的一行调用组合直接得到。标准库接口版本保持 1，不增加 intrinsic、Runtime ABI、
类型能力或归档字段。

### 9.3 路径逐格语义草案

表中 `""` 表示空字符串；反斜杠按普通字符保留。这里只截取原始文本：尾随 `/` 产生空末段，
dirname 不把空父段改成 `/` 或 `.`；`.`、`..` 和点文件也按普通文本处理。

| 输入 | basename | dirname | extension |
| --- | --- | --- | --- |
| `""` | `""` | `""` | `""` |
| `name` | `name` | `""` | `""` |
| `/` | `""` | `""` | `""` |
| `/name` | `name` | `""` | `""` |
| `a/` | `""` | `a` | `""` |
| `a/b` | `b` | `a` | `""` |
| `a//b` | `b` | `a/` | `""` |
| `//` | `""` | `/` | `""` |
| `a//` | `""` | `a/` | `""` |
| `.env` | `.env` | `""` | `.env` |
| `dir.x/.env` | `.env` | `dir.x` | `.env` |
| `a.tar.gz` | `a.tar.gz` | `""` | `.gz` |
| `file.` | `file.` | `""` | `.` |
| `.` | `.` | `""` | `.` |
| `..` | `..` | `""` | `.` |
| `a/../b` | `b` | `a/..` | `""` |
| `C:\tmp\file.txt` | `C:\tmp\file.txt` | `""` | `.txt` |
| `目录/小😀.x` | `小😀.x` | `目录` | `.x` |

### 9.4 后续验收状态

- 步骤 0 已有本地双路径证据；步骤 1 等待星崽确认 §9.2/§9.3。
- 探测阶段的 fmt、定向 Clippy、docs、UseDocs 与 layout 检查通过。
- 产品函数体、逐接口移除验证、模块隔离、归档重放和公开文档尚未实施。
- 三平台维护回归、完整 workspace gates、覆盖率及运行号尚待本批实现后核销。
- 19 发布收口、11C L4、11B I5 和 22E 登记差异保持原状态。
