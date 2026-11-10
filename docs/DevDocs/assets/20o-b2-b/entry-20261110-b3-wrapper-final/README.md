# B3-4 planned 源码包装收口证据

本收口绑定 B3-3A/B 产品提交 `89d782e13f00776b90193f1719b4454c54367db7` 与 B3-4/802/可达性修复后产品提交 `7d8b85e4d8c588b40ec76d641edc865481295032`。审计固定 Git 对象，不要求当前 HEAD；当前受保护产品树必须与固定代码提交相同。所有 Rust 命令由本目录 `run-step.py` 在新任务根 `D:\project\Xiao\.b3-4-closeout\20261110-7d8b85e4` 执行，使用全新 Cargo target、XIAO_HOME 和 TEMP/TMP。未生成 Runtime staticlib，也未执行原生 cmath；所有未执行平台和功能在 closeout 中明确登记。

## 收口结论

B3-4 在**源码包装范围**内完成：15 个 `float -> float` forwarder、模块说明、实际导入闭包隔离、子 VM 别名快照边界、归档 planned 拒绝和固定证据通过。`stdlib.cmath`、UseDocs 教程和 registry 继续 `planned`；语言可用性为 false。B4 类型/IR/TAC/VM/LLVM 接线、归档可执行性、正式向量迁移和完整教程仍未开始。

## 固定执行点

| 检查 | 结果 |
| --- | --- |
| B3-4 wrapper | `20o_b2_b3_wrapper`：8 passed / 0 failed / 0 ignored；15 项接口/移除/变异契约，hex 原生隔离及 30 条 planned 拒绝 |
| 并发与归档 | `22c_vm_concurrency`：29 passed / 0 failed / 0 ignored；两条 cmath 别名子 VM 身份、planned `.xar` 拒绝、普通标准模块归档边界 |
| LLVM 标准库绑定 | `xiao-codegen-llvm`：56 单测通过，`stdlib_bindings` 4 passed；入口可达传递依赖、导入环、namespace selected sibling 和不可达 cmath 反例 |
| B3-CABI | 固定 `89d782e1` 审计：86 文件、22 检查、17,243 条正式 ABI 参考；历史 B3-3 证据，不冒充本批可执行 cmath |
| B1 历史审计 | 固定 B1 证据：18/18；只证明 B1 固定快照 |
| B2 历史审计 | 固定 B2 证据：99 文件、14 检查、4 精确入口；只证明 B2 单入口快照 |
| Workspace | `cargo +1.96.0 test --manifest-path core/rust/Cargo.toml --workspace --locked`：全工作区测试通过 |
| Clippy/格式/锁 | workspace `-D warnings`、fmt、`bun run check:lock` 均通过 |
| 文档门禁 | `bun tools/repo-check/src/cli.ts all`；docs、UseDocs、layout 和仓库覆盖门禁通过 |

各命令原始输出、PID、退出码、源码与锁摘要、日志哈希见同目录 JSON/LOG。`collect.py` 仅在所有日志完成后生成权威 `closeout.json` 与 `evidence.json`；默认 `audit.py` 和反例均不写入证据。收集器迭代期间被审计拒绝或被后续门禁回执取代的自动生成草稿均保留在目录内；正式结论只来自与当前 `evidence.json` 字节清单一致并通过只读 `audit.py` 的 `closeout.json`。权威文档门禁以 `repo_all_review_final` 记录为准。

首轮 workspace 命令从仓库根漏写 `--manifest-path core/rust/Cargo.toml`，Cargo 在编译前退出 101 并报告找不到 Cargo.toml；原始失败记录保留为执行器调用修正，不归类为测试失败。最终全工作区测试使用正确 manifest 单独运行并以 `workspace_tests_final` 记录。

开发期首轮 LLVM 架构断言和 Clippy 布尔简化检查的失败日志也保留；分别修正测试只扫描生产区段、将否定 `is_some_and` 改为等价 `is_none_or` 后，全量后端测试及 workspace Clippy 通过。没有忽略或移除失败项。

## 不在本批宣称

- `std.cmath` / `stdlib.cmath` 已可执行；30 条拒绝仍是显式硬断言；
- 三平台、远端 CI、native cmath O0/O3 或归档运行通过；
- B3 整批、B4、B5、B6、B2-C 或旧 721 条 F10 已完成；
- 19/11/22、22C/22D 或其它历史债务被关闭。

B4 的函数映射、向量迁移和教程 fixture 交接见 [handoff-b4.md](handoff-b4.md)。

## 复核

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261110-b3-wrapper-final/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b2-b/entry-20261110-b3-wrapper-final/self-negative.py
```
