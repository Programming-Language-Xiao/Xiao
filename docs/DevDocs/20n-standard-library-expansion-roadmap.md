# 20N. 标准库扩展路线图：math、time、JSON/TOML、log、文件系统、正则、压缩、CLI、加密与 HTTP/数据库

> **这是 20 系列的路线批次，不是立即实现批。**
> 20C–20M 已交付 `text`/`path`/`set`/`dict`/`os` 与虚拟 `std.concurrent`；本批把用户提出的
> `math`、`time`、`json`、`toml`、`log`、文件系统、正则、数据压缩、命令行参数、加密与安全、
> HTTP 客户端/服务器和数据库驱动按**依赖关系**拆分，先冻结价值模型、宿主能力和原生库边界，
> 再逐批实现。本文不写产品函数体，不新增 intrinsic，也不改变 ABI 1.17。
>
> 状态：**规划中（2026-11-01）**。基线为 20M 完成后的 `e9e87e7`。D-1～D-10 需要星崽确认；
> 确认前只做探测和文档。19 发布收口、11C L4、11B I5 和 22E 登记差异仍按各自文档跟踪，
> 本批不认领。
>
> **本文件保留为路线草案**；完整候选目录、数值计算专项研究和无上下文 Agent 交接见
> [20O. 官方库工程总纲](20o-official-library-program.md)；执行顺序以 20O 为准。

## 一、入口条件与当前约束

### 1.1 前置

1. [20. 内置函数与标准库路线](20-builtins-and-standard-library.md) §2.1、§六、§七：
   纯 Xiao 优先；没有宿主必要性不新增 intrinsic；
2. [20C. 官方 Xiao 库](20c-official-xiao-library.md) §3.2.1：模块清单、兼容策略、
   源码/预编译发布形式和身份字段；
3. [20D-冻结. 资源与能力契约](20d-freeze-resource-contract.md) §5.1：能力上下文的
   `u64` 位集，bit 1 为 `clock`，bit 3 为 `network`，bit 4 为 `native_extension`；
4. [20E. 归档接线](20e-archive-wiring.md)：`.xar` 的标准库模块清单和启动前校验；
5. [20M. 文本与路径扩展](20m-text-and-path-expansion.md)：纯 Xiao 模块的探测、冻结、
   实现和三平台证据模板。

### 1.2 当前事实

| 面 | 当前事实 | 对路线的影响 |
| --- | --- | --- |
| 标准库接口版本 | 1；20M 是兼容扩展 | 新增接口默认不升版本；破坏性变化才升 |
| Runtime ABI | 1.17 | 新增宿主 intrinsic（time 等）必须升次版本并补 ABI 指纹 |
| intrinsic 表 | 现有标量转换、`print`/`input`、`os` 读/进程、`set`/`array_push`、`chr` | 没有 time、random、网络、文件写入或数据库入口 |
| 动态值 | 类型层内部存在 `Type::Dynamic`（动态字典读取等），但公开注解/边界未冻结 | JSON/TOML 的异构值不能假设成已有能力 |
| 容器 | `dict<str, T>`、`[T]`、`set<T>` 已可用；值类型同质 | JSON 数组/对象需要动态值或专用值模型 |
| 原生扩展 | 没有冻结的插件/FFI/动态加载契约 | 数据库驱动不能直接按普通 stdlib 实现 |
| 官方库边界 | 不作为普通包进入依赖求解；`.xar` 只记录模块和身份 | 新模块必须同步 README、registry、UseDocs、兼容身份 |

### 1.3 本批不改变

- 不新增 intrinsic、Runtime ABI 符号或能力位；
- 不改变 `Type::Dynamic` 的公开可见性，也不把内部动态读值外推成语言能力；
- 不把数据库驱动纳入标准库源码批次；
- 不改变 20M 已冻结的 `text`/`path` 语义或 22 的并发虚拟模块。

## 二、依赖关系与分批建议

```text
第 0 层：纯 Xiao，无新 ABI
  math（整数子集）、log（stdout） ── 可直接实现
  CLI 参数解析（需要 argv intrinsic）─ 纯 Xiao 解析器，宿主入口待补

第 1 层：简单宿主 intrinsic + ABI 次版本
  time（clock）
  argv（命令行参数）
  filesystem（写入/追加/枚举/删除/重命名/元数据）
  └─ 都需要 intrinsic 契约、VM/LLVM/Runtime/ABI 和三平台证据

第 2 层：原生库封装（Rust，不是纯 Xiao）
  regex（线性时间引擎、编译/执行上限）
  compression（gzip/zlib，解压炸弹上限）
  crypto（SHA-256/HMAC/CSPRNG/常量时间比较，必要时 AEAD）
  └─ 需要依赖锁定、测试向量、模糊/属性测试、安全审阅和限制策略

语言前置
  JSON/TOML 值模型（dynamic/union/专用值）─ 冻结后才写解析器

第 3 层：网络与平台
  HTTP client（依赖网络能力 + TLS/crypto）
  HTTP server（依赖网络 + 并发 + 请求限制）
  数据库驱动（依赖 native extension 契约与网络/文件能力）
```

推荐顺序：**math → log(stdout) → time → argv/CLI → filesystem → JSON/TOML 值模型 →
JSON → TOML → regex → compression → crypto → HTTP client → HTTP server → 数据库驱动**。
先做不需要新 ABI 的模块；再用 time/argv/filesystem 验证 intrinsic 与 ABI 流水线；然后
处理最大的语言前置；最后才进入原生库封装、网络和数据库。

## 三、候选批次

### 20N-A：math（整数子集，纯 Xiao）

**范围**：`abs(int)`、`min(int, int)`、`max(int, int)`、`clamp(int, int, int)`、
`gcd(int, int)`、`pow(int, int)`（非负指数，溢出沿用既有算术错误）。
**不负责**：浮点 `sqrt`/`floor`/`ceil`/三角函数、数值重载、泛型数值约束、随机数。
**理由**：整数子集只用现有 `if`/循环/算术即可表达；浮点函数需要新的宿主 intrinsic，
不在首次 math 批里混入。
**验证**：边界（最小值取绝对值、负指数、零、负数 gcd、溢出）、VM/native 黄金输出、
逐接口移除验证、UseDocs、registry。

### 20N-B：log（stdout 子集，纯 Xiao）

**范围**：`debug`/`info`/`warn`/`error` 级别和一条结构化输出（键值文本），默认写 stdout。
**不负责**：文件 sink、轮转、异步队列、JSON 日志格式、时间戳和级别过滤配置。
**依赖**：时间戳和配置文件需要 20N-C 或后续语言批；文件 sink 需要 `os` 写入 intrinsic，
本批不做。级别过滤先作为显式参数，不引入全局可变状态。

### 20N-C：time（clock intrinsic + 纯 Xiao 格式化）

**范围（建议冻结草案）**：

```text
clock_wall_millis() -> int        # 自 epoch 的墙上时间
clock_monotonic_millis() -> int   # 单调时间，仅用于差值
format_utc_millis(int) -> str     # ISO-8601 UTC，纯 Xiao 或 Runtime 辅助
```

**前置**：`clock` 能力位已存在；需要新增 intrinsic id、Runtime ABI 次版本（建议 1.18）、
VM/LLVM/Runtime 三处实现和 ABI 指纹升级。
**不负责**：本地时区、时区数据库、日历分解、解析任意日期、`sleep`、定时器和跨线程
时钟语义；这些需要单独冻结，不能顺手塞入。
**验证**：能力缺失 `X06-RUNTIME-026`、单调性、墙钟与单调钟混用禁止、格式化边界
（epoch、负值、闰年）、VM/native 一致、`.xiaoc`/`.xar` 的 ABI 次版本拒绝。

### 20N-D：JSON/TOML 值模型（语言前置，非库实现）

这是最大的决策点。当前容器同质，不能直接表示 JSON 的异构数组/对象。至少三种方案：

| 方案 | 形态 | 代价 |
| --- | --- | --- |
| 甲：公开 `dynamic` 边界 | JSON 返回 `dynamic`，索引/转换在运行时检查 | 需要冻结动态值的类型规则、错误身份、生命周期和两个后端表示 |
| 乙：专用 `json_value` 值类型 | 独立 Runtime 值、标签、构造/访问函数和 ABI 变化 | 类型安全更可控，但新增语言值类型和序列化 |
| 丙：固定表/字典模式 | 只支持用户预先声明的 schema，不做通用 JSON | 实现小，但不满足“JSON 库”的通用预期 |

**建议**：先做一轮探测和设计草案，比较甲/乙；丙只作为受限子集。甲/乙都需要在
`00-decisions` 冻结后才写解析器。JSON 和 TOML 共用同一值模型，避免各写一套动态值。

### 20N-E：json（纯 Xiao，依赖 D）

**范围**：`parse(str) -> json_value`、`stringify(json_value) -> str`，支持 object、array、
string、number、bool、null；错误身份固定（语法/深度/大小/重复键）。
**不负责**：流式解析、注释/尾逗号（非标准）、任意数字精度策略、日期特殊类型。
**前置**：D 值模型、错误身份、大小/深度上限；解析器是纯 Xiao，不新增 intrinsic。

### 20N-F：toml（纯 Xiao，依赖 D）

**范围**：TOML 子集（表、数组、字符串、整数/浮点、布尔、日期时间作为字符串边界），
`parse(str) -> toml_value`、`get`/索引访问，错误身份固定。
**不负责**：完整 TOML 规范、日期时间语义、序列化回写、注释保留。
**前置**：D 值模型；TOML 的 datetime 在 D 未冻结前只作为字符串，不承诺日期语义。

### 20N-G：数据库驱动（原生扩展，单独阶段）

数据库驱动不是纯 Xiao 标准库：需要 socket/文件、凭据、连接池、超时、取消、动态加载或
静态链接原生库，并涉及安全和发布信任模型。当前没有 native extension 契约，因此本路线
**只登记前置，不排进 20N 实现批次**：

1. 冻结 native extension 的声明、构建、隔离、权限、ABI 和发布信任；
2. 定义 database capability、连接/查询/参数绑定/事务和错误身份；
3. 定义阻塞查询在并发/取消下的语义；
4. 再决定是内置 sqlite（静态链接）还是受控扩展。

在以上完成前，不提供“数据库驱动”公开 API，也不把进程调用 sqlite CLI 冒充驱动。

### 20N-H：filesystem（宿主 intrinsic + 纯 Xiao 包装）

**范围**：`read_text`/`write_text`/`append_text`、目录枚举、删除、重命名、元数据查询，
以及 `std.fs` 的纯 Xiao 包装。**不负责**：路径沙箱、权限模型、符号链接跟随策略、文件锁、
内存映射和异步 I/O；这些必须先冻结。
**安全前置**：路径归一化/绝对路径/`..`/符号链接/覆盖和原子写入的语义；错误身份；能力
`filesystem` 缺失时的 `X06-RUNTIME-026`；失败清理不能留下半写文件。
**不复制**官方函数体进入归档，不暴露宿主文件描述符。

### 20N-I：命令行参数解析（argv intrinsic + 纯 Xiao 解析器）

**范围**：`args() -> [str]`（是否包含程序名在 D-9 冻结），以及 `std.cli` 子命令、短/长选项、
重复选项、`--` 终止符、帮助和错误。**不负责** shell 展开、环境变量文件、交互提示和全局
可变配置。
**前置**：argv 的编码/Windows 引号/空参数/程序名契约；解析器是纯 Xiao，不新增语言语义。

### 20N-J：regex（原生库封装）

**范围**：建议封装线性时间正则引擎（如 Rust `regex`），提供编译、匹配、捕获组和替换；
首版不支持 lookaround、反向引用和回溯特性。**不负责** PCRE 兼容、Unicode 属性全集和
无上限大输入。
**前置**：编译/执行大小和时间上限、错误身份、栈/内存保护；**禁止手写回溯引擎**，避免
ReDoS 和灾难性回溯。若探测认为纯 Xiao 子集足够，只能作为受限 API，不能伪装完整 regex。

### 20N-K：compression（原生库封装）

**范围**：gzip/zlib 的整块压缩与解压，后续再评估流式 API。**不负责** zstd/zip 容器、
加密压缩、无上限解压炸弹和流式增量状态机。
**前置**：解压后大小/节点/时间上限、错误身份、能力（是否属于纯计算）、依赖锁定和模糊
测试；失败不能半写输出。

### 20N-L：加密与安全基础（原生库封装）

**范围（建议分阶段）**：SHA-256、HMAC、常量时间比较、CSPRNG；需要时再加 AEAD 和 TLS
基础。**不负责**自研算法、口令数据库、证书链、签名/信任模型、密钥托管和 HSM。
**前置**：只使用经审计的 Rust 实现并锁定版本；密钥/秘密不得进入日志、错误文本或
`Debug` 输出；测试向量、边界和侧信道注意事项；新增 `random`/`crypto` 能力位需要
[20D-冻结](20d-freeze-resource-contract.md) 的能力表修订。
**禁止**用纯 Xiao 实现密码学算法。

### 20N-M：HTTP 客户端（网络能力 + TLS）

**范围**：HTTP/1.1 请求/响应、超时、取消、状态码/头部/正文和连接关闭；HTTPS 依赖
20N-L 的 TLS 基础。**不负责** HTTP/2、连接池调优、代理自动发现、cookie 罐和重定向策略
默认值（需另行冻结）。
**前置**：网络能力（bit 3）的实际入口、DNS/连接/读取超时、响应大小上限、并发取消语义、
错误身份和敏感信息屏蔽。阻塞实现与已冻结的并发模型一致；不引入 async/await。

### 20N-N：HTTP 服务器（网络 + 并发 + 请求限制）

**范围**：绑定地址、接受连接、HTTP/1.1 请求解析、方法/路径/头部/正文、响应和优雅关闭；
基于 22 的线程/池模型。**不负责** HTTP/2、WebSocket、TLS 终止（先依赖外部或后续 TLS
批次）、路由框架和中间件生态。
**前置**：请求头/正文/连接数/超时上限、慢速攻击防护、请求走私边界、取消与关闭 drain；
默认不信任任何客户端输入。

## 四、决策点（待星崽确认）

- **D-1 JSON/TOML 值模型**：甲公开 `dynamic` 边界 / 乙专用 `json_value` / 丙仅 schema；
  建议先做设计探测，推荐甲或乙，丙作为受限子集。
- **D-2 time 范围**：是否接受首批只有 UTC + 墙上/单调毫秒，不含本地时区、`sleep` 和
  日历分解；若需要本地时区，必须另立时区数据库与打包策略。
- **D-3 log 范围**：是否接受首批只写 stdout、级别为显式参数；文件 sink 和时间戳依赖
  time/os 写入，不能提前承诺。
- **D-4 数据库驱动**：确认先冻结 native extension 契约，还是明确“首版不支持数据库”；
  驱动还依赖网络/文件能力和加密基础，不能早于 D-9。
- **D-5 filesystem 策略**：是否接受首版只做显式路径、不提供沙箱，但必须冻结 `..`、
  绝对路径、符号链接和原子写入规则；若需要路径限制，另立受限模式。
- **D-6 CLI argv 契约**：`args()` 是否包含程序名；Windows 引号、空参数、UTF-8/本地编码
  和错误行为必须逐格冻结。
- **D-7 regex 实现**：推荐封装线性时间原生引擎；若选纯 Xiao，只接受受限子集且不能宣称
  完整正则。
- **D-8 compression 范围**：首版只做 gzip/zlib 整块还是包含流式；解压大小/时间上限
  和错误身份必须先冻结。
- **D-9 crypto 范围**：首批算法、AEAD/TLS 是否纳入、`random`/`crypto` 能力位如何扩展，
  以及安全审阅和依赖锁定流程。**没有这些不写密码学代码。**
- **D-10 HTTP 范围**：客户端/服务器是否分开成批、是否要求 HTTPS、HTTP/1.1 子集、超时/
  取消/大小限制和默认安全策略。服务器必须先依赖 22 的并发与关闭 drain 语义。

## 五、每批的统一验收模板

1. 纯 Xiao 批次：VM/native 黄金输出、边界、逐接口移除、模块/registry/UseDocs；
2. 新 intrinsic 批次：先冻结契约（id、签名、效果、能力、错误、ABI 版本），再实现
   VM/LLVM/Runtime/ABI 四处，补 `.xiaoc`/`.xar` 版本拒绝和三平台原生证据；
3. 原生库批次：锁定依赖版本/摘要，补测试向量、模糊/属性测试、大小/时间上限和
   安全审阅；不得把 Rust 内部布局暴露成 Xiao ABI；
4. 值模型批次：语言语义先写进 `00-decisions`，再补类型/IR/两后端/生命周期回归；
5. 网络批次：先冻结能力、超时、取消、大小/连接限制和敏感信息策略，再实现客户端/服务器；
6. 所有批次：不改普通包依赖求解，不复制官方函数体进归档，接口版本按兼容策略处理；
7. 三平台维护回归 + 完整 workspace gates + docs/usedocs/layout/覆盖率。

## 六、风险与禁止

1. **范围爆炸**：一次只做一个语义层；JSON/TOML 共用一个值模型，不各造一套；
2. **把内部 `Type::Dynamic` 当公开能力**：未冻结前不写 `dynamic` 参数/返回注解；
3. **ABI 跳跃**：time 的 intrinsic/ABI 升级必须单独成批，不能混在纯 Xiao 模块里；
4. **日志误导**：stdout logger 不承诺文件、轮转、时间和过滤能力；
5. **数据库安全**：没有 native extension 契约前不做驱动，不用进程调用或字符串拼接 SQL
   冒充正式能力；
6. **密码学**：不得手写算法、不得把秘密写入日志/错误/`Debug`、不得用非加密随机冒充
   CSPRNG；没有审计和测试向量不发布；
7. **正则与压缩**：必须设置输入/编译/执行/解压上限，避免 ReDoS、超时和解压炸弹；
8. **文件系统**：不默认提供沙箱；`..`、绝对路径、符号链接和覆盖语义必须先冻结；
9. **网络默认值**：HTTP 必须显式设置超时、大小和连接上限，不静默信任代理/重定向/头部；
10. **发布口径**：本路线不改变 19/11/22 的未闭合状态，也不宣称发布完成。

## 七、落点

```text
docs/DevDocs/20n-standard-library-expansion-roadmap.md  本路线
docs/DevDocs/20-builtins-and-standard-library.md        20 路线批次表
docs/DevDocs/20c-official-xiao-library.md               兼容策略与模块冻结
docs/DevDocs/20d-freeze-resource-contract.md            能力位与资源契约
docs/DevDocs/20e-archive-wiring.md                      .xar 标准库身份
docs/DevDocs/README.md                                  阶段索引
stdlib/                                                 后续模块源码落点
docs/module-registry.json                               模块登记
docs/UseDocs/library/                                   后续模块公共文档
core/rust/crates/xiao-intrinsics/                       新 intrinsic 契约
core/rust/crates/xiao-runtime-abi/                      ABI 次版本
```
