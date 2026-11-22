# 20O-B3-0B. `time` / `args` / `cli` 探测与契约冻结规划

> **状态（2026-11-23）**　time、args 契约和版本候选已定，cli 两组返回形状已确认，源码注解方式待补充裁定。十二项答复、探测结果和三份 planned 教程候选已记录。[阶段证据](assets/20o-b3/entry-20261123-b3-0b/README.md)
> **Windows 产物政策（2026-11-22）**　自 B3 起，所有 Windows 公共/发布产物必须产品级 COFF `Complete`，声明组件=观察组件；B2-C 同链接补验是历史例外，不得继承。B3-0B 的探测记录不能替代 B3 产品验收。
> 入口：[B3-0A 入口与探测](assets/20o-b3/entry-20261122-b3-0/README.md)、[B3-0 实施规划](20o-b3-0-probe-freeze-plan.md)、[B3 宿主库规划](20o-b3-host-libraries-plan.md)、[B3-0A 裁定台账](assets/20o-b3/entry-20261122-b3-0/decisions.md)。
> B3-0B 不改 ABI/intrinsic/registry/教程，不跑旧 721，不启动 B3-0C/B3-1/B4。

## 一、入口与范围

| 项 | B3-0B 接收状态 |
| --- | --- |
| B3-0A | `b3_0a_baseline_frozen=true`；三平台探测和 B1 20 入口/18 资源/双精确/28 守卫通过 |
| 基线 | 固定产品 `e79fab3c`、ABI 1.19、生成器 22、标准库接口 1；B3-0B 不改产品 |
| 目标模块 | `time`、`args`、`cli` |
| 本批目标 | 回答 B3-0B 首轮问题，冻结三模块精确 API/错误/能力/取消/资源/性能/平台、教程草案和版本候选 |
| 本批不做 | 产品实现、registry/教程 verified、B3-0C/B3-1、B4-B9、旧 721、历史债务核销 |
| 状态 | `b3_0b_passed=false`、`b3_0c_entry_ready=false`。整批 `b3_0_passed=false`、`b3_1_entry_ready=false`、`b3_product_started=false` |

B3-0B 是 **契约冻结批**，不是实现批。方向候选和探测结果都不能写成精确冻结；必须由星崽对 B3-0B 首轮问题作出答复，并在冻结表中逐项登记。

### 1.1 要做

1. 回答并登记 B3-0B-TIME、B3-0B-ARGS、B3-0B-CLI 三个首轮问题；
2. 冻结 `time`：UTC 墙钟/单调钟表示、精度/范围、格式化、sleep 单位/零/负/上限、取消/超时、错误身份、能力和平台差异；
3. 冻结 `args`：程序名与 argv 的来源/快照/传递、空项/重复项、UTF-8/非法 Unicode、Windows 引号、串行化边界、条目/字节上限；
4. 冻结 `cli`：与 `args` 的分工、分隔符/参数分组、无分隔符行为、返回类型、帮助/选项解析的延期范围；
5. 冻结三模块的公开路径、签名、所有权、能力、错误码/优先级、取消/超时、资源/性能预算和平台适用范围；
6. 冻结新增 intrinsic/ABI/生成器候选；未批准不落产品；没有增量时明确写“不新增”；
7. 产出三份教程草案、固定示例和精确测试入口候选，保持 `planned`；
8. 形成 B3-0B 证据、冻结表和 B3-0C 入口。

### 1.2 明确不做

- 不在 B3-0B 写 `time`/`args`/`cli` 产品代码、测试实现或真实 registry/教程升级；
- 不修改 ABI 1.19、生成器 22、接口 1、intrinsic 1～48/ID 100；新增编号只进入候选冻结表；
- 不启动 B3-0C/B3-1/B4，不跑旧 721，不关闭 22C/22D 或历史债务；
- 不修改工作流阈值、超时或预算来取得探测通过；
- 不用 B2-C 同链接补验、`UnverifiedCoffExports` 或导出表抽查替代 B3 Windows 产品级 COFF 验收；
- 不在冻结前写产品再补契约。

### 1.3 结论字段

```text
b3_0b_passed =
  b3_0b0_decisions_recorded
  and b3_0b1_time_frozen
  and b3_0b2_args_frozen
  and b3_0b3_cli_frozen
  and b3_0b4_versions_errors_resources_frozen
  and b3_0b5_tutorials_frozen
  and b3_0b6_evidence_audit_passed

b3_0c_entry_ready = b3_0b_passed and decisions_approved and no_product_change
```

`b3_0b_passed=true` 只表示三模块契约、版本候选、教程草案和证据冻结完成，不表示模块已实现或可发布。`b3_product_started=false`、`registry_verified=false`、`old_721_executed=false` 必须保持。

## 二、开工闸门

1. B3-0A 的只读审计和自反例通过；`b3_0a_baseline_frozen=true`、`b3_0_passed=false`、`decisions_approved=false`；
2. 固定产品 `e79fab3c`、两个 Cargo.lock、Runtime/工具/目标身份可核对；B3-0B 不改产品；
3. B3-0A 的 `decisions.md` 可读，B3-0B 首轮问题未答前不得开始冻结；
4. 20P Day-1、20D 能力/错误/取消、09-B0-E 取消检查点可读；
5. B1 七库 20 入口/18 资源/双精确/28 守卫可复现；旧 721 不启用；
6. 三平台探测环境可用；缺平台记录 `unverifiable`，不能写成通过；
7. 唯一任务根、独立 target/Runtime/缓存、每模块/平台/裁定项独占输出目录；
8. Windows 产物政策已固定：B3 起产品级 COFF `Complete`，同链接/抽查不得替代。

## 三、首轮裁定

| 编号 | 候选 | 必须由星崽定夺 |
| --- | --- | --- |
| B3-0B-TIME | 墙钟/单调钟统一 `int` 毫秒；墙钟 Unix 纪元、单调钟从运行入口起算；UTC `YYYY-MM-DDTHH:mm:ss.SSSZ`；sleep 整数毫秒复用既有取消通道 | 纳秒/毫秒、年份域、时长上限、零值、主请求/子任务错误、竞态与等待上界 |
| B3-0B-ARGS | 运行请求独立程序名 + 有序参数向量，VM/原生首次进入应用锁定；空项/重复项原样、非法 Unicode 整批拒绝 | 嵌入/归档来源、缺省、串行化边界、条目/字节上限、错误码和平台拆引号边界 |
| B3-0B-CLI | 首批只做纯函数分隔辅助；首个 `--` 分组并移除分隔符，后续原样；无分隔符第一组全部、第二组为空 | 精确入口/返回类型、无分隔符行为、输出预算、完整选项/帮助生成的延期范围 |

首轮答复只决定范围；精确签名、程序名来源、错误编号和资源/性能表在 B3-0B4 单独确认。

星崽随后确认 time 首批提供显式 `ms` 和 `ns`，args 使用统一运行快照与 `process` 位，cli 提供 `split_terminator`。错误和性能协议已确认。ABI 1.20、生成器 23、标准库接口 1 作为联合候选交 B3-0D。[实际答复](assets/20o-b3/entry-20261123-b3-0b/decisions.json) · [冻结契约](assets/20o-b3/entry-20261123-b3-0b/contract-draft.md)

## 四、分批执行

### B3-0B0：裁定登记与冻结表骨架

1. 建立 B3-0B 决定台账，逐条记录星崽答复、来源、时点和适用范围；
2. 建立三模块冻结表骨架：公开路径/签名/返回/所有权/能力/错误/优先级/取消/资源/性能/平台/教程状态；
3. 固定 B3-0A 入口、产品/ABI/生成器/接口和现有能力位；
4. 建立 `entry-20261123-b3-0b/` 证据目录和唯一任务根；
5. 未答问题保持“待裁定”，不得标为冻结。

退出：三首轮问题有明确答复或仍为待答；冻结表骨架可复核。

### B3-0B1：`time` 契约

1. 冻结 UTC 墙钟和单调钟的表示、纪元/起点、精度和范围；
2. 冻结 UTC 格式化：固定字段、小数位数、`Z`、年份域和非法输入；
3. 冻结 sleep：单位、零/负、上限、取消/超时、安全点、时钟回退、阻塞期清理和错误；
4. 冻结 `clock` 能力复用；新增能力位必须单独裁定；
5. 冻结公开路径、签名、所有权、错误码/优先级、资源/性能表和平台差异；
6. 编写教程草案和固定示例候选，保持 `planned`。

退出：`time` 契约、错误、资源/性能、平台和教程候选冻结。

### B3-0B2：`args` 契约

1. 冻结程序名和 argv 的来源、快照时点、VM/字节码/归档/原生传递；
2. 冻结空项、重复项、`--`、Windows 引号、末尾反斜杠、非法 Unicode 和字节/条目上限；
3. 冻结缺省、嵌入/归档复用、序列化边界和错误码；
4. 冻结公开路径、签名、所有权、能力、资源/性能表和平台差异；
5. 编写教程草案和固定示例候选，保持 `planned`。

退出：`args` 契约、来源、错误、限额、平台和教程候选冻结。

### B3-0B3：`cli` 契约

1. 冻结与 `args` 的分工，禁止重复解析入口；
2. 冻结首个 `--` 分组、无分隔符行为、空项/重复项、返回类型和输出预算；
3. 明确完整选项、默认值、帮助生成和启动器透传的延期范围；
4. 冻结错误码/C ABI 适用边界和平台差异；
5. 编写教程草案和固定示例候选，保持 `planned`。

退出：`cli` 契约、返回类型、延期范围和教程候选冻结。

### B3-0B4：版本、错误、资源和平台表

1. 汇总三模块签名，判断是否需要新增 intrinsic/ABI/生成器/接口/C 符号；只列候选；
2. 冻结稳定错误码、优先级、span/catch/finally/release 和取消传播；
3. 冻结输入/输出/等待/条目/字节/时间预算和性能测量方法；
4. 冻结三平台目标、工具链、CI 入口和 Windows 产品级 COFF `Complete` 证据路径；
5. 未获批准的编号/能力位不得进入 B3-1；没有版本增量明确写“不新增”。

退出：版本/错误/资源/性能/平台冻结表成立。

### B3-0B5：教程草案、固定示例与测试入口

1. 三模块各一份可执行教程草案：导入、quickstart、边界/错误、能力、资源上限和平台差异；
2. 固定示例按 `tests/fixtures/docs/<module>/` 组织，包含源码和独立黄金。B3-0B 将该目录结构存放于证据的 `candidates/`。产品接入与目录登记同步时按相同路径落地；
3. 设计 VM 和受控原生 O0/O3 精确入口；产品未实现前只登记候选；
4. 教程/registry 保持 `planned`，不升级 verified；
5. 记录教程/示例/黄金 SHA-256；变化更新冻结表。

退出：三份教程草案、固定示例和精确测试入口候选冻结；保持 planned。

### B3-0B6：证据、自反例与 B3-0C 入口

1. 生成 B3-0B 证据清单：裁定台账、冻结表、探测日志、教程草案、示例、平台/工具身份；
2. 只读审计核对固定 Git 对象、探测日志、冻结表、示例字节和用户裁定；
3. 自反例至少覆盖伪造冻结状态、缺文件、改写探测结论、把方向候选写成已冻结；
4. 生成 B3-0C 入口：`fs`/`env`/`log` 的待答项、已冻结契约、版本候选、Windows COFF 要求；
5. B3-0C/B3-1/B4 未开始；旧 721 和历史债务仍后置；
6. 定稿后单次 docs 门禁；最终 Markdown 由回执覆盖。

退出：B3-0B 证据完整；B3-0C 入口明确；没有产品/registry/教程 verified 变化。

## 五、证据矩阵

| 层 | 必测 |
| --- | --- |
| 基线 | B3-0A 入口、产品/ABI/生成器/接口、两个锁、Runtime/工具/目标 |
| 裁定 | B3-0B-TIME/ARGS/CLI 答复、来源、时点、适用范围和未决项 |
| time | 墙钟/单调/精度/范围/时区/格式/sleep/取消/错误/预算/三平台 |
| args | 程序名/argv 来源/快照/空项/重复/Windows 引号/Unicode/限额/错误 |
| cli | 分隔符/分组/无分隔符/返回类型/输出预算/延期范围/能力 |
| 版本 | intrinsic/ABI/生成器/接口/C 符号/归档/缓存；未批准不落产品 |
| 教程 | 三份教程草案、固定示例、独立黄金、精确测试入口；保持 planned |
| Windows | 产品级 COFF `Complete` 政策与证据路径；同链接/抽查不作为验收 |
| 冻结审计 | 固定 Git 对象、探测日志、裁定台账、冻结表、自反例、只读性 |
| 边界 | 无 B3 产品改动、无 registry/教程 verified、无 B3-0C/B3-1、无旧 721 |

## 六、退出条件

1. B3-0B0～B6 完成，三首轮问题有明确答复或逐项标为待答；
2. `time`/`args`/`cli` 精确 API/错误/能力/取消/资源/性能/平台冻结；
3. intrinsic/ABI/生成器/接口/C 符号/归档/缓存候选冻结；未批准不落产品；
4. 三份教程草案、固定示例和精确测试入口候选冻结；registry/教程仍 planned；
5. Windows 产物政策冻结：B3 起产品级 COFF `Complete`，同链接/`UnverifiedCoffExports`/导出表抽查不得替代；
6. 证据审计与自反例通过；冻结表、探测日志、示例字节和用户裁定可复核；
7. B3-0C 入口明确，但 B3-0C/B3-1/B4 和旧 721 未开始；
8. 没有产品/registry/教程 verified 变化，也没有历史债务核销。

## 七、风险与禁止

- 把方向候选或探测结果写成精确冻结；
- 未回答 B3-0B 首轮问题就写产品；
- 用 B2-C 同链接补验、`UnverifiedCoffExports` 或导出表抽查替代 B3 Windows 产品级 COFF `Complete`；
- 冻结 sleep 无取消/无上限、args 有损 Unicode、cli 与 args 重复解析；
- 未批准就预留 intrinsic/ABI/生成器编号或升级 registry/教程；
- 把 B3-0B 扩成 B3-1/B4-B9 或 B2 数值改动；
- 跑旧 721、关闭 22C/22D、19/11/22 或 B1 历史债务。

## 八、B3-0C 交接

B3-0B 的 cli 返回声明补充裁定完成后，B3-0C 接收 `fs`、`env`、`log`。发现与三模块契约的冲突时，回 B3-0B 修订并重新冻结。B3-0D 联合确定编号、协议和 COFF 实施身份。[机器可读状态](assets/20o-b3/entry-20261123-b3-0b/closeout.json)

交接必须包含：

- 三首轮裁定和 `time`/`args`/`cli` 冻结表；
- 版本/intrinsic/ABI/生成器/接口/C 符号/归档/缓存候选；
- 三份教程草案、固定示例、独立黄金和精确测试入口；
- 三平台差异、未决项和明确延期项；
- Windows 产品级 COFF `Complete` 验收要求与证据路径；
- B3-0B 证据归档和只读审计回执。

## 九、落点

```text
docs/DevDocs/20o-b3-0b-time-args-cli-plan.md            本规划
docs/DevDocs/assets/20o-b3/entry-20261123-b3-0b/       B3-0B 证据
docs/DevDocs/20o-b3-0-probe-freeze-plan.md             B3-0 总规划
证据目录下的 candidates/tests/fixtures/docs/          三模块教程候选夹具
docs/module-registry.json                               只读核对 planned，不升级
stdlib/                                                   B3-0B 不改产品
```

本页只做 B3-0B；B3-0C/B3-1 及后续仍未开始。

## 十、新 Agent 先读

1. [B3-0 探测/冻结规划](20o-b3-0-probe-freeze-plan.md)；
2. [B3-0A 入口与探测](assets/20o-b3/entry-20261122-b3-0/README.md)；
3. [B3-0A 裁定台账](assets/20o-b3/entry-20261122-b3-0/decisions.md)；
4. [B3 宿主库规划](20o-b3-host-libraries-plan.md)；
5. [官方库总纲](20o-official-library-program.md)；
6. [20D 能力上下文](20d-capability-context.md)。
