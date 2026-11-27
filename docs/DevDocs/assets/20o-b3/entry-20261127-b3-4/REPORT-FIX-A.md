# B3-4C-FIX-A 修复报告

本批以 `0e983b0f6a3c1104f07541460d029d0ffc4bfd17` 为基线，修复 B3-4C 复审确认的三项阻塞。受测工作区使用 ABI 1.20、标准库接口 1、生成器 27。原生教程测试和共享测试夹具改用新启动握手，复用 B3-4D 的源码、合成输入与黄金输出。

## 修复

| 项目 | 改动与回归 |
| --- | --- |
| FIX-A1 原生环境注入 | 驱动每次发送显式快照时从操作系统生成 128 位随机数，经首个启动参数与私有子进程环境变量交付。运行时先校验能力掩码和位 7，再核对随机数。握手匹配才读取标准输入并跳过该启动参数。回归覆盖旧标记作为普通首参、随机数不匹配、驱动真实随机数注入、显式空数组和宿主捕获。[原生专项](logs/fix-a/driver-env-native.log) |
| FIX-A2 公开 CLI 授权 | `xiao run` 的源码、`.xiaoc`、`.xar` 入口及原生 `xiao build` 增加 `--allow-environment`。开关将默认 `0x7F` 与位 7 合成 `0xFF`，省略时由核心使用 `0x7F`。CLI 运行请求省略环境字段，由 Rust 核心按位 7 捕获一次宿主环境。归档的授权入口是 `xiao run -xar`，归档构建根据依赖闭包登记所需能力。测试覆盖两种语言的帮助、默认能力与显式授权，以及 `--` 后的应用参数。[CLI 专项](logs/fix-a/cli-focused.log) · [公开运行](logs/fix-a/cli-run-authorized-isolated.log) · [公开构建](logs/fix-a/cli-build-authorized-retry.log) |
| FIX-A3 COFF 校验 | COFF 达到 `Complete` 时，可读符号表与绑定链接清单均须满足未分类 Runtime 符号集合为空。缺组件和意外组件检查继续参与判定。两条负例都让声明与观察集合为 `env`，并添加同一个未分类 Runtime 符号，验证器均拒绝。[COFF 专项](logs/fix-a/coff.log) |

## 验证命令

Windows 目标为 `x86_64-pc-windows-msvc`。使用 Rust 1.96.0、LLVM 22.1.2 和本批重新构建的发布 Runtime。原生产物使用 MSVC 链接器。工具、Runtime 和探针产物摘要见日志。[Rust](logs/fix-a/tools.log) · [原生工具与 Runtime](logs/fix-a/native-tools.log) · [探针产物](logs/fix-a/native-probe-hashes.log)

| 实际命令 | 退出码 | 结果 |
| --- | ---: | --- |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-runtime` | 0 | 338 项通过，1 项忽略。[日志](logs/fix-a/runtime.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b3_env -- --nocapture` | 0 | 7 项通过，5 项原生用例由下方命令显式执行。[日志](logs/fix-a/driver-env.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b3_env_tutorial -- --nocapture` | 0 | VM 教程 1 项通过。[日志](logs/fix-a/driver-tutorial.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml --workspace` | 0 | 197 组，1,824 项通过、0 项失败、61 项忽略。[日志](logs/fix-a/workspace.log) |
| `cargo +1.96.0 fmt --manifest-path core/rust/Cargo.toml --all -- --check` | 0 | [日志](logs/fix-a/fmt.log) |
| `npm --prefix cli/ts test` | 1、0 | 默认沙箱中 17 项失败。允许测试启动本机子进程后重跑，258 项通过，5 项跳过。[首次](logs/fix-a/npm-test.log) · [重跑](logs/fix-a/npm-test-retry.log) |
| `cargo +1.96.0 build --locked --manifest-path core/rust/Cargo.toml -p xiao-runtime --release` | 0 | 发布 Runtime 供以下原生测试使用。[日志](logs/fix-a/runtime-release.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-codegen-llvm coff_ -- --nocapture` | 0 | 4 项通过，含两种 COFF 证据的负例。[日志](logs/fix-a/coff.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b3_env -- --ignored --nocapture --test-threads=1` | 0 | 原生 5 项通过，包括旧标记与随机数回归。[日志](logs/fix-a/driver-env-native.log) |
| `cargo +1.96.0 test --locked --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b3_env_tutorial -- --ignored --nocapture --test-threads=1` | 0 | 原生教程 1 项通过，包含双导入及 O0/O3。[日志](logs/fix-a/driver-tutorial-native.log) |
| `bun test cli/ts/src/commands/parser.test.ts cli/ts/src/commands/index.test.ts cli/ts/src/protocol/client.test.ts` | 0 | 44 项通过。[日志](logs/fix-a/cli-focused.log) |

## 原生产物实测

探针读取合成宿主变量 `XIAO_FIX_A_SAMPLE=probe-host` 并逐项输出 `argv()`。公开构建命令使用 `bun cli/ts/src/main.ts build`，授权构建加入 `--allow-environment`。两次构建均退出 0，产物验证均为 COFF `Complete`，声明与观察一致，未分类符号为空。[探针源码](logs/fix-a/native-probe-source.log) · [授权构建命令](logs/fix-a/cli-build-authorized-retry.log) · [默认构建命令](logs/fix-a/cli-build-default-retry.log)

`cmd /d /c "core\rust\target\fix-a-native\marker-probe.exe --__xiao-run-environment-stdin tail < NUL"` 退出 0。应用标准输出逐行为：

```text
probe-host
--__xiao-run-environment-stdin
tail
```

默认构建的探针以相同参数启动，得到 `X06-RUNTIME-026`，退出 3。公开 CLI 从单独的用例目录运行同一份源码，默认路径同样退出 3，授权路径退出 0，并打印上述三行。默认沙箱中 Bun 启动核心和工具探测子进程时返回 `EPERM`。解除子进程限制后，首次运行探针时源码名含连字符，模块发现报 `X05-MODULE-002` 并退出 1。最终用合法模块名在独立目录执行成功。
[授权原生运行](logs/fix-a/native-marker-direct.log) · [默认原生运行](logs/fix-a/native-default-direct.log) · [授权 CLI 运行](logs/fix-a/cli-run-authorized-isolated.log) · [默认 CLI 运行](logs/fix-a/cli-run-default-isolated.log) · [沙箱失败](logs/fix-a/cli-run-authorized.log) · [模块发现失败](logs/fix-a/cli-run-authorized-retry.log)

## 仍存风险与 B3-4D 重跑建议

- 配套部署本批发布 Runtime，并重新构建静态链接的旧原生产物，使受控快照启动的两端使用同一握手。
- 复审记录的原生提交前运行取消及截止仍待覆盖。嵌入 C ABI 仍先解码整份显式 JSON，再检查条目配额。
- 在固定源码和各平台重新构建的 Runtime 上重跑教程双导入、原生 O0/O3 及 env 专项，特别核对 Linux、macOS 的链接组件保留。Windows 对四份教程、两份权限产物逐件绑定链接证据并复验 `Complete`。运行时权限负例应报 026 并退出 3，快照接纳失败仍按 006/007 退出 2。
