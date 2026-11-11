# B4-2A TAC/字节码执行记录

> 本目录是 B4-2A 的实施中间记录，不是阶段收口。TAC/字节码专项覆盖通过；必需的 22c 回归有两项失败，因此 `b4_2a_closed=false`，B4-2B 尚未启动。

实施提交为 `df8a16dbb57c2365ba7a69a4569938e63763f1ff`，起始 HEAD 为 `fe752363702b3613a8a70d89b877e32ad6505f33`。本批没有发现 lowering 或编码器缺口，只增加集成测试与运行证据；没有修改 Runtime、VM、LLVM、模块导入/归档、ABI、版本或公开状态。

`b4_cmath_tac.rs` 对 intrinsic 34～48 生成 33 种调用：12 个一元函数各测位置/命名调用，3 个二元函数各测位置/正序命名/逆序命名。每种形态分别使用 LEB128 和 FixedU16 编码，验证调用 ID、被调入口、实参名称与寄存器顺序、解码往返、确定性重编码、指令源码区间及 PC→span 映射。49、墓碑 100 和 `u32::MAX` 均由生产校验器与编码器拒绝。

版本观测为 `TAC_VERSION=1`、`TAC_BYTECODE_ABI_VERSION=1`、`FORMAT_VERSION=4`；无并发计划的本组字节流头部仍使用格式字节 3。没有修改版本常量或容器布局。

## 验证

- `cargo +1.96.0 fmt --manifest-path core/rust/Cargo.toml --all -- --check`：通过。
- `cargo +1.96.0 test --target-dir D:\project\Xiao\.b4-2a\20261112-fe752363\target --locked --manifest-path core/rust/Cargo.toml -p xiao-bytecode -- --nocapture`：64 passed，0 failed。
- B4-1 前端与 B3-4 wrapper：分别 3/3、8/8 通过，见 [b4-1-b3-4-regression.log](b4-1-b3-4-regression.log)。
- `bun tools/repo-check/src/cli.ts all --format json`：通过；输出包含既存 `A0-COVERAGE-002` 覆盖警告。
- B4-0/B4-1 固定审计：通过，产品提交 `c7a45e10`，1048 个受保护产品文件；B3-4 固定审计：通过，代码提交 `7d8b85e4`，1044 个受保护产品文件。
- B1 固定审计 18/18，B2 固定审计 14 项及 4 个精确入口，B3-CABI 固定审计 22 项及 4 个精确入口通过。没有运行旧 721 条 F10。

## 阻塞

按独立任务根、独立 Cargo target、`XIAO_HOME` 与 `TEMP/TMP` 重跑 22c 后仍为 27 passed / 2 failed，完整记录见 [22c-isolated-replay.log](22c-isolated-replay.log)。混合回归原始记录见 [driver-regression.log](driver-regression.log)。

1. `isolated_child_preserves_cmath_alias_snapshot_identity` 预期 planned 导入返回 `X06-RUNTIME-012`，实际子 VM 成功并输出 `7\n`，事件包含 `ModuleLoaded { module: "stdlib.cmath" }`。
2. `planned_cmath_xar_is_rejected_before_output` 预期归档构建以 `X11-PROTOCOL-007` 拒绝，实际生成了包含 `stdlib.cmath` 的 `.xar`。

这两项涉及子 VM 模块加载与归档构建，属于 B4-2A 明确禁止修改的 VM/归档范围。为遵守冻结边界，本批没有改动它们，也不把 B4-2A 标为退出；B4-2B 需等该回归阻塞按独立裁定处理并重新通过退出门槛后再安排。B4-3 未开始，`language_available=false`，Linux/macOS、远端 CI、性能和 LLVM 证据未执行。

## 原始记录

- [xiao-bytecode-final.log](xiao-bytecode-final.log)：最终格式化源码上的字节码包完整测试，包含 66 条宽度/调用形态 span 记录和负例。
- [b4-1-b3-4-regression.log](b4-1-b3-4-regression.log)：B4-1 与 B3-4 独立通过记录。
- [22c-isolated-replay.log](22c-isolated-replay.log)：隔离任务根下的 22c 重放。
- [driver-regression.log](driver-regression.log)：三个相关测试目标首次合并运行的完整输出。
- [evidence.json](evidence.json)：结果、版本、阻塞与文件 SHA-256 摘要。
