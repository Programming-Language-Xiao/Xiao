# 20O-B1-CLOSE. 七库证据收口与状态升级规划

> **这是 B1-CLOSE 的规划，不写产品或语言实现。**
> B1-IMP 已达到 20/20 入口；最终 F10 采用固定源码身份工具重跑并通过。B1-CLOSE 只做
> 最终证据复核、教程/registry 状态升级和阶段收口，不再修改七库函数体、ABI、生成器或预算。
>
> 状态：**收口完成；七库教程/registry已verified，106b99c7同提交维护回归与工作区门禁通过，实际证据已归档（2026-11-03）**。
> 实际运行与退出对账见[收口记录](20o-b1-closeout.md)；下文保留开工规划。
> 规划基线为已推送的`c407c4c9`，实际开工为70440da2；B1-IMP 的 20/20 产品与最终 721 条 F10 证据见
> [REM 实施证据](20o-b1-rem-evidence.md) 与
> [固定源码身份最终证据](assets/20o-b1-imp/rem-identity-final-41464439400/README.md)。
> 开工时七库教程和registry均为planned；本批按同源产品与完整证据升级为verified，已满足§五退出条件。

## 一、入口条件

开始 B1-CLOSE 前必须确认：

1. 七个模块、20 个入口已全部落地；七份产品夹具、独立黄金和教程逐字一致。
2. `44dc9d6e` 的完整 F10 四报告 721 条通过，工具 Git 对象与 `df27ca75` 一致；报告
   `complete=true`、`failed=false`，产品摘要与当前 `stdlib/*.xiao` 一致。
3. 最终功能回归 `41464439400`（attempt 2）与工作区门禁 `41464421187` 成功；若这些证据
   之后有产品、工具或预算提交，必须先重新取证，不能沿用旧报告。
4. 七库仍为 `planned`；`error_code` 的独立 `verified` 状态不变。
5. 已知 22D 历史失败、19/11/22 发布债务仍按原归属登记，不在 B1-CLOSE 内关闭。

如果入口条件不满足，停止并回到相应 IMP/前置批次；B1-CLOSE 不修产品。

## 二、本批工作

### 2.1 最终证据复核

- 重新核对最终 F10 报告的 run/job/attempt、报告摘要、721 条记录、20 入口/31 负载、
  预算和产品摘要；
- 核对最终功能回归的四个 B1 精确点（默认 VM + 三平台原生 O0/O3），确认教程示例、
  移除、归档、普通包隔离和释放记录均来自同一代码快照；
- 核对生成器 20、Runtime ABI 1.18、工具 Git 对象以及 `ci-logs.zip` 的哈希；
- 不重新认领身份修复前的旧 721 条；旧报告只作历史快照。

### 2.2 教程与 registry 升级

七册 UseDocs 页面和七项 registry 条目是 B1-CLOSE 的主要交付：

1. `docs/UseDocs/library/{math,hex,base64,url,csv,fmt,statistics}.md`
   - front matter `status: planned` → `verified`；
   - 标题/正文移除“教程草案/待收口”表述，保留实际导入与完整示例；
   - 失败恢复、资源/性能边界、ABI 1.18、标准库身份和已验证范围写清；
   - 不把 F10 的历史旧报告或产品外推成未测硬件成绩。
2. `docs/module-registry.json`
   - 七项 `status` 升 `verified`；`stage` 加入 `20O-B1-CLOSE`；
   - 保持 `code`/`tests`/`usedocs` 指向真实源码、产品夹具、契约测试和教程。
3. `docs/UseDocs/library/README.md`
   - “B1 教程草案/planned”改为已交付状态；
   - 保留七库入口、版本、F10 与资源/生命周期证据链接。
4. 同步 `stdlib/README.md`、七份 `stdlib/<module>.md`、B1 冻结草案、B1-IMP/REM 文档、
   `docs/DevDocs/README.md` 和 `20-builtins-and-standard-library.md` 的状态。
5. 不把 `error_code`、其它较早模块或 19/11/22 债务混入升级。

### 2.3 最终门禁与证据

状态升级只改文档/registry，但仍要触发同提交的真实门禁：

- 维护回归：默认 workspace、四个 B1 精确点、移除/归档/教程示例、计数守卫 28/28；
- 工作区门禁：workspace、clippy、fmt、docs/usedocs/layout、覆盖率、benchmark/Bun 等既有项；
- F10：若产品/工具 Git 对象与 `44dc9d6e`/`df27ca75` 完全一致，可携带已有 721 条证据并在
  CLOSE 证据里写明身份；任一产品/工具变化则重新执行完整 `b1_f10=true` 运行；
- 最终证据目录建议 `docs/DevDocs/assets/20o-b1-close/<run-id>/`，保存 README、机器清单、
  关键日志摘录与完整源日志哈希。

## 三、文档与登记落点

```text
docs/UseDocs/library/<module>.md             七册状态升 verified
docs/UseDocs/library/README.md               入口状态与证据索引
docs/module-registry.json                    七项 status/stage
stdlib/README.md、stdlib/<module>.md         模块状态
docs/DevDocs/20o-b1-freeze-draft.md          B1 完成状态
docs/DevDocs/20o-b1-implementation-plan.md   批次/证据状态
docs/DevDocs/20o-b1-imp-remainder.md         REM/CLOSE 分界
docs/DevDocs/20o-b1-rem-evidence.md          最终 F10/功能证据
docs/DevDocs/assets/20o-b1-close/<run-id>/   最终证据
docs/DevDocs/README.md、20-builtins...       阶段索引
```

不得把教程状态改成 `verified` 而不更新 registry，也不得只改 registry 而教程仍写 `planned`；
`check:usedocs` 的门禁会检查已登记 verified 模块至少有一页 UseDocs verified。

## 四、测试与核对矩阵

| 检查 | 期望 |
| --- | --- |
| 七模块导出/签名 | 20/20 名称、参数名、错误表与冻结契约一致；无额外顶层导出 |
| 教程 | 七页与产品 fixture 逐字一致；所有公开示例可执行 |
| registry | 七项 verified，code/tests/usedocs 路径存在 |
| VM 精确 | 默认 VM 20/20 产品 + 辅助场景，恰好 1 passed/0 failed/0 ignored |
| 三平台原生 | O0/O3 产品场景与 F10 数值，同一代码快照；退出 0、stderr 空 |
| 移除/归档/包隔离 | 逐函数移除、真实 `.xar` 重放、普通包不能占用 stdlib |
| F10 | 721 条、20 入口/31 负载、预算和产品摘要；历史旧报告不冒充新结果 |
| 生命周期 | 生成器 20 释放向量匹配；22D 历史失败仍独立登记 |
| 文档门禁 | docs/usedocs/layout、覆盖率、fmt、clippy 全过 |

## 五、退出条件

1. 七库教程和七库 registry 全部 `verified`，且状态与代码/证据一致；
2. 最终功能回归和完整工作区门禁成功，证据带实际 run/job/attempt 和日志摘要；
3. F10 721 条证据与当前产品/工具身份一致；旧报告仅历史；
4. 移除、归档、普通包、教程示例和三平台精确目标均有证据；
5. 22D 历史失败、19/11/22 债务保持独立，不在本批核销；
6. 不修改七库函数体、ABI、生成器、预算或 CI 阈值；如发现产品缺陷，停止并回 IMP。

## 六、交接

1. 先读 [B1 冻结草案 §三～§七](20o-b1-freeze-draft.md#三七库已确认签名与调用边界f2f8f12)；
2. 读 [REM 最终证据](20o-b1-rem-evidence.md) 和 [最终 F10 目录](assets/20o-b1-imp/rem-identity-final-41464439400/README.md)；
3. 读 [B1-IMP 规划](20o-b1-imp-seven-libraries.md) 与 [剩余四库记录](20o-b1-imp-remainder.md)；
4. 按本页状态升级、最终取证、退出对账三步执行；遇到产品缺陷先停，不要用降级教程状态掩盖。

B1-CLOSE 只负责收口；它完成后，19 发布、11C/11B 和 22D/22 债务仍按各自文档继续。
