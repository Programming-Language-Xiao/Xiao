# B4-2B：Runtime 门面与 VM 执行

`57ca957c` 已完成 B4-2B 的 Windows 本机内部验证；与既有 B4-2A/FIX1 合并形成 B4-2 内部退出。直接原语可由 VM 执行，`std.cmath`/`stdlib.cmath` 模块仍 planned，`language_available=false`。B4 整批尚未收口，B4-3 未开始。

| 真实执行 | 结果 |
| --- | --- |
| baseline 入口 | 43/43：22c29、wrapper8、前端3、FIX1边界3 |
| fixed 驱动回归 | 50/50：上述43项及VM专项7项 |
| Runtime / VM 单元测试 | 272/272、35/35，无新增忽略 |
| 字节码 | 65/65，34～48与两种编码宽度往返、版本及未知ID拒绝保持 |
| 数值回归 | 15项通过，3项既有原生测试按原规则忽略，名称/原因保留在日志 |
| Runtime 门面 | 正式离线参考的17,243个输入逐条与私有内核对照；NaN/±Inf及挂起槽隔离 |
| VM 数值/参数 | 1,440项三载体逐位/逐错误码对照；位置与逆序命名、所有8类非float实参及形状负例 |
| 失败与清理 | decode后009/span、catch、O0～O3未使用Pure结果、finally及完整释放序列；45项堆值临时强引用恢复 |
| 工程与历史门禁 | Clippy `-D warnings`、fmt，B1/B2/B3/B3-4/B4-0/1/FIX1固定审计通过 |

生产代码只改8个允许路径，详见 `audit.py` 的 `ALLOWED`；数学实现仍只有私有共享内核。15个 crate 根 Rust 门面只做转发并返回 `RuntimeResult<f64>`，VM完成形参绑定、完整类型检查与错误span附加；不经C ABI挂起槽。合法源码/生产`.xiaoc`与内部TAC注入分开测试：NaN/Inf、畸形形状和动态类型负例使用研究注入入口，不冒充语言公开调用或模块导入证据。测试保留既有VM的finally先于catch展开顺序，未修改释放语义。

所有正式执行分别位于 `D:\project\Xiao\.b4-2b\20261113-runtime-vm\baseline` 与 `fixed`：各有全新target、Xiao Home、TEMP/TMP和Runtime路径；开发target另置于 `development`，不充作正式证据。`fixed/source.json` 的实施快照与 `57ca957c` 产品树一致，记录原始及UTF-8/LF规范摘要。每条回执保留PID、时间、argv、cwd、环境、源码与DevDocs前后摘要。清理继承的 `XIAO_*`/`RUST_BIGDECIMAL_*`，stdlib显式绑定仓库源码，两个性能开关均关闭。

Cargo会随Rust测试从本批源码生成Runtime的rlib和staticlib；两套任务内的实际文件名、字节数和SHA-256由 `baseline/runtime-artifacts.json`、`fixed/runtime-artifacts.json` 记录。`XIAO_RUNTIME_LIBRARY` 指向任务内未构建的占位路径，没有链接该外部Runtime。baseline环境沿用最初模板的 `runtime_staticlib_built_or_used=false` 字段，它不能证明Cargo没有生成staticlib；以这两份实际归档记录为准。fixed环境已改为分别记录外部Runtime未使用与Cargo从源构建，不沿用该含混字段。

原始日志强制写入 `logs/sha256/<前两位>/<完整摘要>.log`；相同字节只有一个对象，每次执行保留独立回执。`collector` 与 `collector_repeat` 两条真实回执共享同一对象，采集器自测还验证损坏对象拒绝。审计拒绝不规范、摘要不符或无人引用的对象。FIX1历史目录和日志不改写。

文档统一定稿后只运行一次 `fixed/docs` 全量门禁。该回执记录 `docs_commit/docs_commit_after`、raw `docs_tree_before/after`、`docs_tree_sha256`、规范前后摘要和Git blob的raw/规范摘要。静态DevDocs包含本README、规划、索引、采集器、审计器及反例源码；只排除本目录运行时日志对象、运行回执、收口清单和反例输出。raw前后相同、HEAD不变、规范摘要等于固定Git树，兼容Windows CRLF检出并证明门禁读到定稿文档。

从仓库根复核：

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261113-b4-2b/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261113-b4-2b/audit.py --self-negative
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261113-b4-2b/audit.py --live-runtime
```

默认审计完全只读，验证固定Git对象及归档证据，不要求当前HEAD等于实施提交，也不要求旧任务目录存在。`--live-runtime` 额外核对仍保留的两套本机Runtime归档字节。支持 `--root`、`--repository-root`。`self-negative-findings.json` 保存9个用例：1个基线及清单、属性文件、孤立日志对象、DevDocs摘要、产品源码、阶段状态、环境逃逸和伪造只读报告8个拒绝反例；即使重建字节清单，仍须按精确错误文本拒绝。每项比较完整文件字节及目录项，审计也强制核对报告的 `read_only=true`、顺序和拒绝身份。

`--collect` 为显式生成模式，从真实运行回执和本机Runtime归档生成 `closeout.json`、Runtime摘要及 `evidence.json`，拒绝覆盖。保存新反例报告后使用 `--refresh-inventory`：它先验证阶段结论不变，只刷新实际字节清单。最终审计应同时返回 `passed=true` 和 `self_negative_checked=true`；报告不存在时会明确返回后者为false。

Linux/macOS、远端CI、性能、旧721未执行。ABI仍1.18、生成器20、标准库接口1；没有修改checker、intrinsic表、C ABI、LLVM、模块/归档加载、版本、锁、UseDocs或registry。B4-3是下一入口，B4-4才接真实模块双路径/归档；22C/22D及19/11/22历史债务独立保留。
