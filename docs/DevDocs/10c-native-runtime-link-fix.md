# 10C. 原生 Runtime 链接缺陷修复交接

> **本文记录一次真实环境验证发现的缺陷。** N0-B 交付时把这条路径标为「未验证」是诚实的，
> 但**一验证就是坏的**——而且坏在 N0-B 的主功能路径上。
>
> 发现方式、完整证据与已实测的修复方向都在本文；导致它长期不可见的机制
> 见 [10D. 环境依赖测试专项规范](10d-environment-gated-test-spec.md)。

## 一、症状

在齐备工具链的环境里跑 `xiao-driver` 的动态原生测试：

```text
test optional_dynamic_string_native_round_trip ... FAILED
动态源码应完成原生构建: Backend(ToolchainFailed { tool: "clang", status: Some(1120),
  stderr: "clang: error: linker command failed with exit code 1120" })
```

**退出码 1120 是 MSVC 的 `LNK1120: unresolved externals`**。

同一批的其他结果（供对照）：

```text
xiao-codegen-llvm/tests/n0_a                  16 passed ✓（含 optional_real_llvm_round_trip）
xiao-codegen-llvm/tests/n0_b_dynamic          13 passed ✓
xiao-driver/tests/n0_b_dynamic_native          6 passed, 1 FAILED ✗
```

## 二、证据

### 2.1 未解析的符号全是 Windows 系统 API

把完整 stderr 打开后，33 个 `LNK2019` 清一色是系统调用：

```text
__imp_WSARecv / __imp_WSASend / __imp_WSASocketW / __imp_accept / __imp_bind
__imp_connect / __imp_select / __imp_recvfrom / __imp_sendto / __imp_listen
__imp_WSAStartup / __imp_WSACleanup / __imp_freeaddrinfo
__imp_NtWriteFile / __imp_NtOpenFile / __imp_NtCreateNamedPipeFile
__imp_GetUserProfileDirectoryW / __imp_GetHostNameW

fatal error LNK1120: 33 个无法解析的外部符号
```

**没有一个是你自己写的符号**——全是 winsock / ntdll / userenv。

### 2.2 根因：Rust staticlib 的原生依赖没有被传递

```text
rustc --print native-static-libs
  → kernel32.lib ntdll.lib userenv.lib ws2_32.lib dbghelp.lib /defaultlib:msvcrt
```

`xiao-runtime` 的 `crate-type` 含 `staticlib`，所以**链接它的程序必须自带这一组系统库**。
这是 Rust 的既有契约（链接 Rust staticlib 的标准做法），不是本仓的发明。

### 2.3 对照实验（双向确证）

同一个 `.ll`（调用 `xiao_runtime_write_i64`）+ 同一个 `xiao_runtime.lib`：

| 链接命令 | 结果 |
| --- | --- |
| `clang ... xiao_runtime.lib -o t2a.exe` | **LNK1120，33 个未解析符号** |
| `clang ... xiao_runtime.lib -lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp -o t2d.exe` | **exit 0** |

而且产物**真的能跑**——程序输出了 `.ll` 里传进去的 `42`：

```text
$ ./t2d.exe
42
```

也就是说：**LLVM IR → clang → MSVC link → Rust staticlib → ABI 函数调用**整条路径是通的，
**只差那 5 个库**。

## 三、修复方向（已实测）

### 3.1 加系统库，注意写法

- **推荐 `-l` 形式**：`-lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp`
  —— 实测干净通过，无警告；
- `-Wl,` 透传形式也能通过，但**带 `/defaultlib:msvcrt` 会触发
  `LNK4098: 默认库"msvcrt"与其他库的使用冲突`**。实测两种写法都成功，**建议选无警告的那种**；
- **裸库名不行**：`clang ... kernel32.lib` 会报 `no such file or directory`
  ——clang 不按 MSVC 的方式解析裸库名，要用 `-l` 或完整路径。

### 3.2 库名不要硬编码（**这一条比修复本身重要**）

把这 5 个名字写死在 `xiao-codegen-llvm` 里能止血，但会引入新问题：

- **平台耦合**：Linux/macOS 的原生依赖是另一组（`-lpthread -ldl -lm` 之类）；
- **随 Rust 版本漂移**：这组库由 Rust 的 std 决定，升级工具链可能变；
- **它会成为第二份真相**：`rustc` 已经能算出这份清单，硬编码等于抄一遍。

**候选方案（请在本批交接记录里写明选择与依据）**：

- **A. 构建时查询**：调用 `rustc --print native-static-libs`（或 `cargo rustc -- --print ...`）
  拿到清单，写进 `Toolchain` 的指纹与链接参数；
- **B. 按目标登记**：在 `llvm-toolchain.toml` 一类的登记处按三元组记录这组库；
- **C. 由 ABI crate 声明**：在 `xiao-runtime-abi` 用 `#[link(...)]` 表达它对系统库的依赖。

**注意与 `10:21` 的边界**：那条说的是「**主机工具链自动发现**留到第 11 阶段」，
指的是"clang 装在哪"。**原生依赖库清单不是工具链发现，是 ABI 契约的一部分**——
它决定"链接一个 Xiao 原生程序需要什么"，所以归本批，不归 11。

### 3.3 顺带确认

修复后要复核：**纯静态标量程序仍不链接 Runtime**（`10:78`）。
N0-A 的 `assert!(!module.uses_runtime)` 是这条的探测器，**不得为了让新测试通过而删掉它**。

### 3.4 落地实现与验证记录（2026-09-22）

本批选择候选 **A：构建时查询**。`Toolchain::probe_native_static_libraries` 接收调用方
提供的 `rustc` 路径和目标描述，执行 `--crate-type=staticlib --print=native-static-libs`，
由 `parse_native_static_libraries` 保留 Rust 报告顺序，把 Windows 的 `.lib` 名称转换为
clang 的 `-l...` 参数，并过滤 `/defaultlib:*` 元参数。原生库清单、Runtime staticlib
内容摘要、目标字段和 `rustc`/LLVM 版本首行共同进入构建指纹；后端不硬编码 Windows 库名，
也不自动发现 clang、VS 或 Rust 路径。选择 A 是因为依赖清单由 Rust 标准库实际计算，能随
目标和 Rust 版本变化，避免登记文件成为第二份真相；B 不能覆盖版本漂移，C 会把链接实现
错误地塞进 ABI 声明层。

实现时又验证了同一 ABI 在 MSVC x64 下的调用约定：16 字节 `XiaoValue` 返回值必须使用
隐藏 `sret` 槽，16 字节 `XiaoAbiBytes` 参数必须按间接指针传递。动态 LLVM 降低器只对
COFF 目标发射这两种形态，ELF/Mach-O 仍使用直接聚合返回/传参；这样既保持固定 C 布局，
也避免“能链接但返回时破坏栈帧”的未定义行为。

本机实际验证环境如下：VS Community 18.9.2 的 `link.exe` 14.51.36256.0，路径位于
`VC\Tools\MSVC\14.51.36231\bin\Hostx64\x64`；MSYS2 clang/llvm-as 22.1.2；Rust
`rustc 1.96.0`；目标 `x86_64-pc-windows-msvc`。Rust 对 Runtime staticlib 报告的清单为：

```text
kernel32.lib ntdll.lib userenv.lib ws2_32.lib dbghelp.lib /defaultlib:msvcrt
```

解析后传给 clang 的参数为 `-lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp`。带这组参数
的真实 `FrontendCompiler -> LLVM -> clang -> MSVC link -> xiao_runtime.lib` 动态字符串
程序已启动并以退出码 0 结束；不带这组参数的对照仍复现 LNK1120。纯静态标量路径的
`!module.uses_runtime` 断言继续通过。由于 COFF 调用约定修复改变了动态 LLVM 文本，
`xiao-codegen-llvm` 的 `CODEGEN_VERSION` 已从 1 升到 2，旧指纹产物不会被复用。

## 四、为什么它长期不可见（修复前）

以下保留事故发生时的旧实现，用来解释为什么缺陷能进入交接记录。跑这个测试需要四个环境
变量（`XIAO_CLANG`、`XIAO_LLVM_AS`、`XIAO_RUNTIME_LIBRARY`、`XIAO_TARGET_TRIPLE`）。
**修复前没设时测试直接 `return`**：

```rust
let Some(runtime_text) = std::env::var_os("XIAO_RUNTIME_LIBRARY") else {
    return;                    // ← 既不算失败，也不算"没跑"
};
```

于是它在修复前的 `cargo test` 汇总里显示为 **`ok`**。

**「静默跳过的测试」与「真的通过的测试」在门禁输出里长得一模一样**——
这是本次缺陷能藏这么久的唯一原因，也是
[10D](10d-environment-gated-test-spec.md) 要解决的问题。

## 五、验收

1. **齐备环境下**：`xiao-driver/tests/n0_b_dynamic_native.rs` 全绿，
   特别是 `optional_dynamic_string_native_round_trip`；
2. **对照实验可复现**：能把「不带系统库失败 / 带系统库成功」作为一条回归记录下来；
3. **不硬编码**：§3.2 的三条候选里选一条并写明依据；
4. **纯静态仍不链 Runtime**：N0-A 的断言仍在且通过；
5. **门禁全绿**，且按 10D 的要求报告**跳过了哪些测试**。

## 六、不负责

- **不改 N0-B 的 ABI 形状**（tagged value、句柄协议、版本策略都保持）；
- **不做工具链发现**（`10:21` 留给 11）；
- **不做 Linux/macOS 的原生依赖库适配**——本批先把 Windows 修对，
  并在 §3.2 的选择里说明**跨平台时这份清单从哪来**。

## 相关页面

- [10D. 环境依赖测试专项规范](10d-environment-gated-test-spec.md) —— 为什么它能藏这么久
- [10B. N0-B Runtime ABI](10b-n0-runtime-abi.md) —— 缺陷所在的批次
- [10A. LLVM 原生构建闭环](10a-n0-native-closure.md) —— 决策一（手写 IR + 外部工具链）
- [10. LLVM 原生后端](10-native-backend.md) —— `:15` ABI 来源、`:78` 零 Runtime 依赖
