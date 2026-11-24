# B3-0E 教程和精确入口候选

B3-0E 已完成前置复核和六模块候选整理。规划 §3 的五项裁定待答，教程状态为 `planned`，B3-0F 入口待签收。[实施规划](../../../20o-b3-0e-tutorials-fixtures-plan.md) · [裁定台账](decisions.json)

| 模块 | 教程 | 运行输入 | 示例能力 |
| --- | --- | --- | --- |
| time | [墙钟、单调钟和等待](tutorials/time.md) | 固定墙钟 946684800000 ms 和单调钟 0 ns | `clock`、`stdout` |
| args | [程序名和参数快照](tutorials/args.md) | `demo.xiao`、空项、空格、第二个分隔符和重复项 | `process`、`stdout` |
| cli | [参数分隔](tutorials/cli.md) | 源码内固定数组 | `stdout` |
| fs | [文件系统路径操作](tutorials/fs.md) | 独占工作目录内预建空 `demo`，结束检查仍为空 | `filesystem`、`stdout` |
| env | [运行环境视图](tutorials/env.md) | 用原 B3-0C 合成键值对注入快照 | `environment`、`stdout` |
| log | [标准输出日志](tutorials/log.md) | 固定墙钟和独立输出端 | `clock`、`stdout` |

夹具候选统一为 `candidates/tests/fixtures/docs/<module>/tutorial.*`。产品阶段的目标是 `tests/fixtures/docs/<module>/tutorial.*`。六份源码和六份独立黄金逐字来自固定 B3-0B/C 对象，args、env、log 保留已审计的 JSON 输入。time 新增固定时钟文件，cli 和 fs 的输入条件记在矩阵里。[候选路径及摘要](tutorials.json)

## 精确入口候选

每个模块使用集成测试目标 `20o_b3_<module>_tutorial`，分别登记 `<module>_tutorial_matches_vm_golden` 与 `<module>_tutorial_matches_native_golden`。VM 用同一示例执行两个导入前缀。原生在 O0、O3 各执行两个前缀，每个优化等级和导入路径使用独立的运行目录及合成输入。别名变体只替换示例首行的模块导入。两项 Cargo 精确入口各期望 1 通过、0 失败、0 忽略，应用退出码为 0，标准输出逐字比对 UTF-8/LF 黄金。[入口矩阵](tutorials.json)

`cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --target <triple> --test <target> <test> -- --exact --nocapture` 是候选 VM 调用。受控原生入口追加 `--ignored`，使用显式 `XIAO_TARGET_TRIPLE`、`XIAO_CLANG`、`XIAO_LLVM_AS`、`XIAO_LLC` 和 `XIAO_RUNTIME_LIBRARY`。产品取证绑定任务专属 Runtime、工具版本和摘要，失败回执带模块、平台、导入路径、后端、优化等级和原始输出。Windows 产品验收要求 COFF `Complete`，允许证据路径的正向实测待 B3-1。[B3-0D 交接](../entry-20261124-b3-0d/handoff.json)

B3-0A 为 Windows x86_64、Linux x86_64 和 macOS arm64 提供宿主与 B1 入口的历史回执。本批在 Windows 用固定解析库核对十二份 `std`／`stdlib` 示例，产品 VM 和原生矩阵处于计划状态。B3-0B 的 CLI 原生观察来自使用 `text` 模块的等价探针。[前置复核](preflight.json) · [本机语法回执](syntax.json) · [CLI 历史探针](../entry-20261123-b3-0b/cli-import.zip)

## 只读复跑

```powershell
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261124-b3-0e/audit.py
python -B -X utf8 docs/DevDocs/assets/20o-b3/entry-20261124-b3-0e/self_negative.py
```

自反例只修改系统临时目录里的副本并打印摘要。只读审计核对固定 Git 来源、源码与黄金、教程字节、运行输入、精确入口及清单。阶段、文档门禁、自反例结果及清单分别见 `closeout.json`、`doc-gates.json`、`self-negative-result.json` 和 `evidence.json`。
