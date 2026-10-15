# 20C-第七批. `str` → `int` 码点转换与 `stdlib/**` 的 CI 覆盖

> **背景**：[20C-第六批](20c-sixth-batch-case-conversion-and-api-closeout.md) §四乙 判定大小写转换
> **绕不开**（63 行机械展开，超过预先冻结的「约 30 行」线），于是**只登记并立项**了
> `str` → `int` 码点转换，把实现留给**下一个语言批次**。**本批就是那个批次。**
>
> **但**：写本批规划时把转换矩阵核了一遍，发现**登记的那一条不是充分前置**——
> 大小写转换还需要**回程**（码点 → 字符），而回程**没有入口**，
> 且 `int as str` 已被第四批冻结成**十进制文本**，**借不了道**（§三，逐条给证据）。
>
> **一句话概括**：**先裁定这一批做几条**（只做去程，还是去程 + 回程），
> 再顺手补一个结构性缺口（`stdlib/**` 改动目前**零 CI 信号**，§二）。

状态：**已实施并完成 CI 取证（2026-10-15）**：§九 六条**全部裁定**已落地；`str` → 三种整数、
`chr` 回程、`text.lower_ascii` 与 R-1/R-2 已完成。真实 push 的工作区门禁与维护回归均已通过，
运行号与原生作业见 §十一。

## 一、为什么是这一批

1. **第六批 §四乙 的登记**已经写清三要素——
   需要什么（`str` → `int` 码点转换）、当前以什么码被拒（`X02-TYPE-006`）、阻塞哪条接口（大小写转换）；
2. **它挡着 `text` 模块的下一个候选**：大小写转换是第六批复探之后**唯一还没解封**的库候选；
3. **并入一笔结构性返工**（§二）：它与本批主题无关，但按「返工项并入下一阶段规划、
   不另开修复批次」的既定做法**随批做**——而且**必须做在本批的库改动之前**（理由见 §2.1）。

## 二、并入的返工项（**先做**）

### 2.1 R-1（中）：`stdlib/**` 不在任何工作流的 `paths` 里

**事实（2026-10-15 实测，不是推断）**：仓库有三个工作流，但 push/PR **只会触发其中两个**
（`platform-reproduction.yml` 只有 `cron "17 3 * * 1"` 与 `workflow_dispatch`）：

| 工作流 | push `paths` | 跑什么 |
| --- | --- | --- |
| `maintenance-regression.yml` | `core/rust/**`、`tests/{fuzz,benchmarks,fixtures}/**`、自身、`.gitattributes` | Rust workspace 测试 + 双平台原生差分 |
| `workspace-gates.yml` | `tools/**`、`cli/**`、`docs/**`、`package.json`、`bun.lock`、`tsconfig*`、`core/rust/rust-toolchain.toml`、自身、`.gitattributes` | `bash tools/gates/run.sh`——含 `bun run check`（全部 A0 文档门禁）、`bun test`、`bunx tsc --noEmit`、`cargo clippy` |

**`stdlib/**` 两边都不在**：官方库源码（`text.xiao`、`path.xiao`）与它的 `.md`，
**只改它们的提交不会触发任何工作流**。`grep -rn "stdlib" .github/workflows/` **零命中**。

**为什么这是问题**——它正好是刚过去的 F-3 的**结构性成因**：

1. [20C-首批](20c-first-modules.md) §4.6 第 2 条要求模块级 `verified` 的依据来自
   「CI 的可复核运行记录」；而**依据所指向的文件正是 `stdlib/*.xiao`**；
2. 第六批 §2.3 已经吃过一次「模块状态领先于 CI 证据」；那次靠一句「待补」补住了，
   但**只要 `stdlib/**` 不在 `paths` 里，同一个形状会再长出来**；
3. **「有 CI 记录」不证明「这类文件被覆盖」**：第五批 `432e87a` 确实留下了 CI 记录，
   但那是**搭了同笔 `tests/fixtures/**` 与 `docs/**` 的车**——`stdlib/` 本身没人监听。

**冻结**：

1. **把 `stdlib/**` 加进 `workspace-gates.yml` 的 `push` 与 `pull_request` 两处 `paths`**
   （该工作流跑 `bun run check`，正是验文档与库一致性最相关的那套）；
2. **不要**顺手把它加进 `maintenance-regression.yml`——那套会跑双平台原生作业，
   成本高一个数量级，而 `stdlib/` 的正确性由 Rust 测试在 `tests/fixtures/**` 变动时已经覆盖；
3. **改的是 `paths`，不是 `paths-ignore`**：本仓库的既有写法是白名单；
4. 改完必须**用一次真实 push 取证**——加路径这件事本身属于「本机跑过不算数」那一类
   （[20C-门禁残余项](20c-native-gate-residuals-fix.md) §5.2.1 第 5 条的教训）。

### 2.2 R-2（低）：`platform-reproduction.yml` 自己没人守

**事实**：另两个工作流各自只列了**自己**那个文件名，所以改 `platform-reproduction.yml`
不会触发任何工作流——**守着其它一切的那个工作流，自己没有被守**。

**本批随 R-1 一起处理**：在轻量 `workspace-gates.yml` 的 `push.paths` 与
`pull_request.paths` 加入 `.github/workflows/platform-reproduction.yml`，让平台复现工作流
的自身改动至少触发一次工作区门禁；不把平台复现的原生矩阵并入维护回归。

### 2.3 不是问题的（记下来免得下次重查）

仓库根目录有一个未跟踪的 `probe_tmp.lib`。它**不被 git 跟踪**（`git ls-files` 报
`did not match any file(s) known to git`），且 `git status --porcelain` 干净，
说明已被 ignore——是本地临时文件，**不是仓库缺陷**。

## 三、这道题比登记的大

第六批 §四乙 登记的是「`str` → `int` 码点转换」。**核完矩阵后，它不足以解除大小写转换的阻塞。**
逐条给证据：

1. **去程确实缺**：`classify_conversion`（`xiao-types/src/conversion.rs:87-106`）的矩阵里
   只有 `Str→Bool`（`conversion.rs:89`）、`Bool→Str`（`:91`）、整数族 `→Str`（`:93`）；
   **`Str` → 数值族一条都没有**，落到 `:101-105` 的 `Err`，前端报 `X02-TYPE-006`。
   第五批复探表已双路径实测过这一行（`int("A")` 两平台均前端 `X02-TYPE-006`）✓；
2. **回程也缺，而且不显眼**：大小写转换要把 `65 + 32 = 97` 变回字符 `"a"`。
   但 `int` → `str` **已经被第四批冻结为十进制文本**
   （[02. 类型系统](02-type-system.md):118-121：「结果是十进制文本，并复用 Runtime 的 `display_text()`」；
   维护回归 `38597139383` 的 `second-int-to-str` 两平台输出 `42:42:42:42:42:42:42`）。
   ⇒ `(97 as str)` 得到的是 `"97"`，**不是 `"a"`**；
3. **没有别的路**：全仓搜 `chr(` / `from_code` / `from_codepoint` / `code_point`
   在 `xiao-intrinsics`、`xiao-types`、`stdlib/`、`02-type-system.md` **零命中**；
   用**下标**从 `"abc…z"` 里取也走不通——非字面量下标在类型层就是 `X03-TYPE-003`
   （第三批已登记）。

⇒ **只做 `str` → `int` 的话，大小写转换仍然写不出来**（回程无路）。
所以本批要回答的是**两条**，不是一条；而回程的形式**不能**顺手定，见 §4.3。

## 四、两道主裁决（**先汇报，再定**）

### 4.1 裁决一：这一批做几条

| | 做法 | 代价 | 结果 |
| --- | --- | --- | --- |
| **甲** | 只做 `str` → `int`（严格照第六批登记） | 小 | **不解除任何阻塞**；回程单独立项，大小写转换继续等 |
| **乙** | 去程 + 回程一起做 | 大（多一道设计裁定，见 4.3） | 大小写转换真正可写，`text` 的下一个接口可以开 |

**审核倾向：乙。** 理由是甲**拿到了零收益**——本批被立项的唯一动机就是解除大小写转换，
而甲做完之后它仍然不可写；一个不解除任何阻塞的语言批次，等于把同一个决定推迟再做一次。
**反方理由**（要一并摆上）：乙的**回程形式**没有现成答案（4.3），
为一个尚未冻结的库候选去动语言表面，与 [20C](20c-official-xiao-library.md) §3.4
「不许为了让官方库跑起来而批量新增 intrinsic」的精神有张力。

### 4.2 裁决二（只在取乙时才有意义）：回程的形式

**约束（先排除掉不可能的）**：

- **不能改 `int as str` 的语义**——第四批已冻结、有 CI 证据，改它是破坏性变更；
- **不能靠库绕开**——库函数体里也得有一个能产出字符的原语，否则会出现
  「为了写这条库函数而先要这条库函数」的循环；
- **不能加下标**——那是第三批登记的另一笔，不在本批。

**候选**（三个，代价依次上升）：

1. **构造式入口**：沿用 `bool(x)` / `str(x)` 的既有形状，新增一个**不叫 `str` 的**构造式
   （例如按 `as` 写不出来、只能写成构造式）。**最省**，但要让「`str(x)` 是十进制、
   `另一个名字(x)` 是码点」这件事在文档与诊断里说清楚，否则是最容易误读的一处；
2. **新标量类型** `char`：语义最干净（码点是码点、文本是文本），但 `ScalarType`
   加一个变体要动转换矩阵、Runtime 表示、两条后端的 lowering —— **本批装不下**；
3. **收敛到库 + 一个受控 intrinsic**：注意这与 §3.4 第 6 条的「不得为让官方库跑起来而批量新增
   intrinsic」直接冲突，只有在星崽明确批准时才可走。

**审核倾向：候选 1。** 它代价最小、不动类型系统，且能把「唯一的安全入口」这句
（[20C](20c-official-xiao-library.md) §3.4.1 的补严口径）写实。

### 4.2.1 取「构造式」之后暴露出来的两件事（**审核侧补记，2026-10-15**）

裁定取 §4.2 候选 1 之后，我去核**现有构造式的机制**，发现两件在选项文字里**没有写**的事：

**其一：构造式命名空间是按标量类型名寻址的。**
`scalar_callee`（`xiao-types/src/checker/conversion.rs:21-26`）先查内置契约表、
要求 `IntrinsicKind::ScalarConversion`，再把 `public_name` 交给
`ScalarType::from_name`（`xiao-syntax/src/ast.rs:328-340`）——**那 8 个标量类型名就是全部转换构造式**。

- **去程没事**：`int("A")` 这个名字**已经存在**（第五批复探用的就是它），本批只是让它别再落到 `Err`；
- **回程不行**：目标类型 `str` **已被十进制文本占用**，而**没有 `char` 标量**——
  候选 2 被排除的那个理由，绕一圈又回来了。

**其二：所以回程必须加一条「名字不是标量类型名」的内置入口。**
先例现成：`IntrinsicKind` 里的 `SetConstructor`、`ErrorConstructor`
（`xiao-intrinsics/src/lib.rs:52-57`）就是这种形状。⇒ 落点是
**在 `xiao-intrinsics` 的契约表里加一个 `IntrinsicKind` 变体**，外加 `scalar_callee` 旁的一条解析分支。

**这与「不许新增 intrinsic」的关系必须摆清楚**：[20C](20c-official-xiao-library.md) §3.4 第 6 条禁的是
「**为了让官方库跑起来而批量新增** intrinsic」。本条的动机是**语言能力**（码点 ↔ 字符），
不是让某个库函数能编译，而且只有**一条**。**但形式上它确实是内核契约表里的一条入口**——
这一点原选项没写清，所以**不许**靠「与审核倾向一致」推定，要在 §九 显式确认。

### 4.2.2 取 `chr` 之后仍未定的边界（**实施前必须在 §五 出真值**）

名字定了，**边界没定**。以下四条在动手之前要有**真值**，不许「实现完了看起来对」：

1. **`chr` 的宽度**：按 §九 第 3 条「三个宽度一起」，`chr` 接受整数族三者，
   与去程 `int(a)` / `sint(a)` / `lint(a)` 对称；
2. **失败与非法输入的语义**（**去程与回程各一套**）：
   去程——空串、多字符（`"AB"`）、非 ASCII（`"小"`）；
   回程——负码点、超出 Unicode 标量范围（`> 0x10FFFF`）、**代理区**（`U+D800`–`U+DFFF`，
   它们不是合法的标量值）。**失败的层与码也要定**（倾向于复用 `X06-RUNTIME-012`，见 §六）；
3. **常量折叠是否随批开放**：**本批不开，只登记**。
   依据是第四批的先例——该批明确没开「整数字面量到文本的常量折叠」
   （[02. 类型系统](02-type-system.md):121 原样有效）。**按对称，本批也不开**；
   这是从先例推出来的冻结，**若星崽要一起开，翻这条**；
4. **`for c in text` 的迭代单位**要与去程口径一致：`length("小雪") == 2` 已被
   `xiao-driver/tests/20c_first_modules.rs:193` 的断言钉住（按**码点**计数），
   所以去程按码点是**与既有语义一致**的选择，不是新决定。

### 4.3 不许

1. **不许**在没裁定 4.1 之前就动手——甲与乙的落点**完全不同**，先写再改会留下两份无意义的证据；
2. **不许**把回程塞进 `int as str`——见 4.2 第 1 条；
3. **不许**顺带做「浮点 → `str`」或「`str` → `float`」——本批只碰整数族，
   与第四批的范围纪律一致；
4. **不许**为了让大小写转换进库而放宽任何既定边界（第二批 §四 第 2 条原样有效）。

## 五、补探（**先做，成表**）

**口径**：逐字沿用 [20C-第二批](20c-second-batch-probe.md) §3.2 的六条（真写真跑、双路径、成表、
不许绕开、一夹具一拒绝、结论先汇报）。

**要回答的问题**（**每一问都要落到「层 + 码」**，不许写「应该会拒绝」）：

| # | 问题 | 为什么必须问 |
| --- | --- | --- |
| 1 | 当前 `int("A")` / `"A" as int` 的层与码 | 第五批已有双路径证据（`X02-TYPE-006`），本批**复核一次**即可 |
| 2 | **动态值**持 `str` 转 `int` 会怎样 | 类型层对 `dynamic` 源一律放行（`conversion.rs:73-80`），所以它**能进后端**，落到 Runtime 的 `convert_to`；该处没有 `(Str, Int)` 臂，会命中 `ops.rs:269-273` 的兜底。**必须实测确认码**，不许推断 |
| 3 | `"42" as int` 是不是也走同一条路 | 若甲的语义是「码点」，`"42"` 该被拒；这决定了**多字符输入**的规则 |
| 4 | `as sint` / `as lint` 的**改动前**基线 | **已裁定三个宽度一起开**（§九 第 3 条）；本问改为**记录改动前的基线**，供验收对照 |
| 5 | 空串、多字符、非 ASCII（如 `"小"`）在被裁定语义下的行为 | 这三条是**去程**的边界（§4.2.2 第 2 条），必须先有真值 |
| 6 | 回程 `chr` 的边界：负码点、`> 0x10FFFF`、**代理区** `U+D800`–`U+DFFF` | §4.2.2 第 2 条的**回程**那一套；代理区**不是**合法标量值，必须实测它在哪一层被挡 |

**判据**：第 1、2 两条**双路径同码**即算复核通过；第 3–5 条**先在探针里写真值**，
再拿来当实现后的验收断言——**不许**先写实现再补真值。

### 5.1 改动前补探实测结果（2026-10-15）

下表记录实现前的真实基线。第 1 条沿用第五批同一夹具的 VM/Windows LLVM 双路径复核；第 2 条
使用临时项目模块的动态成员值（实际值是 `str`），在旧实现中两条路径都被通用动态检查挡住。
第 3–6 条先以 VM 探针取真值，原生入口在同一前端层给出相同拒绝身份。

| # | 夹具形态 | VM | Windows LLVM 原生 | 实现前结论 |
| --- | --- | --- | --- | --- |
| 1 | `int("A")`、`"A" as int` | 前端 `X02-TYPE-006` | 前端 `X02-TYPE-006` | 去程缺臂 |
| 2 | 动态 `str` 成员 → `int` | Runtime `X06-RUNTIME-002` | 进程状态 `3`，stderr `X06-RUNTIME-002` | 通用 `dynamic_conversion` 提前拦截，尚未到 `convert_to` |
| 3 | `int("42")` / `"42" as int` | 前端 `X02-TYPE-006` | 前端 `X02-TYPE-006` | 多字符规则尚无实现入口 |
| 4 | `"A" as sint`、`"A" as lint` | 前端 `X02-TYPE-006` | 前端 `X02-TYPE-006` | 三宽度均缺去程 |
| 5 | 空串、多字符、`"小"` | 前端 `X02-TYPE-006` | 前端 `X02-TYPE-006` | 需由新 Runtime 语义分别给出空/多码点拒绝与非 ASCII 成功 |
| 6 | `chr(-1)`、`chr(1114112)`、`chr(55296)` | 前端 `X02-TYPE-001`（`chr` 未定义） | 前端 `X02-TYPE-001` | 回程无语言入口 |

补探结论先于实现成立：去程必须只接受一个 Unicode 码点，`"小"` 应成功、`"42"` 与空串应报
`X06-RUNTIME-012`；回程三个非法边界也统一报 `X06-RUNTIME-012`。探针没有使用变量下标、临时
intrinsic 或 Rust 下沉逻辑。

### 5.2 统一历史探针的新真值

第七批后，统一探测清单里的两个旧夹具也转为成功用例：`fifth-case-conversion.xiao` 输出 `65`；
`second-case-conversion.xiao` 因其夹具把字符码点加 `32` 后直接打印整数，输出 `9798`。
因此 `c20_capability_probe.rs` 已把两条历史期望从旧 `X02-TYPE-006` 改为 VM 成功输出，并新增
`seventh-str-int-and-char.xiao` 条目。旧批次文档保留其当时的拒绝真值与历史结论，不追溯改写。

## 六、落点（已按两道裁决落地）

现有事实（已核，行号按 2026-10-15 的 `main`）：

```text
core/rust/crates/xiao-types/src/conversion.rs:87-106   转换矩阵（已加入 `StrToInteger`）
core/rust/crates/xiao-types/src/conversion.rs:111-114  `requires_runtime_check`（去程保持 true）
core/rust/crates/xiao-types/src/diagnostics.rs:17      INVALID_CONVERSION_CODE = "X02-TYPE-006"
core/rust/crates/xiao-runtime/src/value/ops.rs:218-274 `convert_to` 的 match（运行时侧）
core/rust/crates/xiao-runtime/src/value/ops.rs:303-314 `convert_str_to_bool` 与码点转换并列
core/rust/crates/xiao-runtime/src/value/ops.rs:261-268 `(Lint|Lfloat, _)` 的兜底——本批不扩数值债
```

**诊断身份**：转换失败**复用现成的 `X06-RUNTIME-012`**（`xiao-diagnostics/src/lib.rs:41`
的 `INVALID_VALUE_CODE`，`message_id = runtime.invalid_value`，经 `RuntimeError::invalid_value` 产生）。
**不需要新档位**，本批也不得顺手新增。

**其余落点（去程 + 回程都取用时）**：

```text
tests/fixtures/20c/                       §五 的补探夹具（一夹具一拒绝）
stdlib/text.xiao + stdlib/text.md         大小写转换接口（**只在乙时**才有意义）
docs/UseDocs/library/text.md              同上，同笔
docs/module-registry.json                 同上，同笔（按模块路径登记，不枚举函数名）
20c-official-xiao-library.md §4.2/§4.3    保留模块 `verified` 的门禁放宽、六条冻结的实情
docs/DevDocs/02-type-system.md            显式转换矩阵的更新（**与代码同笔**）
```

## 七、执行顺序（**顺序本身是约束**）

1. **先做 R-1 与 R-2**（§2.1/§2.2）并**推一次**——理由是顺序性的：本批后半段要往 `stdlib/` 加接口，
   而**加完再补门禁**的话，那个接口的第一次 CI 记录仍然是「搭别的路径的车」得来的；
2. **§五 补探**，成表，**结论先汇报**；
3. **两道裁决**（§四）落定后再动代码；
4. 实现 + 三处文档同笔（`02-type-system.md` 的矩阵、`stdlib/*.md`、UseDocs、registry）；
5. **推一次**，取证；`verified` 的翻转按 §4.6 的三条件走，**不许**提前。

**冻结**：

1. **不许**把 R-1 与 §五 的补探放进**同一笔提交**——一笔里同时有「门禁路径」和「探测结论」，
   出问题时**分不清是哪件造成的**；
2. **不许**在第 1 步的推送里夹带本批的功能改动（[20C-第六批](20c-sixth-batch-case-conversion-and-api-closeout.md) §六 同形）；
3. **不许**跳过补探直接实现——第 3 问与第 5 问的真值**就是验收断言**。

## 八、验收

1. **R-1**：`stdlib/**` 已在 `workspace-gates.yml` 的 `push` 与 `pull_request` 两处 `paths` 里；
   **有一次真实 push 的运行号**证明「只改 `stdlib/` 也能触发」——
   本机跑过**不算**。维护回归运行 `38726737088` 与 Windows/Linux 原生作业已取证；
   `maintenance-regression.yml` 的 `paths` **未被顺手扩大**；
2. **R-2**：同一轻量工作流的两处 `paths` 已加入 `.github/workflows/platform-reproduction.yml`，
   使其自身改动获得工作区门禁信号；没有把它加入维护回归的原生矩阵；
3. **裁决**：§四 的裁决以**直接裁定**记（[20C](20c-official-xiao-library.md) §2.4 **G-2**：
   一致不等于推定），并写明落点；
4. **补探**：六问成表，每问给**层 + 码**；第 1、2 问双路径同码；第 3–6 问的真值先于实现存在；
   **没有**用变量下标、临时 intrinsic 或 Rust 下沉绕开；
5. **实现**：`str` → `int`/`sint`/`lint` 与 `chr` 回程均已接通；去程转换对象的
   `requires_runtime_check == true`，非法输入报 `X06-RUNTIME-012`；**没有**改 `int as str` 的既有语义；
   **没有**新增诊断档位；
   **没有**顺带扩到浮点；
6. **诚实**：本批**不许**把大小写转换写成「已解封」而不给接口与双路径证据；
7. **通用**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit` 全绿；
   提交标题带类型前缀、正文非空。

## 九、裁定结果（星崽直接裁定，2026-10-15）

| # | 问题 | 裁定 | 落在哪 |
| --- | --- | --- | --- |
| 1 | **本批做几条** | **乙：去程 + 回程** | §4.1 |
| 2 | **回程的形式** | **构造式入口** | §4.2，机制落点见 §4.2.1 |
| 3 | **`as sint` / `as lint` 要不要一起开** | **三个宽度一起** | §五 第 4 问、§六 落点 |
| 4 | **R-2 要不要随本批改** | **随本批一起改** | §2.2 |
| 5 | **回程入口的机制**（§4.2.1 的补记带出） | **算语言能力，照做**——按 `SetConstructor`/`ErrorConstructor` 的形状进 `xiao-intrinsics` 契约表 | §4.2.1 |
| 6 | **回程的构造式命名** | **`chr`** | §4.2.2 |

**来源**：星崽直接裁定（AskUserQuestion，2026-10-15，两轮）；六条与审核倾向**全部一致**——
但按 [20C](20c-official-xiao-library.md) §2.4 **G-2** 的口径，一致**不等于**推定，本文以**直接裁定**记。

**第 5 条的连带（这一条最要紧）**：它**不是**「按倾向办」——第 2 条裁定取「构造式」时，
选项文字里**没有**写「这会往内核契约表里加一条入口」。机制是核出来之后才现形的（§4.2.1），
所以第 5 条是**被告知后才裁的**，不是推定的。这一点必须留在纸面上，
否则下次会有人拿「第 2 条已经裁过」当理由跳过它。

**第 2 条的连带（写下来免得反复问）**：取「构造式」之后仍有两处必须先定，
**机制**与**命名**，两处都不在原选项的文字里——逐条见 §4.2.1。
**不许**把第 2 条的裁定直接当成「回程已经定完了」。

## 十、实施与自审核（2026-10-15）

实现按裁定落在以下边界：

| 项目 | 落点 | 自审核结论 |
| --- | --- | --- |
| 去程 | `ConversionKind::StrToInteger`；`int`、`sint`、`lint` 三个构造式与 `as` 共用矩阵 | 只接受恰好一个 Unicode 码点；不做常量折叠、不扩浮点 |
| 回程 | `chr`，契约表新增 `CharConstructor` 类别和单一入口 | 只接受三种整数；不改变 `int as str` |
| Runtime | VM `RuntimeValue::convert_to` 与 `to_char`；原生 Runtime ABI 同步接线 | 空串、多字符、负数、越界和代理区均为 `X06-RUNTIME-012` |
| 动态值 | 标量构造/`as` 直接进入 Runtime 转换；动态赋值边界保持原规则 | VM 动态模块成员值可得到 `65`；原生模块字段仍受既有 `dict_table`/`table` 边界限制，不伪造成功证据 |
| 库接口 | `stdlib/text.xiao` 新增纯 Xiao `lower_ascii(str value) -> str`，只转 ASCII `A`–`Z` | 不新增标准库专用 intrinsic；非 ASCII 与非字母原样保留 |

自审核后补的原生回归修复：标准库模块函数现在在模块 lowering 时使用自身源码的类型结果，保留
`int`/`chr` 等契约入口的 intrinsic ID；普通项目模块仍沿用摘要 lowering。该修复消除了首批 `path`
原生测试中“动态函数调用”的误报，不改变标准库接口或转换边界。

本地验证已通过：VM 第七批夹具 3 项通过，Windows LLVM 原生夹具 1 项通过；完整 Rust、clippy、fmt、Bun、
仓库检查与 TypeScript 门禁均已通过。远端工作区门禁与维护回归也已通过，真实 push 取证见 §十一。
`workspace-gates` 的结构修改已独立提交为 `8a7e8ef`；功能实现和库接口随后另行提交，避免把
触发面修复与语言补探混成一笔。R-1 的真实 push 取证已由 `38726737126`（workspace gates）和
`38726737088`（maintenance regression）完成；原生作业逐项记录见 §十一。

## 十一、CI 取证（2026-10-15）

| 工作流/作业 | 运行号 | 实测执行点 |
| --- | --- | --- |
| `Xiao workspace gates` | `38726737126` | Linux 工作区门禁成功 |
| `Xiao maintenance regression` | `38726737088` | 五个作业全成功 |
| Linux 默认测试 | `116394344161` | Rust workspace default tests 成功，包含第七批 VM 回归 |
| Linux 原生差分 | `116394344192` | `first_modules_match_native_results` 通过；`seventh-str-int-and-char` 输出 `65`、`23567`、`38634`、`a`、`😀`、`abcxyz-az-9小` |
| Windows 原生差分 | `116394343742` | `first_modules_match_native_results` 通过；同一第七批夹具逐字输出一致 |

本次维护回归首次因标准库模块 lowering 的旧摘要路径暴露失败（运行 `38715001722`）；修复后上述
`38726737088` 全绿。第七批没有借用失败运行作通过证据。

## 相关页面

- [20C-第六批](20c-sixth-batch-case-conversion-and-api-closeout.md) —— **本批的直接前置**：§四乙 的登记与立项、§3.3 的规模线
- [20C-第五批](20c-fifth-batch-reprobe-and-library.md) —— 缺口登记口径与复探表（`int("A")` 的 `X02-TYPE-006`）
- [20C-第四批](20c-fourth-batch-int-to-str.md) —— **`int` → `str` 十进制文本的冻结出处**（本批回程不能借它的道）
- [20C-第二批](20c-second-batch-probe.md) §3.2 —— **复探口径的唯一出处**
- [20C-门禁残余项](20c-native-gate-residuals-fix.md) §5.2.1 —— 「全绿运行不构成取证」与「本机不算数」
