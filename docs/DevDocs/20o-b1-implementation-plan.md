# 20O-B1. 七库实现规划：前置、实现与证据

> **这是 B1 的实现前规划，不是 B1 产品代码。**
> B1 旧七项探测已由 ad176c3 的 maintenance 41275847931 三平台与默认 VM 精确执行验证。
> 本轮 FIX-A 已逐条收口 F1～F13 和 E-2，精确契约见 [B1 契约](20o-b1-freeze-draft.md)，
> 新增边界已由 4d34767 的 maintenance 41285853958 / workspace gates 41285853953 取证，
> 默认 VM 与三平台原生均实际执行 14 场景；逐作业链接与未覆盖项见 [FIX-A 记录](20o-b1-fix-a-contract-closeout.md#31-同一提交的远端逐作业证据)。
>
> 状态：**FIX-A 与 FIX-B 已收口；七库未实现（2026-11-01）**。
> FIX-B 的维护回归 41310926623 / 工作区门禁 41310926525 全绿，详见 [实际证据](20o-b1-fix-b-closeout.md)。
> FIX-A 保留 ABI 1.17 历史证据；独立 FIX-B 当前已实现 intrinsic 32 与 ABI 1.18，
> 不因本轮确认自动执行产品实现，不认领 19/11/22 债务。
> FIX-B 的可执行步骤、测试矩阵和交接见 [FIX-B 实现规划](20o-b1-fix-b-implementation-plan.md)。

## 一、当前证据与未完成项

### 1.1 已经有的

| 项 | 证据 |
| --- | --- |
| B1 默认 VM 探测 | maintenance [41285853958](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41285853958) / [默认作业](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41285853958/job/124786889391)：14 场景，精确目标恰好 1 passed |
| B1 三平台原生探测 | 同次维护回归 [Windows](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41285853958/job/124786889338) / [Linux](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41285853958/job/124786889309) / [macOS](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41285853958/job/124786889367)：各七旧+七新、各 1 passed |
| 同提交工作区门禁 | [41285853953](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41285853953)，head=4d347670cdfa05fac2f01888f9d5ff6e3a5db37e，success |
| 计数守卫 | `tools/gates/b1-probe-guards.ps1` 28/28；已修复 Ubuntu 上 `Get-Command bash` 多入口的路径 bug |
| 教程代码一致性 | `b1_tutorials_embed_the_executed_sources` 通过；七页教程仍为 `planned` |
| 文档门禁 | docs/usedocs/layout 通过 |

### 1.2 前置状态与七库剩余门槛

| 编号 | 阻塞 | 现状 | 退出方式 |
| --- | --- | --- | --- |
| E-1 | error_code 真实前置 | id 32、ABI 1.18 已实现并经本地/三平台验收，阻塞解除 | 独立证据见 FIX-B 收口记录 |
| E-2 | helper 导出面 | 已确认 B1 内联，只新增冻结 20 项；不加内部模块/私有语法 | IMP 核对真实模块导出集合与导入 |
| E-3 | 签名/错误/限额/性能验收 | F2～F12 已确认；Base64 两入口含必填 url_safe；限额/预算是契约而非成绩 | IMP 按契约实现并验证资源/性能 |
| E-4 | 教程执行与状态 | 七页 planned，示例已执行；还没有产品导出/移除/归档证据 | 实现批完成后再升级 verified |
| E-5 | 三平台产品证据 | 当前只有探测；产品函数尚不存在 | 实现批后的三平台维护回归与 job 链接 |

## 二、下一批的拆分

### B1-FIX-A：契约收口（本轮，只改文档/探测）

**当前进展**：F1～F13/E-2 已逐条确认；新增七项已随旧七项通过 41285853958 的默认 VM 与三平台原生精确目标。契约和探测不替代真实前置或产品，完整对账见 FIX-A 记录。以下保留该批职责：

1. 逐条答复 F1～F13；未答项明确延期，不能默认取建议值；
2. 冻结 `error_code` 的名称、位置、参数类型、所有权、错误表和读取通路；给出是否需要
   intrinsic/ABI 的结论；
3. 冻结 20 个公开入口的签名、必填/命名参数规则、错误表、资源限额和性能预算；
4. 确定 helper 策略（E-2），并写清导出面；
5. 补边界探测：当前 B0 未覆盖的极值、编码尾位、UTF-8 临界码点、CSV 转义上限等；
6. 更新 B1 冻结草案和教程的 planned 内容，不升级 `verified`。

**退出对账**：F1～F13/E-2 均直接确认，无代为选择或延期；契约唯一落点为 B1 文档，未写产品。
新增边界的本地与三平台远端取证已完成；完整产品资源/性能仍未证明，不把 14 场景通过当作产品实现或性能成绩。

### B1-FIX-B：error_code 前置（已实现并验收）

**执行细则**：[FIX-B 实现规划](20o-b1-fix-b-implementation-plan.md)（intrinsic/类型/VM/LLVM/
Runtime/ABI 1.18/测试/证据的逐层步骤）。本页只保留批次边界，不重复实现设计。

**目标**：把“源码可读错误 code”做成真实通路，而不是快照/清空模拟。

已确认范围：

- 全局 error_code(Error value, /) -> str；intrinsic 规划号 32，ABI 次版本 1.18；
- 类型检查登记 Error 参数类别与动态类型检查，既有 IR IntrinsicCall 携带稳定 id，VM 直接读指定错误对象；
- LLVM 调用 xiao_runtime_error_code(const XiaoValue*, XiaoAbiBytes*) 获得借用视图，
  保持输入存活，再经已有 string_new/str_owned 复制为拥有的 str；
- getter 不分配、不读写挂起槽，C 状态 0/1/2/3 与失败 out 不变；源级错误按 B1 §2.2 正常传播；
- 单独更新 intrinsic/ABI 契约、四处消费者和产物指纹/版本拒绝，不新增 std.errors 模块；
- 禁止 take/raise pending error → snapshot → clear 的模拟读取，也不能把“复制代码文本”混成成员访问。

**硬前置状态**：intrinsic 32、全局 error_code、C getter、VM/LLVM 和 ABI 1.18 已落地。
实际实现与三平台证据见 [FIX-B 收口](20o-b1-fix-b-closeout.md)；前置阻塞已解除，七库 IMP 另批开工。

**证据**：自定义/默认 code、嵌套 catch、返回值脱离绑定后仍有效、重复读取、cause/suppressed
不变、Fatal 边界、类型负例、VM/native 同输出/错误/清理；必须补 ABI 1.18 的 `.xiaoc`/`.xar`/
原生版本拒绝与三平台证据，并逐条通过 [借用视图验收表](20o-b1-freeze-draft.md#25-fix-b-借用视图的实现验收)。

### B1-IMP：七库实现（纯 Xiao）

前置：B1-FIX-A 契约收口；**B1-FIX-B 必须完成**。F13 已选新读取通路，不再是可选前置。

实现顺序建议：**math → hex → base64 → url → fmt → statistics → csv（记录子集）**。
理由：math/fmt/statistics 的状态最简单；hex/base64/url 验证字节与 UTF-8；csv 的状态机
最后做，且已有记录子集冻结和预算公式。

每个模块必须：

1. 在 `stdlib/<module>.xiao` 实现冻结签名；helper 遵守已冻结的 E-2 策略，不得偷偷导出
   未登记的辅助函数；
2. 复用 `tests/fixtures/docs/<module>/probe.xiao` 或新增完整示例，VM/native 独立黄金输出；
3. 加边界/资源/失败清理测试；错误 code 与 B1 冻结表一致；
4. 更新教程、stdlib README、module-registry、DevDocs 索引；
5. 做公开导入、逐函数移除、真实归档重放和普通包隔离验证；
6. 不新增 intrinsic、ABI、泛型约束或公开 `dynamic`。

**实现阶段的验收**：七库功能+错误表+资源限额；教程仍可 planned；三平台证据齐备后才
升级 UseDocs/registry 为 verified。

### B1-CLOSE：证据收口

1. 三平台维护回归：默认 workspace、VM 精确、原生精确、移除/归档/教程示例；
2. 完整 workspace gates、clippy、fmt、docs/usedocs/layout、覆盖率；
3. 用实际运行号和 job 链接更新 B1 文档；
4. 教程和 registry 状态从 planned 升 verified；
5. 记录未覆盖边界；不把本地结果写成三平台结论。

## 三、已确认的错误读取通路

F1/F13 已确认全局 error_code 及独立 intrinsic 32 / ABI 1.18；精确签名、所有权、C 状态、
源级错误表、借用视图和复制顺序唯一登记在 [B1 §二](20o-b1-freeze-draft.md#二error_code-的名称位置与签名)。
不再重新选择 std.errors、成员访问或文本匹配方案；变更契约才再次询问星崽。

## 四、已确认的 helper 与导出面

E-2 取内联：新增辅助逻辑写在公开函数体中，允许复用已交付标准库；不增加未登记的顶层
helper、不靠下划线或缺失注解制造“私有”，不新增内部模块、嵌套辅助函数或可见性语法。
通用私有可见性另立语言批次，不混入本轮。

精确 20 项导出集合见 [FIX-A §1.1](20o-b1-fix-a-contract-closeout.md#11-helper-与导出面)。
探测夹具中的局部 probe 函数不作为产品模块，不借此误报官方导出。IMP 必须检查真实签名表与用户导入。

### 4.1 IMP 的真实导出与代码规模验收

- 从本模块顶层 AST 定义与前端/加载器的实际接口表分别提取名称和完整签名，与冻结 20 项逐项比对；
  不能仅看 README、数量或源码正则。来自既有模块的导入与本模块新增定义分开检查，最终用户可见面不得额外泄漏。
- 对 std/stdlib 两种公开路径做真实导入；冻结入口成功，未登记 helper/下划线名/漏注解辅助函数不能绕过接口。
- 验证跨模块重名入口的别名导入和结果，例如 hex/base64 的 encode/decode、math/statistics 的 min/max；
  不能让静态链接因同名函数覆盖而悄悄调用另一个模块。
- 内联可能造成重复和大函数：执行现有文件规模门禁，评审函数体规模与重复段，不临时放宽门禁或增加未冻结 helper。
  若维护性/性能确需共享抽象，先提出 E-2 契约变更或独立语言批次；不能将 README 计数当作验收替代。

### 4.2 CSV 完整性能仍未证明

FIX-A 的 csv-budget 通过只证明短记录计数与 E 的算术边界；开发期全量大字段局部探测曾在
60 秒看门狗返回 X09-DRIVER-002。IMP 必须用真实 parse_record/format_record 和已确认最大
合法输入复测时间、内存、最坏转义与失败清理，不能把简单预算公式或旧通过日志替代这些结果。

## 五、已确认契约与实施验证表

| 模块 | 已确认的实现契约 | 实现后必须验证 |
| --- | --- | --- |
| math | 极值、gcd、pow 的溢出/指数边界；错误表 | MIN/MAX、gcd 极值、0^0、负指数、未使用平方不制造溢出 |
| hex | `[int]` 载体、大小写、严格解码、空输入、输出上限 | 256 字节、奇偶长度、坏字符、上限+1、输入别名不变 |
| base64 | 标准/URL-safe 两模式与必填 url_safe、严格补位/尾位、真实解码长度 | 两模式独立向量、混用/交叉拒绝、坏补位、空白与上限+1 |
| url | UTF-8 验证、保留字符、百分号大小写、`+` 语义、非法序列 | 0/ASCII/临界码点/代理/截断/孤立续字节、NUL/BOM、上限 |
| csv | 记录子集、严格引号状态机、空记录/零字段、BOM/行尾、E 预算 | 引号四状态、转义上限、字段数/字段字节、往返 |
| fmt | 码点宽度、fill 单码点、负宽度、2～36 进制、最小 int | 宽度 0/负/等长/上限、fill 校验、MIN 全进制 |
| statistics | sum 顺序/中间溢出、空 min/max 错误、输入只读 | 空/单项/极值/中途溢出/最大项数、输入未重排 |

## 六、测试与证据矩阵

- 每模块：成功黄金、边界、错误路径、资源上限、失败无部分输出、输入别名不变；
- 每模块：VM 与受控原生逐字 stdout/退出码/错误 code/清理；未捕获错误与 catch 恢复分开；
- 模块级：公开导入、未导入/错误命名空间拒绝、逐函数移除、真实 `.xar` 重放、普通包隔离；
- 教程：十项内容、完整示例在 CI、页面与 fixture 逐字一致；
- CI：默认 workspace、VM 精确、三平台原生精确、完整 gates；
- 证据必须带实际运行号和 job；没有三平台证据不升级 verified。

## 七、退出条件

1. F1～F13/E-2 已逐条确认并有唯一契约；不重问已确认裁定；
2. 独立 error_code 前置完成、ABI 1.18 兼容证据齐备，没有模拟读取/挂起槽污染；
3. 七库实现与冻结签名/错误表/限额一致，教程和示例齐全；
4. 移除、归档、普通包隔离和三平台证据齐全；
5. UseDocs/registry 状态与实际证据一致；
6. 未修改 22 并发虚拟模块、32B/16B 布局、普通句柄单线程规则或 19/11/22 债务。

## 八、落点

```text
docs/DevDocs/20o-b1-implementation-plan.md          本规划
docs/DevDocs/20o-b1-freeze-draft.md                 B1 精确冻结草案
docs/DevDocs/20o-official-library-program.md        工程总纲与交接
docs/DevDocs/20p-day1-library-architecture.md       Day-1 架构与教程标准
tests/fixtures/docs/<module>/probe.xiao             完整教程示例
stdlib/<module>.xiao                                七库产品实现
docs/UseDocs/library/<module>.md                    教程
docs/module-registry.json                           模块登记
core/rust/crates/xiao-driver/tests/20o_b1_probe.rs  探测/教程一致性
tools/gates/b1-probe-guards.ps1                     计数守卫
.github/workflows/maintenance-regression.yml        三平台精确执行
```
