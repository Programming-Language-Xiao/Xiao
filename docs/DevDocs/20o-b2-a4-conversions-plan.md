# 20O-B2-A4. 转换矩阵实施规划

> **本页保留A4原实施规划；规划提交当时未写产品或执行构建测试。当前见[转换实施记录](20o-b2-a4-conversions-progress.md)。**
> 入口为A2/A3代码快照`5ef17bb2`及flake修复`13ecd687`；A4内核已完成本批验收，代码CI通过；首轮22C失败及重跑记录保留，A5～A7后置。
> A4只实现Runtime转换内核；完整类型/IR/TAC/VM/LLVM/语言路径由A5接线验收。不新增ABI/intrinsic，不实现`lfloat_divide`/`cmath`。
> A0的`result.json`失败身份仍保留；A0只以`closeout.json`作为有效收口。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| A2/A3 | Runtime算术内核已退出；21项内核、B1回归和代码CI通过 |
| 代码入口 | A2/A3 `5ef17bb2`，后续测试隔离修复`13ecd687` |
| 版本 | ABI 1.18、`CODEGEN_VERSION` 20、活动intrinsic最大32、`cmath` planned |
| 数据 | `Lint`/`Lfloat`文本标签保留；A1适配层和A2/A3内核可用 |
| 字典/教程 | 语言字典仍为str键；新增数值教程范围仍planned |
| 后续 | A4内核已验收；下一入口A5全链路，A6性能、A7收口后置 |

A4的产品边界是**Runtime转换矩阵**，不是完整语言转换。A5未汇合前不能声称VM/LLVM语言路径完成。

### 1.1 要做

1. `int`/`sint` ↔ `lint`：精确或009范围错误；
2. `lint`/`lfloat` → `str`：已冻十进制显示；
3. `str` → `int`/`sint`/`lint`：保留单码点契约；仅`str`→`lfloat`按F-11数值语法，恢复lint数值须显式经`lfloat`→`lint`；
4. `float`/`sfloat` ↔ `lfloat`：实际二进制值、最近偶数、下溢/溢出、带符号零按F-8；
5. `lfloat` → `int`/`sint`/`lint`：向零截断，定宽范围和大整数资源分开检查；
6. 转换后整数结果显示scale=0；同类型lfloat保持显示精度；
7. 失败原子性和错误身份按F-12，不隐式窄化，不用宿主`f64`决定十进制舍入。

### 1.2 明确不做

- 不实现A2/A3未覆盖的算术扩展、A5全链路、A6性能或A7收口；
- 不开放`str → float/sfloat`直接转换；
- 不新增intrinsic、不分配33～48、不改C ABI符号/布局/状态；
- 不实现`lfloat_divide`、`cmath`或B2-B的ABI 1.19/21；
- 不改XiaoValue标签、XCON格式、语言字典键或registry状态；
- 不运行B1旧721条F10，不把内核测试写成VM/native/三平台完成。

## 二、开工闸门

A4执行前必须确认：

1. A2/A3有效收口、代码快照和A4入口源码树一致；A4未开始；
2. A1六包锁、033/034、适配层和A2/A3内核可复现；
3. 当前ABI1.18/生成器20/活动intrinsic最大32/cmath planned未变；
4. B1契约/资源基线可复现；旧721条F10仍标历史；
5. 唯一A4任务目录和证据目录已创建，不覆盖A0/A1/A2/A3归档；
6. 任何未冻结语义、错误、ABI或intrinsic变化先停并回B2-FREEZE。

## 三、分批执行

### A4-1：整数族与str

1. `int`/`sint` ↔ `lint`精确；越界009；
2. `str` → `int`/`sint`/`lint`保留单码点契约，不新增数字字符串直接parser；
3. `lint` → `str`按规范/显示身份输出；失败无部分结果；
4. 窄化不做隐式转换；输入不变、错误身份按F-12；
5. 测试前导零/零/负零/最大定宽/上限+1。

退出：整数族↔lint和lint→str内核通过；越界/非法输入边界稳定。

### A4-2：lfloat与文本

1. `str` → `lfloat`按F-11语法解析，拒绝非法语法/超预算；
2. `str`数字恢复lint必须显式经`lfloat` → `lint`，不新开直接入口；
3. `lfloat` → `str`保留显示精度；数字10默认显示scale=0；
4. 同类型lfloat保持显示精度；
5. 解析/输出往返、规范/显示scale、零/负零、强消去测试。

退出：str↔lfloat和lfloat→str内核通过；不开放直接str→float/sfloat。

### A4-3：浮点族与lfloat

1. `float`/`sfloat` → `lfloat`按实际IEEE位模式，不用显示文本近似；
2. 二进制±0规范为lfloat零；最近偶数、下溢/溢出按F-8；
3. `lfloat` → `float`/`sfloat`按最近偶数、下溢和带符号零；范围/资源错误按F-8/F-12；
4. 不承诺再转回保持负零位模式；
5. 不修改A3算子私有提升；A4公开转换矩阵独立测试。

退出：float/sfloat↔lfloat实际值、舍入、下溢和零边界稳定。

### A4-4：lfloat到整数

1. `lfloat` → `int`/`sint`/`lint`向零截断；
2. 定宽范围错误009；大整数资源033；失败无部分结果；
3. 整数结果scale=0；不改变显示身份；
4. 最大D/W/H、上限+1、强消去和符号边界测试。

退出：lfloat→整数转换的范围/资源/符号边界稳定。

### A4-5：回归与证据

1. A4转换内核单元/边界/确定性测试；
2. A1适配层、A2/A3内核和B1契约/资源回归；
3. 不运行旧721条F10；
4. 保存源码、命令、日志、平台和摘要；
5. A5全链路、A6性能、A7收口仍未开始。

退出：A4内核测试和B1回归通过；A5未开始。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 整数族 | int/sint↔lint、越界、上限+1、单码点str、scale=0 |
| 文本 | str→lfloat语法、lfloat→str显示、不允许str→float/sfloat |
| 浮点 | float/sfloat↔lfloat实际值、最近偶数、下溢/溢出、±0 |
| 截断 | lfloat→int/sint/lint向零、范围009、资源033 |
| 身份/显式 | 同类型lfloat显示精度、规范/显示scale、无隐式窄化 |
| 错误/资源 | 009/011/012/033边界、失败原子性、输入不变 |
| 回归 | A1/A2/A3内核、B1契约/资源；旧721条不运行 |
| 边界 | 无ABI/intrinsic/XCON/registry/cmath变化；A5～A7未开始 |

## 五、退出条件

1. 整数族、文本、浮点族和lfloat→整数转换内核完成；
2. 最近偶数、下溢/溢出、带符号零、向零截断和范围错误稳定；
3. str→float/sfloat仍拒绝；数字str→lint只经lfloat→lint；
4. 无隐式窄化、无宿主f64决定十进制舍入；
5. A1/A2/A3回归和B1契约/资源通过；
6. 未新增ABI/intrinsic，未实现`lfloat_divide`/`cmath`；
7. A5/A6/A7未开始；旧721条F10未运行；
8. A4证据完整，A5仍须完成全链路后置验收。

## 六、风险与禁止

- 用显示文本/f64近似做float↔lfloat；
- 开放直接str→float/sfloat或直接数字str→lint；
- 用隐式窄化、宿主舍入或静默截断处理转换；
- 把A4内核测试写成VM/native/三平台完成；
- 在A4新增ABI/intrinsic或提前实现A5/A6；
- 运行旧721条F10或修改22D/22C断言。

## 七、交接

A4退出后下一入口是A5：类型/IR/TAC/VM/LLVM全链路与完整语言差分；A6再做资源/性能，A7整批收口。A5/A6/A7不得由A4提前声明。

新Agent先读：

1. [B2-A实施规划](20o-b2-a-implementation-plan.md)；
2. [A2/A3算术实施规划](20o-b2-a2-a3-arithmetic-plan.md)与[实施记录](20o-b2-a2-a3-arithmetic-progress.md)；
3. [冻结草案](20o-b2-freeze-draft.md)§七～§八；
4. [F-8/F-11语义](20o-b2-freeze-draft.md)§7/§8.1/§8.4；
5. [F-12错误表](assets/20o-b2-freeze-b3/entry-20261105/README.md)。

## 八、落点

```text
docs/DevDocs/20o-b2-a4-conversions-plan.md          本规划
docs/DevDocs/20o-b2-a-implementation-plan.md        A0～A7总规划
core/rust/crates/xiao-runtime/src/value/ops.rs       转换/分派入口
core/rust/crates/xiao-runtime/src/value/numeric.rs   A1接纳/身份
core/rust/crates/xiao-runtime/src/value/arithmetic/  A2/A3内核
core/rust/crates/xiao-vm/...                         A5接线后置
core/rust/crates/xiao-codegen-llvm/...               A5接线后置
docs/DevDocs/assets/20o-b2-a4/entry-<date>/          A4证据（待建）
```

本页正文保留A4原规划；当前实施和有效退出见[转换实施记录](20o-b2-a4-conversions-progress.md)。本批没有启动A5～A7。
