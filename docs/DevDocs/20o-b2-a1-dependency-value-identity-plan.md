# 20O-B2-A1. 依赖、适配层与值身份实施规划

> **这是 B2-A 的 A1 获批实施规划。原规划交付批未解析/构建或写产品；本次独立实施按既有A-P1授权进行。**
> 入口为`f522b593`后的当前提交；A0有效收口见[closeout.json](assets/20o-b2-a0/entry-20261105/closeout.json)（原始`result.json`仍为`passed=false`，不可引用为通过）。
> F-D已签字、30/0、联合冻结已激活；A-P1已批准A1范围。A1本批已退出，A2～A7尚未开始，B2-A整批未收口。
> A1只建立依赖、诊断、适配层和值身份；A2/A3算术、A4转换、A5全链路、A6性能与A7收口不在本批。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| A0 | Windows有效收口通过；契约9/9、资源17/17、VM/Windows原生精确点各1；Linux/macOS本地未验证 |
| 原始结果 | `result.json`仍为`passed=false`、`windows=failed`；有效结论只引用`closeout.json` |
| 版本 | ABI 1.18、`CODEGEN_VERSION` 20、活动intrinsic最大32、`cmath` planned |
| 依赖 | F-7六包已接入，metadata --locked与Windows受控构建通过；两份锁仅增加冻结闭包 |
| 台账/签字 | 30/0；F-D已签字；A-P1/A-P2/A-P3已批准 |
| 历史 | B1旧721条F10只作历史；22D/22C、19/11/22债务独立 |

A1的产物是**可被后续算术复用的精确数值底座**，不是可对外宣称的lint/lfloat算术能力。A2/A3仍须在A1退出后另批实现和验收。

### 1.1 要做

1. 按F-7添加并锁定六包，更新`Cargo.lock`，所有Cargo操作使用`--locked`；
2. 清理继承的`RUST_BIGDECIMAL_*`，Xiao语义显式配置，不使用上游默认精度/舍入/Display；
3. 在`xiao-diagnostics`定义033/034及消息目录，接入Runtime错误门面；
4. 建立`Lint`/`Lfloat`内部BigInt/BigDecimal适配层，保持文本标签/XCON结构；
5. 统一解析、输出、零、符号、scale、指数、规范/显示身份、相等与哈希；
6. 让类型、ABI、VM常量和XCON→Runtime恢复共享同一合法性/预算入口；
7. 保证`Eq`/`Hash`对合法值不分配、不panic、不失败、不回退文本；
8. 建立确定性单元/边界用例，并用B1契约/资源回归确认没有破坏既有产品。

### 1.2 明确不做

- 不实现A2/A3的lint/lfloat算术、比较和`//`/`%`/`**`；
- 不实现A4转换矩阵、A5全链路、A6性能或A7收口；
- 不新增intrinsic、不分配33～48、不改C ABI符号/布局/状态；
- 不实现`lfloat_divide`、`cmath`或`stdlib/cmath.xiao`；
- 不改`XiaoValue`标签、XCON版本/格式或标准库registry状态；
- 不运行B1旧721条F10，不把A0/A1写成B2-A产品交付。

## 二、开工闸门

A1执行前必须确认：

1. A0的`closeout.json`有效收口、`result.json`失败身份仍保留；A1未开始；
2. F-7六包版本、许可证、特性和锁策略与冻结表一致；实际落地只新增这六包；
3. 当前ABI1.18/生成器20/活动intrinsic最大32/cmath planned未被改动；
4. B1契约/资源基线可复现；旧721条F10仍标历史；
5. 唯一A1任务目录和证据目录已创建，不覆盖A0归档；
6. 任何超出A-P1范围的语义、错误、ABI或intrinsic变化先停并回B2-FREEZE。

## 三、分批执行

### A1-1：依赖与锁

1. 按现有工作区依赖方式引入`num-bigint=0.4.8`、`bigdecimal=0.4.11`，仅`std`，关闭不用的默认特性；
2. 锁定`libm 0.2.16`、`num-integer 0.1.47`、`num-traits 0.2.19`、`autocfg 1.5.1`；
3. 更新`Cargo.lock`；锁差异只允许F-7六包及其既有依赖，不引入`rand`/`serde`/`num-rational`/C/GMP；
4. 所有构建、测试和检查使用`--locked`；维护构建清理`RUST_BIGDECIMAL_*`；
5. 记录上游归档/许可证来源摘要、Cargo.lock前后差异和Rust1.96.0/workspace1.85边界；
6. 不把候选来源核对写成已构建，不推断未实测MSRV。

退出：六包精确锁定；锁差异可解释；无额外依赖；`--locked`构建通过。

### A1-2：033/034诊断与消息目录

1. 在`xiao-diagnostics`定义`NUMERIC_LIMIT_CODE`/033和`NON_TERMINATING_DIVISION_CODE`/034及类别；
2. 在`xiao-i18n`接入`runtime.numeric_limit`和`runtime.non_terminating_division`；
3. 按F-12保持002/009/011/012/013边界；033不等于011，034不等于013；
4. 不新增C ABI函数、状态编号或intrinsic；
5. 单元测试核对错误身份、类别、消息ID和挂起错误传播；
6. 活动诊断表只新增这两个已冻规划身份，不占其他编号。

退出：033/034可被语言错误路径引用；C ABI和错误优先级未改；消息目录一致。

### A1-3：Lint/Lfloat适配层与值身份

1. 保持`Lint(String)`/`Lfloat(String)`文本标签和ABI边界；内部按操作临时解析为BigInt/BigDecimal，结束释放；
2. 不加跨调用共享大数缓存，不把Rust布局放进`XiaoValue`；
3. 统一解析/输出、零、符号、显示/规范scale、指数和规范键；
4. D=32,768、W=65,536、H=100,000；零显式规范化，前导零/尾零/负零按F-10/F-3；
5. `1.5`、`1.50`、`15e-1`共享同一数值身份；显示精度与规范身份分开保存；
6. 相等/哈希使用规范数值视图，不用原文或`f64`；
7. 强消去、scale差、最大系数和上限+1都有确定性用例。

退出：同一数值只有一种规范身份；解析/输出往返稳定；Eq/Hash不改变值身份。

### A1-4：类型、ABI、VM常量与XCON入口

1. 类型检查/常量降低/ABI恢复/VM常量恢复/XCON→Runtime恢复共用A1的合法性/预算入口；
2. 不可失败的`Eq`/`Hash`只接受已校验的规范值；非法载荷在可失败入口拒绝，不能进入Eq/Hash；
3. 保留旧ABI结构错误边界和XCON v1格式；不改标签、版本、字段或排序；
4. 不把非法/超限文本静默截断、回退或转换为默认值；
5. 记录每个入口的错误身份、输入不变和失败原子性；
6. 不修改并发协议、22类排序或既有断言。

退出：所有入口共享同一身份/预算规则；XCON/ABI结构未变；Eq/Hash无非法值前置。

### A1-5：确定性与B1回归

1. 单元/边界用例覆盖前导零、尾零、零、负零、强消去、scale差、D/W/H上限+1、`1.5`/`1.50`/`15e-1`；
2. 按星崽本次A1-DICT实际确认：现有Xiao集合验证数值去重/查询，Rust HashMap验证Eq/Hash契约；语言字典仍为str键，数值键后置，不把Rust映射写成语言证据。确定性生成用例不引入`rand`/`proptest`；
3. 依赖/诊断/适配层测试；不运行A2/A3算术；
4. B1契约和资源回归按A0相同入口复跑；旧721条F10不运行；
5. 保存实际命令、日志、退出码、平台和证据摘要。

退出：A1测试与B1契约/资源通过；没有未登记的类型/错误/依赖变化。

## 四、证据矩阵

| 项 | 必存证据 |
| --- | --- |
| 依赖/锁 | 六包版本/特性、Cargo.toml/Cargo.lock前后差异、`--locked`日志、许可证来源 |
| 环境 | `RUST_BIGDECIMAL_*`清理、显式精度/舍入、Rust1.96/workspace1.85 |
| 诊断 | 033/034代码/类别/消息ID、与002/009/011/012/013边界 |
| 值身份 | 解析/输出往返、规范/显示scale、零/负零、强消去、最大D/W/H、上限+1 |
| 相等/哈希 | `1.5`/`1.50`/`15e-1`同值；实际Xiao集合去重/查询、Rust HashMap契约分列，语言字典保留str键；无分配/无panic |
| 入口 | 类型/ABI/VM常量/XCON恢复的合法性、错误身份、输入不变 |
| 回归 | B1契约/资源实际通过；旧721条未运行 |
| 边界 | 不改ABI/intrinsic/XCON/registry/cmath；A2～A7未开始 |

## 五、退出条件

1. F-7六包精确锁定，锁差异可解释，所有Cargo操作`--locked`；
2. `RUST_BIGDECIMAL_*`不改变语义；显式精度/舍入配置可审计；
3. 033/034按F-12进入活动诊断/消息目录，C ABI状态未改；
4. Lint/Lfloat适配层保留文本标签，无Rust布局泄漏；
5. 规范身份唯一，显示身份保留；Eq/Hash无分配/panic/非法值；
6. 类型/ABI/VM常量/XCON入口共享验证，XCON/ABI结构未变；
7. A1测试与B1契约/资源回归通过；
8. A2/A3/A4/A5/A6/A7未开始；旧721条F10未运行；历史债务独立。

## 六、风险与禁止

- 把BigDecimal默认精度/舍入/Display当语言契约；
- 让`1.5`/`1.50`成为两个值，或用原文做Eq/Hash；
- Eq/Hash临时分配、panic、失败回退或接受非法值；
- 为A1“顺手”实现A2/A3算术或A4转换；
- 新增intrinsic/ABI/状态、改XCON格式、registry状态或模块身份；
- 引入F-7之外的包、`rand`/`proptest`或C/GMP；
- 用旧721条F10或A0基线冒充A1/B2-A成绩；
- 修改22D/22C断言或历史证据。

## 七、交接

A1退出后下一入口是A2（`lint`算术/比较）与A3（`lfloat`算术/精确有限除法），执行规划见[A2/A3算术实施规划](20o-b2-a2-a3-arithmetic-plan.md)；A4转换、A5全链路、A6资源/性能、A7收口依次另批。A1不得把A2/A3能力提前声明为完成。

新Agent先读：

1. [B2-A实施规划](20o-b2-a-implementation-plan.md)；
2. [A0基线记录](20o-b2-a0-baseline-plan.md)与[有效收口](assets/20o-b2-a0/entry-20261105/closeout.json)；
3. [冻结草案](20o-b2-freeze-draft.md)§四～§九；
4. [B2数值基础规划](20o-b2-numeric-foundation-plan.md)；
5. [F-7来源复核](assets/20o-b2-freeze-c/entry-20261105/dependency-review.json)与[F-12错误表](assets/20o-b2-freeze-b3/entry-20261105/README.md)。

## 八、落点

```text
docs/DevDocs/20o-b2-a1-dependency-value-identity-plan.md  本规划
docs/DevDocs/20o-b2-a-implementation-plan.md              A0～A7总规划
core/rust/Cargo.toml / Cargo.lock                         A1依赖与锁
core/rust/crates/xiao-runtime/src/value/...               A1适配层
core/rust/crates/xiao-diagnostics / xiao-i18n             033/034
core/rust/crates/xiao-runtime/src/{abi,serialization}.rs  入口校验
core/rust/crates/xiao-vm/src/semantics/exec.rs            VM常量恢复
core/rust/crates/xiao-types/src/numeric.rs                类型/值身份接线
docs/DevDocs/assets/20o-b2-a1/entry-<date>/               A1证据（待建）
```

原规划交付批只规划；后续A1已按A-P1完成本批验收：专项/四个相关包、VM/原生示例、无分配契约、B1 9/17及两个精确点、workspace与28/28守卫通过。A1退出不等于B2-A整批交付，A2～A7尚未开始。
字典证据裁定见[本次答复](assets/20o-b2-a1/entry-20261106/decision-receipts.json)，未修改30项冻结台账或已签正文。

## 九、实际实施与退出（2026-11-06）

代码快照为`39b2c1ea72939b20ea48d096e03b3a515bf81127`，入口为`e06cc88a`。
[完整证据](assets/20o-b2-a1/entry-20261106/README.md)按命令保存真实argv、cwd、PID、原始日志及执行前后源码树。
最终验收绑定964个源码输入的同一树：原始摘要`783299e533053a6f4c7fbdc250b488cb8fcc68bbd4a3e8c73d7a2918255277b8`，规范文本/二进制摘要`ba73d1e5e8ec63f9ed48c2db13d2ec1f4044b0881ec3cd5be886b2fada061f5b`。
这是明确列出的源码输入数量，不是证据目录或全仓库文件总数。

| 项 | 实际结果 |
| --- | --- |
| F-7依赖 | 六包精确版本/特性经metadata --locked核对，两份锁只新增这六包与Runtime依赖边；11份许可证重新对照 |
| 环境 | 从空A1 target构建，受控子进程清除了实际注入的六个RUST_BIGDECIMAL_*干扰变量；workspace的rust-version=1.85未改，不推断未测MSRV |
| 接纳/身份 | 输入D/H、原文边界及W常量；算术工作值验收后置；完整语法、零、显示/规范scale；ABI/VM常量/XCON恢复统一接纳；Eq/Hash实际分配与释放均0 |
| 错误 | 033/034进入诊断/消息目录；033用于接纳，034只建立门面，实际精确除法留A2/A3；既有状态与挂起槽规则不改 |
| 完整示例 | 三份UseDocs完整代码与fixture逐字一致；VM精确1项，Windows原生精确1项，内部O0/O3共6次；新增说明仍planned |
| B1回归 | 契约9/9、资源17/17、VM/原生各唯一1 passed，28/28守卫 |
| workspace | Clippy全workspace/all-targets、workspace/benchmark --locked检查、docs/usedocs/layout/覆盖率通过；原有coverage warning保留 |
| 三平台 | 维护回归42094736112全绿；Windows/Linux/macOS原生实际日志均有A1六次成功标记；workspace 42094736157成功，均绑定39b2c1ea |

现有无状态返回数值C构造器用原来的挂起错误通路报告语义失败，LLVM在构造位置检查并传播；没有新C符号、状态或ABI。
VM执行文件的常量恢复拆到constants.rs，保留2500行门槛，未增加豁免或修改释放计划。
registry只为已批准示例追加rust.xiao-driver.tests路径；所有模块身份、状态与其他字段均未改变，cmath仍planned。

**A1本批已退出，下一入口为另批A2/A3；本轮不启动A2～A7。**
所有1.18/20产物仍仅内部验证，算术、转换矩阵、受控性能与整批文档/registry收口后置；B1旧721条F10未运行，22D/22C及19/11/22历史债务保持独立。
上述三平台CI是代码快照的证据，不冒充后续文档收口提交的CI。
