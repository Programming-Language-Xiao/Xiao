# B3-0/B3-1：独立基线与正式 F-9 私有参考

B3-0 基线与 B3-1 私有参考已完成。入口为 `ae1744e5`，基线提交 `ca63a792`，被测代码为 `88886dd5`。本批只有 `cmath.rs` 和新增 `cmath_tests.rs` 两个产品路径发生变化；现有 15 项内核通过正式参考，无须修改数学算法。

后续[自审修复](self-review-20261108/README.md)补齐了归档参考文件与被测快照的绑定、B0 历史字节校验、精确测试目标校验、pow 汇总核算及参考自检的缓存约束。产品代码、正式参考生成器及向量均保持上述被测身份；自审结果与原始验收分开记录。

ABI 1.18、生成器 20、标准库接口 1、活动 intrinsic 最大 33、100 墓碑；34～48/C ABI/stdlib 包装尚未接线，`stdlib.cmath` 继续 planned。float `**` 调用点没有切换；B3-2、B4、B5 与 B2-C 后置，22C/22D 及 19/11/22 债务保留。

基线采用当前干净入口。相对 B2 代码 `37b92c53`，唯一非 DevDocs 差异是已批准的 `3aa6a290` 所含 UseDocs 状态勘误，已在 [bootstrap.json](bootstrap.json) 单列并核对 Git blob；产品与示例没有继承额外变更。

| 证据 | 本批实测 |
| --- | --- |
| Windows 独立任务根 | 全新 target、Runtime、Home、临时目录；标准库来自本任务源码 |
| Runtime | 259 项单元通过；完整集成/文档测试通过，原有 1 项集成测试保持 ignored |
| Driver / intrinsic | 64 passed / 1 ignored；7 passed |
| B1 契约 / 资源 / 守卫 | 9 / 17 / 28，通过 |
| B1/B2 VM、原生精确入口 | 两阶段各 4 项，均 `--exact` 且恰好 1 passed；原生含 O0/O3 |
| F-9 | 17,243 条，15 函数，0 不可验证；sqrt 与取整/abs/min/max 逐位一致，其余最大 1 ULP |
| 参考生成器 | 7 项自检通过；独立临时目录完整重生成逐字节一致 |
| 工程与文档 | workspace check、benchmark `--locked`、Clippy、fmt、repo-check all 通过 |
| Linux / macOS / 远端 CI | 本批未执行；没有使用旧记录替代本批证据 |

完整结论见 [B3-0 收口](b0/closeout.json)、[B3-1 收口](b1/closeout.json)、[代码身份](code-identity.json) 和 [逐函数误差](b1/kernel-metrics.json)。本批没有运行旧 721 条 F10 或 B2-A 性能开关；这些参考样本不构成新性能成绩。

## 只读复核

在主仓库直接运行以下命令。审计读取固定 Git 对象，不要求 HEAD 等于旧入口，不重写 `closeout.json` 或 `evidence.json`，也不重新执行产品或参考生成器。

```text
python -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261108-b3/audit.py
```

当前机器还保留两份 Runtime 独立存档，可额外核对实际字节：

```text
python -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261108-b3/audit.py --live-runtime
```

`--root` 可指向证据复制目录，`--source-root` 指向含固定对象的主仓库。默认归档审计校验构建窗口、命令前后 Runtime 身份及符号取证，**不声称其他机器仍持有本机 Runtime 文件**；`--live-runtime` 才校验本机存档。

完整产品树范围包括 `core/rust`、`stdlib`、`tests/fixtures`、`tests/benchmarks`、`.github/workflows`、`.cargo`、`tools`、`cli/ts`、`docs/UseDocs`、registry、`package.json`、`bun.lock`、`.gitattributes`，并额外绑定生成器、参考输入和参考输出。源工作树的原始字节与 UTF-8/LF 摘要分开登记；Git 对象按后者核对，不把换行差异误判为产品变化。

## 参考来源与复现

[来源清单](b1/tool-provenance.json) 固定 gmpy2 2.3.2 发布 wheel/sdist URL 和 SHA-256，逐字节核对实际加载模块及捆绑 DLL；加载版本必须是 MPFR 4.2.2/GMP 6.3.0。使用发布者预构建 wheel，没有声称自行重建其底层二进制。归档仅保存在独立任务根的 `reference-tools`，不加入产品依赖。

固定种子 `b203f90020261108`，全部输入以 binary64 位模式精确装入。普通结果从 256 位开始，MPFR 分别向下/向上计算，二者按最近偶数转换为相同 binary64 才采纳；否则依次加倍至 4096 位，仍不唯一则标不可验证。特殊值、域、零符号采用冻结契约独立判定，未用宿主 libm 生成黄金。

已准备该离线工具环境后，可只读重生成和自检：

```text
python -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261108-b3/b1/replay-reference.py
python -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261108-b3/b1/test-reference.py
```

首轮 [kernel-initial.log](b1/kernel-initial.log) 失败已保留：gmpy2 的 `floor/ceil/trunc` 便捷接口返回整数，丢失负零。修正为 `rint_*` 后完整重生成并复验，未放宽契约或修改产品零语义。初轮生成器/输入/输出保留在 [initial-reference](b1/initial-reference/reference-manifest.json)。所有最终有效执行点绑定同一份最终源码及参考清单。

## pow 共享边界

[pow-diff.json](b1/pow-diff.json) 保存 1,075 例真实 `RuntimeValue::Float.power` 与私有 `cmath::pow` 的值/错误差异，来源是 [kernel-tests.log](b1/kernel-tests.log)。旧路径使用宿主 `f64::powf`；私有路径使用冻结的 libm 内核和有限值前检。NaN/Inf 等复合特殊输入上的差异登记在表内，本批没有切换调用点，也没有把私有内核通过写成公开 float 幂或三平台通过。

## 自审与清单

[自反例工具](self-negative.py) 检查基线、清单篡改、缺失 `.gitattributes`、临时重建清单后的语义篡改与 Runtime 身份篡改，结果保存为 `self-negative-result.json`；每项核对确切拒绝文本、退出码和审计前后字节不变。`--core-only` 仅是自反例内部入口，会明确标记，不能替代最终默认审计。

当前集合为 9 项，另覆盖重建清单后的参考摘要重绑、B0 记录改写、pow 汇总改错及精确测试目标替换；原有 5 项报告保留在自审目录中，未用新结果改写旧结论。

`collect-results.py` 与 `refresh-evidence.py` 是显式生成工具，分别从执行日志生成收口文件、从实际字节生成完整清单；`audit.py` 不调用它们。修改文档或证据后必须显式重新封存，并重新运行自反例及最终审计。

初次文档门禁 `b1/docs.log` 因所链接的收口生成文件尚未就绪而失败，原日志保留。先用 `collect-results.py --prepare` 生成明确 `passed=false` 的待验收文件，通过最终文档门禁 `docs-final` 后才正式生成 `passed=true` 结论。

独立工作树的现有 commit-msg 钩子拒绝主仓库内的 Git 元数据路径。本任务仅在外部任务根配置临时钩子，把同一提交信息复制到工作树临时文件，调用原有提交信息校验后删除；校验规则与产品脚本均未修改。
