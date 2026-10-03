# 16A-FIX. 内容寻址边界的编解码对称与单一来源修复

> **16A 的审核修复批。** 16A 主体（`6796298`）与加固（`575c960`）已交付，
> 交接文档 §八 列的 **12 条翻车点逐条核对全部通过**，`cargo test --workspace`、
> 严格 Clippy、`bun run check`（退出码 0）全绿。
>
> **本批不扩大功能范围**，只修审核发现的边界缺陷：**编解码校验不对称**与
> **`.xiaoc` 契约常量的重复定义**。另有两项**明确不做**，见 §五。

## 一、Agent 交接上下文

### 接手前提

先读 [16A 交接文档](16a-content-addressed-objects-and-indexes.md) 的 §三（7 条冻结契约）
与 §八（12 条翻车点）——本批修的是**契约之外的实现缺陷**，那两节是本批的背景而非修改对象。

### 本批边界

| 编号 | 问题 | 处理 |
| --- | --- | --- |
| 2.1 | `ArchiveIndex::decode` 不校验入口字段 | **本批修**（§三） |
| 2.2 | `.xiaoc` 魔数与最小长度重复定义 | **本批修**（§四） |
| 2.3 | 索引锁崩溃残留 | **不做**，登记给 16B（§五） |
| 2.4 | 新增 `A0-COVERAGE-002` 文档警告 | **不做**，见 §五 |

### 证据来源

§二 的每一条都在审核中**实测或精确定位过行号**，接手者不需要重新调查；
行号基于 `575c960` 时点的 `main`。

## 二、问题清单

### 2.1 **归档索引解码不校验入口字段** —— 编解码不对称

`ArchiveIndex::encode` 拒绝空入口（`xiao-artifacts/src/lib.rs:826`）：

```rust
if self.entry.is_empty() {
    return Err(ArtifactError::Index("归档索引缺少入口".to_owned()));
}
```

而 `ArchiveIndex::decode`（`xiao-artifacts/src/lib.rs:866-883`）只有三项检查——
`schema_major`、`record_type`、`required_features`——**唯独不查 `entry`**。
解码时若字段 3 不出现，`entry` 保持 `String::new()`（`xiao-artifacts/src/lib.rs:848`）。

**审核实测**（手工构造 wire 字节，已复现）：

```
[探针] 缺 entry 字段 -> Ok(ArchiveIndex { schema_major: 1, schema_minor: 0, entry: "", entries: [] })
[探针] 空 entry 字段 -> Ok(ArchiveIndex { schema_major: 1, schema_minor: 0, entry: "", entries: [] })
[对照] 非空 entry   -> Ok(ArchiveIndex { schema_major: 1, schema_minor: 0, entry: "mai", entries: [] })
```

**为什么是问题**：一个**没有入口**的 `.xar` 索引会被判定为合法。规范 §3.5 要求
每个 `.xar` **固定且仅有一个**索引，其入口是启动的唯一依据；入口缺失应在**解码时**
就被拒绝，而不是留给消费者在运行时发现。当前 16A 尚未接入 CLI，所以**还没有真实受害路径**——
但 17 阶段接 `.xar` 归档时，这里就是漏洞入口。

**根因不是"漏了一行"**，而是**校验逻辑在 encode 与 decode 各写一份**，必然漂移。
同类重复在文件中还有一处：`validate_archive_entry`（`xiao-artifacts/src/lib.rs:1012`）
检查 `logical_path`/`module`/`target` 非空，而 `decode_archive_entry` 在
`xiao-artifacts/src/lib.rs:1052` **又写了一遍**同样的三项。

### 2.2 **`.xiaoc` 魔数与最小长度重复定义** —— 单一来源违规

权威定义在 `xiao-bytecode`：

- `xiao-bytecode/src/xiaoc.rs:15` —— `pub const XIAOC_MAGIC: [u8; 8] = *b"XIAOC\r\n\x1a";`
- `xiao-bytecode/src/xiaoc.rs:21` —— `pub const XIAOC_HEADER_MIN_SIZE: usize = 72;`

`xiao-artifacts` **已经依赖 `xiao-bytecode`**（`xiao-artifacts/Cargo.toml:17`），却：

- 在 `xiao-artifacts/src/lib.rs:19` **重新定义了一份魔数**（类型还不同：`&[u8; 8]`）
- 在 `xiao-artifacts/src/lib.rs:386` 与 `:433-434` **两处硬编码字面量 `72`**

**为什么是问题**：这正是本仓头号病。14A 的头部尺寸一旦变化，`XIAOC_HEADER_MIN_SIZE`
会改，而这两处字面量不会跟着变——**同一契约两层各写一份然后漂移**。

**顺带**：这两处预检**本身就是冗余的**。`xiao-bytecode/src/xiaoc.rs:740` 的
`decode_xiaoc` 已经做了 `bytes.len() < XIAOC_HEADER_MIN_SIZE || bytes[..8] != XIAOC_MAGIC`，
而 `validate_xiaoc` 就是 `decode_xiaoc(bytes)?.header`。所以 artifacts 侧的手写预检
**没有增加任何保护**，只增加了一处漂移点。

### 2.3 **索引锁的崩溃残留**（不做，转 16B）

`IndexLock::acquire`（`xiao-artifacts/src/lib.rs:712`）用 `create_new` 建立锁文件，
只在 `Drop`（`xiao-artifacts/src/lib.rs:722`）里删除。审核 grep `stale|timeout|retry|过期`
**全无匹配**——进程被 kill 或崩溃时锁文件残留，**后续所有索引操作永久失败**。

**这不算违反**规范验收第 3 条（"中断不会留下可被**误认为有效**的半成品"）——
锁文件不会被误认成索引。但它会让缓存**永久卡死**，属于可用性缺陷。

**为什么归 16B**：修它需要设计决策而非补一行代码。推荐方向是**改用 OS 级文件锁**
（Unix `flock` / Windows `LockFileEx`），由内核在进程死亡时自动释放，**从根上消除
stale 概念**，而不是再写一层 pid/时间戳的启发式判断。该方案的验证面是**多进程行为**，
正是 16B 的范围（16A 文档 §十 已把"多进程测试"划归 16B）。

### 2.4 **新增 44 条文档警告**（不做）

`A0-COVERAGE-002` 在 `xiao-artifacts` 有 44 条。**不设为出口条件**：该规则当前是
warning 级、不阻塞门禁，且全仓普遍存在（`xiao-bytecode` 52 条、`optimize.rs` 34 条、
`version.rs` 21 条）。单独补齐一个 crate 会制造"标准不一致"，而系统性清理不在本批范围。
**若仓库后续有统一清理批次，artifacts 应一并纳入。**

## 三、修复 2.1：把校验收进单一入口

**冻结做法**：抽出 `validate_archive_index`，**encode 与 decode 都必须调用它**。

```rust
fn validate_archive_index(index: &ArchiveIndex) -> Result<(), ArtifactError> {
    if index.entry.is_empty() {
        return Err(ArtifactError::Index("归档索引缺少入口".to_owned()));
    }
    for entry in &index.entries {
        validate_archive_entry(entry)?;
    }
    Ok(())
}
```

- `encode` 用**一次** `validate_archive_index(self)?` 取代当前的内联入口检查
  （`xiao-artifacts/src/lib.rs:826`）与逐条 `validate_archive_entry`
  （`xiao-artifacts/src/lib.rs:834`）；
- `decode` 在现有三项检查之后调用 `validate_archive_index(&index)?`；
- **错误消息沿用 `"归档索引缺少入口"`**，不要另起一句——文案也是单一来源的一部分。

**必须保留的差异**：`decode_*` 里的 `has_kind` / `has_digest` / `has_length`
（`xiao-artifacts/src/lib.rs:1052`、`:1118`）是**存在性**检查，能区分"字段缺失"与
"字段显式为 0"，`validate_*` 做不到这一点。**存在性检查留在 decode，取值约束收进 validate**——
两者语义不同，不要为了"消除重复"把存在性检查也删掉。

`decode_archive_entry`（`xiao-artifacts/src/lib.rs:1032-1061`）里重复的三项非空检查
可以删掉，改由 `validate_archive_entry` 统一负责。

### 回归测试（必须补）

用审核实测过的字节直接断言，**缺字段与空字段两种形态都要覆盖**：

```rust
// 缺 entry 字段（字段 1、2、5、6）
let missing_entry = [0x08, 0x01, 0x10, 0x00, 0x28, 0x01, 0x30, 0x00];

// 显式空 entry（字段 3，长度 0）
let empty_entry = [0x08, 0x01, 0x10, 0x00, 0x1a, 0x00, 0x28, 0x01, 0x30, 0x00];
```

两条都必须 `decode` 失败。另补一条**非空入口正常通过**的对照（`0x1a, 0x03, b'm', b'a', b'i'`
替换字段 3），防止把校验写成一律拒绝。

## 四、修复 2.2：让 `.xiaoc` 契约只有一个来源

**冻结做法**：**删掉 artifacts 侧的手写预检，只保留 `validate_xiaoc`**。

- `put_xiaoc`（`xiao-artifacts/src/lib.rs:386`）删去整个
  `if bytes.len() < 72 || bytes.get(..8) != Some(XIAOC_MAGIC)` 分支；
- `read_checked`（`xiao-artifacts/src/lib.rs:432-436`）删去 `bytes.len() < 72`
  与 `bytes.get(..8) != Some(XIAOC_MAGIC)` 两个条件，只留下
  `kind == ObjectKind::Xiaoc && xiao_bytecode::validate_xiaoc(&bytes).is_err()`；
- 删除 `xiao-artifacts/src/lib.rs:19` 的 `pub const XIAOC_MAGIC` 定义。

**为什么删而不是改引用**：`validate_xiaoc` 已经完整覆盖长度与魔数校验，
保留一份"快速路径"预检只会再造一个漂移点。**审核已确认**：
`xiao_artifacts::XIAOC_MAGIC` **没有任何外部使用者**，删除不破坏调用方。

**若确实要保留快速失败路径**（例如为了避免大文件白读一遍），则**必须**引用
`xiao_bytecode::XIAOC_HEADER_MIN_SIZE` 与 `xiao_bytecode::XIAOC_MAGIC`，
**且不得出现任何字面量 `72` 或重复的魔数字节串**。

**注意副作用**：删掉预检后，原消息 `"不是完整规范 \`.xiaoc\` 文件"`
（`xiao-artifacts/src/lib.rs:389`）不再触发，错误改由 `validate_xiaoc` 给出更具体的原因。
**审核已确认没有测试断言该消息**，但需复核 `put_xiaoc` 的既有测试仍然反映新行为。

### 回归测试（必须补）

补一条**架构约束测试**：断言 `xiao-artifacts/src/lib.rs` 源码中不再出现
`0x1a`-独立的魔数字面量与裸 `72`。具体形式由实施者选择（读源码字符串断言，
或 `include_str!` 后检索），**目的是让"再次硬编码契约常量"在 CI 上失败**。

## 五、不负责与不要重复做的事

- **不改** 2.3 的锁机制——**归 16B**，且需先就"OS 级锁"方向达成一致再动手。
- **不为** 2.4 单独补文档——见 §二 2.4 的理由。
- **不改**索引格式、字段号、wire 编码——16A 已交付部分**保持字节兼容**；
  本批只改**校验时机**，不产出不同的合法字节。
- **不改** `xiao-bytecode` 的权威常量定义——只让 artifacts 去引用它。
- **不重开** 16A 文档 §三 的 7 条冻结契约与 §八 的 12 条翻车点。
- **不顺手**清理 `A0-COVERAGE-002`、不重构 `ProtoReader`、不动五类对象命名空间。

## 六、验收

1. **缺入口被拒**：§三 的两组字节 `decode` 均失败，非空入口对照仍通过；
2. **对称性可证**：`validate_archive_index` 被 `encode` 与 `decode` **共同调用**，
   不再各自内联；
3. **无字面量**：`xiao-artifacts/src/lib.rs` 中不含裸 `72` 与重复魔数串，
   架构测试守住这条；
4. **无外部破坏**：`xiao_artifacts::XIAOC_MAGIC` 的移除不引起任何 crate 编译失败；
5. **不回归**：`cargo test --workspace` 全绿；`xiao-artifacts` 既有 7 条测试仍全过；
   `cargo clippy --workspace --all-targets -- -D warnings` 绿；
6. **门禁全绿**：`bun run check` 退出码为 0（含 `check:lock`）；
7. **登记**：更新 [开发文档主表](README.md) 的 16A 行与本文件状态。

## 相关页面

- [16A. 内容寻址对象与二进制索引](16a-content-addressed-objects-and-indexes.md) —— **本批的修改对象**；§三 冻结契约、§八 12 条翻车点
- [16. SHA-256 内容寻址与二进制索引](16-content-addressed-artifacts.md) —— 权威规范；§3.5 索引职责、验收第 3 条
- [14A. `.xiaoc` 格式与编解码](14a-xiaoc-format-and-codec.md) —— `XIAOC_MAGIC` 与 `XIAOC_HEADER_MIN_SIZE` 的权威来源
- [11A-CONC. 并发模型边界](11a-concurrency-model.md) —— 索引锁的并发契约；2.3 的 16B 方案需与之一致
- [10K. Runtime 裁剪验证的跨平台收口](10k-n0d-runtime-trimming-verification.md) —— "看不见 ≠ 没问题"的同源判据
