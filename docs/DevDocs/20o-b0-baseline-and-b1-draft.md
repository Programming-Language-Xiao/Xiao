# 20O-B0. 仓库基线、B1 能力探测与冻结草案

> **本批只交付 B0，不是 B1 产品实现或 API 冻结生效。**
> 日期：2026-11-01；开工提交 `681e70b944bf72d8dca611ef4d3beb00ecc9292d`。
> D-2～D-6、D-8～D-22 已在本次会话逐条询问星崽并回填；D-1、D-7、B1 模块范围、
> B7→B8→B9 沿用此前确认。裁定全文分别见 [20O §五](20o-official-library-program.md#五决策清单)
> 与 [20P §六](20p-day1-library-architecture.md#六架构决策)。
> B1 的具体签名、资源限额、错误身份及本页新增阻塞仍待确认，不能从方向裁定推导实现授权。

## 一、基线与阅读复核

已按 20O §6.1 读取 20O → 20P → 20 §二/六/七 → 20C §3.2.1/3.4.1 →
20D 能力契约 → 20M → 20N → 02 数值/03 容器/22 并发 → 阶段索引、registry 与现有教程。

| 项 | 开工实测与边界 |
| --- | --- |
| 分支/工作区 | `main...origin/main [ahead 4]`；工作区干净，没有需要避让的未提交文件 |
| 最近五条提交 | `681e70b`、`fe080f6`、`e9fd3b8`、`6dc0773`、`546e3ff`；前四条为 20N/O/P 规划文档 |
| 构建配置 | `.cargo/config.toml`：jobs=4、incremental=false、RUST_TEST_THREADS=4；dev debug=line-tables-only |
| 接口与 ABI | `xiao-standard-library/src/lib.rs` 的接口版本为 1；`xiao-runtime-abi/src/lib.rs` 为 1.17 |
| 已交付源码目录 | `stdlib/` 的 text、path、set、dict、os；text 11 项、path 4 项；`std.concurrent` 仍为虚拟模块 |
| 数值与容器 | 定宽数值可用；lint/lfloat 不能当作完整高精度算术；同质函数注解可用，不能据内部异构容器推导公开 JSON 或嵌套数组签名 |
| 模块身份 | `std.*` 为用户别名、`stdlib.*` 为固定身份；官方库不进入普通包求解，不复制到归档 |
| CI 静态复核 | maintenance 展开为 6 个作业；已有默认 workspace 与三平台 workspace ignored 执行点，且 `core/rust/**`、`tests/fixtures/**` 在触发路径内 |
| 平台 | maintenance macOS 为 macos-15，platform-reproduction 为 macos-26；本批不改工作流或矩阵 |
| 不负责 | 产品函数、intrinsic、ABI、能力位、19 发布、11C L4、11B I5、22E 登记差异 |

### 1.1 目录与交接勘误

- 20O/20N 原文引用的“20D-冻结 §5.1”不存在；位集、默认值和入口检查的权威位置是
  [20D §2.7](20d-freeze-resource-contract.md#27-g能力上下文的形状契约部分)，已修正交接引用。
- 20P §3.2 的示例执行目录对应 D-20，状态升级对应 D-21；原来标题仅标 D-21，现已分清。
- 20O 所说“容器同质”仅可用于公开同质注解；02/03 允许内部异构数组/集合，不能把两层混写。
- 既有 UseDocs/registry 状态保留。本批七个未来模块单独登记为 `planned`，`code` 为空，
  测试落在夹具与测试字段中，避免把探测函数伪装成已注册标准库源码。

## 二、本次裁定与后续冻结责任

20O 的 D-1～D-12 与 20P 的 D-13～D-22 是本工程编号；20M/20N 的同号决策是各自历史编号，
不能拿旧建议覆盖本次答复。

本次与原建议不同的三项必须贯穿后续批次：

1. **D-4：cmath 首批包含超越函数。** 具体清单、NaN/Inf、域错误、舍入和平台一致性尚需 B2 冻结；
2. **D-8：time 首批增加 sleep。** B3 必须一起冻结单位、负值/零值、能力、超时与 22 取消安全点；
3. **D-18：数据库首批内置有界连接池。** 连接数上限、获取超时、关闭 drain、池内句柄与事务绑定
   必须在 B9 冻结；不再把“先单连接”当作裁定，也不放宽普通资源的跨线程边界。

D-12 明确接受安全原则并将算法、能力位、HTTP 子集与原生扩展精确契约延期到 B7/B8/B9；
D-16 确认独立数据库能力位，但编号/ABI 要到 B9 冻结。本批不分配新编号、能力位、依赖或 ABI。
D-20/D-21 确认固定教程目录、双路径逐字执行及状态硬门槛；B0 的探测成功不允许把教程升级为 verified。

## 三、B1 探测夹具与结果

执行入口：[20o_b1_probe.rs](../../core/rust/crates/xiao-driver/tests/20o_b1_probe.rs)。
七份完整源码存放于 `tests/fixtures/docs/<module>/probe.xiao`，UseDocs 逐字嵌入相同代码。
独立黄金 stdout 在 Rust 测试常量中维护，不从算法运行结果自动生成。

| 模块 | 本批实测的能力 | 未被这份探测证明的能力 | 教程 |
| --- | --- | --- | --- |
| math | 负数/零 gcd、整数幂、负指数拒绝、溢出与 catch | abs/clamp 极值、最小负数 gcd、平方求幂的中间溢出 | [math](../UseDocs/library/math.md) |
| hex | 0/127/128/255 字节、双位查表、大小写解码原语、超范围拒绝 | 完整解码、奇数长度、失败原子性 | [hex](../UseDocs/library/hex.md) |
| base64 | 三字节分组、1/2 字节补位、反向查表、非法字符与规范尾位判据 | 完整严格解码、所有非法补位/空白与空输入 | [base64](../UseDocs/library/base64.md) |
| url | 雪/😀 的 UTF-8 字节算术与百分号表示、空格/+/斜杠 | 完整 UTF-8 验证、ASCII/二字节/临界码点、完整 URL 解析 | [url](../UseDocs/library/url.md) |
| csv | 字段数组、逗号、双引号转义、空字段、字段内换行、未闭合引号 | 严格引号位置、全文记录、BOM/行尾；不得称为完整 RFC 4180 解析器 | [csv](../UseDocs/library/csv.md) |
| fmt | 码点宽度、左填充、超宽不截断、十六进制原语、最小 int 十进制 | 任意进制极值、右填充、字素簇/终端显示宽度 | [fmt](../UseDocs/library/fmt.md) |
| statistics | int 数组 sum/min/max、空 sum、空 min/max 拒绝、溢出、不改输入 | 浮点、NaN、均值/中位数/方差/标准差、大输入性能 | [statistics](../UseDocs/library/statistics.md) |

本机 Windows `x86_64-pc-windows-msvc`：七份源码分别在 VM 与受控 LLVM 原生上逐字通过。
原生先用真实 `llvm-as` 验证，再链接当前 Runtime staticlib；每项退出码 0、stderr 为空。
VM 与原生执行均限制十秒。`caught-*` 只证明恢复分支执行，不作为机器错误身份。

### 3.1 必须保留的边界

| 真实夹具 | 结果 | B1 影响 |
| --- | --- | --- |
| `csv-nested-signature.xiao` | 共同前端拒绝 `[[str]]`，包含 `X04-PARSE-005`；未进入任一后端 | 全文 CSV 的公开返回形状有硬前置，不可通过内部动态数组绕开 |
| `error-code-access.xiao` | VM 执行 `catch` 绑定的 `.code` 后报 `X06-RUNTIME-002`（error 被当作 table） | 不能教用户直接读取 error.code；机读恢复需要单独解决，不能宣称 Day-1 错误访问完成 |
| `numeric-overflow.xiao` | VM 驱动器实际返回 `X06-RUNTIME-009`、退出码 3 | 既有溢出身份有独立机器证据 |
| `explicit-error.xiao` | 纯 Xiao 局部错误跨函数传播为 `B0_PROBE`、退出码 3 | 证明 Error(code=...) 保留机器身份；不等价于源码能读错误字段 |

四个文件位于 [20o-b1](../../tests/fixtures/20o-b1/README.md)。运行时负例当前只登记 VM 证据；
本批原生七项是成功/捕获恢复场景，**不外推为未捕获错误诊断完全一致**。
`B0_*` 名称均为测试数据，没有加入产品诊断族。

### 3.2 CI 执行点与证据限度

现有 `maintenance-regression.yml` 默认命令运行整个 workspace，自动发现本目标的四个默认测试。
三个原生作业的 `--workspace -- --ignored` 未排除本目标，能执行原生探测；`platform-reproduction`
也采用 workspace ignored。没有新增 CI 作业或改变 release/CI 矩阵。

这些是**仓库接线复核**，不是远端成功证据。本批没有远端运行号，Linux/macOS 尚未实跑。
B1 冻结生效前仍要逐平台检查同一提交日志；精确执行必须看到恰好 `1 passed; 0 failed`。

## 四、B1 冻结草案（待审，不生效）

以下所有名称、签名和边界均为候选。D-5 已确认的统计范围单独标出；本页不把探测中的偶然行为
或局部错误码升级为公开契约。各教程的候选表与本节口径一致。

| 模块 | 候选签名 | 提议冻结的语义增量 | 尚待裁定 |
| --- | --- | --- | --- |
| math | `abs(int)->int`、`min(int,int)->int`、`max(int,int)->int`、`clamp(int,int,int)->int`、`gcd(int,int)->int`、`pow(int,int)->int` | 固定 int 域；gcd 非负、gcd(0,0)=0；负指数拒绝；建议 0^0=1 | 最小负数、非法区间、错误身份、完整极值探测 |
| hex | `encode([int])->str`、`decode(str)->[int]` | 字节 0～255；小写输出，解码接受大小写；拒绝空白、奇数长度及坏字符 | 是否接受 [int] 字节载体、输入/输出限额、错误身份 |
| base64 | `encode([int])->str`、`decode(str)->[int]` | 标准字母表、强制补位、不换行、严格尾位；不默认忽略空白 | URL-safe 是否后置、空输入、完整解码与限额 |
| url | `encode_component(str)->str`、`decode_component(str)->str` | UTF-8、保留 -._~ 与 ASCII 字母数字、大写十六进制、+ 保持加号 | 组件子集是否足够；非法 UTF-8/损坏转义、限额与错误 |
| csv | `parse_record(str)->[str]`、`format_record([str])->str` | 一条逻辑记录、引号双写、字段均为字符串；**已确认先交付记录子集** | 全文表示与 `[[str]]` 语言前置、CRLF/LF/BOM/空文件、严格错误与限额 |
| fmt | `pad_left(str,int,str)->str`、`pad_right(str,int,str)->str`、`format_int(int,int)->str` | 码点宽度、单码点 fill、建议进制 2～36/小写/无前缀/负号最前 | 负宽度、上限、最小负数进制算法；不是 str() 的零增量包装 |
| statistics | `sum([int])->int`、`min([int])->int`、`max([int])->int` | **D-5 已确认**：sum([])=0、空 min/max 报错、溢出沿用现有错误；其余统计留 B5 | 精确签名、空数组错误身份、资源上限和别名证据 |

### 4.1 共用冻结字段

| 字段 | 草案与待补证据 |
| --- | --- |
| 导入 | 用户 `std.<domain>`，固定身份 `stdlib.<domain>`；实现批验证两种导入、别名与未导入拒绝 |
| 所有权 | 标量按值；输入字符串/数组只读且不消费调用方引用，输出容器新建；须有别名与失败路径证据 |
| 能力 | 纯计算不新增能力，示例打印沿用 stdout；不依赖隐式环境/服务 |
| 版本 | 计划 ABI 1.17、标准库接口版本 1；不新增 intrinsic、泛型约束、公开 dynamic 或值类型 |
| 错误 | 已有方向：自由函数 `error_code`（`error_cause` 按需），不开放 `error.code` 成员访问；既有数值错误复用，不占用 027–032 | 精确名称/位置/签名、库错误表与类别；B1 冻结时确认 |
| 资源 | 字节数、码点数、数组长度、字段数、结果大小需给出具体限额及超限行为；本轮不擅自填默认值 |
| 性能 | 当前字符串逐步拼接/码点下标可能重复扫描或复制；小样本只证明可表达，不能宣称线性或 Day-1 性能达标 |
| 产物 | 不进入普通包求解；归档记录实际模块/身份；不复制官方函数体；实现批补 loader、逐接口移除与真实归档重放 |
| 教程 | 每库十项内容、固定完整示例、VM/native 独立黄金、错误路径与三平台证据；缺任一项不能 verified |

### 4.2 B1 开工前的裁定（2026-11-01）

1. **CSV**：已确认先做记录子集 `parse_record`/`format_record`；全文解析和 `[[str]]`
   嵌套签名后置，不能把内部异构数组冒充公开全文 API。
2. **错误访问**：已确认先加自由函数 `error_code`（`error_cause` 等按需同批评估），
   不开放 `error.code` 成员访问；精确名称、位置、签名和错误表在 B1 冻结草案中确认。
3. **冻结范围**：七项一起冻结；CSV 用记录子集，错误访问方案作为共同前置，冻结后再写
   产品函数体。
4. **CI**：把默认探测和原生探测都加入维护回归的精确执行点，显式 `--exact` 并断言
   恰好 `1 passed; 0 failed`；不能用 workspace ignored 批量命令代替。
5. 逐模块签名、字符/字节策略、错误表、具体资源限额和性能预算仍需在冻结草案中确认；
   完整算法补探测后决定是否保留候选。三平台证据、移除与归档验收齐备后，才允许写产品
   函数体。

## 五、本地门禁记录

原始日志保存在本机临时目录，文件前缀 `xiao-b0-`；本页记录可复跑命令与汇总，
不把本机路径作为其他机器的执行前置。

| 检查 | 本批结果 |
| --- | --- |
| 开工全量 `cargo test --manifest-path core/rust/Cargo.toml --workspace --no-fail-fast` | 169 组、1517 passed、0 failed、48 ignored；默认全量没有执行这些 ignored 原生项 |
| 开工仓库 all（docs/usedocs/layout/覆盖率） | 退出码 0；既有代码文档覆盖警告保留 |
| 开工 fmt | 通过 |
| Bun 测试 | 301 passed、5 skipped、0 failed；38 文件 |
| 新探测默认测试与文档一致性 | 4 passed、0 failed、1 ignored；七份示例、四份边界夹具及七页完整代码一致性均通过 |
| Windows 原生精确测试 | 七项同源 VM/native 成功；`1 passed; 0 failed`、四项 filtered；日志 `xiao-b0-native.log` |
| 最终 fmt/Clippy | workspace fmt 检查通过；全目标 Clippy `-D warnings` 通过 |
| 最终 docs/usedocs/layout/覆盖率 | 仓库 all 退出码 0；覆盖率达到现有门槛，既有文档覆盖警告未借本批核销 |
| 锁文件与聚合门禁 | workspace/benchmarks 两个 cargo check 通过，两个 Cargo.lock 无差异；已完成 `bun run check` 的等价子门禁 |

执行默认探测：

```powershell
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- --nocapture
```

原生环境按 [10D §四](10d-environment-gated-test-spec.md) 加载 Windows `vcvars64.bat`，
显式配置 `XIAO_CLANG`、`XIAO_LLVM_AS`、`XIAO_LLC`、`XIAO_STRIP`、`XIAO_TARGET_TRIPLE`、
`XIAO_RUNTIME_LIBRARY`，并先构建当前 Runtime release staticlib。随后执行：

```powershell
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- --ignored b1_capabilities_match_native_golden_outputs --exact --nocapture
```

必须检查退出码及 `test result: ok. 1 passed; 0 failed;`，只看到测试名不算运行。
本轮遵守 Git 必须经 cmd 的要求；聚合门禁的 lock 检查拆为两个 cargo check 和
`cmd /c git diff --exit-code -- core/rust/Cargo.lock tests/benchmarks/Cargo.lock`，不修改 package.json。

## 六、交接与退出判据

**B0 已交付（2026-11-01）**：基线、逐条方向裁定、探测、教程草稿与本地门禁齐备。
B1 尚未开工；其 API 冻结条件不能用 B0 完成状态替代。

- B0 的交付物为基线事实、逐条裁定、真实探测、七页 planned 教程、示例执行入口与本冻结草案。
- B1 没有产品模块、兼容身份或完成状态；`stdlib/`、Runtime/intrinsic/ABI 及工作流均不修改。
- §4.2 已确认：CSV 记录子集、自由函数 `error_code`、七项一起冻结、CI 精确执行点。
  这些是方向裁定，不是 API 冻结；精确签名/限额/错误表仍需 B1 草案确认。
- Linux/macOS/远端证据与 CI 精确执行点保持未完成；不能用 Windows 探测通过代替三平台冻结。
- 后续 Agent 从本页与 20O/20P 继续，不重问已确认方向；只询问尚未冻结的具体契约。
- 19/11/22 发布债务保持原归属和状态。
