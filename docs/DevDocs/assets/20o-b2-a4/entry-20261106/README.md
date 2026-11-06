# B2-A4 转换内核验收

工程期：20O-B2-A4。代码快照`1838fae1be0c038fd888dde2675126598458b74f`；入口`2abea3f321d1a145106e623d9f1e6fef4576f7e8`包含A2/A3代码5ef17bb2及测试隔离修复13ecd687。

17项转换、A1/A2/A3及B1回归、代码CI全部通过。有效结论和16项检查映射见`closeout.json`，原始命令见`commands.json`；维护第二次执行通过，首轮22C失败保留。A4只交付Runtime内核，A5完整语言差分、A6性能、A7整批收口后置。

## 范围与实际检查

新增转换直接按IEEE位模式承接二进制数值，反向以有界整数有理数、一次商余和最近偶数规则构造binary32/64。整数目标先检查可证明的位数/范围，再向零截断或补零；不会先展开巨型指数。lint/lfloat到str使用A1规范渲染，str到整数保留单码点契约。

A3私有提升算法保持原样；仅更新A3测试中过时的公开转换拒绝断言，A4公开方向另独立验收。ABI1.18、生成器20、intrinsic最大32、六包依赖/锁、XCON、stdlib与registry不变。cmath及新增数值教程范围继续planned，旧721条F10未运行。

| 检查 | 实际记录 |
| --- | --- |
| A4内核 | [17项](close-kernel-tests.json)：36个数值方向、文本/码点、显示精度、最近偶数/溢出中点/子正规/负零/二次舍入反例、确定性位模式、预算和输入不变 |
| A2/A3当前回归 | [四包日志](close-package-tests.json)逐项核对[21个测试标识](a2-a3-regression-test-names.json)；不是把历史源代码原封不动重跑 |
| A1已有完整示例 | [VM](close-a1-vm-exact.json)和[原生](close-a1-native-exact.json)各--exact且唯一1 passed；原生三个示例O0/O3六份实际产物摘要记为`native-artifacts-final.json` |
| B1契约/资源 | [9/9](close-b1-contract.json)、[17/17](close-b1-resources.json)，保留导出、逐函数移除、真实归档与普通包隔离 |
| B1精确入口 | [VM](close-b1-vm-exact.json)、[原生](close-b1-native-exact.json)均--exact，实际恰好1 passed/0 failed/0 ignored |
| B1守卫 | [28/28](close-b1-guards-report.json)，四路径模拟计数守卫，不冒充真实平台执行 |
| 静态/构建 | [Clippy](close-clippy.json)、[格式](close-rustfmt.json)、[workspace](close-workspace-check.json)、[benchmark](close-benchmarks-check.json)、[新Runtime](close-build-runtime-release.json) |
| 文档 | 最终docs/usedocs/layout/覆盖率入口为`close-docs-closeout`；已有coverage warning保留 |

工作轨迹记录最大系数截断、33092位边界有理数舍入的预检上界及实际位数。这些是逻辑预算证据，未执行F-3五进程性能/OS峰值协议，不作为A6成绩。未诱发真实OOM；受控输出复用ResourceError/011，第三方无法安全建错的OOM沿既有Fatal边界。

## 来源身份

[入口](bootstrap.json)、[环境](environment.json)和[实际产物/工具身份](product-identity.json)固定唯一任务根`D:\project\Xiao\.a4-conversions\20261106-600facf4186b`。target、Runtime、Xiao缓存与临时输出隔离；原生使用本任务release Runtime。TypeScript5.9.3为源码根内实体复制，无junction。

[执行器](run-step.py)记录每条argv/cwd/PID/起止/退出码/原始日志；构建、测试、检查均固定Rust/Cargo1.96.0且--locked，子进程清理RUST_BIGDECIMAL_*。命令前后源码输入变化即拒绝。
最终[source-manifest](source-manifests/c18ed16cf8a86982aa3176531970f01f68c6c82f88a7c04e55a80b942111471a.json)含989项源码输入，覆盖产品、UseDocs、registry、锁及工具；这是明列范围的计数，不是全仓库文件数。
原始树为`c18ed16cf8a86982aa3176531970f01f68c6c82f88a7c04e55a80b942111471a`，UTF-8/LF规范树为`e66ea70ba185c372f9dea548de154c6338733c13985d21867df1d14905152854`。

DevDocs滚动证据不计入产品输入树，本目录.gitattributes保留原始字节。`evidence.json`逐文件登记全部实际字节，不对自身递归计数；`commands.json`聚合真实run-step记录，完整构成以清单为准，不另报无法复核的文件总数。

## CI与独立债务

代码的workspace运行号为[42184105820](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/42184105820)，维护回归为[42184105863](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/42184105863)。A4内核主计数点绑定Windows本地及Linux默认workspace，三平台现有原生回归单独保存实际作业日志；不扩写成A5新转换三平台语言证据。

维护首轮macOS在[22C任务取消清理顺序断言](independent-22c-record.json)失败：[原始作业](ci-attempt1-macos.json)记录127780245568，25通过/1失败。源码/工作流不改，以[原始重跑命令](ci-rerun-macos.json)对同提交请求一次重跑；[API重新排程](ci-attempt2-scheduling.json)列出第二次执行的实际新作业号。后续绿色也不关闭这条22C债务。

22C/22D及19/11/22债务独立，A0/A1/A2/A3归档和已签冻结草案未改。代码CI绑定1838fae1，后续文档收口提交不冒称被同一运行覆盖。

## 失败与复核

`a4-kernel-first`首轮15通过/2失败保留：转str误用只复制载荷的display_text，漏过接纳与规范化。改用A1渲染后17/17通过，并补lint直接窄化中点及大整数默认scale0的独立检查，没有删例或修改预算。

只读复核归档字节、实际测试计数、源码范围、固定Git对象和CI作业：

```powershell
python -X utf8 docs/DevDocs/assets/20o-b2-a4/entry-20261106/audit.py
```

完整重跑另建任务目录并检出1838fae1，准备新target/缓存/Runtime，以各JSON实际argv和环境替换新根；所存任务脚本按scripts/source/environment结构还原，禁止覆盖历史日志。
全部工作树范围由`boundary.json`核对，不过滤产品、UseDocs、stdlib、.cargo或benchmarks：

```powershell
cmd /c git diff --name-only 2abea3f321d1a145106e623d9f1e6fef4576f7e8 --
cmd /c git ls-files --others --exclude-standard
```

历史源码内容由固定Git对象审计证明，不把未来HEAD的registry当作本次快照。下一入口为A5，B2-A整批尚未收口。

[最终维护run响应](ci-maintenance-run.json)和[全部attempt实际作业](ci-maintenance-jobs.json)保留历史；[Windows](ci-native-windows.json)、[Linux](ci-native-linux.json)、[macOS](ci-native-macos.json)分别绑定最新成功作业及其原始日志，核对B1精确点与A1六条完整示例标记。
