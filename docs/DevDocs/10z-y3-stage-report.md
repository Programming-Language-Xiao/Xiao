# 10Z-Y3. Y3 阶段汇报与收尾对照

> **这是什么**：`10Z-收尾` 的 Y3（B 系列逐条分类）阶段汇报，以及四组候选的最终对照记录。
> 本文把 VM、原生构建和产物运行放在同一份证据链中；前一版把字典成员读取误写成“原生编译期拒绝”，
> 本次自审核已按真实运行结果更正。
>
> **一句话概括**：动态成员读写和动态扩展赋值在表形态下两边输出一致；非表形态两边都拒绝，
> 但拒绝阶段/错误身份仍按口径待定。合法的五种选择形态均有选择计划且原生输出与 VM 一致，
> `动态选择器缺少规范选择计划` 暂判为防御性不可达；动态错误构造参数另有一组待定对照。
>
> 状态：**Y3 对照完成，待星崽定拒绝阶段口径和 Z-1 范围**。按 [10Z-收尾](10z-closeout-execution.md) §2.1 的 S1，
> 本阶段不开工实现。

## 一、给星崽的汇报

**Y3 的现状**：`7d41d98` 已把 28 处权威枚举逐行填上最小源码 / VM 结果 / 分类 / 可达性
（审核已抽 4 行独立复跑，逐字吻合）。其中：

| 处置 | 条数 | 说明 |
| --- | --- | --- |
| 两边一致拒绝 / 独立边界 | **21** | 含 A3 与构造参数形状（引用既有结论，不重复取证） |
| `dynamic/` 外的独立项 | **4** | 目标平台与 IR 层边界，[10Z-收尾](10z-closeout-execution.md) §2.2 第 3 条已要求单独判定 |
| **Y3 原生缺口候选** | **4 组（对应 5 处），均已完成对照** | ①/② 的表形态一致；非表形态两边均拒绝但阶段或错误身份不同，口径待定；③ 未发现无计划 IR；④ 待定 |

候选组按语义合并，成员读取和成员写入的两处权威字符串合为①组；权威枚举仍是 28 处，表中的“条数”和“组数”不直接相加。
四组候选的最终对照如下。所有原生命令均在同一台 Windows 受控工具链上执行；环境见 §2.1.1，原始输出见 §2.3.1。

### ① 动态成员读写（`expression.rs:735` / `:752`）—— **表形态一致；非表形态均拒绝**

表接收者读取的最小源码为：

```xiao
[[Item]]
    member = 7
    def init(self) -> none
        self.member = 7
item = new Item()
def read(value) -> int
    return value.member
print(read(item))
```

VM 的 `--json` 原始结果含 `"exit_code":0` 和 `"intrinsic_output":{"text":"7\\n"}`；
受控原生 `build` 返回 `"operation":"build","exit_code":0`，运行同一产物的原始 stdout 为
`7`，退出码 0。写入形态把 `read` 换成 `value.member = 42` 并打印字段，VM 和原生均输出
`42`、退出码 0。因此**表形态按 §2.2 第一行判定：不是缺口**。

自审核又把非表接收者拆成了两个可复现形态，修正了上一版“原生统一编译期拒绝”的错误：

```xiao
# 字典接收者：命中 IrType::DictTable 的 Runtime 读取路径
def read(value) -> int
    return value.member
print(read({member = 1}))

# 标量接收者：命中 expression.rs:735 的 Unsupported
def read(value) -> int
    return value.member
print(read(1))
```

字典读取的 VM 原始 JSON 为 `X06-RUNTIME-002`、消息“期望类型 table，实际为 dict_table”、退出码 3；
原生 `build` 成功，但带诊断启动 shim 的产物运行原始 stderr 为
`xiao-error class=recoverable code=X06-RUNTIME-012 message_id=runtime.invalid_value ... exit_code=3`，
退出码 3。标量读取的 VM 同样为 `X06-RUNTIME-002`、退出码 3；原生 `build` 原始 stderr 为
`X11-PROTOCOL-007: 原生后端不支持 动态非表成员访问（34..46）`，构建退出码 2。

代码原因是 `emit_table_get` 先对 `IrType::Dynamic | IrType::DictTable` 发射
`xiao_runtime_dict_get`，只有之后才进入“动态非表成员访问”守卫；调用点的参数推断会分别得到
`DictTable` 或标量。因此本组的结论是：表形态不是缺口；非表形态都没有“VM 成功、原生拒绝”
的结果，但拒绝阶段或错误身份有差异，归入待定决策 1，不能再写成单一分歧。


### ② 动态扩展赋值（`control.rs:206`）—— **表形态已对照；非表形态两边均拒绝**

表形态探针为：

```xiao
[[Item]]
    member = 0
item = new Item()
def write(value) -> none
    value.member = 42
write(item)
print(item.member)
```

VM 的 `--json` 原始结果含 `"exit_code":0`、`"intrinsic_output":{"text":"42\n"}`；
原生 `build` 返回 `"operation":"build","exit_code":0`，产物原始 stdout 为 `42`，退出码 0。
按 §2.2 第一行，表形态不是缺口。

字典形态仍使用最小探针 `write({member = 0})`：VM 原始 JSON 为
`X06-RUNTIME-002`、退出码 3；原生 `build` 原始 stderr 为
`X11-PROTOCOL-007: 原生后端不支持 动态非表字段写入（29..45）`，构建退出码 2。
按 §2.2 第二行，这是“两边都拒绝”；是否把运行期语言错误与编译期能力边界视为差分，
与①共用待定决策 1。

### ③ 动态选择计划缺失（`expression.rs:270`）—— **前端可达性已证伪，暂判防御性不可达**

一次源码覆盖五种合法形态，第三方可原样复现：

```xiao
values = [1, 2, 3, 4]
part = values[1~2]
print(part[0] + part[1])
random_part = values[?2]
print(random_part[0] + random_part[1])
all_part = values[=]
print(all_part[3])
open_part = values[<2]
print(open_part[0] + open_part[1])
multi_part = values[0, 2]
print(multi_part[0] + multi_part[1])
```

VM `--json` 原始输出事件依次为 `5\n`、`7\n`、`4\n`、`3\n`、`4\n`，退出码 0；
原生 `build` 返回退出码 0，运行产物原始 stdout 完全相同，退出码 0。前端额外探针使用同一
源码调用 `FrontendCompiler`，检查 `artifact.ir.selection_plans` 以及每个选择表达式的
`selection_plan` 调试字段，原始测试输出为：

```text
selection_plans=5 selector_plan_references=5
test legal_selector_forms_always_carry_a_plan_reference ... ok
test result: ok. 1 passed; 0 failed
```

合法源码没有产生 `selection_plan: None`；类型检查器只在集合索引或其他前端诊断已经拒绝时
提前返回，正常选择路径都会追加计划，IR lowering 再按源码区间取得该计划。因此
`expression.rs:270` 当前没有可由合法源码触发的缺口，先登记为防御性守卫，不改后端。

### ④ 动态错误构造参数（`expression.rs:670`）—— **VM 接受，原生编译期拒绝；口径待定**

最小源码为：

```xiao
code = "DYNAMIC"
raise ArithmeticError(code = code)
```

VM `--json` 原始结果为用户错误码 `DYNAMIC`、`runtime.user_error`、退出码 3；
原生 `build` 原始 stderr 为
`X11-PROTOCOL-007: 原生后端不支持 动态错误构造参数（当前只支持字符串字面量）（46..50）`，
构建退出码 2。VM 没有正常值输出，但已接受动态参数并运行到用户错误；原生在产物前拒绝。
按 §2.2 的严格可观察行为口径，这不是“VM 成功值/原生拒绝”的第三行，但也不能抹成同一错误：
它与①/②共用待定决策 1；若按“拒绝阶段和错误身份必须一致”，应单独登记为原生错误 ABI 缺口。

## 二、对照的施工规格

### 2.1 两侧都要实跑，原生侧用 `xiao build`

**原规格只有 VM 侧方法**（`bun cli/ts/src/main.ts --json run …`），**没有原生侧方法**——
这正是「对照」迟迟没做掉的原因：没人说清原生侧怎么观察。补上：

```text
# VM 侧
bun cli/ts/src/main.ts --json run <目录>/main.xiao

# 原生侧（需要 10D §4 的受控环境）
bun cli/ts/src/main.ts build -o <输出>.exe <目录>/main.xiao
<输出>.exe            # 运行产物，观察真实输出与退出码
```

原生环境的准备按 [10D](10d-environment-gated-test-spec.md) §4；`xiao build` 失败（非 0 退出）时
**把它的完整 stderr 记下来**——那就是「编译期拒绝」的证据形态。

### 2.1.1 本次受控环境

以下特征由同一台 Windows 主机在取证时直接输出，原生构建使用 MSVC 环境脚本、
MSYS2 UCRT64 clang 和 `core/rust/target/release/xiao_runtime.lib`：

```text
Microsoft Windows [Version 10.0.26200.9168]
bun 1.4.0
rustc 1.96.0 (ac68faa20 2026-05-25)
host: x86_64-pc-windows-msvc
clang version 22.1.2 (MSYS2 UCRT64)
Microsoft (R) Incremental Linker Version 14.51.36256.0
```

仓库没有安装名为 `xiao` 的独立 shim，本次命令按同一入口实际执行：
`bun cli/ts/src/main.ts --json build -o <out>.exe <dir>/main.xiao`；
这与文档中的 `xiao build` 路由相同。未导入 `vcvars64.bat` 时曾得到
`clang: error: linker command failed with exit code 1120`，该环境失败没有被当成探针结论；
导入后再以 release Runtime 重跑，表格中的成功/拒绝才作为证据。

### 2.2 判定口径（三选一，与前表和 §2.2 一致）

| 观察 | 判定 |
| --- | --- |
| VM 成功产出结果 **且** 原生产出同值 | **两边一致，不是缺口** |
| VM 与原生**都拒绝**该程序 | **两边都拒绝**（编译期/运行期之别是否算差异，见待定决策 1） |
| VM 成功产出结果 **而** 原生拒绝构建 | **原生缺口**，登记并排实现 |

### 2.3 证据要求

1. **两侧都要有实跑输出**（VM 的 `--json` 摘要、原生的构建结果 + 产物实际输出）；
2. **贴带机器特征的原始文本**——不得只写摘要（[10Z-收尾](10z-closeout-execution.md) §1.3(6) 的教训：
   规格里给过预期输出时，摘要不构成证据）；
3. 探针源码**写进文档或提交说明**，使第三方可原样复现；
4. 若判为「不是缺口」，**同样要记**——把候选降级为一致，是结论的一部分，不是没做事。

### 2.3.1 已实跑原始行

本次原始 CLI 行的关键字段如下（请求编号和 Windows 路径均保留，未用预期值代替实际结果）。
完整的 VM JSON、原生构建 stderr/stdout 和产物运行输出见[原始对照日志](10z-y3-stage-report-log-20261010.md)，
下面的表只做索引，不能替代日志原文：

```text
VM table-read:  run-mv179on9-yjl3t6f6  exit_code=0  intrinsic_output=7\n
VM table-write: run-mv17i0ng-jkxr1vmv  exit_code=0  intrinsic_output=42
VM selector-plan: run-mv17yql2-pbnpqko5  exit_code=0  intrinsic_output=5
                  7
                  4
                  3
                  4
VM dict-read:   run-mv18e7li-629tkfyn  X06-RUNTIME-002  exit_code=3
VM dict-write:  run-mv17sxia-6eyx7r19  X06-RUNTIME-002  exit_code=3
VM scalar-read: run-mv18i8ug-w9dh2r2m  X06-RUNTIME-002  exit_code=3
VM dynamic-error: run-mv197nb0-8y7dh0ee  user_error=DYNAMIC  exit_code=3

native table-read:   build-mv17nxqt-viqg5gdq  build exit_code=0; stdout=7; run exit_code=0
native table-write:  build-mv17mkg4-meu5g0qx  build exit_code=0; stdout=42; run exit_code=0
native selector:     build-mv17znsi-hp93zx56  build exit_code=0; stdout=5\n7\n4\n3\n4\n; run exit_code=0
native dict-read:    build-mv18f2qy-9hrvj77  build exit_code=0; debug run X06-RUNTIME-012; exit_code=3
native dict-write:   build-mv17tt0y-9vqryi24  X11-PROTOCOL-007 动态非表字段写入; build exit_code=2
native scalar-read:  build-mv18j5az-2svb245j  X11-PROTOCOL-007 动态非表成员访问; build exit_code=2
native dynamic-error: build-mv198imb-y2v0yt6m  X11-PROTOCOL-007 动态错误构造参数; build exit_code=2
```

上述每一行都来自对应命令的 stdout/stderr；构建成功后又单独启动了输出中的 `.exe`，
没有把“构建成功”当成语义成功。

### 2.4 落点与顺序

1. ②③④ 三组已补齐两侧对照，并回写 [10P](10p-differential-exemption-and-coverage.md)；旧判定保留，
   新增列说明本次证据和改判依据；
2. ① 组已按表、字典、标量三种接收者形态拆开记录；表形态一致，非表形态的阶段/错误身份口径待定；
3. 四组对照已经完成，**停在汇报节点**；星崽确认拒绝阶段口径后再定 Z-1 范围。

## 三、不要做的事

- **不改后端**：本阶段只分类；判为缺口也要先汇报，实现按 [10Z](10z-19-closeout-and-b-series-implementation.md) §2.2 走；
- **不重跑已有结论**：A3、构造参数形状、以及审核已实测的①组表形态，引用即可；
- **不要开新的工具侧批次**（[10Z-收尾](10z-closeout-execution.md) §2.3 的冻结仍然有效）；
- **不要用 VM 单侧结果冒充对照**——本组的整个教训就是「只有 VM 侧方法」；
- **不要为了让候选组变少而放宽判定口径**（§2.2 的三行表）。

## 四、最可能翻车的地方

1. **只跑 VM 侧**就写「原生缺口」（原规格的口子）；
2. **用字典形态冒充表形态**：①组的探针原本就是 `{member = 1}` 字典，命中 `dict_table` 边界，
   与「动态表成员访问」不是一回事；
3. **只看构建成功就下结论**：还要**运行产物**看实际输出——`xiao build` 成功不等于语义正确；
4. **把编译期拒绝直接当成缺口**（或直接当成一致），不区分「两边都拒绝」的两种形态（待定决策 1）；
5. **③组按①②的模板跑**：它要判的是「前端会不会产出无计划 IR」，不是「接收者是什么类型」；
6. **摘要当证据**（§2.3 第 2 条）；
7. 顺手改后端或开新批（§三）。

## 五、验收

1. ②③④ 三组各有**两侧实跑**的最小源码与输出；① 组按审核实测更新并标明表/非表三种形态；
2. 每条判定都能对上 §2.2 的三行表之一，且写明依据；
3. 结论与探针源码都在文档或提交说明里可复现；
4. 10P 的旧判定未被抹掉，改判附依据；
5. 没有改后端、没有开新批；
6. `bun run check` / `cargo test --workspace` / `bun test` / `bunx tsc --noEmit` 全绿。

## 待定决策

1. **非表接收者的拒绝阶段/错误身份算不算缺口？**（①组的字典读取、标量读取和②组字典写入共用）
   - **算一致**：两边都拒绝该程序；编译期拒绝、运行期拒绝以及同类运行期错误码的差异只登记，不进入 B 实现。
   - **算缺口**：按差分测试“四样都要比”的口径，阶段或错误身份的可观察差异需要单独消除。
   审核**建议先按“算一致”处理**：这些程序都没有 VM 成功结果，原生的拒绝是既有能力边界；
   把所有非表接收者的阶段差异一律列为缺口会把范围放大到整个拒绝面。若星崽要求严格对齐，
   再把具体形态分别立项，不能把字典读取的 `X06-RUNTIME-002`/`X06-RUNTIME-012` 与标量编译期拒绝混为一项。
2. **Z-1 的范围**：四组对照已完成，待上项口径确认。**建议**只对「VM 成功产出结果而原生拒绝」的条目排实现；
   本次四组没有发现这类结果，若按建议口径则只改文档。
3. **③组若判为防御性不可达**：是否要给那处 `Unsupported` 加一条守卫/注释，标注「前端不会产出该形态」？
   （属实现改动，本阶段不做，先登记。）

## 相关页面

- [10Z-收尾. 10 系列收束](10z-closeout-execution.md) —— 本批的序列与 S1 入口条件；§1.0 是上一轮审核补正
- [10P. 差分豁免收敛与剩余拒绝面](10p-differential-exemption-and-coverage.md) —— Y3 的回写对象与 28 行分类表
- [10X. B 系列取证与分类](10x-b-series-triage.md) §2.2 —— 分类方法与三分类口径
- [10Z. 19 收口](10z-19-closeout-and-b-series-implementation.md) §2.2 —— 判为缺口后的实现范围口径
- [10D. 环境依赖测试专项规范](10d-environment-gated-test-spec.md) §4 —— 原生侧环境准备
