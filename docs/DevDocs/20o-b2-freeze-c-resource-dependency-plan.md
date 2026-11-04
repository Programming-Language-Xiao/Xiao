# 20O-B2-FREEZE-C. 依赖、资源与性能裁定规划

> **这是 F-C 的裁定规划，不写产品函数、依赖锁、intrinsic、ABI 或测试实现。**
> 当前入口为 `8dfbc89b`；F-A、F-5、F-6、F-12 已确认，台账 28/2；F-3、F-7 尚待裁定。
> 本批只冻结依赖闭包、资源接纳、工作量和性能测量契约，不进入 F-D 签字或 B2-A/B 产品实现。
>
> `cmath` 仍为 planned，Runtime ABI 仍为 1.18、`CODEGEN_VERSION` 仍为 20；本批不改 Cargo、活动 intrinsic 或 ABI。

## 一、入口与事实

| 项 | 当前状态 |
| --- | --- |
| 代码基线 | `8dfbc89b`；已推送 |
| 已完成 | F-A 25/5、F-B1 26/4、F-B2 27/3、F-B3 28/2 退出 |
| F-C 范围 | F-3（资源/工作量/性能）、F-7（依赖闭包/构建） |
| 台账 | 28 confirmed / 2 pending（F-3、F-7） |
| 版本/产品 | ABI 1.18、生成器 20、活动 intrinsic 最大 32；cmath planned；无产品实施 |
| 最近 CI | `8dfbc89b` 的维护回归 `41880682303` 六作业全绿，workspace `41880682319` 成功；此前 b846/59a 的 macOS 容量取消保留原归属 |
| 文档门禁 | `check:docs`、`check:usedocs`、`check:layout`、`git diff --check` 通过 |

F-C 是 B2-FREEZE 的最后一组待审项。完成后台账应为 **30/0**，但 F-C 仍只是文档冻结，不是 B2-A 实现。

## 二、F-3：资源、工作量与性能

### 2.1 已确认的资源数值

| 维度 | 已确认数值 |
| --- | --- |
| 数值文本 | 最多 65,536 UTF-8 字节 |
| 输入/最终系数 | 最多 32,768 十进制位 |
| 显示/规范 scale 与显式指数 | 绝对值各最多 100,000 |
| 中间工作系数 | 最多 65,536 位 |
| 显式除法 precision | 1～32,768 有效数字 |
| 非平凡幂指数 | 最多 1,000,000，且受结果预算限制 |
| F10 | VM ≤10s、原生 ≤2s、增量内存 ≤64 MiB |

### 2.2 仍需冻结的接纳规则

1. 原文、系数、scale、指数、中间值和结果的**共同接纳公式**；
2. 文本内部顺序（原文 65,536 → 语法 → 系数/指数/scale 预算），且与 F-12 优先级一致；
3. 加/减对齐、乘法、精确除法、显式精度除法、幂的**工作量上界**；
4. 新增 `lfloat //` 与 `%` 的资源/工作量/错误顺序；`%` 不得先物化完整商；
5. 零/±1 快路径、强消去、极大指数、字段/数组最大合法输入；
6. 超限 **上限+1**、失败无部分结果、输入不变、预算检查先于结果构造；
7. F10 测量协议：VM/原生 release、预热、5 批中位数、短操作≥50ms、校准至 100ms、重复上限、O0/O3 分记；
8. 性能结果只是受控验收，不是任意硬件时延保证。

### 2.3 建议起点（未生效）

沿用冻结草案的公式：非零显示系数位数为 `d`，零为 1 位；原文/系数/scale/指数/中间值分别计量；
加减按对齐长度，乘法按 `d_left+d_right`，精确除法先约分并判断 2/5 因子，幂用平方求幂并逐步检查工作预算。
具体公式和性能阈值仍须星崽逐项确认，不能从已确认数值自动推导。

## 三、F-7：依赖闭包与构建

### 3.1 已确认直接依赖

| 包 | 版本 | 许可 | 特性 |
| --- | --- | --- | --- |
| `num-bigint` | `0.4.8` | MIT OR Apache-2.0 | `default-features=false`，`std` |
| `bigdecimal` | `0.4.11` | MIT/Apache-2.0 | `default-features=false`，`std` |

已下载六包归档并核对 SHA-256，保存 11 份许可证原文；尚未做 Cargo 解析、锁文件修改或三平台构建。

### 3.2 仍需冻结

1. `libm`、`num-integer`、`num-traits`、`autocfg` 的精确传递版本；
2. 六包闭包的许可证、校验值、Rust 版本和平台构建策略；
3. `Cargo.lock` 锁定方式、`--locked` 验证与允许的版本漂移；
4. `bigdecimal` 的 `RUST_BIGDECIMAL_*` 构建环境不得改变 Xiao 数值语义；
5. 不引入 C/GMP、随机数或 serde 默认特性；不把候选归档当成已构建；
6. 实际 Cargo.toml/Cargo.lock 修改留 B2-A 实施批，F-C 只冻结策略。

### 3.3 建议起点（未生效）

- `libm=0.2.16`、`num-integer=0.1.47`、`num-traits=0.2.19`、`autocfg=1.5.1`，以当前索引/归档为准；
- 精确约束 + `Cargo.lock` + `--locked`；实际解析超出闭包时回冻结；
- 构建环境清理 `RUST_BIGDECIMAL_*`，语言语义显式配置；
- 三平台构建证据留 B2-A/B，不在 F-C 声称已构建。

## 四、需要星崽确认

| 编号 | 需要确认 | 建议 |
| --- | --- | --- |
| F-3a | 接纳公式与预算顺序 | 采用冻结草案公式；原文→语法→系数/指数/scale→工作/结果；与 F-12 一致 |
| F-3b | `//`/`%` 资源与工作量 | 各自独立预算；`%` 不物化完整商；失败无部分结果 |
| F-3c | 性能/采样 | VM≤10s、原生≤2s、内存≤64MiB；预热+5批中位数；短操作≥50ms；O0/O3分记 |
| F-7a | 四个传递/构建依赖版本 | 采用 `libm 0.2.16`、`num-integer 0.1.47`、`num-traits 0.2.19`、`autocfg 1.5.1` |
| F-7b | 构建/锁策略 | 精确约束、Cargo.lock、--locked；清理 `RUST_BIGDECIMAL_*`；实际修改留实施批 |

这些是建议，不是已生效规则；只有实际答复后才更新 ledger。

## 五、分批执行

### C-A：资源与工作量（F-3）

1. 冻结接纳公式、工作量和预算顺序；
2. 冻结 `//`/`%` 的资源/工作量规则；
3. 冻结性能、内存和采样协议；
4. 记录上限+1、失败无部分结果和输入不变；
5. 不生成测量报告或产品实现。

### C-B：依赖闭包（F-7）

1. 冻结六个包/构建依赖的精确版本与许可证；
2. 冻结 Cargo.lock/--locked/构建环境策略；
3. 记录实际归档摘要；不修改 Cargo.toml/Cargo.lock；
4. 不对三平台构建作产品结论。

### C-C：F-C 收口

1. F-3/F-7 从 pending 改为 confirmed；
2. 台账从 28/2 变为 **30/0**；
3. 更新冻结草案、F-C/F-D 规划和索引；
4. 通过 docs/usedocs/layout 与 `git diff --check`；
5. 记录 F-D 仍待签字，B2-A/B 仍未开工。

## 六、验收与证据

| 检查 | 期望 |
| --- | --- |
| F-3 | 接纳公式、工作量、性能/内存/采样规则唯一；包含 `//`/`%` |
| F-7 | 六包/构建依赖版本、许可证、校验值和锁策略唯一 |
| 台账 | 28/2 → 30/0；F-3/F-7 不再 pending |
| 文档 | 冻结草案、F-C/F-D 规划、索引一致；docs/usedocs/layout 通过 |
| 产品边界 | `core/rust`、`stdlib`、Cargo 文件、intrinsic/ABI、诊断表、测试、工作流无差异 |
| 证据 | 实际答复、命令和静态来源入库；不伪造构建、性能或三平台成绩 |

## 七、退出条件

1. F-3、F-7 全部 confirmed；
2. 资源/工作量/性能与依赖闭包均有唯一精确表；
3. 台账为 30/0；
4. 活动 intrinsic/ABI/诊断表未改；
5. 文档门禁通过，产品边界无差异；
6. F-D 签字未开始；B2-A/B 仍未开工；22D/22C、19/11/22 债务保持独立。

## 八、交接

新 Agent 先读：

1. [B2 冻结收口规划](20o-b2-freeze-closeout-plan.md)；
2. [B2 冻结草案](20o-b2-freeze-draft.md) §六～§九；
3. [依赖归档](assets/20o-b2-freeze-close/entry-20261103/dependency-archives.json)；
4. [F-B3 收口证据](assets/20o-b2-freeze-b3/entry-20261105/README.md)；
5. [30 条决策台账](assets/20o-b2-freeze-close/entry-20261103/decision-ledger.json)。

不得：

- 在 F-C 修改 Cargo.toml/Cargo.lock、活动 intrinsic/ABI 或产品函数；
- 把候选依赖归档写成已构建；
- 把资源公式或性能目标写成已有产品成绩；
- 把 `cmath` planned 升为 verified；
- 把 B1 历史 721 条 F10 写成 B2 当前性能。

## 九、落点

```text
docs/DevDocs/20o-b2-freeze-c-resource-dependency-plan.md 本规划
docs/DevDocs/20o-b2-freeze-closeout-plan.md              F-C/F-D 总规划
docs/DevDocs/20o-b2-freeze-draft.md                      精确契约
docs/DevDocs/assets/20o-b2-freeze-close/entry-20261103/  依赖归档与台账
docs/DevDocs/assets/20o-b2-freeze-b3/entry-20261105/     F-12 证据
```

F-C 收口后只剩 F-D 冻结签字；B2-A 仍是后续实现批，不是本阶段。
