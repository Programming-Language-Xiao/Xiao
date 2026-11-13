# 20O-B2-B4-4. cmath 双路径、归档与序列化实施规划

> **B4-2A 与 FIX1 已完成 Windows 本机内部退出；B4-2B 已在246bea46完成，B4-3 尚未完成；B4-4 仍未开始，B5～B7 与 B2-C 后置。**
> 入口为 B4-3 的 LLVM 动态/静态路径、B4-2 的 VM 执行与 B4-1 的类型/IR。
> B4-4 只做 `std.cmath`/`stdlib.cmath` 双路径、导出/移除、`.xar` 归档、序列化和普通包隔离。
> 不升 ABI 1.19/21，不升级 registry/教程；`stdlib.cmath` 仍 planned，B2-C 才最终发布。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B4-1 | 15 项类型/IR 接通 |
| B4-2 | VM 15 项执行已在246bea46完成Windows本机内部验收；模块导入仍planned |
| B4-3 | LLVM 动态/静态与 float `**` 共核（待完成） |
| 当前导入 | 30 条 `std.cmath`/`stdlib.cmath` 路径仍 planned 拒绝 |
| 归档 | archive closure 可收集 `stdlib.cmath`，但 `xiao-xar` runner 尚无标准库加载器 |
| 本批目标 | 双路径可执行、导出/移除、真实 `.xar`/`.xiaoc`、序列化/隔离 |

B4-4 是 **内部语言可用性批**，不是最终发布。B4-5 记录内部 `language_available=true`；教程/registry 仍 planned，B2-C 才做最终 721 与公共升级。

### 1.1 要做

1. 让 `std.cmath` 与 `stdlib.cmath` 经固定标准库加载器解析为同一 `stdlib.cmath` 身份；
2. 15 个公开包装真实导出，逐函数移除后导入失败；
3. 普通包不得覆盖 `std`/`stdlib` 的 cmath 命名空间；
4. 真实 `.xar`/`.xiaoc` 归档包含 34～48 调用并可重放；
5. `xiao-xar` 运行器接入标准库加载器，执行 cmath 模块；
6. 序列化/并发值承载保持既有 float/文本边界，不暴露 Runtime 布局；
7. 30 条 planned 导入测试改为内部可执行断言，旧拒绝证据保留；
8. 保存双路径/移除/归档/序列化/隔离/回归证据。

### 1.2 明确不做

- 不升级 `stdlib.cmath` registry 或最终教程；
- 不升 ABI 1.19/生成器 21，不改 `.xiaoc`/`.xar` 容器 schema；
- 不新增 intrinsic 49+/C 符号，不改 1～48/ID 100；
- 不实现 sfloat cmath、默认参数、重载或新转换；
- 不跑旧 721，不关闭历史债务；
- 不把 B4-4 内部可用写成 B2-C 最终发布或三平台产品。

## 二、开工闸门

1. B4-2/B4-3 审计通过，VM/LLVM 15 项与 float `**` 共核可复现；
2. 30 条 planned 导入仍拒绝，B3-4 证据完整；
3. 15 个 C 符号、intrinsic 34～48、双路径 canonical identity 存在；
4. `xiao-xar` 当前无标准库加载器，错误为“未注册官方标准库加载器”；
5. ABI 1.18/生成器 20/接口 1/cmath planned 未变；
6. 唯一任务根、独立 target/Runtime/缓存已建立。

## 三、分批执行

### B4-4A：双路径与导出/移除

1. 固定标准库加载器把 `std.cmath` 与 `stdlib.cmath` 都解析为 `stdlib.cmath`；
2. 15 个公开包装可被 `from std.cmath import ...`/`from stdlib.cmath import ...` 导入；
3. 逐函数移除后，对应导入失败；未移除函数仍可用；
4. 普通包依赖不得覆盖 `std.cmath`/`stdlib.cmath`；包命名空间冲突按既有诊断拒绝；
5. 不改变 B1 七库的双路径/移除语义；
6. 保存导出集合、双路径、移除和包覆盖负例。

退出：双路径 15 项可用；移除/隔离正确。

### B4-4B：`.xar`/`.xiaoc` 归档

1. 归档 closure 记录 `stdlib.cmath` 身份和 34～48 调用；
2. 归档索引不写 `std.cmath` 别名，统一 `stdlib.cmath`；
3. `xiao-xar` 运行器注册标准库加载器，能从归档执行 cmath；
4. 重放同一源码/黄金；缺加载器时不再以 planned 拒绝作为最终行为；
5. 旧 1.18 容器格式/索引 schema 不变；不改 replayer 版本规则；
6. 保存真实 `.xar`/`.xiaoc`、索引、加载器和重放证据。

退出：真实归档可执行 15 项 cmath；身份/别名/加载器一致。

### B4-4C：序列化与并发承载

1. cmath 输入/输出为 float，复用既有值承载/序列化规则；
2. 不新增 XCON 字段、标签或 Rust 布局；
3. 并发/任务值携带 cmath 结果时按既有 float 规则；
4. 归档中的模块身份与普通包隔离保持；
5. 保存序列化/并发/归档承载证据。

退出：无序列化/并发格式变化；cmath 值承载一致。

### B4-4D：测试与证据

1. 双路径 15 项正常/错误；
2. 逐函数移除、普通包覆盖负例；
3. `.xar`/`.xiaoc` 重放，含 34～48 调用；
4. 30 条 planned 测试改为内部可执行；旧 planned 拒绝证据保留；
5. B4-1/2/3 回归、B3-4 wrapper 8/0、22c 29/0、B1/B2/B3 历史；
6. 保存源码、工具链、目标、PID、日志、前后摘要；
7. 记录 Windows 本机与未执行平台；
8. 复核 registry/教程仍 planned，旧 721 未跑。

退出：B4-4 内部证据完整；B4-5 未开始；公共状态仍 planned。

### B4-4E：收口与 B4-5 交接

1. 复核双路径/归档/序列化/隔离；
2. 复核 30 条路径已内部可执行，但公共 registry/教程仍 planned；
3. 复核无 ABI 1.19/21、无 721、无历史债务关闭；
4. 形成 B4-5 入口：整批语言收口、三平台/证据边界。

退出：B4-4 内部收口；B4-5 未开始。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 双路径 | `std.cmath`/`stdlib.cmath` 同身份、15 导出 |
| 移除 | 逐函数移除失败、其余仍可用 |
| 隔离 | 普通包覆盖拒绝、B1 七库不变 |
| 归档 | `.xar`/`.xiaoc` 34～48、索引身份、加载器、重放 |
| 序列化 | 无 XCON/布局/schema 变化 |
| 并发 | float 值承载、归档隔离 |
| 测试迁移 | 30 条 planned 改内部可执行，旧证据保留 |
| 边界 | registry/教程 planned、ABI1.18/20、721 未跑 |

## 五、退出条件

1. 双路径 15 项可用；逐函数移除和普通包隔离通过；
2. 真实 `.xar`/`.xiaoc` 含 34～48，标准库加载器可执行；
3. 序列化/并发/归档无格式或布局变化；
4. 30 条路径内部可执行，旧 planned 拒绝证据保留；
5. registry/教程仍 planned；无 ABI 1.19/21 或 721；
6. B4-1/2/3、B3-4、B1/B2/B3 回归通过；
7. B4-5 未开始；公共发布仍未成立。

## 六、风险与禁止

- 归档中写 `std.cmath` 别名，或加载器把别名当项目模块；
- 用旧 planned 拒绝测试冒充双路径可用，或删掉旧证据；
- 逐函数移除后仍能导入；
- 普通包覆盖标准库命名空间；
- 改 `.xar`/`.xiaoc` schema 或 XCON 来“接上” cmath；
- 提前升级 registry/教程或宣称 B2-C 发布；
- 跑旧 721 或关闭历史债务。

## 七、交接

B4-4 完成后进入 B4-5：整批语言收口、三平台/证据边界；B5 切 ABI 1.19/21，B6 性能，B2-C 最终 721/发布。

新 Agent 先读：

1. [B4 语言全链路规划](20o-b2-b4-language-integration-plan.md)；
2. [B4-2A/B 执行规划](20o-b2-b4-2a-b-execution-plan.md)；
3. [B4-3 LLVM 规划](20o-b2-b4-3-llvm-plan.md)；
4. [B3-4 收口](assets/20o-b2-b/entry-20261110-b3-wrapper-final/README.md)；
5. [F-5 原语/C ABI](20o-b2-freeze-b1-closeout-plan.md)。

## 八、落点

```text
core/rust/crates/xiao-driver/src/archive*.rs            归档闭包/索引
core/rust/crates/xiao-xar/src/runner.rs                 标准库加载器
core/rust/crates/xiao-vm/src/modules/                   双路径加载
stdlib/cmath.xiao                                       15 个包装
docs/DevDocs/assets/20o-b2-b/entry-<date>-b4/           B4-4 证据
```

本页只做 B4-4；B4-5、B5～B7 与 B2-C 仍后置。
