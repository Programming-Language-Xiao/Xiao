# 20O-B1. 七库实现规划：前置、实现与证据

> **这是 B1 的实现前规划，不是 B1 产品代码。**
> B1 冻结草案和 CI 接线已落地，`245789b` 的三平台原生探测与默认 VM 探测已经全绿；
> 但 F1～F13 尚未答复，`error_code` 前置、私有 helper 策略和逐模块限额仍未冻结。
> 本文件把下一批拆成“契约收口 → 错误访问前置 → 七库实现 → 证据收口”，先解决阻塞，
> 不把 B1 草案或 CI 绿色当成产品实现授权。
>
> 状态：**规划中（2026-11-01）**。基线 `ad176c3`；D-1～D-26 已确认方向，F1～F13 待答。
> 不写产品函数、不新增 intrinsic/ABI、不认领 19/11/22 债务。

## 一、当前证据与未完成项

### 1.1 已经有的

| 项 | 证据 |
| --- | --- |
| B1 默认 VM 探测 | maintenance `41275847931` 的 Linux 默认作业：`b1_capabilities_have_vm_golden_outputs` 恰好 1 passed |
| B1 三平台原生探测 | 同一次 maintenance 的 Windows/Linux/macOS 原生作业：七项 `20O-B0-PROBE ... vm/native success`，各 1 passed |
| 计数守卫 | `tools/gates/b1-probe-guards.ps1` 28/28；已修复 Ubuntu 上 `Get-Command bash` 多入口的路径 bug |
| 教程代码一致性 | `b1_tutorials_embed_the_executed_sources` 通过；七页教程仍为 `planned` |
| 文档门禁 | docs/usedocs/layout 通过 |

### 1.2 仍然阻塞实现

| 编号 | 阻塞 | 现状 | 退出方式 |
| --- | --- | --- | --- |
| E-1 | `error_code` 契约 | 位置/签名只是候选；catch 绑定是 `Type::Dynamic`；需要类型/IR/VM/LLVM/Runtime 通路，可能新增 intrinsic/ABI | F1/F13 答复 + 前置实现批 + 证据 |
| E-2 | 公共 helper 策略 | Xiao 顶层函数默认进入模块接口，没有已冻结的 `private def`/内部模块可见性 | 选择内联、内部模块或私有可见性；写进契约并测试 |
| E-3 | 逐模块签名/限额/错误表 | B1 草案给出 20 项候选，F2～F12 未答；CSV 预算仅 P3-1 已确认 | 逐条答复并冻结 |
| E-4 | 教程执行与状态 | 七页 planned，示例已执行；还没有产品导出/移除/归档证据 | 实现批完成后再升级 verified |
| E-5 | 三平台产品证据 | 当前只有探测；产品函数尚不存在 | 实现批后的三平台维护回归与 job 链接 |

## 二、下一批的拆分

### B1-FIX-A：契约收口（先做，只改文档/探测）

**目标**：让 B1 从“待审草案”变成可实现的冻结契约，不写产品函数。

1. 逐条答复 F1～F13；未答项明确延期，不能默认取建议值；
2. 冻结 `error_code` 的名称、位置、参数类型、所有权、错误表和读取通路；给出是否需要
   intrinsic/ABI 的结论；
3. 冻结 20 个公开入口的签名、必填/命名参数规则、错误表、资源限额和性能预算；
4. 确定 helper 策略（E-2），并写清导出面；
5. 补边界探测：当前 B0 未覆盖的极值、编码尾位、UTF-8 临界码点、CSV 转义上限等；
6. 更新 B1 冻结草案和教程的 planned 内容，不升级 `verified`。

**退出**：F1～F13 有答复/延期记录；§二～§七的候选文本不再有“待定但按建议执行”的项；
helper 和 error_code 契约有唯一落点；没有产品函数。

### B1-FIX-B：error_code 前置（若 F1/F13 选择需要）

**目标**：把“源码可读错误 code”做成真实通路，而不是快照/清空模拟。

范围取决于 F1/F13：

- 若采用全局内建 `error_code`：类型检查特判、IR 节点/降低、VM 读取错误对象、LLVM Runtime
  ABI 读取、错误对象生命周期和释放计划；
- 若改为 `std.errors` 模块：额外模块身份、导出和归档闭包；
- 若需要新 intrinsic/ABI：单独冻结 id/符号/版本，升级 ABI 次版本；不得混入纯 Xiao 七库；
- 禁止“先 take pending error，再读 snapshot，再 clear”的模拟路径，因为它会改变嵌套
  catch/finally 的挂起错误。

**证据**：自定义/默认 code、嵌套 catch、返回值脱离绑定后仍有效、重复读取、cause/suppressed
不变、Fatal 边界、类型负例、VM/native 同输出/错误/清理；若改 ABI，补 `.xiaoc`/`.xar`
版本拒绝和三平台原生证据。

### B1-IMP：七库实现（纯 Xiao）

前置：B1-FIX-A 完成；若 F1/F13 要求 error_code 前置，则 B1-FIX-B 完成。

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

## 三、错误读取的设计选项（F1/F13）

| 方案 | 形态 | 代价 |
| --- | --- | --- |
| 甲 | 全局内建 `error_code(Error value, /) -> str` | 需要类型检查特判、IR/两后端/Runtime 通路；可能 ABI |
| 乙 | `std.errors.code(Error value, /) -> str` | 需要模块身份/导出/归档；读取通路同甲 |
| 丙 | 开放 `error.code` 成员访问 | 用户已拒绝；不重新引入 |
| 丁 | 只提供格式化文本/内部诊断 | 不满足 Day-1 机读错误，拒绝 |

无论甲乙，都不走“take pending error + snapshot + clear”的模拟；错误对象必须能被
独立读取且不改变挂起槽。若实现需要新增 Runtime C ABI，必须单独冻结版本和错误身份。

## 四、helper 与导出面（E-2）

当前事实：Xiao 顶层函数会进入模块接口；没有已冻结的 `private def`/模块私有可见性。
候选：

- **甲：内联**：公开函数自包含，不增加导出；代码重复、维护成本高，适合小算法；
- **乙：内部模块**：如 `stdlib._b1`，但用户仍可能导入；需要 loader/typecheck 的保留内部
  命名空间规则，否则只是“约定私有”；
- **丙：语言私有可见性**：新增 `private def` 或等价语法；影响面最大，但对所有后续官方库
  都长期有用。

**建议**：先评估甲/乙；若多个算法需要共享状态机/表，选乙并加“内部命名空间禁止用户
导入”的负例，或把丙单独立语言批次。不得让内部 helper 悄悄成为公开 API。

## 五、逐模块冻结检查表

| 模块 | 实现前必须确认 | 实现后必须验证 |
| --- | --- | --- |
| math | 极值、gcd、pow 的溢出/指数边界；错误表 | MIN/MAX、gcd 极值、0^0、负指数、未使用平方不制造溢出 |
| hex | `[int]` 载体、大小写、严格解码、空输入、输出上限 | 256 字节、奇偶长度、坏字符、上限+1、输入别名不变 |
| base64 | 标准字母表、强制补位、尾位、严格解码、真实解码长度 | RFC 向量、任意字节、坏补位/URL-safe、上限+1 |
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

1. F1～F13 有答复或明确延期；helper/error_code 契约有唯一落点；
2. error_code 前置（若需要）完成，且没有模拟读取/挂起槽污染；
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
