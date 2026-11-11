# 20O-B4. JSON/TOML 值模型与解析器实施规划

> **B4 尚未开始；B2-C、B3 完成或明确授权后才有开工入口。**
> B4 交付专用 `json_value`/`toml_value` 值模型与 `json`/`toml` 解析/序列化，不把值类型并入公开 `dynamic`。
> B4 不实现 B5 decimal/fractions/random、B6 regex/compression、B7安全、B8网络、B9数据库。
> 值模型必须与 B5 数值类型分开评审，不把 JSON 数字直接等同 lfloat/decimal。

## 一、入口与范围

| 项 | 当前状态 |
| --- | --- |
| B2 | B2-C 最终公共收口尚未完成；B4 开工须以其最终产物或明确授权为基线 |
| B3 | 简单宿主库尚未完成；B4 按路线在其后，不能把 B3 能力当默认前置 |
| B0 | 总纲/D-1～D-22 已确认方向；D-7 专用 json_value，不扩大公开 dynamic |
| 本批目标 | JSON/TOML 值模型、解析、序列化、错误/资源、语言承载、教程 |
| 未做 | B5数值库、B6原生工具、B7安全、B8网络、B9数据库 |
| 状态 | json/toml registry/教程初始planned；验证后按范围升级 |

B4 是 **数据格式批**，不是数值批。JSON/TOML 的 number 表示、精度和转换必须在 B4-0 冻结，不能借用 B2 的 lfloat/decimal 语义。

### 1.1 要做

1. 探测 JSON/TOML 现有语法、平台/依赖和宿主边界；
2. 冻结专用 `json_value`/`toml_value` 类型、所有权、相等/哈希、显示；
3. 冻结 JSON 严格语法、数字/字符串/Unicode/重复键/深度/大小限制和错误身份；
4. 冻结 TOML 子集/完整版本、日期时间、表/数组表、重复键、数字和限制；
5. 实现解析器、序列化器和错误/资源原子性；
6. 接通类型/IR/TAC/VM/LLVM/归档、std/stdlib双路径、导出/移除/隔离；
7. 编写可执行教程、固定示例和三平台证据；
8. 只在验证范围升级 registry/教程；B4 值模型与 B5 数值类型分开。

### 1.2 明确不做

- 不把 json/toml 值并入公开 `dynamic` 或扩大 JSON→lfloat 自动转换；
- 不实现 B5 decimal/fractions/statistics/random；
- 不实现网络/数据库/正则/压缩/安全；
- 不改变 B2 数值语义、intrinsic 1～48/ID100；
- 不新增未冻结能力位/错误码；
- 不提前升级 registry/教程，不跑旧721。

## 二、开工闸门

1. B2-C 最终公共收口完成，或星崽明确授权 B4-0 探测/冻结；
2. B3 完成或明确授权 B4 并行；B4 不把 B3 未交付能力当默认；
3. B0/D-7、20P Day-1 标准、现有类型/生命周期/能力/错误/取消边界可复现；
4. 当前 intrinsic/ABI/生成器/产物版本身份已固定；
5. B1七库20入口/17资源/双精确/28守卫可复现；
6. 三平台目标/工具链/CI可用；
7. 唯一任务根、独立target/Runtime/缓存已建立；
8. JSON/TOML 精确语法/值模型/限制/教程未冻结项列入 B4-0，不得先写产品后补契约。

## 三、分批执行

### B4-0：探测、冻结与教程草案

1. 探测 JSON/TOML 语法、现有解析/序列化依赖、平台差异、错误/资源/取消；
2. 冻结值模型：
   - `json_value`：null/bool/number/string/array/object；
   - `toml_value`：string/integer/float/bool/datetime/array/table；
   - 不并入公开 dynamic；对象/数组/表的所有权与生命周期；
   - number 表示（文本/整数/浮点边界），不直接等同 lfloat/decimal；
   - 相等/哈希/显示或明确不提供；
3. 冻结 JSON：
   - RFC 8259 严格语法；重复键、尾随逗号、注释、NaN/Inf、BOM、非法UTF-8/Unicode转义；
   - 深度/节点/字符串/输入字节上限；错误身份与优先级；
4. 冻结 TOML：
   - TOML 版本/子集；表/数组表/内联表/数组；日期时间/时区；
   - 重复键、非法数字、深度/节点/输入上限；错误身份；
5. 冻结解析/序列化 API、参数/返回类型、效果/所有权、能力；
6. 冻结 intrinsic/ABI/生成器/`.xiaoc`/`.xar`身份增量（如需）；
7. 为 json/toml 各写可执行教程草案、固定示例和registry候选；
8. 记录后置项（流式解析、JSON Pointer、TOML扩展等）。

退出：值模型/语法/限制/错误/版本/教程草案冻结；未写产品。

### B4-1：`json_value` 与解析器

1. 专用值模型、所有权、生命周期；
2. 严格 JSON parser：字符串/Unicode/数字/对象/数组；
3. 深度/节点/字节/字符串限制；
4. 错误身份/顺序/失败无部分结果；
5. 序列化：确定性键序/转义/数字文本；
6. 单元/边界/教程测试。

退出：JSON 值模型/解析/序列化内部可用。

### B4-2：`toml_value` 与解析器

1. 专用值模型、日期时间表示；
2. TOML表/数组表/内联表/数组/字符串/数字；
3. 重复键/非法数字/日期时间/深度/节点限制；
4. 错误身份/顺序/失败无部分结果；
5. 序列化：确定性输出；
6. 单元/边界/教程测试。

退出：TOML 值模型/解析/序列化内部可用。

### B4-3：语言、双路径与承载

1. json/toml类型/IR/TAC/VM/LLVM/归档；
2. `std.json`/`stdlib.json`、`std.toml`/`stdlib.toml`双路径；
3. 导出/移除/普通包隔离；
4. 值承载/序列化/并发不暴露Rust布局；
5. 能力/错误/取消/资源；
6. B1七库20/17/双精确/28守卫；
7. 三平台VM/原生O0/O3。

退出：两库内部全链路一致；无第二套值模型。

### B4-4：测试、教程与收口

1. JSON/TOML正常/边界/错误/资源/Unicode/日期时间；
2. 六份教程示例进入VM/原生精确入口；
3. registry只在验证范围升级；
4. 三平台/CI/失败样本；
5. 与B5数值类型边界复核；
6. 形成B5交接。

退出：B4内部收口；B5未开始；未升级未验证范围。

## 四、证据矩阵

| 层 | 必测 |
| --- | --- |
| 值模型 | json_value/toml_value、所有权、相等/显示、与dynamic分离 |
| JSON | 严格语法、Unicode/转义、重复键、NaN/Inf、深度/大小 |
| TOML | 表/数组表/内联表、日期时间、数字、重复键、限制 |
| 序列化 | 确定性、转义、数字文本、失败无部分结果 |
| 管线 | 类型/IR/TAC/VM/LLVM/归档/双路径/移除/隔离 |
| 平台 | Windows/Linux/macOS、O0/O3、CI |
| 边界 | 无B5-B9、无数值语义/registry冒进、无721 |

## 五、退出条件

1. B4-0值模型/语法/限制/错误/教程冻结；
2. JSON/TOML解析/序列化内部可用；
3. 双路径/导出/移除/隔离/能力/三平台；
4. 教程示例进入CI，registry只升验证范围；
5. B1回归/28守卫通过；
6. 无B5-B9、无数值语义/未冻结ABI/721；
7. B5交接明确；B4内部收口。

## 六、风险与禁止

- 把json/toml值并入公开dynamic；
- JSON数字直接等同lfloat/decimal；
- 重复键/NaN/Inf/BOM/尾随逗号规则未冻结；
- TOML日期时间/表/重复键语义模糊；
- 解析失败留下部分值；
- 提前升级registry/教程；
- 把B4扩成B5-B9或B2数值改动。

## 七、交接

B4完成后进入B5：decimal/fractions/statistics浮点/random。B4值模型与B5数值类型分开评审；B4的XML/流式/JSON Pointer等后置。

新 Agent 先读：

1. [官方库总纲](20o-official-library-program.md)；
2. [20P Day-1架构](20p-day1-library-architecture.md)；
3. [B3宿主库规划](20o-b3-host-libraries-plan.md)；
4. [B2-C最终收口](20o-b2-c-final-closeout-plan.md)；
5. [B1收口](20o-b1-closeout.md)。

## 八、落点

```text
stdlib/json.xiao / toml.xiao
docs/UseDocs/library/json.md / toml.md
docs/module-registry.json
core/rust/crates/xiao-*/                               类型/IR/VM/LLVM/解析
docs/DevDocs/assets/20o-b4/entry-<date>/               B4证据
```

本页只规划B4；B5-B9仍后置。
