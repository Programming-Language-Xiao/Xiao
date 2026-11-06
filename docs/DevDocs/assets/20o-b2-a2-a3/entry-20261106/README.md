# B2-A2/A3 算术内核验收

工程期：20O-B2-A2/A3。入口为`0ffffbe6fdbb3f502e793196a184ba39f6862a7a`，阶段提交A2为`9ef111a5`，最终代码快照为`5ef17bb23453f1e308a56f390734467822db3231`。

**本批仅验收Runtime算术内核。** 21项内核、A1回归、B1契约/资源/精确入口及代码CI通过；有效退出、检查映射及实际命令分别保存在`closeout.json`、`checks.json`、`commands.json`。A4转换、A5完整语言接线、A6性能、A7整批收口后置。

## 实现与边界

lint/lfloat已接入精确系数加减乘、有限除法、floor商余、非负整数值幂及顺序比较。精确除法先约分和判2/5因子，034先于展开预算033；余数独立用模幂/虚拟补零，不物化巨大商，D+1位差必须实际核对。
高精度负号共享接纳后再归一化零；bool±lint不窄化i128，浮点混合仅按IEEE位模式做算子私有精确提升。

[复合错误顺序实际答复](compound-error-order-receipt.json)确认两侧输入预算→非有限009→算子域。A1的语言字典仍为str键；SetHandle证明实际Runtime集合，Rust HashMap不冒充语言数值字典键。

ABI1.18、生成器20、活动intrinsic最大32、F-7锁、XCON、stdlib及registry不变。cmath和新增数值教程范围继续planned。旧721条F10未执行，完整新算术语言差分、性能及教程升级仍交A5/A6/A7和B2-C。

## 实际验证

| 组 | 本次记录 |
| --- | --- |
| 内核 | [21项](final-arithmetic-admission.json)：独立小有理数参考、D/W/H及上限+1、顺序/原子性；同一日志保留最大模乘W、对齐W/W+1、D+1余数强消去工作轨迹 |
| A1四包 | [Runtime/types/diagnostics/VM](close-package-tests.json)，含既有无分配身份及恢复回归 |
| A1完整示例 | [VM](close-a1-vm-exact.json)、[原生](close-a1-native-exact.json)各精确1项；三个示例O0/O3共六份实际产物摘要记为`native-artifacts-final.json` |
| B1契约/资源 | [9/9](close-b1-contract.json)、[17/17](close-b1-resources.json)，保留真实导入/导出/移除/归档/普通包隔离 |
| B1精确 | [VM](close-b1-vm-exact.json)、[原生](close-b1-native-exact.json)，均--exact且实际恰好1 passed/0 failed/0 ignored |
| B1守卫 | [28/28](close-b1-guards-report.json)，四路径模拟计数守卫，不冒充平台产品执行 |
| 静态/构建 | [Clippy](close-clippy.json)、[格式](close-rustfmt.json)、[workspace](close-workspace-check.json)、[benchmark](close-benchmarks-check.json)、[新Runtime](close-build-runtime-release.json) |
| 文档 | docs/usedocs/layout/覆盖率最终收口执行点为`close-docs-final`，原有coverage warning保留 |

本批未诱发真实OOM。受控输出缓冲失败复用A1的ResourceError/011；第三方内部无法安全建错的OOM沿既有Fatal边界，不把预算拒绝或错误工厂检查写成真实OOM恢复成绩。

## 源码与产物

[入口](bootstrap.json)及[环境](environment.json)固定独立任务根`D:\project\Xiao\.a2-a3-arithmetic\20261106-68ec96014f24`；target、Xiao缓存、临时目录均隔离。[实际身份](product-identity.json)记录Rust/Cargo1.96.0、目标、工具输出及Runtime/文档适配器路径/大小/摘要，原生明确使用本次release Runtime。
TypeScript5.9.3实体复制在源码根内，无junction；子进程清理RUST_BIGDECIMAL_*，宿主全局环境不改。

最终[source-manifest](source-manifests/eb60c82563d5162705e1e5275851992e92d3ca21d71ad5334990c5fd26c6fa40.json)列出979项输入，含core/rust、stdlib、fixtures、benchmarks、工作流、.cargo、工具、CLI、UseDocs、registry及锁；这不是全仓库文件数。
原始树为`eb60c82563d5162705e1e5275851992e92d3ca21d71ad5334990c5fd26c6fa40`，UTF-8/LF规范树为`6bf5ed7cab74f719afe2200534fbcfd283ec7408e26eb1cd33887461e1083a08`。
[执行器](run-step.py)逐条记录argv/cwd/PID/起止/退出码，前后源码不一致即拒绝；[审计](audit.py)还对照固定Git对象的全部输入范围，不能删一项后仍称完整清单。
DevDocs滚动证据不计入产品输入树；本目录`.gitattributes`保留原始字节，`evidence.json`不对自身递归计数。

## CI与历史

[workspace 42169630412](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/42169630412)及[维护回归42169630252](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/42169630252)均成功。
[run响应](ci-maintenance-run.json)、[实际作业](ci-maintenance-jobs.json)及[内核日志身份](ci-kernel-linux.json)绑定5ef17bb2。21项在Windows本地及Linux默认workspace作业127734143987执行；三平台现有原生维护回归通过，不扩写为A5新算术三平台语言证据。

初次API直连超时保留，随后仅为gh子进程复用既有Git代理，见[网络记录](ci-network.json)。本归档绑定代码CI，后续纯文档收口提交不冒称被这些运行覆盖。
22C/22D及19/11/22债务独立，本轮绿色不关闭历史债务；A0/A1归档和已签草案未改。

首轮失败均保留：A2测试乘数类型推断、A3 SetHandle路径、缺证据README/六字段Rustdoc、B1资源16通过/1项Unicode URL解码看门狗失败。修正后另名复跑完整17/17；没有改看门狗、并行、预算、用例或断言。

## 复核与交接

只读审计归档字节、实际计数、CI作业和固定Git源码，不执行新产品，也不拿未来HEAD的registry替代历史快照：

```powershell
python -X utf8 docs/DevDocs/assets/20o-b2-a2-a3/entry-20261106/audit.py
```

完整重跑须另建任务目录，检出5ef17bb2，准备新target/缓存/Runtime，以各JSON实际argv/环境替换新根执行，不在历史目录覆写日志。所保存的任务脚本需还原到同一scripts/source/environment目录结构。

`boundary.json`比较全部Git已跟踪差异与未跟踪路径，包含产品、UseDocs和工具；命令不筛掉stdlib、.cargo或benchmarks：

```powershell
cmd /c git diff --name-only 0ffffbe6fdbb3f502e793196a184ba39f6862a7a --
cmd /c git ls-files --others --exclude-standard
```

两命令核对本批工作树范围，历史内容由固定Git对象审计证明。下一入口A4转换矩阵；B2-A整批尚未收口。
