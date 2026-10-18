---
id: tooling.cli.build
title: xiao build 原生与归档构建
status: verified
audience: developer
module: ts.xiao-cli
stage: 11X0-E/18B
related:
  - README.md
  - shell.md
  - packaging.md
  - debug.md
  - ../../../DevDocs/11x0e-build-and-toolchain.md
---

# `xiao build` 原生与归档构建

`xiao build` 读取真实的 `.xiao` 源码，经 TypeScript CLI、Rust 核心协议和
`FrontendNativeDriver` 生成 LLVM 原生程序。命令不等待 `xiao run`，也不接受手写 TAC；源码、
配置和目标信息由同一条前端管线处理。

## 命令格式

```text
xiao build -o <output> <file.xiao> [-debug] [--emit-llvm <path>] [-O0|-O1|-O2|-O3]
xiao build -xar <config.xiao|file.xiao> [-o <output.xar>] [-O0|-O1|-O2|-O3] [-debug]
```

`-o`（或 `--output`）指定可执行文件路径。省略时，输出为
`build/<源码基名>`；Windows 自动追加 `.exe`。`--emit-llvm` 可额外保存 LLVM 文本，父目录
不存在时会由构建后端创建。支持 `-O0` 至 `-O3`；缺省按命令行、项目配置、全局配置、
内建 `-O0` 的优先级确定。原生路径使用 clang 对应的 `-O` 参数，并明确报告
`optimization_backend=clang`、`xiao_passes_registered=false`。输入必须是 `.xiao` 文件。

例如：

```text
xiao build -o build/hello.exe examples/hello.xiao
xiao build examples/hello.xiao --emit-llvm build/hello.ll -O0
xiao build -o build/hello-debug.exe examples/hello.xiao -debug
```

## 构建字节码归档

`-xar` 生成由已安装的 `xiao`/Runtime 运行的 `.xar`，无需 clang。单个 `.xiao` 使用自身
脚本顶层或 `[main]` 入口；指定 `config.xiao` 时，静态查找项目中唯一带 `[main]` 的
`.xiao`，缺失或多个都拒绝。默认输出在输入同目录：`hello.xiao` → `hello.xar`，
`config.xiao` → `config.xar`；`-o` 可覆盖位置。`--emit-llvm` 不适用于归档构建。

```text
xiao build -xar examples/hello.xiao
xiao build -xar config.xiao -O2 -o release/app.xar
xiao verify release/app.xar --detail
xiao -xar release/app.xar
```

归档携带入口可达的本地模块和 `[resources]` 显式声明的文件。资源映射仍采用
`"归档逻辑路径" = "项目根相对来源文件"`，不收录未声明文件。官方标准库与 Runtime
由宿主提供，归档只记录实际可达的官方模块及 intrinsic 能力要求。例如仅调用
`stdlib.os.open_file` 不要求进程能力；调用 `spawn` 才要求进程能力。

首版遇到第三方包依赖会以 `X11-PROTOCOL-006` 拒绝，包括配置声明或依赖锁中的外部包，
不会省略依赖后继续打包。项目锁文件使用现有完整校验器读取，不会被构建改写。
调用图无法静态解析的动态调用同样明确拒绝；现有前端及字节码未支持的程序形态保持拒绝。

`.xiaoc` 写入全局内容寻址缓存，源码旁边不产生 `.xiaoc`。命中时仍重新计算完整 SHA-256
并调用权威字节码解码器；损坏命中直接拒绝。归档完成全部校验后才原子写出，失败不会
留下部分归档。`--json` 返回 `operation=build_archive`，`value.kind=archive_build` 的
JSON 摘要包含输出、归档摘要、官方模块、能力位和每个模块的命中、重编译及验证字段。

普通归档保留紧凑源码映射；`-debug` 写入持久激活位及独立调试符号对象，不自动携带源码
正文。调试归档启动继续遵守独立诊断窗口要求。

## 工具链发现

CLI 按以下顺序寻找每个工具：显式环境变量覆盖、独立 CLI/输出目录相邻目录、`PATH`、检测到
仓库根后的开发目录。显式路径无效时不会静默回退。`clang` 是必需工具；`llvm-as`、`llc`、
`rustc`、Runtime 静态库和诊断组件按构建需要记录。发现结果会保留候选路径、来源、版本、
目标和失败原因。

常用环境变量：

```text
XIAO_CLANG             clang 可执行文件
XIAO_LLVM_AS           llvm-as 可执行文件
XIAO_LLC               llc 可执行文件
XIAO_RUSTC             rustc 可执行文件
XIAO_LLVM_BIN          LLVM bin 目录
XIAO_TOOLCHAIN_ROOT    工具链根目录
XIAO_RUNTIME_LIBRARY   Runtime 静态库路径
XIAO_DIAGNOSTICS_PATH  -debug 构建使用的 xiao-diagnostics 路径
XIAO_CORE_PATH         Rust 核心 xiao-core 路径
```

clang 主版本低于仓库 `core/rust/llvm-toolchain.toml` 的最低版本会返回
`X11-CLI-TOOLCHAIN-VERSION-001`。目标三元组不兼容返回
`X11-CLI-TOOLCHAIN-TARGET-001`；真实 C 编译或链接探测失败返回
`X11-CLI-TOOLCHAIN-LINK-001`；显式路径无效返回
`X11-CLI-TOOLCHAIN-OVERRIDE-001`。这些错误发生在生成最终产物前。

Windows 原生构建需要在 Visual Studio Developer Command Prompt（例如 `vsdevcmd.bat -arch=x64
-host_arch=x64`）中启动，使 SDK、链接器和目标环境可用。也可以通过 `XIAO_CLANG` 指定兼容
的 clang；MSYS2 GNU 目标不能冒充冻结的 MSVC 目标。

## 输出与调试构建

普通构建生成可执行文件；若提供 `--emit-llvm`，同时生成 LLVM 文本。构建响应带产物路径、
工具链指纹和目标信息，并在 `artifact.artifact_runtime` 中报告链接后产物事实：
`object_format`（`coff`/`elf`/`macho`）、IR 声明的 `declared_components`、符号表或导出表中
观察到的 `observed_components`、`runtime_symbols`、外部 `dependencies` 和调试相关
`diagnostic_symbols`，以及无法映射到已知组件的 `unclassified_runtime_symbols`。
`artifact.optimization_level` 报告实际使用的 `0` 至 `3`。

这些字段只表示最终文件中实际可观察到的对象格式、符号和依赖，不解释组件为什么被带入，也
不替代平台运行期测试；构建验证在链接后完成，组件证据不完整或与 IR 登记不一致时构建失败。
纯静态标量产物的 Runtime 组件和 Runtime 符号列表为空；动态产物的声明组件与观察组件必须
一致。Windows PE/COFF 的代表符号由构建链接阶段按声明导出，以避免默认链接器剥离 COFF
符号表后无法验证。

`-debug` 构建会在用户入口前启动独立的 `xiao-diagnostics`，并把诊断组件复制到可执行文件
目录。启动桥只记录组件文件名，运行时从自身可执行文件目录寻找相邻组件，所以将可执行文件
和诊断组件整体搬迁后仍可运行。Windows 产物使用独立控制台，并等待诊断首屏就绪后才执行用户代码。旁置的
`<executable>.xiao-debug.json` 是持久激活位；普通构建不会生成它，`config.xiao` 中的
`[debug]` 表也不能自行打开窗口。无法复制组件、创建窗口或在五秒内就绪时返回
`X11-DIAGNOSTIC-START-001`，用户代码不会执行。

经过验证的 `config.xiao` 会固化为 `<executable>.xiao-runtime.json`。文件声明格式版本和
`cli_overrides = true`；命令行覆盖只影响当前构建/进程，不回写源配置，且不能关闭调试产物的
强制激活位。

## 平台证据

Windows 原生已完成真实 Xiao 源码的 build、脱离 CLI 直接运行、独立调试控制台、就绪握手和
缺失组件失败回环。Linux Docker 只提供功能与构建证据；Linux 原生、WSL 和 macOS 尚未复现。
容器或 WSL 的性能数字不与 Windows 原生并列，也不能据此宣称跨平台验收完成。

## 已知边界

项目测试由[`xiao test`](test.md)负责。归档首版只支持本地模块、显式资源和宿主标准库，
第三方包打包留待后续。构建不改变 Xiao 的语言、退出码或诊断协议语义。
