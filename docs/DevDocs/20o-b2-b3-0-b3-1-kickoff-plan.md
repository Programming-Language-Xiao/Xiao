# 20O-B2-B3-0/B3-1. cmath 开工基线与 F-9 参考流水线规划

> **这是 B3 的开工细化；B3 产品尚未开工，B4～B7 与 B2-C 仍未开始。**
> 入口为 B2 内部收口代码 `37b92c53`、B3 规划 `f23c4493`；当前活动 intrinsic 最大 33，ABI 1.18、生成器 20。
> B3-0 只做基线，B3-1 只做 MPFR/GMP 参考与私有内核升级；不激活 34～48、不写 C ABI、不升版本。
> `stdlib.cmath` 仍 planned，旧 721 条 F10 不运行。

## 一、入口与目标

| 项 | 值 |
| --- | --- |
| B2 代码 | `37b92c53`（全局 `lfloat_divide`/intrinsic 33 内部收口） |
| B2 证据 | B2 审计 99 文件、14 执行点、4 精确入口通过 |
| 当前公开面 | ABI 1.18、生成器 20、活动 intrinsic 1～33、ID 100 墓碑 |
| B3-0 目标 | 独立工作树内复现 B2 基线并记录 34～48 未激活 |
| B3-1 目标 | MPFR 4.2.2/GMP 6.3.0 参考流水线、15 项私有内核、float `**` 共享边界 |
| 不在本页 | intrinsic 34～48、15 个 C 符号、`stdlib/cmath.xiao`、语言全链路、ABI 1.19/21 |

B3-0/B3-1 完成后，B3-2 才做 15 项 Runtime/C ABI，B3-3 才激活 34～48；B4 才接类型/IR/VM/LLVM。

### 1.1 要做

1. 从当前 HEAD 建独立工作树，验证 B2 产品树与 `37b92c53` 一致；
2. 复现 B1/B2 的 Runtime、C ABI、VM/原生精确入口和 28/28 守卫；
3. 记录 intrinsic 1～33 活动、34～48 不活动、ID 100 墓碑；
4. 建立离线 MPFR/GMP 参考生成器、输入位模式格式和摘要；
5. 把 B1 的常量 ULP 初检替换为正式参考向量；
6. 完成 15 项 cmath 私有内核的特殊值/域/符号/ULP 测试；
7. 建立 float `**` 与 `cmath.pow` 共享内核的行为差表；
8. 保存工具来源、版本、生成器摘要、参考输入/输出摘要和核对日志。

### 1.2 明确不做

- 不激活 intrinsic 34～48，不新增 C 符号或 `stdlib/cmath.xiao`；
- 不升 ABI 1.19/生成器 21，不改 `.xiaoc`/`.xar`/原生元数据；
- 不实现 sfloat 版 cmath、默认参数、重载或新自动转换；
- 不在产品代码引入 C/GMP/MPFR 依赖，不让运行时依赖宿主 libm 黄金；
- 不升级 registry/教程，不跑旧 721，不关闭历史债务。

## 二、B3-0 开工基线

### 2.1 工作树与身份

1. 在当前仓库外新建唯一任务根，例如 `D:\project\Xiao\.b3-cmath\<run-id>`；
2. 从当前 HEAD 建工作树，记录 `git rev-parse HEAD`；
3. 验证产品树与 B2 代码一致：

```text
git diff --name-only 37b92c53 <current-head> -- ':!docs/DevDocs'
```

结果必须为空；`docs/DevDocs` 的规划/证据差异不算产品差异。

4. 固定 Rust 1.96.0、目标平台、`Cargo.lock`、`--locked`，清理 `RUST_BIGDECIMAL_*`；
5. 固定独立 `CARGO_TARGET_DIR`、`XIAO_HOME`、`TEMP`/`TMP`、`XIAO_STDLIB_ROOT`；
6. 保留 `XIAO_B1_F10=0`、`XIAO_B2_A_PERF=0`。

### 2.2 基线命令与期望

| 检查 | 命令要点 | 期望 |
| --- | --- | --- |
| Runtime 全测 | `cargo +1.96.0 test --locked -p xiao-runtime` | 259 passed / 0 ignored |
| Driver 单元 | `cargo +1.96.0 test --locked -p xiao-driver --lib` | 64 passed / 1 ignored |
| intrinsic 表 | `cargo +1.96.0 test --locked -p xiao-intrinsics` | 7 passed / 0 ignored |
| B2 单入口 | `20o_b2_b2_divide` 两后端精确入口 | 各恰好 1 passed |
| B1 契约/资源 | `20o_b1_imp_contract` / `20o_b1_imp_resources` | 9 / 17 passed |
| 原 28 守卫 | `tools/gates/b1-probe-guards.ps1` | 28/28 |
| 工程门禁 | workspace check、benchmark `--locked`、Clippy、fmt | 全部退出 0 |
| 文档门禁 | `bun tools/repo-check/src/cli.ts all` | 退出 0 |

若计数与 B2 收口不一致，停止定位；不能改断言或重跑补绿。

### 2.3 公开面基线

1. 保存 `intrinsics.json`：1～33 active、34～48 不存在、100 tombstone；
2. 保存当前 C 符号表：有 `xiao_runtime_lfloat_divide`，无 `xiao_runtime_cmath_*`；
3. 记录 `ABI_MINOR_VERSION=18`、`CODEGEN_VERSION=20`、标准库接口 1；
4. 记录 `stdlib.cmath` planned、registry 未改、无 `stdlib/cmath.xiao`；
5. 记录 B2 的 99 文件/14 执行点/4 精确入口身份。

### 2.4 B3-0 证据

```text
docs/DevDocs/assets/20o-b2-b/entry-<date>-b3/b0/
  bootstrap.json            任务根/源码/工具链
  environment.json          overrides 与清理变量
  source-manifest.json      被测源码清单
  checks/*.json             命令/PID/退出码/前后摘要
  logs/*.log                原始日志
  intrinsic-baseline.json   1～33 active/34～48 absent/100 tombstone
  native-symbols.json       现有 C 符号基线
  closeout.json             B3-0 结论与未执行项
```

B3-0 退出：独立工作树与 B2 产品树一致；上述计数全部匹配；34～48 未激活，ABI/registry 未变。

## 三、B3-1 F-9 参考流水线与私有内核

### 3.1 工具与参考身份

1. 离线使用 MPFR 4.2.2/GMP 6.3.0；记录工具来源、构建方式、版本和二进制/源码摘要；
2. 产品仓库不新增 C/GMP/MPFR 依赖；参考工具只在证据任务根运行；
3. 输入按 binary64 位模式精确装入，不先转十进制文本；
4. 输出记录 binary64 位模式，或稳定错误身份（009/域/不可验证）；
5. 参考生成器版本、随机种子、输入/输出摘要写入 manifest；
6. 参考向量不放在运行时热路径，不参与产品构建。

### 3.2 区间判定

1. 从 256 位精度起向下/向上求值；
2. 两端都按最近偶数转换为同一 binary64 才接受；
3. 不同则逐次加倍到 4096 位；
4. 仍不能唯一确定时标不可验证，不生成猜测黄金；
5. 零/符号/域/非有限按精确表判断，不用数值容差掩盖错误身份。

### 3.3 验收标准

| 函数 | 标准 |
| --- | --- |
| sqrt | 正确舍入，0 ULP |
| floor/ceil/round/trunc/abs/min/max | 逐位精确，含零符号 |
| sin/cos/tan/exp/ln/log10 | 普通非零结果 ≤4 ULP |
| pow | 普通非零结果 ≤4 ULP，特殊值精确 |
| 域/NaN/Inf | 稳定 009 或确认不可验证，不容差 |
| 下溢/子正规 | 保留正确有符号零/子正规，不错误 flush |

### 3.4 私有内核

1. 把 B1 的 `cmath.rs` 常量初检替换为正式参考向量；
2. 15 项函数仍保持私有，不接 intrinsic/C ABI；
3. 全部入口先拒绝 NaN/Inf；`sqrt(-0)=-0`、`round(-0.5)=-0` 等按 F-2；
4. `exp` 正常下溢允许子正规/有符号零；非有限结果 009；
5. `pow` 处理 `0**0`、负底整指数、`-0` 奇偶、0 负幂和负底非整数域错误；
6. `tan` 不设 epsilon 极点，只按有限输入/输出；
7. 建立 float `**` 与 `cmath.pow` 共享内核的行为差表；
8. B3-1 **不切换** `ops.rs` 的 float `**` 调用点；B3-2/B3-3 再一次性接线，避免半状态。

### 3.5 B3-1 证据

```text
docs/DevDocs/assets/20o-b2-b/entry-<date>-b3/b1/
  reference-manifest.json   MPFR/GMP 版本/来源/摘要/种子
  reference-vectors/        输入/输出位模式与错误
  generator.log             生成命令与原始输出
  kernel-tests.json         Runtime cmath 测试
  pow-diff.json             float ** vs cmath.pow 行为差
  source-manifest.json      允许路径与源码摘要
  closeout.json             B3-1 结论
```

B3-1 退出：

- 15 项函数有正式参考向量，无宿主 libm 自证；
- 特殊值/域/符号/下溢/ULP 测试通过；
- float `**` 与 `cmath.pow` 行为差已记录，尚未半接线；
- 没有 intrinsic 34～48、C 符号、stdlib/cmath、ABI/registry 改动；
- B2 的 33 单入口和 B1 回归仍通过。

## 四、允许改动

```text
core/rust/crates/xiao-runtime/src/value/arithmetic/cmath.rs
core/rust/crates/xiao-runtime/src/value/arithmetic/cmath_tests.rs（如新建）
docs/DevDocs/assets/20o-b2-b/entry-<date>-b3/**
```

B3-0 不写产品；B3-1 不改 `ops.rs`、`intrinsics.json`、`xiao-runtime-abi`、`stdlib`、registry。

## 五、风险与禁止

- 用宿主 `std`/`libm` 结果生成同算法黄金；
- 只取单边高精度值，未做向下/向上区间判定；
- 用 4 ULP 容差放过错误符号、错误域或错误错误身份；
- 把参考工具带进产品依赖或运行时；
- 在 B3-1 提前激活 34～48/C ABI 或切 float `**`；
- 为通过参考测试改语义、删特殊值或降低精度；
- 把 B3-1 私有参考写成 B3 Runtime/C ABI 或 B2-C 完成。

## 六、交接

B3-1 完成后进入 B3-2：15 项 Runtime/C ABI、intrinsic 34～48 与 `stdlib/cmath.xiao` 包装。B4 再接语言全链路，B5 切 ABI 1.19/21。

新 Agent 先读：

1. [B3 cmath 规划](20o-b2-b3-cmath-plan.md)；
2. [B2 lfloat_divide 实施记录](20o-b2-b2-lfloat-divide-progress.md)；
3. [已签冻结正文](20o-b2-freeze-draft.md) §七～§九；
4. [F-9 误差与共享内核](20o-b2-freeze-draft.md#七cmath行为误差与二进制转换f-2f-8f-9已确认)；
5. [F-5 原语/C ABI](20o-b2-freeze-b1-closeout-plan.md)。

## 七、落点

```text
core/rust/crates/xiao-runtime/src/value/arithmetic/cmath.rs  15 项私有内核
docs/DevDocs/assets/20o-b2-b/entry-<date>-b3/b0/             B3-0 基线证据
docs/DevDocs/assets/20o-b2-b/entry-<date>-b3/b1/             B3-1 参考证据
```

本页只细化 B3-0/B3-1；B3-2 之后与 B4～B7、B2-C 后置。
