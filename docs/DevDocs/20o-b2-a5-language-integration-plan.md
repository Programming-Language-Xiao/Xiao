# 20O-B2-A5. 类型、IR、TAC/VM/LLVM 全链路实施规划

> **这是 A5 的全链路实施规划；不写产品代码，不执行构建或测试。**
> 入口为A4代码快照`1838fae1`及A4有效收口；A5尚未开始，A6性能、A7整批收口后置。
> A5只把A1～A4的Runtime能力接入语言全链路；不新增ABI/intrinsic，不实现`lfloat_divide`/`cmath`。
> A0的`result.json`失败身份仍保留；A0只以`closeout.json`作为有效收口。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| A1 | 依赖、033/034、值身份、入口接纳和Eq/Hash已退出 |
| A2/A3 | Runtime算术内核已退出（21项） |
| A4 | Runtime转换内核已退出（17项）；自审12,092次比较0差异 |
| 代码入口 | A4产品代码`1838fae1`；A4自审修复`6ebdd5b1`；flake修复`13ecd687` |
| 版本 | ABI 1.18、`CODEGEN_VERSION` 20、活动intrinsic最大32、`cmath` planned |
| 数据 | `Lint`/`Lfloat`文本标签保留；语言字典仍为str键；数值教程范围仍planned |
| 后续 | A5全链路；A6资源/性能、A7整批收口后置 |

A5的产品边界是**语言全链路接线与差分**，不是A6性能或A7整批交付。A1～A4内核已在Runtime层通过；A5未完成前不能声称VM/LLVM语言路径完成。

### 1.1 要做

1. 类型检查与IR使用同一套已冻结的提升/转换规则；
2. TAC/字节码承载高精度运算和常量，保持现有标签/版本；
3. VM调用A1～A4共享Runtime内核，不另写近似实现；
4. LLVM动态Runtime调用同一内核，不用Rust布局、f64或宿主libm替代；
5. 序列化/并发值承载沿用既有文本标签和XCON边界；
6. 源码位置、可恢复错误、资源释放和失败原子性贯通两后端；
7. 完成VM/native语言级差分、三平台和A1示例回归；
8. 不把旧721条F10写成A5当前成绩。

### 1.2 明确不做

- 不实现A6性能、A7整批收口或B2-C的721条重跑；
- 不新增ABI/intrinsic，不分配33～48，不改C ABI符号/布局/状态；
- 不实现`lfloat_divide`、`cmath`或B2-B的ABI 1.19/21；
- 不重写定宽float幂或提前接入libm cmath内核；
- 不开放新语言类型、新容器模型或新错误成员访问；
- 不把能catch的009/033/034折叠成不可catch的编译错误。

## 二、开工闸门

A5执行前必须确认：

1. A4有效收口、代码快照和自审证据一致；A5未开始；
2. A1～A4的依赖、诊断、适配层、算术、转换和B1回归可复现；
3. 当前ABI1.18/生成器20/活动intrinsic最大32/cmath planned未变；
4. AST/IR/TAC现有大数文本和标签可承载，不需要新增ABI/tag；
5. B1契约/资源基线可复现；旧721条F10仍标历史；
6. 唯一A5任务目录和证据目录已创建，不覆盖A0～A4归档；
7. 若必须改XCON、C ABI状态、intrinsic或序列化schema，先回B2-FREEZE。

## 三、分批执行

### A5-1：类型检查与IR

1. 类型检查/IR对lint/lfloat算术、比较和转换使用同一提升规则；
2. 混合类型按F-4/既有规则；仅提升到float/sfloat的组合继续拒绝；
3. 大数字面量和常量保持原始文本，不在前端窄化为i128/f64；
4. 静态非法类型与运行时值错误分开；错误身份不新造；
5. 不因运算符结果未使用而删除可报错调用。

退出：类型检查与IR对A2/A3/A4路径一致；无未登记提升或窄化。

### A5-2：TAC/字节码与常量

1. TAC/字节码使用现有标签/版本承载lint/lfloat运算、常量和转换；
2. 不增加intrinsic id，不改指令流版本；
3. 大数常量保留完整文本，不通过f64或显示近似；
4. 序列化/反序列化沿用现有文本标签/XCON v1；不新增schema；
5. 失败构造、非法载荷和Eq/Hash前置不变量在入口检查；
6. 若现有编码无法安全承载，停止并回冻结，不自建替代接口。

退出：TAC/字节码/常量/序列化承载A1～A4路径；版本和标签未变。

### A5-3：VM全链路

1. VM调用A1～A4共享内核，不复制另一套算术/转换；
2. 运算、转换和常量恢复的错误经既有可恢复通路；
3. 失败无部分结果、输入不变；资源/预算按F-12/F-3；
4. 源码位置、挂起错误、finally/清理路径与既有语言契约一致；
5. 并发/XCON承载使用既有边界，不改22类协议或排序；
6. 禁止f64、字符串比较或默认Display替代。

退出：VM语言路径覆盖A2/A3/A4；错误、资源、清理和并发承载稳定。

### A5-4：LLVM动态Runtime全链路

1. LLVM调用与VM相同的Runtime内核和C ABI边界；
2. 大数常量/运算/转换降低后仍保留精确文本，不暴露Rust布局；
3. 可恢复错误在调用位置检查并传播，不能被优化器删除；
4. 数值构造失败、挂起错误和清理路径与VM一致；
5. 不新增C符号/状态；若需要，回冻结；
6. 不用宿主f64/libm凑高精度结果。

退出：LLVM与VM语义一致；无ABI/布局泄漏；错误/清理路径稳定。

### A5-5：语言级差分与三平台

1. 对A2/A3/A4每个运算和转换建立VM/native同一语言用例；
2. 覆盖混合提升、//%、幂、精确/非终止除法、转换、范围/资源错误和输入不变；
3. 三平台Windows→Linux→macOS实际执行；未执行平台明确标未验证；
4. A1三个完整示例和B1契约/资源回归；
5. 不运行旧721条F10；不写A6性能成绩；
6. 保存源码、命令、日志、平台、产物和摘要。

退出：A2/A3/A4语言路径在两后端和三平台有一致证据；未执行平台不改写成通过。

### A5-6：证据与收口

1. 汇总类型/IR/TAC/VM/LLVM/序列化/并发证据；
2. 记录实际CI运行、平台作业、失败样本和独立债务；
3. 复核没有ABI/intrinsic/XCON/registry/cmath变化；
4. 明确A6/A7和B2-B/C未开始；
5. 形成A6入口，B2-A整批仍未收口。

退出：A5全链路证据完整；A6性能、A7收口仍未开始。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 类型/IR | 提升、混合类型、拒绝组合、大数字面量、静态/运行时错误分开 |
| TAC/字节码 | 常量文本、标签/版本、指令流、序列化/XCON、无新intrinsic |
| VM | 算术/转换、错误/源码位置、资源/清理、并发/XCON |
| LLVM | 同一内核、常量/转换、错误检查、无布局泄漏、清理 |
| 差分 | A2/A3/A4每个语言用例VM/native一致；混合/边界/资源 |
| 平台 | Windows→Linux→macOS实际执行；未验证平台明确标注 |
| 回归 | A1示例、B1契约/资源；旧721条不运行 |
| 边界 | 无ABI/intrinsic/XCON/registry/cmath变化；A6/A7未开始 |

## 五、退出条件

1. A2/A3/A4全部路径通过类型/IR/TAC/VM/LLVM语言级用例；
2. VM与native结果、错误、资源和清理一致；
3. 大数常量/转换不窄化、不f64、不默认Display；
4. 三平台实际执行或明确未验证；未执行平台不写成通过；
5. 未新增ABI/intrinsic/XCON/registry/cmath；
6. B1契约/资源回归通过；旧721条F10未运行；
7. A6/A7未开始；A5证据完整；
8. B2-A整批尚未收口。

## 六、风险与禁止

- 只在Runtime层通过，却声称语言/VM/LLVM完成；
- 前后端各自实现一套算术或转换；
- TAC/字节码丢大数文本、窄化或改标签；
- 把可恢复错误折叠成不可catch的编译错误；
- 序列化/并发路径绕过接纳或改变XCON；
- 为接线新增C符号/状态/intrinsic而不回冻结；
- 用f64、字符串比较或宿主libm凑结果；
- 运行旧721条F10或把A4/CI成绩写成A5/A6。

## 七、交接

A5退出后下一入口是A6资源/性能：按F-3测量VM/原生、内存和翻倍比；A7再做整批文档/registry/教程与三平台收口。B2-B/C仍后置，B2-C才完整重跑721条F10。

新Agent先读：

1. [B2-A实施规划](20o-b2-a-implementation-plan.md)；
2. [A4转换实施记录](20o-b2-a4-conversions-progress.md)与[A4自审](20o-b2-a4-self-review.md)；
3. [A2/A3实施记录](20o-b2-a2-a3-arithmetic-progress.md)；
4. [冻结草案](20o-b2-freeze-draft.md)§五～§九；
5. [F-3资源/性能](assets/20o-b2-freeze-c/entry-20261105/README.md)与[F-12错误表](assets/20o-b2-freeze-b3/entry-20261105/README.md)。

## 八、落点

```text
docs/DevDocs/20o-b2-a5-language-integration-plan.md  本规划
docs/DevDocs/20o-b2-a-implementation-plan.md         A0～A7总规划
core/rust/crates/xiao-types/src/...                  类型/IR提升
core/rust/crates/xiao-ir / xiao-bytecode/...         TAC/字节码
core/rust/crates/xiao-vm/...                         VM语言路径
core/rust/crates/xiao-codegen-llvm/...               LLVM降低
core/rust/crates/xiao-runtime/src/value/             A1～A4共享内核
core/rust/crates/xiao-runtime/src/{abi,serialization,vm_concurrency}.rs 承载
docs/DevDocs/assets/20o-b2-a5/entry-<date>/          A5证据（待建）
```

本页只规划A5；产品代码仍须按已批准范围另批执行。
