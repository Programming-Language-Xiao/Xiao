# 20O. 官方库工程总纲：目录、优先级、依赖与 Agent 交接

> **这是一期独立工程的规划入口，不是实现提交。**
> 20C–20N 已交付 `text`/`path`/`set`/`dict`/`os`、虚拟 `std.concurrent` 和扩展路线；
> 本文件把“官方库要做什么、按什么顺序做、每批的边界和证据是什么”写成可交接的总纲，
> 并对数值计算做专项研究。新接手的 Agent 必须从本文件开始读。
>
> 状态：**规划中（2026-11-01）**。基线为 `6dc0773` 之后的仓库状态（开工时以
> `git log`/`git status` 为准）。本文件只做研究和拆分，不写产品函数体、不新增 intrinsic、
> 不改变 ABI 1.17。**已确认（2026-11-01）：D-1 取甲（成熟 Rust 数值库）、B1 首批范围、
> D-7 取专用 `json_value` 类型、B7→B8→B9 后置**。B0 已逐条询问并记录其余方向裁定，
> 见 §五及 20P §六；具体 API 不因此自动冻结。基线、探测、教程与待审草案见
> [20O-B0](20o-b0-baseline-and-b1-draft.md)。

## 一、工程目标与边界

### 1.1 目标

1. 给出一份**完整候选目录**：已交付模块、待建模块、明确不做的模块；
2. 给出每个模块的**前置层级**：纯 Xiao、简单 intrinsic/ABI、语言值模型、原生库封装、
   网络能力、native extension；
3. 按“开发者第一天就会用”和“依赖深度/安全风险”确定批次顺序；
4. 为数值计算（math/cmath/decimal/fractions/statistics/random）做专项研究和冻结建议；
5. 每个库都必须有 UseDocs 教程和可执行示例；模板、示例测试和数据库驱动架构见
   [20P. Day-1 官方库架构](20p-day1-library-architecture.md)；
6. 把新 Agent 需要的上下文、命令、门禁、陷阱和交接步骤写全。

### 1.2 不负责

- 不实现任何库函数、intrinsic、ABI 或 CI 作业；
- 不改变 20A/20C 的模块冻结、接口版本、能力位或归档身份；
- 不认领 19 发布收口、11C L4、11B I5、22E 登记差异；
- 不在没有探测/冻结的情况下写 JSON、TOML、crypto、HTTP 或数据库代码。

### 1.3 当前基线（开工时复核）

| 项 | 事实 |
| --- | --- |
| 已交付 stdlib | `text` 11 项、`path` 4 项、`set`、`dict`、`os`（读/目录/进程）、虚拟 `std.concurrent` |
| 标准库接口版本 | 1 |
| Runtime ABI | 1.17 |
| 语言数值 | `int`/`sint`/`float`/`sfloat` 已实现；`lint`/`lfloat` 是不限位宽文本表示，但算术/转换仍不完整 |
| 容器 | 公开同质注解 `[T]`、`dict<str,T>`、`set<T>` 已有；内部异构容器不等于公开动态/嵌套签名已冻结 |
| 动态值 | 类型层内部有 `dynamic` 退化，但公开注解/边界未冻结 |
| 宿主能力 | 能力位 `filesystem/clock/process/network/native_extension/stdout/stdin` 已存在；没有 time/argv/网络/文件写入 intrinsic |
| 并发 | 22 已完成；没有 async/await、Actor 或官方并发容器 |
| 官方库边界 | 不作为普通包进入依赖求解；`.xar` 只记录模块和身份 |
| 发布债务 | 19 发布收口、11C L4、11B I5、22E 层次差异仍各自未闭合 |

## 二、官方库候选目录

优先级：**P0** 是开发者第一天最可能用的基础库；**P1** 是常用数据/工具；**P2** 是需要
安全或网络前置的库；**P3** 是大体量/生态型或暂不承诺的库。

### 2.1 已交付

| 模块 | 状态 | 备注 |
| --- | --- | --- |
| `text`、`path` | P0，已交付 | 20C/20M；纯 Xiao |
| `set`、`dict` | P0，已交付 | 20F–20L；含方法式 API |
| `os` 读/目录/进程 | P1，已交付 | 20D；写入/枚举/元数据未交付 |
| `std.concurrent` | P0，已交付 | 22；虚拟模块，无 `.xiao` 文件 |

### 2.2 数值计算

| 模块 | 优先级 | 层 | 前提 |
| --- | --- | --- | --- |
| `math`（整数） | P0 | 纯 Xiao | 无 |
| `cmath`（浮点） | P1 | intrinsic + ABI | 浮点原语、NaN/Inf/舍入策略 |
| `decimal` | P1 | 语言/运行时数值基础 | `lint`/`lfloat` 算术、精度/舍入 |
| `fractions` | P1 | 语言/运行时数值基础 | 大整数、约分、溢出 |
| `statistics` | P1 | 纯 Xiao（依赖数值） | 数组、浮点/整数策略、NaN |
| `random`（确定性 PRNG） | P0/P1 | 纯 Xiao 或运行时 | 种子、可复现、线程局部状态 |
| `random`（CSPRNG） | P2 | 原生库 + 能力 | 安全随机、能力位、审计 |
| `bigint`/任意精度 | P1 | 语言/运行时 | 与 `lint` 统一，不能两套 |
| `complex`/矩阵/线代 | P3 | 数值类型或纯 Xiao | 需求与数值语义未冻结 |

### 2.3 时间、数据与文本

| 模块 | 优先级 | 层 | 前提 |
| --- | --- | --- | --- |
| `time`（clock + sleep） | P0 | intrinsic + ABI | `clock` 能力；UTC 首批，D-8 已确认加入 sleep，取消语义待冻 |
| `datetime`（本地时区/解析） | P2 | 数据表 + 时区策略 | 时区数据库/打包 |
| `json` | P0 | 纯 Xiao | JSON 值模型 |
| `toml` | P1 | 纯 Xiao | 同一值模型 |
| `csv` | P1 | 纯 Xiao | 字符串/数组 |
| `yaml` | P3 | 纯 Xiao/原生 | 规范复杂度高，暂不承诺 |
| `url`、percent-encoding | P0 | 纯 Xiao | 无 |
| `base64`、`hex` | P0 | 纯 Xiao | 无 |
| `regex` | P1 | 原生库封装 | 线性时间引擎、限制 |
| `fmt`/格式化 | P0 | 纯 Xiao | 数字/宽度/对齐语义 |
| Unicode 分类/大小写 | P3 | 数据表/原生 | Unicode 数据版本和体积 |

### 2.4 系统、安全与网络

| 模块 | 优先级 | 层 | 前提 |
| --- | --- | --- | --- |
| `fs` 扩展（写/枚举/删除/重命名/元数据） | P0 | intrinsic + ABI | 路径/符号链接/原子写策略 |
| `env`（环境变量） | P1 | intrinsic + ABI | 能力与敏感值策略 |
| `args`/`cli` | P0 | intrinsic + 纯 Xiao | argv 契约 |
| `log` | P0/P1 | 纯 Xiao（stdout） | 时间戳/文件 sink 依赖 time/fs |
| `testing`/断言 | P1 | 纯 Xiao | 失败报告与退出码 |
| `compression`（gzip/zlib） | P1 | 原生库 | 解压上限、流式策略 |
| `archive`（zip/tar） | P3 | 原生库/纯 Xiao | 路径穿越与炸弹防护 |
| `hash`/`hmac` | P1/P2 | 原生库 + 审计 | 算法清单、常量时间 |
| `crypto` CSPRNG/AEAD/TLS | P2 | 原生库 + 审计 | 能力位、秘密生命周期 |
| TCP/UDP/DNS | P2 | intrinsic + 网络能力 | 连接/超时/取消 |
| `http` client | P2 | 原生库/网络 | TLS、超时、大小上限 |
| `http` server | P2 | 原生库/网络 + 并发 | 请求限制、慢速攻击、关闭 drain |
| `websocket` | P3 | 网络扩展 | 协议与安全 |
| SQLite 驱动 | P2 | native extension | 原生扩展契约；本地文件 |
| PostgreSQL 驱动 | P2 | native extension + TLS | 网络/凭据/连接池/加密 |
| Redis 驱动 | P2 | native extension + TLS | RESP/网络/认证/加密 |
| MySQL 驱动 | P3 | native extension + TLS | 复用 PostgreSQL 骨架 |

## 三、专项研究：数值计算

### 3.1 当前事实

1. `int`/`sint`/`float`/`sfloat` 有定宽实现和溢出错误；
2. `lint`/`lfloat` 在 Runtime 中以十进制文本保存，`negate` 和部分转换有实现，但
   **算术运算没有统一实现**：`NumericPair` 只接受定宽数值，`lint`/`lfloat` 没有加减乘除
   路径；转换矩阵也明确拒绝多数 `lint`/`lfloat` 方向；
3. 没有浮点数学 intrinsic（`sqrt`/`sin`/`exp`/`floor`/`ceil`/`pow` 等）；
4. 没有公开的 `decimal`、`fractions`、复数或大整数类型；
5. `random.seed` 目前只服务选择器的确定性 `RandomSource`，不是公开随机库；
6. 容器同质；统计函数需要先决定数值类型和数组元素策略。

### 3.2 数值分层建议

```text
第 N0 层：定宽数值（已实现）
  int / sint / float / sfloat

第 N1 层：任意精度基础（必须先做）
  lint（大整数）算术/比较/转换/文本往返
  lfloat（高精度十进制）算术/舍入/科学计数法
  └─ 选择运行时高精度库或自研；不能用“字符串比较”冒充数值运算

第 N2 层：数值库
  math（整数）→ cmath（浮点原语）→ statistics
  decimal（定点/任意精度十进制）
  fractions（有理数，基于 N1 大整数）
  random（确定性 PRNG；CSPRNG 另立安全批次）

第 N3 层：复杂数值
  complex、矩阵/线代、任意精度浮点格式化（按需求后置）
```

### 3.3 三个必须冻结的选择

**D-1 任意精度实现**：

- 甲：引入高质量 Rust 数值库（大整数/十进制定点/有理数），统一承载 `lint`/`lfloat`
  和库类型；需要 ABI/值表示和依赖锁定；
- 乙：纯 Xiao 实现大整数/十进制/有理数；无 ABI 变化，但性能和内存风险高；
- 丙：只做受限精度（如 128 位），明确不叫任意精度；
- **已确认取甲（2026-11-01）**：用成熟 Rust 数值库承载底层，Xiao 只做便利 API；
  不采用乙/丙的“部分任意精度”口径。

**D-2 decimal 语义（方向已确认，2026-11-01）**：任意精度十进制，与 `lfloat` 共用成熟
Rust 数值底座；字符串往返精确，除法需显式精度与舍入。精确表示和错误身份在 B2/B5 冻结。

**D-3 fractions 表示（方向已确认，2026-11-01）**：`lint` 大整数分子/分母、自动约分、
分母归一为正、零分母报错；转 decimal 需精度与舍入，转 float 显式允许精度损失。
精确签名、资源限制与错误身份仍需对应批次冻结。

### 3.4 各数值库的最小范围

- **math**：`abs`/`min`/`max`/`clamp`/`gcd`/整数 `pow`；整数域，纯 Xiao；
- **cmath**：`sqrt`/`floor`/`ceil`/`round`/`trunc`/`abs`/`min`/`max`/`pow`；D-4 已确认
  **首批包含超越函数**，`sin`/`cos`/`tan`/`exp`/`ln`/`log10` 为待冻结具体清单；
  必须冻结 NaN/Inf、域错误、舍入和平台一致性；
  先做宿主原语，纯 Xiao 只做包装；
- **statistics**：D-5 已确认 B1 只做 int 数组 `sum`/`min`/`max`，空 sum=0、空 min/max
  报错、溢出沿用现有错误；`mean`/`median`/`variance`/`stddev` 与浮点/NaN/精度留 B5；
- **random**：D-6 已确认显式生成器、显式种子与固定算法版本，不依赖隐式全局/线程局部
  状态；序列可复现，CSPRNG 归 B7，不混入普通 PRNG；
- **decimal**：构造（字符串/整数/scale）、加减乘除、比较、舍入、格式化；不先做
  任意精度超越函数；
- **fractions**：构造、约分、加减乘除、比较、转 decimal/float；不先做连分数等扩展。

### 3.5 推荐实现顺序（数值部分）

`lint`/`lfloat` 基础算术 → `math` 整数 → 浮点原语（cmath 基础）→ `statistics` →
`decimal` → `fractions` → 确定性 `random` → CSPRNG（并入 crypto 批次）。

## 四、优先级与批次计划

### 4.1 排序原则

1. 开发者第一天会用、且无新 ABI 的纯 Xiao 库优先；
2. 同一 ABI/能力升级合并成一批，避免每加一个函数就升版本；
3. 语言值模型（JSON/TOML）与数值基础是硬前置，不能绕；
4. 安全/网络/数据库最后做，且必须有审计、限制和失败关闭；
5. 每批独立可验收，不把“半个 API”写进公开文档。

### 4.2 建议批次

| 批次 | 内容 | 前置 | 交付证据 |
| --- | --- | --- | --- |
| B0 | 总纲/目录复核、D-1～D-22 逐条裁定、基线门禁、B1 探测/教程与冻结草案 | 无 | [B0 记录](20o-b0-baseline-and-b1-draft.md)；方向已确认，API 草案待审 |
| B1 | 纯 Xiao 快速批：`math` 整数、`base64`/`hex`、`url`、`csv`、`fmt`、`statistics`（整数子集）；**首批范围已确认（2026-11-01）** | 无 | VM/native 黄金、移除、docs/registry |
| B2 | 数值基础批：`lint`/`lfloat` 算术与转换、浮点原语、`cmath`（首批含超越函数） | B0；ABI 次版本 | intrinsic/ABI 四处 + 版本拒绝 + 三平台 |
| B3 | 简单宿主批：`time`（UTC + sleep）、`args`/`cli`、`fs` 扩展、`env`、`log` 时间戳 | B0；ABI | 能力/错误/取消/三平台 |
| B4 | JSON/TOML 值模型 + `json`/`toml` 解析器 | B0；语言冻结 | 类型/IR/两后端/生命周期 |
| B5 | 数值库批：`decimal`、`fractions`、`statistics` 浮点、确定性 `random` | B2 | 精度/舍入/边界/三平台 |
| B6 | 原生工具批：`regex`、`compression`、`fmt` 扩展 | B0；原生库策略 | 上限/模糊/依赖锁定 |
| B7 | 安全批：`hash`/`hmac`/CSPRNG/AEAD/TLS 基础 | B2；安全审阅 | 测试向量/侧信道/能力位 |
| B8 | 网络批：HTTP client/server | B3+B7 | 超时/限制/取消/关闭 drain |
| B9 | 原生扩展/数据库批：SQLite → PostgreSQL → Redis → MySQL；首批有界连接池；每驱动必须有教程和示例 | 原生扩展契约；B7 的 TLS/crypto | 独立数据库能力/隔离/凭据/审计/池关闭/服务测试 |

**已确认（2026-11-01）**：B7→B8→B9 的先后顺序保持不变；安全、HTTP 和数据库后置，
先把数值、数据、工具和简单宿主库交付。B1 与 B2 可以并行探测，但 B2 的 ABI 升级必须
先冻结；B4 的值模型必须与 B5 的数值类型分开评审，不能把 JSON 的值类型和 decimal 混成
一个类型。

## 五、决策清单

来源：星崽 2026-11-01 直接裁定；D-1/D-7/B1/批次顺序沿用此前确认，其余各项在 B0 会话
逐条询问。下表区分已确认方向与尚未冻结的细项；“建议”不能替代星崽的实际选择。

| 决策 | 星崽已确认 | 尚需对应批次冻结 |
| --- | --- | --- |
| D-1 任意精度 | 甲：成熟 Rust 数值库，统一 lint/lfloat | 库选择/版本、值表示、ABI、限制 |
| D-2 decimal | 任意精度十进制，共用 lfloat 底座；文本往返精确，除法显式精度/舍入 | B2/B5 的表示、错误身份与舍入参数 |
| D-3 fractions | lint 分子分母、自动约分、正分母、零分母报错；转 decimal 指定精度/舍入，转 float 显式允许精度损失 | 精确 API、错误与资源边界 |
| D-4 cmath | **首批包含超越函数** | 具体函数集合、NaN/Inf/域错误/舍入/平台一致性 |
| D-5 statistics | B1 仅 int 数组 sum/min/max；sum([])=0，空 min/max 报错，溢出沿用现有错误 | 均值/中位数/方差/标准差、浮点/NaN/精度在 B5 冻结；B1 错误身份待审 |
| D-6 random | 显式生成器/种子、固定算法版本以复现序列；不依赖隐式全局/线程局部状态；CSPRNG 归 B7 | 算法、种子范围、状态表示与并发边界 |
| D-7 值模型 | 专用 json_value，不扩大公开 dynamic | B4 的类型/生命周期/两后端契约 |
| D-8 time | UTC 墙钟/单调钟/格式化，**首批增加 sleep**；本地时区/日历/解析后置 | sleep 的单位/负值/零值/取消/能力与错误 |
| D-9 fs | 宿主权限，普通操作按平台路径解析 .. 和符号链接，覆盖显式；原子写限同目录替换，无法保证则报错；受限模式另冻 | 三平台路径表、原子保证与错误身份 |
| D-10 argv | 数组不含程序名，程序名另取；保留空参数，Windows 不二次拆引号；非法 Unicode 报错，不有损替换 | 精确入口/错误与平台测试 |
| D-11 regex/compression | 成熟线性时间引擎；gzip/zlib 整块；输入/编译/输出/解压上限必备，流式后置 | B6 的依赖与具体限额 |
| D-12 安全/HTTP/数据库 | 成熟依赖、TLS 默认校验、参数绑定、强制限额/超时、未经安全审阅不发布 | **明确延期**：算法、能力位、HTTP/1.1 子集、原生扩展契约分别在 B7/B8/B9 再逐项确认 |
| D-23 CSV 形状 | 先交付 `parse_record`/`format_record` 记录子集；全文 CSV 和 `[[str]]` 后置 | 记录子集的错误/限额/CRLF/BOM 边界 |
| D-24 错误访问 | 先加自由函数 `error_code`（`error_cause` 按需），不开放 `error.code` 成员访问 | 精确名称、位置、签名、错误表 |
| D-25 B1 冻结 | 七项一起冻结，CSV 用记录子集，错误访问作为共同前置 | 各模块签名、限额、性能预算 |
| D-26 CI 证据 | 默认和原生探测都加维护回归精确执行点，断言恰好 1 passed | 三平台运行号与日志 |

D-13～D-22 的本次答复见 [20P §六](20p-day1-library-architecture.md#六架构决策)。
本轮只做 B0；API 冻结草案与真实探测边界见 [20O-B0](20o-b0-baseline-and-b1-draft.md)。

## 六、新 Agent 交接（无上下文必读）

### 6.1 先读什么

1. 本文件；
2. [20P. Day-1 官方库架构](20p-day1-library-architecture.md)：模块/错误/教程/数据库驱动；
3. [20. 标准库路线](20-builtins-and-standard-library.md) §2/§六/§七；
4. [20C. 官方库冻结](20c-official-xiao-library.md) §3.2.1/§3.4.1；
5. [20D-冻结. 能力与资源](20d-freeze-resource-contract.md) §2.7；
6. [20M. 最近的完整批次](20m-text-and-path-expansion.md)：探测→冻结→实现→三平台证据的模板；
7. [20N. 扩展路线草案](20n-standard-library-expansion-roadmap.md)；
8. [02. 类型系统](02-type-system.md) 数值章节、[03. 容器](03-collections.md)、
   [22. 并发](22-concurrency-execution-and-shared-handles.md)；
9. `docs/DevDocs/README.md`、`docs/module-registry.json`、`docs/UseDocs/library/`。

### 6.2 仓库与门禁（开工时复核）

- 工作目录：仓库根；Rust workspace 清单在 `core/rust/Cargo.toml`。
- 本机构建配置：`.cargo/config.toml` 把 `jobs` 限为 4、关闭 incremental；
  `core/rust/Cargo.toml` 把 dev debug 降为 line-tables-only。不要改回 32 路并行。
- 本地全量：`cargo test --manifest-path core/rust/Cargo.toml --workspace --no-fail-fast`；
  clippy：`bun run check:clippy`；格式：`cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check`。
- 文档/仓库门禁：`bun run check:docs`、`check:usedocs`、`check:layout`；
  汇总：`bun run check`。
- 原生测试需要显式环境：`XIAO_CLANG`、`XIAO_LLVM_AS`、`XIAO_LLC`、`XIAO_STRIP`、
  `XIAO_TARGET_TRIPLE`、`XIAO_RUNTIME_LIBRARY`（Windows 为
  `core/rust/target/release/xiao_runtime.lib`）；Windows 需要先加载 VS 的
  `vcvars64.bat`；本地参考 20M/22D 的测试命令。
- CI：push 触发 `maintenance-regression`（6 个作业）和 `workspace-gates`；
  `platform-reproduction` 是定时/手动。当前维护矩阵的 macOS 为 `macos-15`，
  平台复现为 `macos-26`；改 runner 要单独验证，不要在同一提交里混业务代码。
- 精确原生测试必须 `--exact` 运行并断言 `test result: ok. 1 passed; 0 failed;`；
  三平台日志分别核对，不能只贴本地输出。

### 6.3 每批固定流程

1. **探测**：候选 API 用真实源码在 VM 和受控原生上跑通，记录独立黄金输出；
2. **冻结**：写清签名、所有权、错误、边界、能力和 ABI 影响，提交给星崽确认；
3. **实现**：只做已冻结范围；纯 Xiao 优先；新 intrinsic 同时改 intrinsic 契约、
   VM/LLVM/Runtime/ABI 四处；新值类型先写 `00-decisions`；
4. **文档**：stdlib README、模块 `.md`、UseDocs 教程（十项内容）、可执行示例、
   registry、路线/阶段索引同步；教程示例必须进 CI；
5. **本地门禁**：全量测试、clippy、fmt、docs/usedocs/layout、覆盖率；
6. **远端证据**：push 后记录 maintenance 运行号和逐平台 job，必要时手动
   `platform-reproduction`；
7. **收口**：更新本总纲的批次状态和证据表，再开下一批。

### 6.4 固定禁忌

- 不手写密码学、正则回溯引擎或压缩格式；用审计过/成熟的实现；
- 不把 `lint`/`lfloat` 的字符串表示直接当数值算法；先补基础算术；
- 不在没有值模型时写 JSON/TOML；不把 `dynamic` 内部类型直接暴露成公开注解；
- 不把数据库驱动塞进纯 Xiao 库；先冻结 native extension；
- 不在没有 ABI/能力位冻结时加 time/fs/network intrinsic；
- 不改 32B/16B 布局、22 的虚拟并发模块、冻结的 027–032 或普通句柄单线程规则；
- 不提交其他 Agent 的未完成改动；`git status` 有别人文件时只用显式路径提交自己的；
- 不用 shell heredoc 写源码（中文/转义会静默失配）；不要只看测试名就宣称执行过，
  必须看到 `1 passed` 的真实日志；
- 不声称 19/11/22 的发布债务已关闭。

### 6.5 遇到模糊点先问星崽

以下事项的未确认细项或拟变更的已确认结论必须先问，不得自行决定；已确认方向不重复询问：
D-1～D-22；新增 intrinsic/ABI/能力位；引入新的第三方
运行库或安全算法；扩大公开 API；改变已有接口签名/错误身份；修改 release/CI 矩阵；
数据库驱动选择、连接池/事务/结果集语义；教程示例执行方式和文档状态升级。

### 6.6 第一小时清单

1. `cmd /c git log -5`、`cmd /c git status --short --branch`，确认基线和是否有并发改动；
2. 读 §6.1 的文档，确认当前 ABI/接口版本；
3. 跑一次文档门禁和一次全量测试，记录基线数字；
4. 按 B0 决策表复核已确认范围：math 整数、base64/hex、url、csv、fmt、statistics
   整数子集；不重问已确认方向，只逐项提交尚未冻结的具体契约；
5. 读 [20P](20p-day1-library-architecture.md) 的教程模板、数据库驱动架构与 D-13～D-22
   已确认方向，精确细项留对应批次逐条询问；
6. 为 B1 写探测夹具和冻结草案，同时先写教程/示例草稿；不要直接写产品函数体。

## 七、本规划批的退出条件

1. 候选目录、依赖分层、批次顺序和每批验收模板已写入本文件；
2. D-1～D-12 有明确确认或明确延期；
3. 新 Agent 能从 §六 独立启动，不需要口头补上下文；
4. 不修改产品代码、ABI、intrinsic 或标准库函数体；
5. 本文件与 20 路线、README 索引交叉一致，docs/usedocs/layout 通过。

## 八、落点

```text
docs/DevDocs/20o-official-library-program.md          本总纲与交接
docs/DevDocs/20n-standard-library-expansion-roadmap.md 路线草案
docs/DevDocs/20-builtins-and-standard-library.md       路线批次表
docs/DevDocs/20c-official-xiao-library.md              模块/兼容冻结
docs/DevDocs/20d-freeze-resource-contract.md           能力位与资源契约
docs/DevDocs/20m-text-and-path-expansion.md            最近完整批次模板
docs/DevDocs/README.md                                 阶段索引
stdlib/                                                纯 Xiao 模块
core/rust/crates/xiao-intrinsics/                      intrinsic 契约
core/rust/crates/xiao-runtime-abi/                     ABI 与值表示
core/rust/crates/xiao-runtime/                         Runtime 实现
core/rust/crates/xiao-codegen-llvm/                    LLVM lowering
.github/workflows/                                     三平台证据
```
