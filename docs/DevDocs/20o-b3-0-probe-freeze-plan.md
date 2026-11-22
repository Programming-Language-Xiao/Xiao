# 20O-B3-0. 简单宿主库探测、冻结与教程草案实施规划

> **状态（2026-11-23）**　B3-0A 的固定基线、三平台探测和 B1 前置已复核。B3-0B 的 time、args、cli 已冻结，B3-0C 入口就绪。[B3-0A 证据](assets/20o-b3/entry-20261122-b3-0/README.md) · [B3-0B 收口](assets/20o-b3/entry-20261123-b3-0b/README.md)
> **Windows 产物政策（2026-11-22）**　自 B3 起，所有 Windows 公共/发布产物必须产品级 COFF `Complete`，声明组件=观察组件；B2-C 的同链接 10 份候选只是历史例外，不得继承。B3-0 的探测记录不能替代 B3 产品验收。
> 入口：[B3 宿主库规划](20o-b3-host-libraries-plan.md)、[B2-C 收口证据](assets/20o-b2-c/entry-20261122-b2-c/README.md)、[20P Day-1 架构](20p-day1-library-architecture.md)、[D-8/D-9/D-10 方向](20o-official-library-program.md#五决策清单)。
> B3-0 不实现六个模块，不改 ABI/intrinsic/registry/教程，不跑旧 721，不启动 B3-1/B4。

## 一、入口与范围

| 项 | B3-0 接收状态 |
| --- | --- |
| B2-C | `b2c_passed=true`，但 Windows 同链接 10 份候选只是历史例外；B3 起必须产品级 COFF `Complete` |
| B3 基线 | 固定产品 `e79fab3c`、ABI 1.19、生成器 22、标准库接口 1；B3-0 不改产品 |
| 目标模块 | `time`、`args`、`cli`、`fs` 扩展、`env`、`log` 六个模块 |
| 本批目标 | 探测平台能力、冻结精确 API/错误/能力/取消/资源/性能/版本、产出六份教程草案与固定示例、形成 B3-1 入口 |
| 本批不做 | 六模块产品实现、registry/教程 verified、B3-1 启动、B4-B9、旧 721、历史债务核销 |
| 历史债务 | 22C/22D、19/11/22、B1 原件缺失、B2-C 的 Windows 历史例外均独立保留 |

B3-0 是 **契约冻结批**，不是实现批。冻结通过只表示“可以按冻结表进入 B3-1”，不表示模块已实现、可用或可发布。

### 1.1 要做

1. 探测六模块在 Windows/Linux/macOS 的现有宿主能力、平台差异、错误、取消和资源行为；
2. 冻结每个模块的公开路径、签名、参数/返回类型、所有权、能力、稳定错误码、错误优先级、取消/超时、资源上限、性能预算和平台适用范围；
3. 冻结 `os`/`fs` 归属、`args`/`cli` 分工、`env` 快照/实时语义、`log` 输出目标和时间语义等有歧义项；
4. 冻结新增 intrinsic、ABI/生成器增量、C 符号、VM/LLVM 绑定、`.xiaoc`/`.xar` 拒绝与缓存身份；未获批准的编号不得预留；
5. 为六个模块各写一份可执行教程草案和固定示例，按 D-20 放入候选夹具；尚未验证的 registry/教程保持 `planned`；
6. 冻结宿主状态测试隔离：fs 只用任务临时目录，env 在隔离子进程，log 捕获到独立流；
7. 形成决定台账、冻结表、教程草案、探测证据和 B3-1 入口；
8. 在冻结表中固定 B3 的 Windows 产物验收方式：产品级 COFF `Complete`，声明=观察，不接受同链接/导出表抽查替代。

### 1.2 明确不做

- 不在 B3-0 写六模块产品代码、测试实现或真实 registry/教程升级；
- 不修改 ABI 1.19、生成器 22、接口 1、intrinsic 1～48/ID 100；新增编号只在冻结表候选，未批准不落产品；
- 不启动 B3-1，不跑 B4-B9，不跑旧 721，不关闭 22C/22D、19/11/22 或 B1 历史债务；
- 不修改工作流阈值、超时或预算来取得探测通过；
- 不用 B2-C 同链接补验、`UnverifiedCoffExports` 或导出表抽查替代 B3 Windows 产品级 COFF 验收；
- 不在冻结前写产品再补契约。

### 1.3 结论字段

B3-0 只派生冻结/交接结论，不派生产品验收：

```text
b3_0_passed =
  b3_0a_baseline_frozen
  and b3_0b_time_args_cli_frozen
  and b3_0c_fs_env_log_frozen
  and b3_0d_version_coff_frozen
  and b3_0e_tutorials_frozen
  and b3_0f_evidence_audit_passed

b3_1_entry_ready = b3_0_passed and decisions_approved and no_product_change
```

`b3_0_passed=true` 只表示契约、版本、教程草案和证据冻结完成，不表示六模块已实现或可发布。`b3_product_started=false`、`registry_verified=false`、`old_721_executed=false` 必须在证据和交接中保持。

## 二、开工闸门

1. B2-C 已完成公共收口；B3-0 规划经确认；B3-1/B4 未开始；
2. 固定产品 `e79fab3c`、两个 Cargo.lock、Runtime/工具链/目标身份可核对；B3-0 不修改产品；
3. 20P Day-1 标准、D-8/D-9/D-10 方向、20D 能力位/错误/取消边界可读；
4. B1 七库 20 入口、18 项资源、VM 与原生双精确入口及 28 项守卫可复现。28 项含 4 项实测输出和 24 项派生负例，资源数沿 B2-C 基线。旧 721 不启用；
5. 三平台目标/工具链/CI 或等价探测环境可用；缺平台只能记 `unverifiable`，不能写成通过；
6. 唯一任务根、独立 target/Runtime/缓存、每模块/平台/探测项独占输出目录；
7. 有歧义的 API/能力/env/log 语义已列入待裁定清单，未裁定项不得标为冻结；
8. Windows 产物政策已写入 B3 规划：B3 起必须产品级 COFF `Complete`。

## 三、分批执行

### B3-0A：基线、决定台账与探测矩阵

本轮以 `e79fab3c` 及两个锁文件为基线，Windows 本机、Linux/macOS Runner 使用独立 release Runtime。B1 双精确、18 项资源与 28 项守卫通过，`b3_0a_baseline_frozen=true`。[入口回执](assets/20o-b3/entry-20261122-b3-0/entry.json)

1. 固定 B2-C 最终产品、ABI/生成器/接口、两个锁、Runtime/工具/目标；
2. 建立六模块探测矩阵：模块、平台、现有 API/能力位、可探测项、预期风险、证据路径；
3. 建立决定台账：待星崽裁定的 API 形状、模块归属、能力、错误码、取消、资源/性能预算、intrinsic/ABI/生成器、教程范围；
4. 逐项标注“已确认方向 / 待冻结 / 明确延期”；方向确认不等于精确契约冻结；
5. 建立 `docs/DevDocs/assets/20o-b3/entry-20261122-b3-0/`，记录源码/Runtime/工具/平台身份和原始探测日志；
6. 任何探测不得修改产品、registry、教程或测试夹具。

退出：基线、探测矩阵和待裁定清单冻结；未开始产品实现。

### B3-0B：`time`、`args`、`cli` 探测与冻结

B3-0B 已登记十二项实际答复。显式毫秒和纳秒、argv 快照、两组数组分隔、错误和性能协议均已冻结。[B3-0B 实施规划](20o-b3-0b-time-args-cli-plan.md) · [收口证据](assets/20o-b3/entry-20261123-b3-0b/README.md)

1. `time`：探测 UTC 墙钟/单调钟来源、精度、时区、格式化能力、时钟回退、sleep 单位/零/负/上限/取消/安全点；冻结阻塞期取消、清理和错误传播；
2. `args`：探测 argv 快照时点、程序名/数组边界、空参数、非 UTF-8/非法 Unicode、Windows 引号规则、`--`/重复项/选项解析边界；
3. `cli`：冻结与 `args` 的分工，禁止重复解析入口；只保留冻结范围内的辅助；
4. 冻结三模块的公开路径、签名、返回/所有权、能力、错误码/优先级、取消/超时、资源/性能预算和三平台适用范围；
5. 为每个可执行语义写教程草案和固定示例；示例必须可在 VM 和受控原生执行，尚未验证的状态保持 `planned`；
6. 记录平台差异和明确延期的 API；不得用本地时区/日历/高级解析扩大首批范围。

退出：`time`/`args`/`cli` 的精确契约、版本身份、教程草案和未决项冻结。

### B3-0C：`fs` 扩展、`env`、`log` 探测与冻结

1. `fs`：探测既有 `stdlib.os` 文件/目录句柄与拟议 `stdlib.fs` 扩展的归属；冻结相对路径基准、`..`、绝对路径、符号链接、覆盖、同目录原子替换、失败清理、权限失败和受限模式；
2. `env`：冻结读取/写入/删除范围、快照或实时语义、名称比较（含 Windows 大小写）、名称/值编码、非法 Unicode、子进程继承、并发可见性和能力；
3. `log`：冻结级别、时间戳来源/格式/时区、输出目标、并发记录边界、刷新/写入失败、能力和资源上限；禁止隐式全局配置；
4. 为三模块冻结公开路径、签名、所有权、能力、错误码/优先级、取消/超时、资源/性能预算和三平台范围；
5. 教程草案与固定示例按 D-20 放入候选夹具；环境变更/文件操作只在隔离任务目录或子进程执行；
6. 任何需要新增能力位、intrinsic 或 ABI 增量的项进入决定台账，未批准不落产品。

退出：`fs`/`env`/`log` 的精确契约、教程草案和平台差异冻结。

### B3-0D：版本、intrinsic、ABI、C ABI 与 Windows COFF 政策

1. 依据 B3-0B/C 的冻结表，列出新增 intrinsic 编号、ABI/生成器增量、接口版本、C 符号、所有权和布局变化；只列候选，不修改产品；
2. 冻结 `.xiaoc`/`.xar` 兼容身份、旧产物拒绝、缓存按生成器/版本失配重生成；保留 intrinsic 1～48/ID 100 墓碑；
3. 冻结 B3 的 Windows 产物验收：产品级 COFF `Complete`，声明组件=观察组件；同链接 10 份候选、`UnverifiedCoffExports`、导出表抽查均不能替代；
4. 冻结三平台目标、工具链、CI 入口和 Windows COFF 证据路径；B3-0 只探测，不将探测结果写成产品验收；
5. 未获星崽批准的新增编号/版本增量不得进入 B3-1；没有版本增量时也须明确写出“不新增”。

退出：版本/兼容/Windows COFF 政策冻结表成立，B3-1 可据此实现。

### B3-0E：教程草案、固定示例与精确测试入口

1. 六模块各一份可执行教程草案：导入、quickstart、边界/错误、能力、资源上限和平台差异；
2. 固定示例按 `tests/fixtures/docs/<module>/` 组织，包含源码和独立黄金；教程正文与夹具逐字对应；
3. 设计 VM 和受控原生 O0/O3 的精确测试入口；示例尚未实现前只登记候选，不写产品断言；
4. 教程/registry 候选保持 `planned`；B3-0 不升级任何 verified；
5. 记录示例/黄金/教程的 SHA-256；示例变化必须显式更新冻结表。

退出：六份教程草案、固定示例和精确测试入口候选冻结；未验证状态保持 planned。

### B3-0F：证据、冻结审计与 B3-1 入口

1. 生成 B3-0 证据清单：基线、探测日志、决定台账、冻结表、教程草案、示例、平台/工具身份、用户裁定；
2. 冻结表逐项标注 API/错误/能力/取消/资源/性能/版本/平台/教程状态；未裁定项不得写“已冻结”；
3. 只读审计核对固定 Git 对象、探测日志、冻结表和示例字节；自反例至少覆盖伪造冻结状态、缺文件、改写探测结论和把方向确认写成精确冻结；
4. 生成 B3-1 入口：六模块冻结表、版本增量、教程候选、Windows COFF 验收要求和未决/延期项；
5. B3-1 未开始；B4-B9、旧 721 和历史债务仍后置；
6. 定稿后单次 docs 门禁；最终 Markdown 由门禁回执覆盖。

退出：B3-0 冻结证据完整；B3-1 入口明确；没有产品/registry/教程 verified 变化。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 基线 | B2-C 产品/ABI/生成器/接口、两个锁、Runtime/工具/目标 |
| time | 墙钟/单调/Linux/macOS/Windows 精度/时区/sleep/取消/错误/预算 |
| args/cli | argv/程序名/空参数/Windows 引号/非法 Unicode/解析边界/能力 |
| fs | os/fs 归属/路径基准/`..`/符号链接/覆盖/原子写/权限/清理/限额 |
| env | 读取/写入/删除/快照/大小写/编码/非法 Unicode/并发/能力 |
| log | 级别/时间戳/时区/输出目标/并发/写入失败/资源/能力 |
| 版本 | intrinsic/ABI/生成器/接口/C 符号/归档/缓存；未批准不落产品 |
| 教程 | 六份教程草案、固定示例、独立黄金、精确测试入口；保持 planned |
| Windows | 产品级 COFF `Complete` 政策与证据路径；同链接/抽查不作为验收 |
| 冻结审计 | 固定 Git 对象、探测日志、决定台账、冻结表、自反例、只读性 |
| 边界 | 无 B3 产品改动、无 registry/教程 verified、无 B3-1/B4、无旧 721 |

## 五、退出条件

1. B3-0A～F 完成，B2-C 基线、六模块探测矩阵和决定台账冻结；
2. 六模块精确 API/错误/能力/取消/资源/性能/平台范围冻结，未裁定项逐项列出；
3. intrinsic/ABI/生成器/接口/C ABI/归档/缓存身份冻结；新增编号未批准不落产品；
4. 六份教程草案、固定示例和精确测试入口候选冻结；registry/教程仍为 planned；
5. Windows 产物政策冻结：B3 起产品级 COFF `Complete`，同链接/`UnverifiedCoffExports`/导出表抽查不得替代；
6. 证据审计与自反例通过；冻结表、探测日志、示例字节和用户裁定可复核；
7. B3-1 入口明确，但 B3-1、B4-B9 和旧 721 未开始；
8. 没有产品/registry/教程 verified 变化，也没有历史债务核销。

## 六、风险与禁止

- 把探测结果或方向确认写成精确冻结；
- 未冻结先写产品，或边实现边改 API；
- 用 B2-C 同链接补验、`UnverifiedCoffExports` 或导出表抽查替代 B3 Windows 产品级 COFF `Complete`；
- 冻结 sleep 无取消/无上限、fs 无权限/无路径解析/无原子保证、env 隐式缓存、log 隐式全局配置；
- 未批准就预留 intrinsic/ABI/生成器编号或升级 registry/教程；
- 把 B3-0 扩成 B3-1/B4-B9 或 B2 数值改动；
- 跑旧 721、关闭 22C/22D、19/11/22 或 B1 历史债务。

## 七、B3-1 交接

B3-1 按 B3-0 冻结表实现 `time`，随后按 B3-2～B3-7 推进；B3-1 不得自行改冻结 API，发现冲突回 B3-0 修订并重新冻结。B3 完成后进入 B4 JSON/TOML；B4-B9 仍后置。

交接必须包含：

- 六模块冻结表与决定台账；
- 版本/intrinsic/ABI/生成器/C ABI/归档/缓存身份；
- 六份教程草案、固定示例、独立黄金和精确测试入口；
- 三平台探测差异、未决项和明确延期项；
- Windows 产品级 COFF `Complete` 验收要求与证据路径；
- B3-0 证据归档和只读审计回执。

## 八、落点

```text
docs/DevDocs/20o-b3-0-probe-freeze-plan.md              本规划
docs/DevDocs/assets/20o-b3/entry-20261122-b3-0/          B3-0 证据
docs/DevDocs/20o-b3-host-libraries-plan.md               B3 总规划
tests/fixtures/docs/                                     六模块教程候选夹具
docs/module-registry.json                               只读核对 planned，不升级
stdlib/                                                   B3-0 不改产品
```

本页只做 B3-0；B3-1 及后续仍未开始。

## 九、新 Agent 先读

1. [B3 宿主库规划](20o-b3-host-libraries-plan.md)；
2. [官方库总纲](20o-official-library-program.md)；
3. [20P Day-1 架构](20p-day1-library-architecture.md)；
4. [20D 能力上下文](20d-capability-context.md)；
5. [B1 收口](20o-b1-closeout.md)；
6. [B2-C 最终收口](20o-b2-c-final-closeout-plan.md)。
