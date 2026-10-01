# 10K. Runtime 裁剪验证的跨平台收口（Linux 侧交接）

> **独立交接档，写给 Ubuntu 服务器上的接手者。**
> 背景：N0-D 的产物验证在 CI 上**只有 Windows 通过，Linux/macOS 三个平台全红**
> （run `36841654803`）。本档给出根因、一个**必须一起处理的判断**，以及任务与验收。
>
> **⚠️ 本档与 N0-D 的实现（[10I](10i-n0d-artifact-verification-and-trimming.md)）是同一批的两半**：
> 10I 把验证机制建起来了，本档解决它**暴露出来的真实缺口**。**本档不完，10 阶段不收口。**

## 一、事故摘要

**运行**：`workflow_dispatch`，run `36841654803`（2026-10-01 09:16 UTC，7m19s）

| 平台 | 结果 |
| --- | --- |
| `windows-amd64` | ✅ success |
| `linux-amd64` | ❌ failure |
| `linux-arm64` | ❌ failure |
| `macos-arm64` | ❌ failure |

**失败信息**（linux-amd64，`n0_a_native_driver.rs:81`）：

```
同一份前端产物应能构建原生程序：
Backend(ArtifactVerification {
  path: "/tmp/xiao-driver-n0-c2-caught-8171/program",
  message: "产物观察到未由 IR 登记的 Runtime 组件：containers, tables, weak" })
```

失败的正是 10G 那两条差分用例（`caught` / `catch-boundary`）。

---

## 二、根因：链接器没有裁剪，整个 Runtime 静态库被拉进来了

ELF 产物里观察到的组件是 IR 登记的**超集**——`containers` / `tables` / `weak` 都进去了。
含义很直接：**Linux 上的链接没有做死代码消除**，静态库成员被整体拉入。

`10:78` 要求的是「**纯静态标量程序不会因为动态运行时能力而完整装载 Runtime**」——
**这句话在 ELF/Mach-O 上目前不成立。**

---

## 三、⚠️ 必须一起处理的判断：**Windows 那个绿是「看不见」，不是「真裁掉了」**

**这一条比根因本身更重要**，因为它改变了修复的性质。

`toolchain.rs:419-420`（`1d66f9c` 引入）：

```rust
for symbol in runtime_export_symbols(runtime_components) {
    args.push(format!("-Wl,/export:{symbol}"));
}
```

**Windows 产物导出的符号是【按 IR 登记列表自己写上去的】**，验证再去看导出表——
**这是一次自证**：导出了什么，就只可能看到什么。

而 ELF / Mach-O **保留完整符号表**，看到的是**链接器实际拉进来的东西**——
所以它把真相暴露了出来。

**推论（接手者请自行复核，别照抄）**：

- **Windows 上 `10:78` 从未被真正验证过**；那条 `success` 不构成证据。
- 因此本次修复**不能只修 Linux 的链接选项**——那样 Windows 会继续用一个没有验证能力的
  机制报绿，`10:78` 在 PE 上永远无法证伪。
- **两个方向要同时处理**：让链接器真裁剪（Linux/macOS）+ 让验证在 Windows 上能看见真相。

---

## 四、任务清单

| # | 任务 | 说明 |
| --- | --- | --- |
| 1 | **先复现** | 在 Linux 上跑 `reproduce.sh native`，看到 `containers, tables, weak` 那条失败 |
| 2 | **让 Linux 真裁剪** | 见 §五 4.1 |
| 3 | **复核 Windows 的自证问题** | 见 §三；给出结论与处置（§五 4.2） |
| 4 | **`n0_a.rs` 的静态断言要在 Linux 上真的过** | 现在它只证明了 Windows |
| 5 | **反例仍然要成立** | 用了容器的程序，产物里**必须能看到**对应组件 |
| 6 | **macOS** | 本机不做，推 CI 让 `macos-arm64` 说话（`-dead_strip`） |
| 7 | **推 CI 四平台** | 含 macOS 的结论 |

---

## 五、修法要点

### 4.1 Linux：让链接器真的做死代码消除

需要**编译期与链接期成对**的选项：

- 编译：`-ffunction-sections -fdata-sections`（每个函数/数据各自成节）
- 链接：`-Wl,--gc-sections`（回收未被引用的节）

**⚠️ 只加链接期那半是无效的**——没有 `-ffunction-sections`，函数都挤在 `.text` 里，
`--gc-sections` 无从下手。**成对加，并验证产物组件确实收敛到 IR 登记的那几个。**

macOS 对应的是 `-Wl,-dead_strip`（Mach-O 的节粒度天然更细，通常不需要 `-fdata-sections` 那半）。

### 4.1.1 本次 Linux 修复落点

`Toolchain::compile_inner` 现在按对象格式加入 ELF 的
`-ffunction-sections -fdata-sections -Wl,--gc-sections`，以及 Mach-O 的
`-ffunction-sections -Wl,-dead_strip`；调试启动 shim 的 C 编译也按函数分节。
验证器继续读取最终符号表，未放宽组件超集检查。

弱表析构值此前复用了普通 ABI 槽覆盖路径，导致普通错误/字符串路径把
`xiao_runtime_weak_release` 牵进 ELF。现已拆分普通强值覆盖和可含弱值的复制覆盖，
并新增 `xiao_runtime_value_release_strong` 作为 LLVM 普通释放窄入口；公开兼容 ABI
仍保留完整弱值处理。弱组件不会再因通用清理路径被错误保留。调试符号判定也收窄为真正的激活入口
（`xiao_native_debug_start` 与 prepare/ready/finish），普通 Runtime 的
`xiao_runtime_diagnostic_event` 不再被误判为 `-debug` 激活位。

Linux 实测：10K 复现中的 `optional_frontend_artifact_differential_round_trip` 与
`optional_native_catch_does_not_terminate` 均通过；容器反例仍观察到 `containers`，
字符串动态样例仍观察到 `value`/`rc`。

Windows 的 `artifact_runtime.verification` 现在明确为
`unverified-coff-exports`；ELF/Mach-O 为 `complete`。CLI/协议会保留这一事实，
Windows 的 CI 绿色只表示构建与导出表边界通过，不把它升级成 PE 内部节裁剪证据。

### 4.2 Windows：让验证能看见真相（**请自行判断，别默认照做**）

可选方向，**各有代价，需要你给出结论而不是直接选一个**：

- **保留 COFF 符号表**（链接期 `/DEBUG` 之类），让验证能读真实符号——
  代价是产物体积与调试信息残留，**可能与 `10:73`「普通产物不携带强制诊断激活位」产生张力**；
- **改看导入表 / 重定位表**等 PE 上确实反映引用的结构——
  覆盖面可能不足，需要先测能观察到什么；
- **明确记录"PE 上验证能力有限"**，把该平台的裁剪结论标为**未验证**——
  **最诚实，但要写清代价**：`10:78` 在 Windows 上将长期没有证据。

**⚠️ 不要保留「自己导出、自己检查」这个循环**——它给出的是虚假的安全感。

### 4.3 反例仍然要成立

`10I §3.1` 冻结过：**裁剪验证必须配反例**，否则「什么都没找到」可能只是**探测方法失效**——
**Windows 这条已经踩实了这个坑**。加 GC 选项之后，反例（用了容器的程序能看到对应组件）
必须**仍然成立**，不能因为裁得太狠而把该有的组件也丢掉。

---

## 六、环境准备（Ubuntu）

**⚠️ 以 [10D §4.2](10d-environment-gated-test-spec.md) 为权威**，别另写一套。要点：

```sh
sudo apt-get install clang llvm lld build-essential xvfb xterm
cargo build --manifest-path core/rust/Cargo.toml -p xiao-runtime --release
export XIAO_CLANG="$(command -v clang)"
export XIAO_LLVM_AS="$(command -v llvm-as)"
export XIAO_LLC="$(command -v llc)"
export XIAO_RUNTIME_LIBRARY="$PWD/core/rust/target/release/libxiao_runtime.a"
export XIAO_TARGET_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
```

**三个易踩点**（10J 已记过一次，这里再列）：

- Linux 上是 **`libxiao_runtime.a`**（Windows 是 `xiao_runtime.lib`）；
- **`XIAO_TARGET_TRIPLE` 取自 `rustc -vV` 的 `host`**，不能沿用 COFF 配置；
- **诊断测试要 `XIAO_DIAGNOSTICS_PATH`**，且先 `cargo build -p xiao-driver -p xiao-diagnostics`。

跑：`XIAO_USE_XVFB=1 bash tools/platform-reproduction/reproduce.sh native`

---

## 七、验收

1. **Linux**：`reproduce.sh native` 通过，两条差分用例不再出现
   「产物观察到未由 IR 登记的 Runtime 组件」；
2. **Linux**：纯静态程序（`n0_a.rs` 那条）**在 Linux 上**也断言
   `artifact_runtime.observed_components` 为空——**不是只在 Windows 上成立**；
3. **反例仍成立**：用了容器的程序产物里**能看到**对应组件；
4. **Windows 的自证问题有明确结论**（修了 / 记录为未验证 / 换观测面），
   **不允许保留原样而宣称通过**；本批保留 COFF 导出表作为现有可见观测面，
   并在四平台 CI 中确认构建/验证路径通过；PE COFF 真正未导出的内部节仍不声称已被
   导出表证明裁掉。
5. **CI 四平台全绿**——**含 `macos-arm64`**（`-dead_strip`）；
6. **不回归**：10J 已验过的内容（平台异常、释放追踪、差分、`-debug` 三不）保持绿。

### 本次交付验证记录

修复提交：`6177931`（`fix(10k): verify runtime trimming across object formats`）。

本地 Ubuntu 验证使用 Rust `1.96.0`、LLVM 21、Xvfb 和独立 Bun 缓存完成：

- `cargo clippy --workspace --all-targets -- -D warnings` 通过；
- `XIAO_USE_XVFB=1 bash tools/platform-reproduction/reproduce.sh native` 通过；
- Linux 两条差分用例通过，纯静态 ELF 未观察到 Runtime 组件，容器反例仍观察到 `containers`；
- 普通 ELF 不含 Runtime 组件，`-debug` 仅观察到声明的 `rc`/`value`，未误带入 `weak`。

跨平台 CI run [`36850150346`](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/36850150346)
于 2026-10-01 完成，四个平台均成功：

| 平台 | Job | 结果 |
| --- | --- | --- |
| `linux-amd64` | `110329621734` | ✅ success |
| `windows-amd64` | `110329621893` | ✅ success |
| `linux-arm64` | `110329622030` | ✅ success |
| `macos-arm64` | `110329622127` | ✅ success |

Windows 的成功只证明 COFF 导出表观测路径和构建验证流程通过；该平台仍标记为
`unverified-coff-exports`，不把导出表当作 PE 内部节裁剪的证据。

---

## 八、本次不负责

- **不重开** N0-D 的验证机制设计（三种对象格式的解析、剥离产物拒绝、fat Mach-O）——
  那是 [10I](10i-n0d-artifact-verification-and-trimming.md) 已交付的部分，**本档是它的收口**。
- **不做交叉编译**、**不做主机工具链自动发现**（`10:57` 留到第 11 阶段）。
- **不做任何优化级别**（`10:80` 的基线固定 `-O0`，第 15 阶段才谈优化）。
- **不改** `10D §4.2` 的准备流程——若发现它不对，**改它并说明原因**，别在别处另写一份。

---

## 相关页面

- [10I. N0-D 产物验证与裁剪](10i-n0d-artifact-verification-and-trimming.md) —— **本档是它的收口**；§3.1 的两层证据与反例
- [10J. 跨操作系统验证专项](10j-cross-os-verification.md) —— 上一次跨平台收口的先例与踩坑记录
- [10. LLVM 原生后端](10-native-backend.md) `:78` —— 「纯静态标量程序不会完整装载 Runtime」
- [10D. 环境门控测试规范](10d-environment-gated-test-spec.md) **§4.2** —— Linux/macOS 准备的权威
