# 20O-B3-0E. 六模块教程、固定示例与精确测试入口冻结规划

> **状态（2026-11-24）**　B3-0E 已开工。六份教程、夹具、运行输入与精确入口候选已整理，五项裁定待答。`b3_0e_started=true`、`b3_0e_passed=false`、`b3_0f_entry_ready=false`、`b3_0_passed=false`、`b3_product_started=false`。[B3-0E 候选证据](assets/20o-b3/entry-20261124-b3-0e/README.md)
> **B3-0D 输入（2026-11-24）**　15 个原语为 49～63；`environment` 为位 7；目标版本为 ABI 1.20、生成器 23、标准库接口 1；`RunOptions.arguments` 使用 `{program_name, argv}`；Windows 产品级 COFF 仍为 `complete_required`/`pending`。
> **Windows 产物政策（2026-11-22）**　自 B3 起，所有 Windows 公共/发布产物必须产品级 COFF `Complete`，声明组件=观察组件；B3-0E 的教程候选和精确入口记录不能替代 B3 产品验收。
> 入口：[B3-0D 收口](assets/20o-b3/entry-20261124-b3-0d/README.md)、[B3-0B 收口](assets/20o-b3/entry-20261123-b3-0b/README.md)、[B3-0C 收口](assets/20o-b3/entry-20261123-b3-0c/README.md)、[B3-0 实施规划](20o-b3-0-probe-freeze-plan.md)、[B3 宿主库规划](20o-b3-host-libraries-plan.md)。
> B3-0E 不修改产品、intrinsics 登记、VM/LLVM、registry、教程 verified 或旧 721；B3-1/B4 仍未开始。

## 一、入口与范围

| 项 | B3-0E 接收状态 |
| --- | --- |
| B3-0D | `b3_0d_passed=true`、`b3_0e_entry_ready=true`；15 个原语、能力位、版本、C ABI、协议和 COFF 政策已冻结 |
| B3-0B/C | time/args/cli 与 fs/env/log 各有三份教程候选；源码、独立黄金和适用输入保存在各自证据的 `candidates/` |
| 基线 | 固定产品 `e79fab3c`、ABI 1.19、生成器 22、标准库接口 1；B3 目标为 ABI 1.20、生成器 23、接口 1 |
| 目标模块 | `time`、`args`、`cli`、`fs`、`env`、`log` |
| 本批目标 | 冻结六份教程、固定示例/输入/黄金、std/stdlib 导入路径、VM 与原生 O0/O3 精确入口候选及哈希 |
| 本批不做 | 产品实现、真实教程执行、registry/教程 verified、B3-1/B4、旧 721、历史债务核销 |
| Windows COFF | 教程候选只记录产品阶段要求；B3-0E 不产生 Windows 产品级 COFF 证据 |
| 当前状态 | `b3_0e_started=true`、`b3_0e_passed=false`、`b3_0f_entry_ready=false`、`b3_0_passed=false`、`b3_1_entry_ready=false` |

B3-0E 是 **教程与精确入口冻结批**，不是教程执行批。六份教程的内容已在 B3-0B/C 形成候选，本批把它们整理成同一套路径、名称、哈希和测试入口；在产品实现前，教程状态只能是 `planned`，不能把候选解析或黄金检查写成产品执行通过。

### 1.1 要做

1. 汇总 B3-0B/C 的六份教程候选，冻结模块顺序 `time/args/cli/fs/env/log` 和每份教程的导入、quickstart、边界/错误、能力、资源/平台说明；
2. 冻结固定示例目录结构和文件名：源码、独立黄金、适用输入 JSON，以及与教程正文逐字对应的代码块；
3. 冻结 `std.*` 与 `stdlib.*` 双路径覆盖规则；每个模块至少验证两条导入身份，不复制两套正文语义；
4. 设计六模块的 VM 精确入口：目标名、测试名、输入注入、期望退出码、黄金和失败身份；
5. 设计六模块的受控原生 O0/O3 精确入口：目标、工具链、平台、输入和环境变量；Windows COFF 只登记产品阶段要求；
6. 记录教程、示例、输入和黄金的 SHA-256；变化必须显式更新冻结表；
7. 生成 B3-0E 证据、只读审计、自反例和 B3-0F 入口；
8. 保持 registry、UseDocs 和教程状态为 `planned`。

### 1.2 明确不做

- 不在 B3-0E 写 15 个原语、教程执行器、VM/LLVM 绑定或标准库产品代码；
- 不把候选解析、黄金检查或 `tutorial_parse.rs` 结果写成 VM/原生产品执行通过；
- 不升级 registry、UseDocs 教程或任何 `verified` 状态；
- 不启动 B3-1/B4，不跑旧 721，不核销 22C/22D、19/11/22 或 B1 历史债务；
- 不用 B2-C 同链接补验、`UnverifiedCoffExports` 或导出表抽查替代 B3 Windows 产品级 COFF `Complete`；
- 不修改 B3-0B/C 已冻结 API 或 B3-0D 已冻结编号/版本/C ABI/COFF 判据。

### 1.3 结论字段

```text
b3_0e_passed =
  b3_0e0_decisions_recorded
  and b3_0e1_six_tutorials_frozen
  and b3_0e2_fixtures_inputs_goldens_frozen
  and b3_0e3_vm_entries_frozen
  and b3_0e4_native_entries_frozen
  and b3_0e5_evidence_audit_passed

b3_0f_entry_ready = b3_0e_passed and decisions_approved and no_product_change
```

`b3_0e_passed=true` 只表示六份教程、固定示例/输入/黄金和 VM/原生精确入口候选冻结完成，不表示教程已真实执行、标准库已实现或 registry 已升级。`b3_product_started=false`、`registry_verified=false`、`old_721_executed=false` 必须保持。

## 二、开工闸门

1. B3-0D 收口审计、自反例和九条负例通过；`b3_0d_passed=true`、`b3_0e_entry_ready=true`；
2. B3-0D 的 15 个 ID、`environment` 位 7、ABI 1.20/生成器 23/接口 1、C ABI、`arguments` 协议和 COFF 政策可读；
3. B3-0B/C 六份教程、候选源码、独立黄金和输入可读，且当前状态为 `planned`；
4. 固定产品 `e79fab3c`、两个 Cargo.lock、B1 28 守卫和当前常量可复核；B3-0E 不改产品；
5. 三平台目标、Rust 1.96.0 工具链、VM 入口和受控原生 O0/O3 环境可用；缺平台只能记 `unverifiable`；
6. `tutorial_parse.rs` 的解析库身份和 B3-0B/C 教程解析回执可复核；解析通过不等于产品执行；
7. 唯一任务根、独立 target/Runtime/缓存和每模块/平台/输入独占目录；
8. Windows 产物政策已固定为产品级 COFF `Complete`，B3-0E 只登记要求，不产生产品级证据；
9. 教程/registry 当前均为 `planned`，本批不得升级 verified。

## 三、首轮裁定

| 编号 | 提请冻结的候选 | 裁定 |
| --- | --- | --- |
| B3-0E-PATH | 从固定 B3-0B/C 复制源码、黄金和已有输入，B3-0E 候选统一用 `tutorial.xiao`、`tutorial.stdout`、`tutorial.input.json`。产品阶段从 `candidates/` 下按相对路径迁入 `tests/fixtures/docs/<module>/` | 待星崽答复 |
| B3-0E-ALIAS | 每模块正文采用 `std.<module>`。产品精确入口使用同一源码的 `stdlib.<module>` 导入变体和同一黄金。B3-0B 的 `cli-import.zip` 为等价模块历史探针 | 待星崽答复 |
| B3-0E-ENTRY | 每模块一个目标 `20o_b3_<module>_tutorial`，内含 VM 和原生两个独立筛选的测试名。VM 覆盖双路径，原生测试在 O0 和 O3 各覆盖双路径。每次 Cargo 精确调用期望 1 通过、0 失败、0 忽略 | 待星崽答复 |
| B3-0E-INPUT | time 固定墙钟与单调钟，args 保留 `{program_name, argv}` 输入，env 保留 `{environment: [[name, value]]}`，log 固定墙钟及输出端。cli 用源码字面量，fs 独占目录预建空 `demo`。每条导入和优化路径独立运行，UTF-8/LF 黄金直接按字节比较 | 待星崽答复 |
| B3-0E-STATE | 六份候选和精确入口保存在 DevDocs 证据区，教程为 `planned`。产品阶段迁入 UseDocs 和正式夹具路径并按产品结果登记 | 待星崽答复 |

五项答复登记来源、时点和适用范围。可复核的教程、候选路径、输入值和入口集中在证据目录。[候选证据](assets/20o-b3/entry-20261124-b3-0e/README.md) · [精确入口矩阵](assets/20o-b3/entry-20261124-b3-0e/tutorials.json)

## 四、分批执行

### B3-0E0：库存、裁定与冻结表骨架

1. 建立 B3-0E 决定台账，记录五项答复、来源、时点和适用范围；
2. 清点 B3-0B/C 的六份教程、源码、黄金、输入和解析回执；
3. 建立冻结表骨架：模块、教程路径、源码、黄金、输入、std/stdlib 路径、VM 目标、原生目标、平台、状态；
4. 固定 B3-0D 的 15 个 ID、版本和 COFF 政策作为教程实现目标；
5. 建立 `entry-20261124-b3-0e/` 证据目录和唯一任务根；
6. 未答问题保持“待裁定”，不得复制产品断言。

退出：决策台账、六模块库存和冻结表骨架可复核。

### B3-0E1：六份教程正文冻结

1. 以 B3-0B/C 三份教程候选为输入，按模块顺序整理六份教程；
2. 每份包含适用范围、导入、quickstart、完整示例、边界/错误、能力、资源上限和平台差异；
3. 教程中的代码块必须与固定源码逐字一致；黄金必须独立于源码执行结果；
4. 统一 `std.*` 与 `stdlib.*` 的导入说明；保留已有双路径探针结论；
5. 记录六份教程的 SHA-256；变更必须显式更新冻结表；
6. 继续标记 `planned`，不改 UseDocs/registry verified 状态。

退出：六份教程正文、导入说明和哈希冻结。

### B3-0E2：固定示例、输入和黄金冻结

1. 六个模块各固定源码、独立黄金和适用输入；无输入的模块明确写“无输入”；
2. 输入 JSON 只含合成数据：time 固定墙钟，args 程序名/argv，env 环境对，log 固定时钟；fs 由任务入口创建空 `demo`；
3. 固定示例不得读取用户环境、用户工作区外文件或真实终端；
4. 逐字核对教程代码块、源码、黄金和输入；记录四者摘要；
5. 固定路径和名称；产品阶段按同一相对路径迁入 `tests/fixtures/docs/`；
6. 不执行产品函数；解析/黄金检查与产品执行分开登记。

退出：六模块源码/黄金/输入和逐字对应关系冻结。

### B3-0E3：VM 精确入口候选

1. 为六模块登记 VM 目标、测试名、源码、输入、黄金和期望退出码；
2. 目标使用 `20o_b3_<module>_tutorial`，测试使用 `<module>_tutorial_matches_vm_golden`；
3. 输入注入覆盖 args/env/log 的合成输入、time 的固定墙钟和 fs 的独立工作目录；
4. 能力掩码使用 B3-0D 的位 0～7；log 同时满足 `clock` 和 `stdout`；env 使用位 7；
5. 每项只登记候选，不写真实产品断言；产品实现后每目标恰好 1 passed；
6. 记录目标名单和准入条件。

退出：六模块 VM 精确入口候选冻结。

### B3-0E4：受控原生 O0/O3 精确入口候选

1. 为六模块登记原生目标、`<module>_tutorial_matches_native_golden` 测试名、O0/O3、工具链和平台；
2. 记录 `XIAO_CLANG`、`XIAO_LLVM_AS`、`XIAO_RUNTIME_LIBRARY` 等受控环境变量和独立 Runtime；
3. Windows 只登记产品级 COFF `Complete` 要求，状态保持 `pending`；同链接候选、导出表-only、`UnverifiedCoffExports` 不通过；
4. 原生输出、退出码、输入和黄金必须与 VM 语义一致；平台差异显式列出；
5. 不执行真实产品；不把 `UnverifiedCoffExports` 或历史同链接映射写成通过；
6. 记录三平台目标、工具身份和入口名单。

退出：六模块原生 O0/O3 精确入口候选冻结。

### B3-0E5：证据、自反例与 B3-0F 入口

1. 生成 B3-0E 证据清单：教程、源码、黄金、输入、解析回执、入口矩阵、哈希和用户裁定；
2. 只读审计核对固定 Git 对象、B3-0B/C 候选、B3-0D 版本/编号、教程/夹具字节和入口矩阵；
3. 自反例至少覆盖伪造教程哈希、替换黄金、缺输入、把解析写成执行、把 `planned` 写成 verified、伪造 VM/原生入口；
4. 生成 B3-0F 入口：六份教程、固定示例/输入/黄金、VM/原生候选入口、COFF 待验证项；
5. B3-0F/B3-1/B4 未开始；旧 721 和历史债务仍后置；
6. 定稿后单次 docs 门禁；最终 Markdown 由回执覆盖。

退出：B3-0E 证据完整；B3-0F 入口明确；没有产品/registry/教程 verified 变化。

## 五、教程与入口候选

### 5.1 六模块候选来源

| 模块 | 当前教程 | 当前源码/黄金/输入 | VM 目标 | 原生目标 |
| --- | --- | --- | --- | --- |
| time | B3-0B `tutorials/time.md` | `entry-20261123-b3-0b/candidates/tests/fixtures/docs/time/b3-0b-candidate.{xiao,stdout}`；无输入 | `20o_b3_time_tutorial` | `time_tutorial_matches_native_golden` |
| args | B3-0B `tutorials/args.md` | `.../args/b3-0b-candidate.{xiao,stdout}` + `.input.json` | `20o_b3_args_tutorial` | `args_tutorial_matches_native_golden` |
| cli | B3-0B `tutorials/cli.md` | `.../cli/b3-0b-candidate.{xiao,stdout}`；无输入 | `20o_b3_cli_tutorial` | `cli_tutorial_matches_native_golden` |
| fs | B3-0C `tutorials/fs.md` | `entry-20261123-b3-0c/candidates/tests/fixtures/docs/fs/b3-0c-candidate.{xiao,stdout}`；任务入口创建空 `demo` | `20o_b3_fs_tutorial` | `fs_tutorial_matches_native_golden` |
| env | B3-0C `tutorials/env.md` | `.../env/b3-0c-candidate.{xiao,stdout}` + `.input.json` | `20o_b3_env_tutorial` | `env_tutorial_matches_native_golden` |
| log | B3-0C `tutorials/log.md` | `.../log/b3-0c-candidate.{xiao,stdout}` + `.input.json` | `20o_b3_log_tutorial` | `log_tutorial_matches_native_golden` |

VM 与原生测试名沿用各批 `tutorials.json` 的命名。候选按模块设置一个 Cargo 目标，每个目标登记两个精确测试名。

### 5.2 输入和隔离

| 模块 | 输入 | 隔离 |
| --- | --- | --- |
| time | 固定墙钟 946684800000 ms 和单调钟 0 ns | 运行入口注入合成时钟，期望后一次单调读数大于或等于前一次 |
| args | 程序名 + argv 快照 | 不读宿主 argv；空项、重复项和 `--` 原样 |
| cli | 无额外输入 | 纯分组；不读启动器参数 |
| fs | 无输入；任务入口创建空 `demo` | 每平台独立任务目录；结束检查 `demo` 为空 |
| env | 合成环境对 `{"environment": [[name, value]]}` | 由运行入口提供独立的合成快照 |
| log | 固定 `wall_clock_unix_ms`；独立输出端 | 不读真实时钟；捕获 stdout；不写用户终端 |

### 5.3 导入与路径

- 正文主路径为 `std.<module>`；规范身份为 `stdlib.<module>`；
- 每个模块至少一条 `stdlib.<module>` 导入探针，使用同一冻结语义，不复制完整教程；
- `cli` 的 `[[str]]` 返回形状按 B3-0B 前端推导；教程正文不新增嵌套类型语法；
- 候选夹具在 B3-0E 证据中保存；产品阶段按同一相对路径迁入 `tests/fixtures/docs/<module>/`；
- 教程正文、源码、黄金和输入四者必须逐字一致；任一项变化都更新冻结表。

### 5.4 平台与 COFF

| 平台 | 目标 | 教程候选状态 |
| --- | --- | --- |
| Windows x86_64 MSVC | `x86_64-pc-windows-msvc` | 产品级 COFF `Complete` 要求；B3-0E 不产生产品证据 |
| Linux x86_64 GNU | `x86_64-unknown-linux-gnu` | 独立任务目录、VM/原生 O0/O3 |
| macOS arm64 | `aarch64-apple-darwin` | 独立任务目录、VM/原生 O0/O3 |

B3-0E 的平台记录只证明候选入口和输入设计可复核；真实产品执行、原生 O0/O3 和 Windows COFF 验收属于 B3-1/B3-7。

## 六、证据矩阵

| 层 | 必测 |
| --- | --- |
| 教程 | 六份正文、导入、quickstart、边界/错误、能力、资源、平台差异 |
| 夹具 | 六份源码、独立黄金、适用输入、逐字对应、SHA-256 |
| 别名 | `std.<module>` 与 `stdlib.<module>` 导入探针；不复制第二套语义 |
| VM | 六目标名、输入、能力、黄金、退出码、恰好 1 passed 的候选设计 |
| 原生 | 六目标名、O0/O3、工具链/Runtime、平台、输入和黄金 |
| COFF | Windows 产品级 `Complete` 要求、状态 pending、负例边界 |
| 状态 | registry/UseDocs/教程保持 planned；不升级 verified |
| 审计 | 固定 Git 对象、B3-0B/C 候选、B3-0D 编号/版本、入口矩阵、哈希、只读性 |
| 边界 | 无产品改动、无 B3-1/B4、无旧 721、无历史债务核销 |

## 七、退出条件

1. B3-0E0～E5 完成，五项首轮问题有实际答复或逐项待答；
2. 六份教程正文、导入、边界/错误、能力和平台差异冻结；
3. 六模块源码、独立黄金、输入和逐字对应关系冻结，哈希登记；
4. `std.<module>`/`stdlib.<module>` 导入探针规则冻结；
5. 六模块 VM 目标、测试名、输入、能力和退出码候选冻结；
6. 六模块原生 O0/O3 目标、工具链、Runtime、平台和 COFF 要求候选冻结；
7. 证据审计与自反例通过；没有产品/registry/教程 verified 变化；
8. B3-0F 入口明确，B3-0F/B3-1/B4 和旧 721 未开始。

## 八、风险与禁止

- 把 B3-0B/C 的解析回执或黄金检查写成真实 VM/原生执行；
- 在 B3-0E 写产品代码、测试执行器或标准库实现；
- 把 `planned` 教程或 registry 升级为 verified；
- 复制两套 `std`/`stdlib` 教程正文，导致语义漂移；
- 教程代码块、源码、黄金或输入不一致；
- 依赖真实用户环境、真实时钟、终端或工作区外文件；
- 用 `UnverifiedCoffExports`、同链接候选或导出表-only 冒充 Windows `Complete`；
- 启动 B3-1/B4 或跑旧 721。

## 九、B3-0F 交接

B3-0F 接收六份教程、固定示例/输入/黄金、std/stdlib 导入规则、VM/原生入口候选、平台矩阵和 COFF 待验证项。B3-0E 不得自行改 B3-0B/C 契约或 B3-0D 编号/版本；发现冲突回对应批次修订并重新冻结。

交接必须包含：

- 六份教程正文及状态 `planned`；
- 六模块源码、独立黄金、适用输入和 SHA-256；
- `std.<module>`/`stdlib.<module>` 双路径探针；
- VM 目标、测试名、输入、能力、黄金和退出码；
- 原生 O0/O3 目标、工具链、Runtime、平台和 COFF 要求；
- 未决项、延期项和 B3-0F 审计入口。

## 十、落点

```text
docs/DevDocs/20o-b3-0e-tutorials-fixtures-plan.md   本规划
docs/DevDocs/assets/20o-b3/entry-20261124-b3-0e/    执行时建立的证据
docs/DevDocs/20o-b3-0-probe-freeze-plan.md          B3-0 总规划
docs/DevDocs/20o-b3-host-libraries-plan.md           B3 宿主库规划
tests/fixtures/docs/<module>/                       产品阶段迁入的最终夹具
docs/module-registry.json                           只读核对 planned，不升级
stdlib/                                             本批不改产品
```

B3-0E 候选已进入裁定准备。B3-0F 的入口状态由本批 `closeout.json` 登记。[B3-0E 候选证据](assets/20o-b3/entry-20261124-b3-0e/README.md)

## 十一、新 Agent 先读

1. [B3-0 探测/冻结规划](20o-b3-0-probe-freeze-plan.md)；
2. [B3-0D 收口](assets/20o-b3/entry-20261124-b3-0d/README.md)；
3. [B3-0C 收口](assets/20o-b3/entry-20261123-b3-0c/README.md)；
4. [B3-0B 收口](assets/20o-b3/entry-20261123-b3-0b/README.md)；
5. [B3 宿主库规划](20o-b3-host-libraries-plan.md)；
6. [20P Day-1 架构](20p-day1-library-architecture.md)。
