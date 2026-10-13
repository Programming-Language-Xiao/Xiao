# 20C-首批. 官方库首批模块：`text` 与 `path`

> **背景**：[20C-入口](20c-entry-and-skeleton.md) 已把骨架落地——三处清单与目录**同笔**（`codeRoots` 5、`members` 24、
> 00A 顶层目录 8，`bun run check` 退出 0）、`.xiao` 已纳入 `sourceExtensions` 并登记进 `module-registry.json`、
> 身份块 `StandardLibraryIdentity` 并列落地、依赖求解负例（`X05-PACKAGE-006`）**有执行点**。
> 骨架自己写着：**「在下一批之前不得添加标准库模块代码」**——本批就是那一批。
>
> **一句话概括**：**在冻结的六条与探测边界内写 `text`/`path` 的首批操作**——**先把接口清单冻结，再实现**；
> 并把 P-5 与两处口径订正一起并进来。
>
> 状态：**已实施（2026-10-13）**。接口清单已按星崽裁定冻结，源码、测试和同步元数据已落地；
> 原生 ignored 测试仍需受控工具链运行记录，未把本机缺少的运行号写成通过。

## 一、Agent 交接上下文

### 1.1 接手前提

1. [20C](20c-official-xiao-library.md) **§3.2.1（六条冻结）/ §3.1（探测表）** —— **本批的唯一权威**；
   六条里「首批逻辑模块为 `text` 与 `path`，范围限于已探测操作、均不含新增 intrinsic」是**边界不是建议**；
2. [20C-入口](20c-entry-and-skeleton.md) —— 上一批；骨架、`.xiao` 发现、身份块、依赖负例都在那里；
3. [20. 内置函数与标准库](20-builtins-and-standard-library.md) **§六（增量实施与验收）/ §6.1（测试族）/ §6.2（规模纪律）**
   —— 每个模块要走的那条流水线；
4. `stdlib/README.md` 与 `core/rust/crates/xiao-standard-library/README.md` —— **骨架自己写的边界**
   （不放预编译对象/`.xar` 字段/普通包配置；每模块登记 README、UseDocs、DevDocs 阶段索引、兼容元数据）；
5. [10R-收尾](10r-closeout.md)、[20C-冻结](20c-freeze-and-10-series-closeout.md) §2.2 —— 仍然欠的证据项（§一 表 B）；
6. [20AB](20ab-intrinsics-contract-and-minimal-entry.md) 与 [20AB-FIX](20ab-fix-lifetime-and-removal-verification.md)
   —— **「能编译不算」与移除验证**的教训出处；
7. [00A](00a-project-layout.md) —— 「模块交付必须同时包含实现、测试和 UseDocs」这条 A0 起生效的硬门槛。

### 1.2 现状盘点（2026-10-13，审核实测）

**表 A：库代码的前置是否具备**

| 前置 | 现状 | 判定 |
| --- | --- | --- |
| 10 系列关闭 | 已关闭（2026-10-12） | **具备** |
| 20C 六条冻结 | 已按星崽裁定冻结（模块清单、兼容策略、发布形式、身份、依赖边界、落点） | **具备** |
| 落点骨架与三处清单 | 同笔落地；`bun run check` 退出 0 | **具备** |
| `.xiao` 发现与登记 | `sourceExtensions` 含 `.xiao`；`module-registry.json` 已登记 `stdlib` 条目 | **具备** |
| 依赖求解负例 | `official_standard_library_is_rejected_by_normal_dependency_solver`（直接 + 传递，`X05-PACKAGE-006`）**有执行点** | **具备** |
| **首批接口清单** | 已冻结：`text.contains_char` 与 `path.join`，均为完整签名、纯 Xiao、无新增 intrinsic | **具备**（§三） |

**表 B：仍然欠的证据（并列推进，不阻塞本批）**

| 项 | 卡在哪里 | 归属 |
| --- | --- | --- |
| P-2/F-3 的真实 push 运行号 | 需要一次真推送 | 星崽推送 |
| 10Z-D 的 Linux/macOS 受控复跑与**变异红色证据** | 需要受控机 | 10R 后续 |
| 产物级两次构建（[10R-收尾](10r-closeout.md) §2.5） | 审核已在本机按 CI 的原命令跑通 15E 的 Runtime 产物比较（24 passed / 0 failed）；**10R 指定的 D1/D2 静态数值剖面仍待实测** | 10R-收尾 |
| O-1（19D 是否受影响） | 已判「数据不足」；要判「不受影响」需受控取数 | 19D |

**表 C：探测边界（写库时必须遵守，`20C` §3.1 原表）**

| 事实 | 含义 |
| --- | --- |
| `for c in s` **双路径通过** | 逐字符扫描**必须**用迭代器式 `for` |
| 非字面量下标 **`X03-TYPE-003` 双路径拒绝** | **不许**试图用变量下标写库——那是**迭代器式语言的既定边界**，不是待修缺陷 |
| `int` → `str` **`X02-TYPE-006` 双路径拒绝** | **数字格式化不在首批**；不许为它新增转换 intrinsic |

### 1.3 本批边界

| 子任务 | 内容 | 本批 |
| --- | --- | --- |
| **P-5 修补** | `d19a` 差分判定放宽的登记 | **做，排第一**（§二 2.1） |
| **口径订正** | 权威计数与 `15e_ci_gated` 的执行点表述 | **做**（§二 2.2） |
| 首批**接口清单** | 模块名、函数与签名、错误身份、明确不做的操作 | **已冻结并实施**（§三） |
| `text` / `path` 实现 | 探测边界内的首批操作 | **做**（§四） |
| 测试与元数据 | 双路径差分、错误路径、移除验证、容器边界、UseDocs/阶段索引/兼容元数据 | **做**（§四 4.2） |
| 表 B 的欠项 | —— | **并列推进**，不阻塞 |
| 静态 `catch`/`finally` 清理链、数字格式化、集合模块 | —— | **不做**（D7 / 未探测 / 未探测） |

## 二、并进来的修补（先做）

### 2.1 **P-5：`d19a` 的差分判定被放宽了，只写在代码注释里**

**事实**（审核逐行核 `d19a_differential.rs` 的 `compare_native_cases`）：

```text
改前：Err(NotFound) if !native.module.uses_runtime && vm_tuples.is_empty() => Vec::new()
改后：Err(NotFound) if (!native.module.uses_runtime || !case.has_drops) && vm_tuples.is_empty() => Vec::new()
```

**方向我判为对**：旧判据 `!uses_runtime` 是「静态产物不链接 Runtime」的**代理**；而
[10R-实现](10r-static-overflow-implementation.md) **D2 之后静态产物也会链接 Runtime**（`ir.rs:679` 置 `uses_runtime = true`）
⇒ 代理失效、测试会**误红**。新判据 `!has_drops`（该用例没有堆生命周期）**语义上更准**。

**但它改的是差分豁免的接受条件**，而本仓库的惯例是这类判定改动要在
[10P](10p-differential-exemption-and-coverage.md) / [19D](19d-performance-comparison.md) **登记**（「加列不抹旧判定」）；
该登记已在本批补齐。

**冻结**：

1. 补一行登记：**改了什么 / 为什么（D2 让旧代理失效）/ 残留语义**；
2. **残留语义要写明**：对 `has_drops == false` 的用例，「轨迹文件缺失」与「轨迹为空」**不再区分** ——
   可接受（输出另有逐项比对，完全没跑会被别的断言抓到），但**必须说出来**，不许留成暗坑；
3. **不得**借此顺带放宽别的条件（本次只动了这一处 `if`）；
4. 若后续再改豁免面，**同一批内登记**，不许只改代码。

### 2.2 **两处口径订正（审核自己的账）**

**事实**：

1. **权威计数不对**：[20C-入口](20c-entry-and-skeleton.md) 表 B 的「权威（11 个）」是审核**只扫 `crates/*/tests/*.rs`** 得出的，
   而 `#[ignore]` 也存在于 `src/` 内嵌单元测试里。**实际是 13 个文件 / 30 个 `#[ignore]` 用例**
   （漏的两个文件：`xiao-driver/src/diagnostics.rs`、`xiao-runtime-abi/src/lib.rs`）；
2. **表述不够准确**：说 `15e_ci_gated`「没有执行点」不准——它**有周定时执行点**
   （`reproduce.sh:132` 的 `--workspace -- --ignored`），**缺的是 push 门**；这正是它选择**补 `XIAO_STRIP`**
   而不是排除的依据。

**冻结**：

1. 把这两处**就地订正**（表 B 加一行「审核订正」，并注明「以 glob 口径为准，枚举计数只作历史基线」）；
2. 订正的意义**不只在这两个数字**：它再次说明**枚举口径不可靠**——凡「哪些目标/文件/成员受门禁覆盖」，
   **一律用 glob 或 `--workspace` 表达，枚举只作对账记录**；
3. 后续任何「权威 N / 清单 M / 差集」的对账，**必须写明扫描范围**（`tests/` 还是含 `src/`），
   否则数字本身会被误读。

## 三、首批接口清单（**实现前必须冻结**）

### 3.1 **范围由探测决定，不由「想写什么」决定**

**冻结**：

1. 首批只列 **`text` 与 `path`** 两个逻辑模块；**每个拟列出的操作都要能在 §一 表 C 的探测形状里找到对应**——
   找不到就先**补探测**（[10R-实现](10r-static-overflow-implementation.md) 的 D5 那套做法：真写、真跑、双路径、成表），
   **不许**先写实现再补探测；
2. **明确不做**：数字格式化（`X02-TYPE-006`）、集合模块（未探测）、任何**新增 intrinsic**
   （方向稿 §6.2 的规模纪律：普通算法优先 Xiao 实现）；
3. **逐字符扫描一律用 `for c in s`**——**不许**用变量下标绕（那是 `X03-TYPE-003` 的既定边界，见 G-5 的教训）；
4. 清单要**写清不做的部分**（例如「`text` 首批不含切分/查找/大小写转换」），
   免得后来人以为「没写就是还没做」。

### 3.2 **清单要能直接照着实现**

**冻结结果（星崽裁定）**：

| 模块 | 函数与完整签名 | 返回与错误身份 | 实现与探测依赖 |
| --- | --- | --- | --- |
| `text` | `contains_char(str text, str needle) -> bool` | 返回布尔值；参数类型错误由现有前端稳定诊断，成功路径不新增运行时错误 | 纯 Xiao；依赖 `for c in s`、字符比较、条件和布尔赋值 |
| `path` | `join(str base, str leaf) -> str` | 返回字符串；参数类型错误由现有前端稳定诊断，成功路径不新增运行时错误 | 纯 Xiao；依赖已探测的字符串拼接 |

本批明确不做：数字格式化（`X02-TYPE-006`）、变量下标/切片（`X03-TYPE-003`）、切分、子串查找、
大小写转换、路径规范化、绝对路径、集合和任何新增 intrinsic。模块粒度冻结为单文件
`stdlib/text.xiao` 与 `stdlib/path.xiao`。

## 四、实现与验收要求

### 4.1 **源码即规范实现**

**冻结**：

1. 模块写成 `stdlib/` 下的 `.xiao` 源码（**发布形式已冻结为源码**）；预编译对象只允许作为**本地/构建缓存**，
   **不作发布身份**、**不作为普通包发布**；
2. 模块**不通过普通依赖解析**——由工具链专用加载器按固定命名空间提供（依赖求解负例已证明普通解析会拒绝）；
3. **不许**写 `.xar` 兼容字段（归档接线归 20E）。

### 4.2 **每个模块的「四件套」验收（缺一不算完成）**

取自方向稿 §6.1，逐条可查：

1. **双路径差分**：同一模块在 VM 与 LLVM 原生上的**结果、错误身份、释放顺序**一致；
   **LLVM 尚不可用时不许伪造通过**（写「数据不足」）；
2. **移除验证**：删掉模块、绑定或包装里的**任一**环节，专门用例必须失败——
   **只测「能编译」不算**（20AB 的教训、20AB-FIX 才补上）；
3. **容器边界与兜底文本**：模块的值不进集合/字典键/常量池/序列化对象/`.xar`；
   类型名与错误参数**不出现 `dynamic` 等兜底文本**；
4. **同步**：模块级 README、`stdlib/README.md`、crate README、UseDocs、DevDocs 阶段索引、
   `docs/module-registry.json` 的路径登记、以及**兼容元数据**——
   这条是 [00A](00a-project-layout.md) 从 A0 起的硬门槛，且**已由仓库检查器校验**（N-2 的机制）。

### 4.3 **兼容身份怎么动**

**冻结**：

1. **增接口 = 兼容**；**删接口、改签名、改错误身份、改释放顺序 = 破坏**，必须提升
   `STDLIB_INTERFACE_VERSION`（现为 `1`，定义在 `xiao-driver/src/compatibility.rs:187`）并给稳定拒绝诊断；
2. 身份字段（`StandardLibraryIdentity`）里的 **Runtime ABI 主/次直接取 `xiao-runtime-abi` 的常量**——
   **不许**在别处再写一份字面量（现为 **1.12**）；
3. **首版不设迁移别名窗口**（已冻结）。

### 4.4 **不许为写库扩语言**

**冻结**：

1. 实现中若发现需要的语言能力不在此前的探测形状内（例如字符串比较/切分/切片），
   **停下来登记为语言缺口**（写清：需要什么、当前以什么码被拒、阻塞哪个模块的哪条接口），
   **不在本批修**——这正是 §3.1 探测存在的意义；
2. **不许**为了让库跑起来而：绕开类型系统、加临时 intrinsic、把模块逻辑下沉到 Rust；
3. **不许**新增 intrinsic 来替代 Xiao 能表达的实现（规模纪律）。

### 4.5 **门禁与执行点**

**冻结**：新增的差分/移除验证用例**必须有 push 执行点**——本仓库已在这一类上栽过多次
（G-1、P-2、P-3、`tests/fixtures/**`）。新用例若需要原生工具链，就走**已改成 glob 口径**的两处原生作业；
若不需要，确认它进了 `workspace-default-tests` 覆盖的范围。**不许**出现「新加了一条关键用例但没人跑它」。

## 五、落点

```text
stdlib/text.xiao、stdlib/path.xiao                  §四 4.1：模块源码（.xiao）+ 模块级 README
docs/DevDocs/20c-first-modules.md                   本文（含 §三 冻结的接口清单）
docs/DevDocs/20c-official-xiao-library.md           §3.2.1 回写「首批接口清单已冻结」与实施状态
docs/DevDocs/10p-differential-exemption-and-coverage.md、19d-performance-comparison.md   §2.1 P-5 登记
docs/DevDocs/20c-entry-and-skeleton.md              §2.2 两处口径订正
docs/module-registry.json、stdlib/README.md、crate README   §四 4.2 第 4 条：路径与元数据同步
docs/UseDocs/library/…                              §四 4.2 第 4 条：UseDocs（A0 起硬门槛）
core/rust/crates/xiao-driver/src/compatibility.rs   §四 4.3：仅在破坏性改动时提升 STDLIB_INTERFACE_VERSION
```

## 六、验收

1. **P-5**：差分判定的放宽已在 10P/19D 登记（含残留语义）；**没有**顺带放宽其它条件；
2. **口径订正**：表 B 有「审核订正」行（13 文件 / 30 用例；`15e_ci_gated` 有周定时执行点、缺 push 门）；
   后续对账都写明扫描范围；
3. **接口清单**：实现前已冻结，逐条含签名/返回/错误身份/依赖的探测形状；
   **每条都能在表 C 的探测里找到对应**，找不到的已补探测或明确不做；
4. **四件套**：每模块的双路径差分、移除验证、容器边界与兜底文本、元数据/UseDocs 同步齐全；
   差分有**原生侧实跑**；无法测的写「数据不足」；
5. **兼容身份**：若有破坏性改动，`STDLIB_INTERFACE_VERSION` 已提升且有稳定拒绝诊断；
   **ABI 主/次仍来自常量**；
6. **不许扩语言**：语言缺口只登记、未实现；**没有**新增 intrinsic、**没有**把逻辑下沉 Rust；
7. **执行点**：新增关键用例都能说清「push 时谁跑它」；
8. **欠项如实**：表 B 四项与事实一致，**没有**被写成已完成；
9. **通用**：`cargo test --workspace`、clippy、fmt、`bun test`、`bun run check`、`bunx tsc --noEmit` 全绿；
   提交标题带类型前缀、正文非空。

## 七、裁定结果与实施记录

1. **接口范围**：按星崽裁定冻结 `text.contains_char(str text, str needle) -> bool` 与
   `path.join(str base, str leaf) -> str`；两者均为纯 Xiao、无新增 intrinsic。
2. **模块粒度**：按星崽裁定采用单文件 `stdlib/text.xiao`、`stdlib/path.xiao`，各自配模块 README、
   UseDocs 和测试登记。
3. **补探测策略**：首批不补探测；未被表 C 覆盖的数字格式化、变量下标、切分、切片、大小写转换、
   路径规范化、集合和新增 intrinsic 均登记为后续范围。
4. **工具侧门禁**：沿用已有 `A0-LAYOUT-005` 与 `workspace-default-tests`；本批不新增工具侧规则。

## 八、实施记录（2026-10-13）

| 模块 | VM | LLVM 原生 | 移除验证 | 元数据与边界 |
| --- | --- | --- | --- | --- |
| `text` | `text_module_vm_result_and_removal_boundary_are_stable` 通过，输出 `ok` | `first_modules_match_native_results` 已加入受控 ignored 门禁，当前等待工具链运行号 | 去掉实现后调用者前端失败；错误参数类型前端拒绝 | `text.xiao`、`text.md`、UseDocs、入口测试和注册项已同步；源码无容器/序列化/`dynamic` 兜底 |
| `path` | `path_module_vm_result_and_removal_boundary_are_stable` 通过，输出 `lib/core` | 同上，使用同一份源码和调用者 | 去掉实现后调用者前端失败；错误参数类型前端拒绝 | `path.xiao`、`path.md`、UseDocs、入口测试和注册项已同步；源码无容器/序列化/`dynamic` 兜底 |

VM 测试命令为 `cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20c_first_modules`。
原生测试由 Windows/Linux 两处 `--workspace -- --ignored` 作业执行；本地未设置受控工具链时只保留 ignored，
不把数据不足写成原生通过。标准库接口版本仍为 1，Runtime ABI 主/次仍从 `xiao-runtime-abi` 常量读取，
没有改 `.xar` 字段，也没有加入普通依赖图。

## 相关页面

- [20C. 官方 Xiao 库：能力探测、模块清单与兼容身份](20c-official-xiao-library.md) —— §3.2.1 六条冻结与 §3.1 探测边界（本批的唯一权威）
- [20C-入口. 20C 入口准备：骨架、清单同笔与首批模块的前置](20c-entry-and-skeleton.md) —— 上一批；骨架、`.xiao` 发现、身份块、依赖负例
- [20C-冻结. 10 系列关闭与 20C 六条冻结](20c-freeze-and-10-series-closeout.md) —— 六条的裁定依据与落点守卫
- [20. 内置函数与标准库](20-builtins-and-standard-library.md) §六 —— 每模块的生产流水线与测试族
- [20AB-FIX. 生命周期诊断与移除验证](20ab-fix-lifetime-and-removal-verification.md) —— 「能编译不算」的出处
- [10R-收尾. 静态数值路径的收尾、证据接线与 20C 入口准备](10r-closeout.md) —— 仍欠的证据项
- [00A. 工程框架与目录布局](00a-project-layout.md) —— 模块交付必须含实现、测试与 UseDocs
