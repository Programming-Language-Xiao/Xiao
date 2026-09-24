# 11A-D1. 外部包契约与依赖图骨架

> **实现状态：已完成（2026-09-24）。** 本文的冻结边界、规格夹具和真实 Rust 加载入口
> 已落地；缓存、锁文件、远程源和 CLI 仍明确留给后续批次。

> **这是 11A 的第一批。** 退出条件四条（`12-tests-and-milestones.md:724-731`）：
> ① `config.xiao` 能声明直接外部依赖及版本/来源约束；② 依赖解析器读取本地路径包配置、
> 在内存中构建完整依赖图；③ 包身份冲突/缺失依赖/依赖环有稳定诊断；
> ④ **只建立包契约和解析边界，不要求缓存、锁文件或远程下载**。
>
> **第 4 条是本批的边界**——它把 D1 与 E1 切开：E1 才要缓存。

## 一、Agent 交接上下文

### 接手前提

1. [11A. 虚拟环境与包管理](11a-environments-and-packages.md) —— 阶段方向稿。
   **⚠️ 但它没有 D1 章节**（见 §二）。
2. [12. 测试与开发里程碑](12-tests-and-milestones.md) **`:724-731`** —— **D1 的权威描述只在这里**。
   相邻的 `:733`（E0）、`:742`（E1）、`:751`（E2）给出前后批次的边界。
3. [05D. `config.xiao` 声明式配置静态闭环](05d-config-static-closure.md) **`:103`**
   —— 「11/11A 接手时**直接消费 `ConfigDocument`/`NormalizedConfig`，不得重新扫描文本**」。
   这是 D1 条件 1 的**硬前置**。
4. [11A.1. 包源协议审核](11a1-package-source-protocol-review.md) —— 对 D1 的三条约束（见 §四.3）。
5. [00. 决策基线](00-decisions.md) **`:258-298`** —— 包管理的 39 条冻结项。
   **本批只消费其中与「包契约」相关的**，多源/联邦/索引那些属 E3。
6. [00A. 工程框架与目录布局](00a-project-layout.md) —— 登记要求与依赖方向（**注意 §五.2 的缺口**）。

### 现状盘点（2026-09-24 实测，D1 实现前）

```text
xiao-package      core/rust/crates/xiao-package/src/lib.rs 只有 1 行 //! 注释
                  module-registry.json:36 登记为 planned / stage 11A
                  Cargo.toml 无任何 [dependencies] 段
config.xiao       xiao-config 能解析 [dependencies]/[devdependencies]/[toolchain]/[sources]
                  （在 RESERVED_TABLES 里），但走 `_ => {}` 空分支，**无语义、零测试**
依赖图            xiao-modules 有项目内 ModuleGraph（文件/命名空间粒度），
                  **不含包名/版本/来源**；discovery.rs:76-80 遇到嵌套 config.xiao 直接 return
solver/lockfile   全仓零命中（venv / lockfile / content_address / sha256 均 0 处）
CLI               parser.ts 无 venv/sync/install 分支；
                  cli/ts/src/{environments,packages}/ 只有 README
```

---

## 二、⚠️ 第一件事：D1 在 `11a` 里**没有章节**

`11a` 的「接手前提」（`:9`）明写「先阅读 …… `12` 的 **D1**/E0–E2 条目」——
**但它的「二级实现任务」从 E0 直接开始**（`:380`），**没有 D1**。

结果是：**11A 的权威设计文档没有描述自己的第一个实现批次**，
D1 的全部内容只存在于 `12-tests-and-milestones.md:724-731` 的四行退出条件。

**本批要做的第一件事，就是消除这个缺口**——二选一：

- **(a)** 在 `11a` 补一个 D1 章节（与 E0–E3D 并列），把 §三/§四 的结论落进去；
- **(b)** 明确声明 D1 的权威位置在 `12-tests-and-milestones.md`，并在 `11a:9` 改成
  「D1 的权威描述见 `12`」+ 一句为什么。

**在 (a)/(b) 完成前，任何接手者都会按 `11a:9` 去 `11a` 找 D1 而找不到。**

**本批已采用方案 (a)**：`11a` 现在包含 D1 权威章节，本文保留这段历史问题作为本批
设计动因，不再要求后续接手者从 `12` 的四行退出条件反推实现边界。

---

## 三、D1 四条件的实现基础（**决定工作量**）

| # | 条件 | 现状 | 判定 |
| --- | --- | --- | --- |
| **1** | `config.xiao` 声明外部依赖及版本/来源约束 | `dependencies`/`devdependencies`/`toolchain`/`sources` **已在保留表内、能解析通过**，值以 `ConfigValue::Dictionary` 原样保存；`ConfigValue` 已支持字符串/数值/布尔/递归数组/字典 | **半成品**：缺字段白名单与类型校验、缺约束语法冻结、**零测试零夹具** |
| **2** | 依赖解析器读取路径包配置、构建依赖图 | **全新**。`xiao-package` 是空壳；全仓无 solver | **全新** |
| **3** | 包身份冲突/缺失/环的稳定诊断 | **全新**。最接近的 `X05-MODULE-003/004/007` 是**文件粒度**，语义与粒度都不同 | **全新** |
| **4** | 不要求缓存/锁文件/远程 | 约束性条款 | 无工作量，**是设计约束** |

**所以重心是条件 2 与 3**：一个新 crate 的骨架 + 一套新诊断。
条件 1 是「给保留表补语义」，条件 4 划边界。

**可复用的只有**：`ConfigDocument` 的读取 API（`document.table("dependencies")`）
与 `ModuleGraph` 的**确定性形状**（`BTreeMap` + 拓扑序的工程习惯）。

⚠️ **不要扩展现有 `ModuleGraph` 来承载包身份**——那会污染 05 阶段已验证的契约
（文件/命名空间粒度）。包图是**另一个图**。

---

## 四、**必须先冻结的输入**（否则实现会先于设计）

### 4.1 `11a` 留下的 8 项「仍待定」，有 8 项落在 D1/E0 范围

`11a:52` 与 `:58` 自陈「仍待定」的项里，**直接落在 D1/E0 范围内**的有：

- 多环境并存时 `sync` 的目标选择顺序；
- 钩子初始化命令、取消激活命令；
- 不支持钩子的 Shell 的行为；
- 找不到 `config.xiao` 时的脚本模式行为；
- 全局环境的**实际路径**；
- 版本冲突处理；
- 全局环境是否参与**非项目脚本**的导入解析。

**其中「版本冲突处理」与「全局环境参与解析」直接约束 D1 的图模型**。
**本批要么冻结它们，要么显式声明「D1 阶段不涉及，留到 E0/E1」并写明理由。**

### 4.2 D1 条件 1 的**约束语法本身**仍未冻结

`11a1:57-59` 的「仍未决断」明列「**依赖约束表达**」。
而 D1 条件 1 要求「版本/来源约束」。

**所以 D1 必须在自己的范围内冻结一个最小约束语法**（不必是完整的 semver 方案）：

- `11a:120-127` 给了**候选**（`http = "^1.4"` / `utils = { path = "../utils" }`），
  但明说是「候选分组，并非已冻结表名」；
- **D1 只需覆盖本地路径包**（条件 4 排除了远程），所以最小集合是
  「`path = "..."` + 可选的版本约束占位」。

⚠️ **`11a:204-212` 的多源示例用了 `[PackageSources]`，而 `packagesources` 不在
`RESERVED_TABLES` 里**——按现在的 `xiao-config`，它会报 `X05-CONFIG-006` 未知顶层表。
（`11a:214` 自陈表名待冻结，缓解但不消除。）**本批顺手把这个示例改准，或注明待冻结。**

### 4.3 `11a1` 对 D1 的三条约束

1. **`source_id` / `alias` / 显示名三个概念必须从一开始就分开**
   （`00-decisions:289` 冻结）。D1 的数据模型**必须预留这三者的形状**，
   否则 E3A 无法在不破坏契约的前提下插入；
2. **不得把 `name` 当引用键**（`11a1:36-37` 的判据）；
3. **D1 只做 source_id 概念的形状预留，不实现源协议**
   （`11a1:22` 冻结的是远程/多源维度）。

---

## 五、落点与依赖方向

### 5.1 落在 `xiao-package`（已存在）

`core/rust/crates/xiao-package` 的 crate README 已写好职责边界：

> 提供多包源协议、联邦源索引、依赖求解、来源优先级、锁文件和环境物化的核心逻辑。
> 显式源优先，其次按配置顺序，无法唯一确定时报告歧义。
> **禁止**：不执行包安装脚本或包代码，不绘制提示符，**不把不同源的版本简单按高低混选**。

工程期：**11A 建核心逻辑 → 16 接内容寻址缓存 → 18 由 CLI 调用**。
`module-registry.json` 已登记为 `11A-D1/verified`，workspace 已收，**不需要新建 crate**。

### 5.2 依赖方向图已补齐

原先 `00a-project-layout.md:57-72` 的依赖方向图画了 `xiao-artifacts` / `xiao-xar` /
`xiao-platform`，唯独遗漏 `xiao-package`；同一文档的 crate 分配表却已经登记了它。

本批已按下列方向补进图中，作为新增依赖（`xiao-package → xiao-config`）的冻结依据：

```text
xiao-source / xiao-diagnostics
        ↓
xiao-syntax → xiao-config ─┐
        ↓                   ├→ xiao-package → xiao-driver (protocol)
xiao-modules ──────────────┘
```

即 `xiao-package` 依赖 `xiao-config`（读包配置）+ `xiao-modules`（模块身份/图形状）。

### 5.3 CLI 侧的落点

命令路由在 `cli/ts/src/commands/`（`00a:121` 已列 `venv`/`sync`/`install`）；
`environments/` 放 Shell 钩子与提示符前缀、`packages/` 放命令交互与进度展示——
**两者的「不应放入」都写着「不实现依赖求解」**（`00a:124-125`）。

⚠️ **本批大概率不碰 CLI**（D1 是核心逻辑批次）。若要碰，注意 §六.2 的门禁。

---

## 六、硬约束

### 6.1 单一来源：`config.xiao` 只解析一次

`05d:103` 明令：「11/11A 接手时**直接消费 `ConfigDocument`/`NormalizedConfig`，
不得重新扫描文本**」。**包管理器不拥有自己的配置解析器。**

### 6.2 ⚠️ `A0-LAYOUT-002` 会**立即**拦住新目录

`cli/ts/src/environments/` 与 `packages/` 现在**只有 README，所以不报错**——
但门禁的 `discoverSourceDirectories` 只把「含 `.rs`/`.ts`/`.tsx` 的目录」当源目录。
**一旦放入第一个 `.ts` 文件，`A0-LAYOUT-002` 立刻报「未登记」**，
除非把路径加进 `module-registry.json`（新增条目或并入 `ts.xiao-cli` 的 `code` 数组）。

Rust 侧同理：`xiao-package/src/` 下新增文件要同步 `module-registry.json:36` 的
`code` 数组，**测试文件同步 `tests` 数组**。

### 6.3 不执行包代码

`00-decisions:288`：**首个远程闭环禁止安装前后脚本；元数据解析与安装都不得执行包代码**。
本批虽然只做本地路径包，但**读取包配置也必须走 `xiao-config` 的静态解析**，
不得执行任何 `.xiao` 代码。`xiao-package` 的「禁止事项」第一条也是这条。

### 6.4 诊断编号要定

现有的 `X05-MODULE-*` 是**模块粒度**（缺目标/循环/冲突），语义不同。
本批需要**包粒度**的稳定编号——是复用 `X05-MODULE-*` 还是新开 `X05-PACKAGE-*`，
**必须在文档里写明理由**，不能随手挑。

### 6.5 新增 spec 目录要**有执行入口**

`11x0spec` 立下的判据：「**真正的门槛不是建目录，而是有执行入口**」。
而且**门禁已经能抓了**（`A0-DOCS-003` / `a0.spec.loader_missing`）——
新增 `tests/spec/11a-*/` 会被自动检查。

---

## 七、分步提交

| 步 | 内容 | 为什么这个顺序 |
| --- | --- | --- |
| **1** | **补 `11a` 的 D1 章节**（或声明权威位置）+ 冻结 §四的输入 | §二：不补就没有权威设计 |
| **2** | 补 `00a-project-layout.md` 的依赖方向图（§五.2） | 不补，新增依赖没有依据 |
| **3** | `xiao-config` 的依赖表字段语义 + 约束语法（条件 1） | 半成品，先补语义 |
| **4** | `xiao-package` 的包身份与图模型（条件 2 的一半） | 先有模型 |
| **5** | 依赖解析器 + 诊断（条件 2/3） | 最重的一块 |
| **6** | 文档与登记收尾 | `module-registry.json`、README、UseDocs |

**第 1、2 步是「先冻结再实现」**——本批的前两条是文档，不是代码。
这与前面几批（先写交接文档再实现）是同一个道理。

---

## 八、最可能翻车的地方

1. **跳过 §二 直接写代码**——那样 11A 会有一个**没有设计章节的实现**，
   而 `11a:9` 还指着它。
2. **扩展现有 `ModuleGraph` 承载包身份**（§三末尾）——污染 05 已验证的契约。
3. **重新扫描 `config.xiao` 文本**（§六.1）——违反单一来源。
4. **在一个未登记的目录里放第一个源码文件**（§六.2）——门禁会拦，
   但拦在提交时不如提前知道。
5. **把 `source_id`/`alias`/显示名混成一个字段**（§四.3）——E3A 会无法插入。
6. **约束语法随手定为完整 semver**——`11a1:57` 明说它「仍未决断」，
   本批只该冻结**最小集**（本地路径 + 版本占位）。
7. **为 D1 写缓存/锁文件**（条件 4 明确排除）——那是 E1/E2。
8. **诊断编号随手挑**（§六.4）。

---

## 九、验收

1. **`11a` 有 D1 章节或明确的权威位置声明**（§二）；
2. **`00a-project-layout.md` 的依赖方向图含 `xiao-package`**（§五.2）；
3. **D1 条件 1**：`config.xiao` 能声明路径依赖，字段有白名单与类型校验，
   且**有 `tests/spec` 夹具 + 真实加载者**；
4. **D1 条件 2**：解析器能读取本地路径包配置并构建内存依赖图；
5. **D1 条件 3**：包身份冲突/缺失/环三条诊断各有正反例测试；
6. **未实现缓存、锁文件、远程下载**（条件 4）；
7. **`source_id`/`alias`/显示名三概念在模型里分开**（§四.3）；
8. **`module-registry.json` 同步**，`xiao-package` 的 `code`/`tests` 数组更新；
9. **门禁全绿**，含 `check:lock`、`bunx tsc`。

---

## 十、不负责与不要重复做的事

- **不做缓存、锁文件、`sync`/`install`**——归 E1/E2。
- **不做远程源、联邦索引、GitHub 适配器**——归 E3A–E3D。
- **不做 `xiao venv`**——归 E0。
- **不碰 CLI 命令实现**（本批是核心逻辑批次，§五.3）。
- **不执行任何包代码**（§六.3）。
- **不动 `xiao-modules` 的文件粒度 `ModuleGraph` 契约**（§三末尾）。

---

## 十一、实现收口（2026-09-24）

本批已按前述顺序完成：

1. `11a-environments-and-packages.md` 补入 D1 权威章节，`00a-project-layout.md` 补入
   `xiao-config`/`xiao-modules` → `xiao-package` 的依赖方向。
2. `xiao-config` 增加本地路径依赖字段白名单、类型/路径诊断和结构化声明提取；配置表
   只消费静态 `ConfigDocument`，不重新扫描文本。
3. `xiao-package` 增加独立 `PackageGraph`、`source_id`/`alias`/`display_name` 分层、
   本地 `config.xiao` 递归读取、依赖优先顺序和 `X05-PACKAGE-001` 至 `003` 稳定诊断。
4. `tests/spec/11a-package/` 的正反例由
   `core/rust/crates/xiao-package/tests/d1_package.rs` 真实写入隔离项目并执行；没有实现
   缓存、锁文件、远程下载、安装脚本或 CLI 包命令。

目标验证包括 `cargo test -p xiao-config -p xiao-package`、格式/工作区检查、规格门禁、
文档覆盖率和锁文件检查；提交前必须保持 `module-registry.json` 与新增源码/夹具同步。

---

## 相关页面

- [11A. 虚拟环境与包管理](11a-environments-and-packages.md) —— 阶段方向稿及已补入的 D1 章节
- [12. 测试与开发里程碑](12-tests-and-milestones.md) `:724-731` —— D1 退出条件基线
- [05D. `config.xiao` 声明式配置静态闭环](05d-config-static-closure.md) —— 硬前置，`:103` 的消费约定
- [11A.1. 包源协议审核](11a1-package-source-protocol-review.md) —— 对 D1 的三条约束
- [00. 决策基线](00-decisions.md) `:258-298` —— 39 条冻结项
- [00A. 工程框架与目录布局](00a-project-layout.md) —— 登记要求与已补齐的 `xiao-package` 依赖方向图
