# 20O-B2-FREEZE-B1. 原语与 C ABI 收口规划

> **这是 F-B1 的收口规划，不写产品函数、依赖锁、intrinsic、ABI 或测试实现。**
> 当前入口为 `7fffc32d`；F-A 已 25/5 退出，F-B1 的 F-5a/F-5b/F-5c 三问仍待实际答复。
> 台账当前 25 confirmed / 5 pending；`cmath` 仍为 planned，ABI 1.18、生成器 20。
>
> **本批只收 F-5（原语、编号、效果与 C ABI 所有权），不进入 F-6/F-12，更不进入 B2-A/B。**

## 一、入口与事实

| 项 | 当前状态 |
| --- | --- |
| 代码基线 | `7fffc32d`；已推送 |
| F-A | 七项全部确认，25/5；F-4 采用既有提升到 lfloat 的类型边界 |
| F-B1 | 16 个候选入口/符号已列出；F-5a/F-5b/F-5c 待答 |
| 当前台账 | 25 confirmed / 5 pending（F-3、F-5、F-6、F-7、F-12） |
| 产品/ABI | 无 `cmath`/高精度产品；活动 intrinsic 最大 32；ABI 1.18、生成器 20 |
| 最近 CI | `7fffc32d` 的 workspace gates 成功；本次只改 DevDocs，维护回归未触发 |
| 文档门禁 | `check:docs`、`check:usedocs`、`check:layout`、`git diff --check` 通过 |

F-A 的 25/5 和类型表已经收口；F-B1 可以在文档层开始，但候选表仍是“待审”，不是活动契约。

## 二、F-5 的三个精确问题

### F-5a：参数名、调用规则与公开/底层边界

- `lfloat_divide` 的四参数源级签名已确认，不重问：
  `lfloat_divide(lfloat numerator, lfloat denominator, int precision, str rounding) -> lfloat`。
- 15 项 `cmath` 的候选参数名：一元函数 `value`；`min/max` 为 `left/right`；`pow` 为 `base/exponent`。
- 全部参数必填，支持既有位置/命名调用；不增加默认参数、重载或方法别名。
- `stdlib/cmath.xiao` 只导出 15 个公开包装；底层全局原语按既有 `os_` 模式登记，不新增私有可见性语法。
- `cmath` 公开包装与底层原语一一对应，不泄漏未登记 helper。

候选选项：

1. 采用上述候选参数名/调用规则/包装边界（建议）；
2. 采用参数名但调整底层原语命名；
3. 其它命名或边界，需给出完整替换表。

### F-5b：规划 ID、C 符号、效果与能力

| 规划 ID | 源级入口 | 效果 |
| --- | --- | --- |
| 33 | 全局 `lfloat_divide` | Allocates |
| 34–48 | `std.cmath.sqrt/floor/ceil/round/trunc/abs/min/max/pow/sin/cos/tan/exp/ln/log10` | Pure |

- C 符号采用 `xiao_runtime_lfloat_divide` 与 `xiao_runtime_cmath_*`；
- 全部无新宿主能力；`Pure` 不等于无错/`readnone`，失败调用不得被优化器因结果未使用删除；
- ID 33–48 只是规划编号，不改活动 `intrinsics.json`；ID 100 继续墓碑。

候选选项：

1. 接受上述 ID、符号、效果和能力（建议）；
2. 接受 ID 但调整符号名；
3. 重新分配编号或效果，需给出完整替换表。

### F-5c：C ABI 借用、out 所有权、状态与挂起错误

- 输入以借用视图传入，调用期间保持有效；输入与 out 不重叠。
- out 必须已初始化且拥有；成功时释放旧 out 并写入结果；失败时保留 out。
- 状态族建议：
  - 0 成功；
  - 1 必需指针为空；
  - 2 未知/非法 XiaoValue 标签；
  - 3 句柄无效或种类错配；
  - 6 有效标签的可恢复类型/参数/域/预算/受控分配错误，设置正常挂起错误；
  - 8/宿主致命终止沿用既有 Fatal 边界，不伪装成普通状态 6。
- 源级类型错误、参数域、预算、除零/域和输出分配的具体错误身份由 F-12 统一，不在此提前冻结。

候选选项：

1. 接受借用输入、拥有 out、上述状态族（建议）；
2. 接受所有权但调整状态映射；
3. 其它 ABI 形状，需给出完整状态/所有权表。

## 三、分批执行

### B1-A：收答复与台账

1. 保存 F-5a/F-5b/F-5c 的实际答复，不把建议预选为回答；
2. F-5 从 pending 改为 confirmed；
3. 台账从 25/5 变为 **26/4**，剩余 F-3、F-6、F-7、F-12；
4. 不修改活动 intrinsic 或 ABI。

### B1-B：回写精表

1. 把参数名、调用规则、15 个公开包装/底层原语边界写入冻结草案；
2. 把 33–48、C 符号、效果和能力写入 F-5 表；
3. 把借用/out/状态/挂起错误写入 C ABI 表；
4. 重新运行 16 项静态对账，确认 ID/符号唯一、按 ID 排序、无活动表变更；
5. 检查与 F-A 的 `lfloat_divide` 签名、F-2/F-9 的 `cmath` 语义一致。

### B1-C：F-B1 退出

1. F-5 有唯一参数/ID/符号/效果/所有权/状态表；
2. 台账为 26/4；
3. 文档门禁通过，证据保存实际答复/命令/摘要；
4. F-6/F-12 仍未开始；F-C、B2-A/B 仍未开工。

## 四、验收与证据

| 检查 | 期望 |
| --- | --- |
| F-5a | 15 个 cmath + lfloat_divide 参数/调用/包装唯一，无未登记 helper |
| F-5b | 16 个 ID/符号/效果/能力唯一；活动表未改 |
| F-5c | 输入借用、out 拥有、状态/挂起错误/失败保留 out 明确 |
| 台账 | 25/5 → 26/4；F-5 不再 pending |
| 文档 | 冻结草案、F-B/F-B1 规划、索引一致；docs/usedocs/layout 通过 |
| 产品边界 | `core/rust`、`stdlib`、Cargo 文件、intrinsic/ABI、测试、工作流无差异 |
| 证据 | 实际答复、静态对账、命令和摘要入库；不伪造 C ABI 链接或产品成绩 |

## 五、退出条件

1. F-5a/F-5b/F-5c 全部 confirmed；
2. 16 项参数/ID/符号/效果/所有权/状态表唯一；
3. 台账为 26/4，剩余 F-3、F-6、F-7、F-12；
4. 活动 intrinsic/ABI/诊断表未改；
5. 文档门禁通过，产品边界无差异；
6. F-B2/F-B3 仍未开始；B2-A/B 仍未开工；22D/22C、19/11/22 债务保持独立。

## 六、交接

新 Agent 先读：

1. [F-B 契约裁定规划](20o-b2-freeze-b-contract-plan.md)；
2. [B2 冻结草案](20o-b2-freeze-draft.md) §5.1、§5.2；
3. [F-B 审阅证据](assets/20o-b2-freeze-b/entry-20261104/README.md)；
4. [30 条决策台账](assets/20o-b2-freeze-close/entry-20261103/decision-ledger.json)；
5. [F-A 收口](20o-b2-freeze-a-closeout-plan.md)。

不得：

- 把 F-5 候选表直接写成 confirmed；
- 在 F-B1 改动活动 `intrinsics.json`、ABI 版本或产品函数；
- 把 F-6 版本矩阵或 F-12 错误表提前塞进 F-5 收口；
- 把 `cmath` planned 升为 verified；
- 把 B1 历史 721 条 F10 写成 B2 当前性能。

## 七、落点

```text
docs/DevDocs/20o-b2-freeze-b1-closeout-plan.md 本规划
docs/DevDocs/20o-b2-freeze-b-contract-plan.md  F-B 总规划
docs/DevDocs/20o-b2-freeze-draft.md            精确契约
docs/DevDocs/assets/20o-b2-freeze-b/entry-20261104/ 候选表与源码事实
docs/DevDocs/assets/20o-b2-freeze-close/entry-20261103/decision-ledger.json 30条台账
```

F-B1 收口后下一入口是 F-B2：版本与产物矩阵；仍不是 B2-A 代码。
