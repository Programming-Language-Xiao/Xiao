# 10Z-取数. 受控主机性能取数交接（O6 条件 5 的 Linux 格）

> **写给谁**：在**受控主机**（固定硬件 + Temurin/OpenJDK 21）上执行本轮的操作者（人，不是 Agent）。
> **这一轮要什么**：用已经做好的**计时驱动器**跑一轮三路对照，产出**可复核的原始数字**。
> **这一轮不做什么**：不判达标、不调优、不碰 C 档（已完成）。
>
> 状态：**待执行**（驱动器已就绪并通过 CI，见 [10Z-驱动](10z-performance-driver.md)）。

## 〇、这一轮的定位（先读）

1. **它与 C 档那轮不是同一轮**。原先定的是「C 档与取数同一轮」，但审核实测那个前提**不成立**——
   当时两侧的计时驱动器都不存在。现在已拆开：**C 档已完成**（PR #5 合并 `efa9fd2`），
   本轮**只做取数**；
2. **产出的是「原始数字 + 统计」，不是「达标结论」**。阈值在取数完成之前是 `unset`，
   所以本轮**只能**给出比值与置信区间——**不许写「达到 Java」**（[10Y](10y-b-series-triage-rework.md) §2.7 第 4 条、
   [10Z-驱动](10z-performance-driver.md) D5 第 4 条）；
3. **本轮只覆盖 Linux 那一格**。Windows 按已定口径**标缺**（无专用受控主机，不用本机数字顶替）；
   macOS **不可验证**（无宿主）——**不要**试图在本轮补齐它们。

## 一、要产出的东西

| 产物 | 判据 |
| --- | --- |
| **驱动器报告 JSON** | 顶层 `status`、`summary`、`bootstrap_determinism`、每 case 的 `samples_ns` 与 bootstrap 区间 |
| 驱动器**完整日志** | 原样回传，不要只给摘要 |
| **人工环境清单** | 见 §六——**跑之前落盘** |
| 是否执行 | `summary.measured_cases > 0` |

## 二、检出提交

```bash
git fetch origin && git checkout main && git pull --ff-only
git log --oneline -1        # 回传里要写明实际 sha
```

参考：截至本文撰写时为 `c45f2d0`。**以实际检出为准，并把 sha 写进回传。**

## 三、前置环境

### 3.1 必需

| 依赖 | 要求 | 为什么 |
| --- | --- | --- |
| `java` / `javac` | **Eclipse Temurin / OpenJDK 21**，且在 `PATH` 上 | 驱动器会**校验 major 版本与 `baseline.json` 一致**；版本不符**直接失败**——**这是设计，不是故障** |
| `XIAO_CLANG` | 能编译 LLVM IR 的 clang | 原生侧构建 |
| `XIAO_RUNTIME_LIBRARY` | **先构建好**的 release staticlib | 原生侧链接 |
| `XIAO_TARGET_TRIPLE` | 与 Runtime staticlib 相同的目标（Linux 上取 `rustc -vV` 的 `host`） | 原生侧目标 |

**驱动器的代码只读上面这三个 `XIAO_*`（另加可选的 `RUSTC`，缺省用 `rustc`）。**
[10D](10d-environment-gated-test-spec.md) §4 的清单里还有 `XIAO_LLVM_AS` / `XIAO_LLC` / `XIAO_STRIP`——
**驱动器不读它们**（若原生构建路径间接需要，会以实际报错为准）。`bun` 只是**可选**：驱动器用它**记录版本**，
装了更好（环境清单更全），不装也能跑。

### 3.2 准备

```bash
# 1) Runtime staticlib（必须先有）
cd <仓库>/core/rust && cargo build --release -p xiao-runtime && cd -

# 2) 原生工具链环境（按 10D §4；Linux 用发行版 LLVM）
export XIAO_CLANG="$(command -v clang)"
export XIAO_RUNTIME_LIBRARY="$PWD/core/rust/target/release/libxiao_runtime.a"
export XIAO_TARGET_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"

# 3) 确认 Java 是 21（不是 21 就先别跑，跑了也会被拒）
java -version
javac -version
```

## 四、跑法

```bash
cargo run --release --manifest-path tests/benchmarks/Cargo.toml \
  --bin performance_driver -- --output /tmp/10z-performance-report.json 2>&1 | tee /tmp/10z-performance-log.txt
```

- `--id <benchmark>` 可以**只跑一个基准**，用于定位问题（正式取数要跑全套）；
- 驱动器**自己**按 `baseline.json` 的协议跑（预热 3 / 测量 11 / percentile-bootstrap / 10000 / seed 19015）
  ——**不要**在命令行上调这些，协议是冻结的；
- 三路（Java / 原生 / VM）由驱动器自己调度，**你不需要分别跑**；
- 每一次测量之后驱动器都会**校验输出与 `manifest.json` 的期望值**——不符就报错，**不会记进样本**。

**跑之前**先把 §六 的人工环境清单落盘。**不要跑完再补**。

## 五、怎么读报告（判据）

| 看什么 | 应然 | 不对就报回来 |
| --- | --- | --- |
| `summary.total_cases` | 与 `manifest.json` 的基准数一致 | 不一致 |
| `summary.measured_cases` | **> 0** | **= 0 时必须看下一行** |
| 顶层 `status` | 有实测时 `development-evidence` | **`measured_cases == 0` 却是 `development-evidence`**——那是缺陷（[10Z-驱动](10z-performance-driver.md) D5 第 6 条），把报告发回来 |
| `bootstrap_determinism.byte_identical` | **`true`** | `false` 说明 bootstrap 不可复现，停手报回来 |
| 各 case 的 `performance_status` | `measured` 或 `data-insufficient` | 其它值 |
| `data-insufficient` 的 case | **是诚实结果，不是失败** | **但也不要当成通过**——它没有数字 |

**逐个 `data-insufficient` 都要看 `reason`**：里面写清了为什么不可比（例如溢出语义不同、原生拒绝构建）。
**不要**为了让它变成 `measured` 而反复重跑或改源码——**不可比就是不可比**。

## 六、要回传的东西

**三样一起**，缺一不可：

1. **报告 JSON**（`--output` 的那个文件）——它已经含**原始样本 `samples_ns`**、主机快照、构建指纹；
2. **驱动器完整日志**（命令输出原样，不要只给摘要）；
3. **人工环境清单**——跑**之前**落盘，至少含：

```text
检出提交（git log --oneline -1）：
CPU 型号与核数：
内存：
OS 版本与内核（uname -a）：
java -version 完整文本（逐行）：
javac -version：
rustc -vV 的完整 host 行：
clang / llvm 版本：
bun 版本（若装了）：
XIAO_CLANG / XIAO_RUNTIME_LIBRARY / XIAO_TARGET_TRIPLE 的实际取值：
执行时间点（含时区）：
主机负载与后台进程（跑之前与跑之后各记一次）：
```

> 驱动器自己的 `HostSnapshot` 也会记一部分主机信息，**但人工清单仍要**——
> 它是**可读的**，而且**不依赖驱动器正确性**。两者不一致时要指出。

**只要结论不要原始数据的回传会被退回**。

## 七、先别做的事

1. **不要剔除离群样本**——协议写死 `record-host-load-and-background-processes; do-not-discard-samples`，
   负载与后台进程要**记录**，不是删样本；
2. **不要调预热/测量次数**，也不要改 `baseline.json` 的协议——那是基线变更，属另一件事；
3. **不要用开发机或 CI 的数字顶替**受控取数（[10Z-驱动](10z-performance-driver.md) D6）；
4. **不要因为某 case 是 `data-insufficient` 就重跑到它变 `measured`**；
5. **不要换 JDK 版本让它过**——版本校验是刻意的；
6. **不要写达标结论**——阈值取数后才冻结；
7. **不要顺手改驱动器的源码**——发现问题把报告与日志发回来，改是实现的活。

## 八、回传之后会发生什么（供你理解这轮的分量）

1. **取数一完成就冻结阈值**（星崽已定，不是拖到下一批）：填 `threshold` 的
   `status` / `allowed_error` / `regression_limit`，并写出**冻结依据**（比值、置信区间、误差来源），
   同步 `baseline.json` 的 `baseline_id` 与 sha256；
2. 然后才按阈值给出「**通过 / 回归 / 数据不足**」的判定；
3. **Z-4 的 19 O6 终局对账**等这一步——条件 5 是 O6 五条里唯一还「不满足」的一条。

也就是说：**这一轮的数字会直接决定 19 能不能收口**。

## 九、回传模板

```text
检出提交（git log --oneline -1）：
本轮目标：O6 条件 5 的 Linux 取数

== 环境（跑之前落盘）==
<粘贴 §六 第 3 项的全部字段>

== 执行 ==
命令：
退出码：
耗时：

== 报告判读 ==
summary.total_cases / measured_cases / data_insufficient_cases：
顶层 status：
bootstrap_determinism.byte_identical：
各 case 的 performance_status 与（若为 data-insufficient）reason：
<逐条贴>

== 附件 ==
报告 JSON 路径 / 文件名：
日志文件路径 / 文件名：

== 其他 ==
遇到的问题：
不确定的地方：
```

## 相关页面

- [10Z-驱动. 性能对照的计时驱动器](10z-performance-driver.md) —— 本轮所用工具；D1–D7 的冻结条款
- [10Z-Linux. C 档交接](10z-linux-bare-metal-handoff.md) —— **上一轮**（C 档，已完成；其 §〇 的「本轮不做取数」已被本文取代）
- [19D. 性能对照](19d-performance-comparison.md) —— O6 条件 5、`baseline.json` 的协议与平台登记口径
- [10Y. 返工与 B 系列分类收口](10y-b-series-triage-rework.md) §2.7 —— 受控取数的七条冻结
- [10D. 环境依赖测试专项规范](10d-environment-gated-test-spec.md) §4 —— 原生工具链准备
