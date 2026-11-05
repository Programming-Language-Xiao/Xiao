# 20O-B2-FREEZE-C. 依赖、资源与性能裁定规划

> **这是 F-C 的裁定规划，不写产品函数、依赖锁、intrinsic、ABI 或测试实现。**
> 产品基线为`8dfbc89b`、工作入口为`35ccdb30`；F-3三项及F-7两项全部确认，以30/0退出（2026-11-05），当时F-D仍待单独签字；后续签字状态见F-D记录。
> 本批只冻结依赖闭包、资源接纳、工作量和性能测量契约，不进入 F-D 签字或 B2-A/B 产品实现。
>
> `cmath` 仍为 planned，Runtime ABI 仍为 1.18、`CODEGEN_VERSION` 仍为 20；本批不改 Cargo、活动 intrinsic 或 ABI。

## 一、入口与事实

| 项 | 当前状态 |
| --- | --- |
| 代码基线 | `8dfbc89b`；已推送 |
| 已完成 | F-A 25/5、F-B1 26/4、F-B2 27/3、F-B3 28/2 退出 |
| F-C 范围 | F-3（资源/工作量/性能）、F-7（依赖闭包/构建） |
| F-C退出台账 | 30 confirmed / 0 pending；F-3/F-7全部确认，当时F-D未签字 |
| 版本/产品 | ABI 1.18、生成器 20、活动 intrinsic 最大 32；cmath planned；无产品实施 |
| F-C收口CI | 收口提交`38d6120d`的workspace `41895438930`、maintenance `41895439116`第1次尝试均成功，维护回归六作业全绿，见[收口CI记录](assets/20o-b2-freeze-c/review-fix-20261105/ci-38d6120d.json)；入口8df及此前b846/59a的容量取消仍按原节点保留 |
| 后续复核CI | `0a93f957`的workspace `41953275349`成功；该提交只变更DevDocs及其assets，未命中维护工作流任何push.paths，maintenance未触发，不能记为通过，见[F-D前置CI记录](assets/20o-b2-freeze-d/entry-20261105/ci-entry-0a93f957.json) |
| 文档门禁 | `check:docs`、`check:usedocs`、`check:layout`、`git diff --check` 通过 |

F-C以**30/0**退出；后续F-D已实际签字，当前状态见[F-D记录](assets/20o-b2-freeze-d/entry-20261105/README.md)。B2-A/B产品不在F-C实施。
本轮[答复与来源记录](assets/20o-b2-freeze-c/entry-20261105/README.md)保存实际答复、上游清单/许可证及入口CI，不把归档核对写成构建成绩。
该证据以35ccdb30为工作入口，38d6120d为文档收口提交；[后续复核](assets/20o-b2-freeze-c/review-fix-20261105/README.md)补齐清单计数构成、UseDocs范围与收口CI，不改变30/0裁定；当时F-D未签字，后续状态另按F-D实际答复登记。

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

### 2.2 已确认的接纳规则

1. 原文、系数、scale、指数、中间值和结果的**共同接纳公式**；
2. 资源接纳与F-12已确认的文本内部顺序对齐，不重新裁定原文→语法→系数/指数/scale的错误优先级；
3. 加/减对齐、乘法、精确除法、显式精度除法、幂的**工作量上界**；
4. 新增 `lfloat //` 与 `%` 的资源/工作量/错误顺序；`%` 不得先物化完整商；
5. 零/±1快路径、强消去、极大指数和真实最大合法数值；B1字段/数组最大输入沿原契约复跑，不新增B2容器限额；
6. 超限 **上限+1**、失败无部分结果、输入不变、预算检查先于结果构造；
7. F10 测量协议：VM/原生 release、预热、5 批中位数、短操作≥50ms、校准至 100ms、重复上限、O0/O3 分记；
8. 性能结果只是受控验收，不是任意硬件时延保证。

### 2.3 已确认资源与性能协议（尚未实测）

完整已确认精确表见[冻结草案§6.1/§6.2](20o-b2-freeze-draft.md)：共同计量、加减/乘法、精确/显式除法、幂、//、%各有独立公式。
%按scale差采用有界模幂或虚拟补零比较，不先物化巨大商；工作上界和实际最终系数分开检查。
F-3c单独确认B2的VM≤10秒、原生≤2秒、OS驻留增量峰值≤64MiB，线性翻倍≤3、超线性≤5；这是验收目标，不是已有成绩，不从B1旧预算自动推导。

## 三、F-7：依赖闭包与构建

### 3.1 已确认直接依赖

| 包 | 版本 | 许可 | 特性 |
| --- | --- | --- | --- |
| `num-bigint` | `0.4.8` | MIT OR Apache-2.0 | `default-features=false`，`std` |
| `bigdecimal` | `0.4.11` | MIT/Apache-2.0 | `default-features=false`，`std` |

已下载六包归档并核对 SHA-256，保存 11 份许可证原文；尚未做 Cargo 解析、锁文件修改或三平台构建。

### 3.2 已确认范围（F-7a/F-7b）

1. `libm=0.2.16`、`num-integer=0.1.47`、`num-traits=0.2.19`、构建依赖`autocfg=1.5.1`；
2. 六包闭包的许可证、校验值、Rust 版本和平台构建策略；
3. `Cargo.lock` 锁定方式、`--locked` 验证与允许的版本漂移；
4. `bigdecimal` 的 `RUST_BIGDECIMAL_*` 构建环境不得改变 Xiao 数值语义；
5. 不引入 C/GMP、随机数或 serde 默认特性；不把候选归档当成已构建；
6. 实际 Cargo.toml/Cargo.lock 修改留 B2-A 实施批，F-C 只冻结策略。

### 3.3 已确认策略（尚未实施）

- 六包精确版本、特性、上游SHA-256和许可已由F-7a确认，详见[来源复核](assets/20o-b2-freeze-c/entry-20261105/dependency-review.json)；
- 直接依赖精确约束，六包版本/校验值由`Cargo.lock`固定，维护构建用`--locked`；新增包、版本或特性超出冻结表时回裁定；
- 仅受控构建子进程清理`RUST_BIGDECIMAL_*`，语言语义显式配置；不改全局环境；
- 沿用Rust1.96.0与现有三平台目标；不改workspace的rust-version=1.85、不推断未实测MSRV兼容；三平台构建证据留B2-A/B。

## 四、实际裁定

| 编号 | 实际答复 | 已确认范围 |
| --- | --- | --- |
| F-3a | 确认接纳与工作量精确表 | §6.1.1～6.1.5的共同计量、加减/乘法、精确/显式除法、幂及快路径；错误顺序沿F-12 |
| F-3b | 确认商余资源与工作量 | §6.1.6～6.1.7的数量级、有界对齐/模乘、floor修正、D+1边界和独立预算 |
| F-3c | 确认性能与测量精确表 | §6.2的时限/翻倍比、五批独立进程、校准、OS内存、失败保留与来源身份协议 |
| F-7a | 确认版本、特性和来源表 | 六包精确版本/特性/许可/归档SHA-256 |
| F-7b | 确认锁定与构建策略 | 精确约束、Cargo.lock/--locked、子进程清理环境和固定Rust/平台；实际修改与构建留实施批 |

五张问题卡均有[实际答复原文](assets/20o-b2-freeze-c/entry-20261105/decision-receipts.json)，未使用预选或默示确认；30/0不等于F-D已经签字。

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

本轮F-C退出对账已完成（2026-11-05）：五条实际答复、30/0台账、精确资源/工作量/性能协议、六包归档与11份许可证复核、
17个独立数学算例、7个只读源码摘要及入口CI记录均已归档；文档门禁与产品边界检查通过，见[收口记录](assets/20o-b2-freeze-c/entry-20261105/README.md)。
本轮没有执行Cargo解析/构建、产品或性能测量；F-D仍待签字，不能将本页读作联合冻结已经生效。

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
