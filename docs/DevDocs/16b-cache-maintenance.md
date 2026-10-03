# 16B. 缓存维护

> **16 阶段的第二批。** [16A](16a-content-addressed-objects-and-indexes.md) 交付了对象与索引核心，
> [16A-FIX](16a-fix-content-addressed-boundaries.md) 修掉了边界的编解码对称与单一来源问题。
> 本批做[规范](16-content-addressed-artifacts.md) §「缓存维护」的 **16.9–16.12**。
>
> **一句话概括本批**：**先把锁收成一套**（16A 审核发现仓库里有两套跨进程锁，
> 而既有那套更完善却没人复用），**再让缓存能被安全地清理**——清理的前提是
> 知道「谁还在用」，所以引用集合与清理策略是同一件事的两面。

## 一、Agent 交接上下文

### 接手前提

按顺序读：

1. [16A 交接文档](16a-content-addressed-objects-and-indexes.md) §三（7 条冻结契约）、§八（12 条翻车点）、§十（批次边界）；
2. [16A-FIX](16a-fix-content-addressed-boundaries.md) §二 2.3（锁残留的原始判定）与 §五（本批范围）；
3. [16 规范](16-content-addressed-artifacts.md) `:100-105`（缓存维护四条）与 `:107-118`（验收）；
4. [11A-CONC](11a-concurrency-model.md) §一（**工具链内部并发已冻结**，§1.2 定下「零新依赖」）；
5. [11A-E1](11ae1-local-dependencies-and-cache.md) §三 3.1（`~/.xiao/` 与 `XIAO_HOME` 已冻结）。

### 现状盘点（本批实现前）

| 环节 | 现状 | 判定 |
| --- | --- | --- |
| 跨进程锁（**既有**） | `xiao-package/src/entry_lock.rs` 的 `EntryLock`：owner 信息、30 秒超时、陈旧锁回收、Drop 校验 owner | **完善，但 `pub(crate)` 且被 6 处使用** |
| 跨进程锁（**16A 新造**） | `xiao-artifacts/src/lib.rs` 的 `IndexLock`：`create_new` + Drop 删除，**无超时、无陈旧回收** | **重复且更弱**——本批要消除 |
| 索引读写 | `IndexStore` 的 `write_global`/`read_global`/`write_archive`/`read_archive` 各自加锁 | 迁移到共用锁 |
| 对象扫描 | `ArtifactStore::scan` 只读已验证对象；`rebuild_global` 可由扫描重建 | **本批的引用扫描可复用** |
| 归档索引 | `ArchiveIndex` 含 `entry` 与 `entries`，条目有 `logical_path`/`digest`/`module`/`target`/`length` | **引用来源之一** |
| 依赖锁文件 | `xiao.lock.json`（11A-E2A 交付） | **引用来源之二** |
| 项目配置 | `config.xiao`（05D 静态闭环） | 项目身份来源 |
| 淘汰策略 | **不存在** | 本批建立（**仅显式**） |
| 陈旧锁回收测试 | `EntryLock` 有实现，**但多进程与终止恢复未系统测过** | 本批补齐（16.12） |

**⚠️ 一句话说明第一件事为什么是锁**：16A-FIX §二 2.3 把「锁残留」判为可用性缺陷并转给本批；
调研后发现**问题比当时描述的更值得处理**——仓库里**已经有一套完善的跨进程锁**，
16A 却另造了一份更弱的。所以本批不做「给 IndexLock 补超时」这种止血，
而是**让它消失**。

### 本批边界

| 条目 | 内容 | 本批处理 |
| --- | --- | --- |
| `16.9` | 记录项目、锁文件和 `.xar` 对对象的引用 | **做**（扫描现算，见 §2.5） |
| `16.10` | 显式清理、可选 LRU/引用追踪、损坏对象隔离 | **做显式清理**；**不做 LRU/自动淘汰**（见 §2.3） |
| `16.11` | 离线读取与跨项目复用，不绕过验证 | **做**（复用 16A 的 `read_checked`） |
| `16.12` | 多进程、终止恢复、只读权限、跨平台路径 | **做**（含本批的锁迁移验证） |
| — | 统一两套跨进程锁 | **做**（决策见 §2.1） |
| — | 架构测试精度（16A-FIX §六 第 3 条） | **做**（见 §2.9） |

## 二、必须先冻结的 9 条

### 2.1 **锁只有一个来源：新建 `xiao-lock`**

**已决策**（2026-10-03）：抽独立 crate `xiao-lock`，`EntryLock` 迁入，
`xiao-package` 与 `xiao-artifacts` 都依赖它。

**冻结**：

- `xiao-lock` 是**全仓唯一**的跨进程锁实现；`IndexLock` **删除**，`entry_lock.rs` **删除**；
- **不得**保留第二份实现，也**不得**让某一个 crate 通过复制粘贴"对齐语义"；
- 新 crate 登记进 `core/rust/Cargo.toml` 的 `members`（当前是显式列表，无 `[workspace.dependencies]`）。

**为什么不是「下沉到 xiao-platform」**：`xiao-platform` 目前**零依赖**，
而锁需要序列化 owner 信息；下沉会逼它引入 `serde_json` 或改写成手写格式，
**后者会让 owner 格式与既有锁文件不兼容**。平台相关的只是 `process_alive` 那一小块，
不值得为它扭曲 crate 定位。

**为什么不是「artifacts 依赖 package」**：`xiao-package` 依赖 `xiao-codegen-llvm` 等一长串，
artifacts 反向依赖它是**架构倒挂**（底层存储依赖上层包管理）。

### 2.2 **迁移对外行为不变：错误码与文案都不许变**

`EntryLock::acquire` 返回 `SourceError`（`xiao-package` 的类型），
而 `sync.rs:182` 直接用 `error.code`：

```rust
EntryLock::acquire(&lock_path).map_err(|error| failure(error.code, "无法获取项目包操作锁"))
```

**冻结**：

- `xiao-lock` 的错误类型**自带路径与原因**，**不携带任何 crate 专有的错误码**；
- `xiao-package` 在边界处转换，**必须**继续产出 `SOURCE_CACHE_IO_CODE`（`X05-SOURCE-008`，
  `xiao-package/src/diagnostics.rs:67`）与**既有文案格式** `"缓存锁 {path}：{error}"`；
- `xiao-artifacts` 同样在边界处包装成自己的 `ArtifactError`；
- **判定标准**：迁移前后 `xiao-package` 的**既有测试一条都不许改**——
  它们是「对外行为不变」的守门人。若某条测试必须改，说明对外行为变了，**停下来说明**。

**迁移影响面（6 处调用点，已核）**：

| 文件 | 行 | 处理 |
| --- | --- | --- |
| `federation_cache.rs` | `:42`、`:189` | 换类型，保持 `?` |
| `fetch.rs` | `:120` | 换类型，保持现有 `map_err` 行为 |
| `snapshot_store.rs` | `:70` | 换类型，保持 `?` |
| `source_lists.rs` | `:45` | 换类型，保持现有 `map_err` 行为 |
| `sync.rs` | `:182` | **注意**：需继续产出相同的 `code` 与文案 |

### 2.3 **淘汰只做显式清理，不做自动淘汰**

**已决策**（2026-10-03）：只提供显式清理 API，**不做** LRU、**不做**后台/自动淘汰。

**冻结**：

- 规范把 LRU 写成「**可选**」、淘汰算法「**待确认**」——本批**不冻结它**，
  也**不得**以"顺手实现"的方式把它冻下来；
- 不引入任何**按访问时间**的回收逻辑。注意规范 `:70` 的另一条约束：
  **不把可变的本机访问时间写进不可变归档索引**——自动淘汰会逼着记录访问时间，
  与本批的克制方向相反；
- **但「损坏对象隔离」要做**（16.10 明写）：损坏对象进隔离区，不参与命中。

### 2.4 **清理必须两阶段：先出计划，再执行**

**冻结**：

- 提供 `plan_*`（**只计算、不修改任何字节**）与 `apply_*`（才真正删除）两步；
- **不得**提供"一步删除"的 API——清理是不可逆操作，必须让人先看到将删什么；
- 这与 13A 的「快照回滚 / 验证阻断」、15D 的「不可读时拒绝而非猜测」是**同一种克制**。

### 2.5 **引用集合现算，不持久化**

**已决策**（2026-10-03）：清理时扫描现算，**不建立持久化的引用记录**。

**冻结**：

- **不新增**引用记录文件、**不扩展**全局索引记录来存引用；
- 引用来源**由调用方显式给出**：项目目录（读 `xiao.lock.json`）、`.xar` 归档（读 `ArchiveIndex`）、
  以及可选的显式对象引用列表；
- **依据**：16A-FIX §五 已冻结「不重开 16A 的索引格式与 wire 编码」，
  而 16A 的索引格式刚冻结就连着改了两次（16A → 16A-FIX）。

**必须写明的代价**：现算意味着**离线或未挂载的项目扫不到**。
所以 §2.6 的保守规则不是可选项，是这条决策的**必要配套**。

### 2.6 **保护集保守：判不准就保留**

**冻结**：

- **任何一个引用来源读取失败**（文件缺失、格式错误、权限不足）时，
  **不得**按"它没有引用"处理——必须**整体失败**，或**把该来源能声明的对象全部保留**；
- 默认方向永远是**保留**而非删除。误删是**不可逆的数据损失**，误留只是**占空间**；
- 这与 10K 的「**看不见 ≠ 没问题**」、15D 的「不可读时拒绝或标不可用」是**同一条判据**。

### 2.7 **不重开** 16A 的索引格式与 wire 编码

- 冻结索引的字段号、wire 编码、确定性排序规则；
- 本批**只做引用扫描与清理**，不产出不同的合法索引字节；
- `ArchiveIndex` 的读取**沿用** 16A-FIX 的 `validate_archive_index` 路径。

### 2.8 `xiao-lock` **零新依赖**

[11A-CONC](11a-concurrency-model.md) §1.2 已冻结「零新依赖」。
`EntryLock` 的既有实现**已经做到了**——Unix 用 `unsafe extern "C" { fn kill(..) }`、
Windows 用 `#[link(name = "kernel32")]` 直接声明，**没有引入 `libc`**。

**冻结**：

- 迁移时**保持**这个做法，**不得**为了方便引入 `fs2`/`fs4`/`fd-lock` 之类；
- **不得**顺手升 MSRV 去用 `std::fs::File::lock`（该 API 稳定版高于本仓 `rust-version = "1.85"`）；
- 既有常量语义**照搬不改**：`LOCK_WAIT = 30s`、`INCOMPLETE_LOCK_GRACE = 60s`。

### 2.9 **架构测试改为行为断言**

16A-FIX §六 第 3 条留下的改进项：`contains("72")` 过宽（将来一处合理的 `72` 会误报，
而误报会诱导改断言而非改代码），`split("#[cfg(test)]")` 的切片位置也脆弱。

**冻结**：

- 改为**行为断言**：传入长度不足的 `.xiaoc`，断言错误**来自权威校验**
  （16A-FIX 实测为 `InvalidError` 且带 `XIAOC-001` 之类的错误码），
  **而不是**旧预检的文案「不是完整规范 `.xiaoc` 文件」；
- 它守的是「预检没有回来」这个**真实语义**，不依赖文本、不会误报；
- 字符串测试**可保留作辅助**，但匹配必须收紧到词边界。

## 三、实现基础

| 环节 | 复用点 |
| --- | --- |
| 跨进程锁 | `xiao-package/src/entry_lock.rs` 全文迁入 `xiao-lock`，**逻辑不改** |
| 对象扫描 | `ArtifactStore::scan`（只读已验证对象） |
| 读取前验证 | `ArtifactStore::read_checked`（重算摘要 + 长度 + 格式） |
| 归档索引 | `ArchiveIndex` 的 `entries` 提供对象引用 |
| 依赖锁文件 | `xiao.lock.json`（11A-E2A）提供项目对对象的引用 |
| 解析配置 | `config.xiao` + `XIAO_HOME`（11A-E1 §3.1 已冻结） |
| 隔离区 | `ArtifactStore::quarantine_path` / `quarantine_collision` |

## 四、落点

```text
core/rust/crates/xiao-lock/         新增：唯一跨进程锁实现（EntryLock 迁入）
  src/lib.rs                        锁协议、超时、陈旧回收、process_alive
core/rust/crates/xiao-package/      删 entry_lock.rs；6 处调用点改为依赖 xiao-lock
core/rust/crates/xiao-artifacts/    删 IndexLock；索引读写改用 xiao-lock；
                                    新增引用扫描、两阶段清理、损坏隔离
core/rust/Cargo.toml                members 增加 crates/xiao-lock
```

**⚠️ `read_archive` / `read_global` 的只读副作用**：现有实现在**读取**时也要建锁，
即**只读场景需要写权限**。本批须一并裁定：读路径**不应**要求写权限
（读是幂等的，且 16A 的 `read_checked` 已用重算摘要兜住正确性）。
处理方式由实施者在 §六 第 3 步中给出并测试。

## 五、硬约束

1. **锁文件里不得出现凭据、令牌或绝对路径**——owner 信息只含 pid、时间戳、nonce；
2. **锁文件权限**与 `~/.xiao/credentials` 的 0600 要求**不冲突**，但也不得放宽既有文件的权限；
3. **`xiao-lock` 不得依赖任何 `xiao-*` crate**——它是叶子，否则会形成环；
4. **`process_alive` 返回 `None`（无法判定）时不得回收**——保守语义必须保留；
5. **陈旧锁回收前必须二次比对**（现有实现 `:102` 的做法），防回收竞态；
6. **Drop 时只删自己写的锁**（现有实现 `:79` 比对 owner 字节）——不得退化为无条件删除。

## 六、分步提交

1. **建 crate 并纯迁移**：`xiao-lock` 落地，`EntryLock` 逻辑**逐字节搬**（除错误类型外不改），
   `xiao-package` 改依赖，**既有测试全绿且一条不改**。**此步不碰 artifacts。**
2. **artifacts 换锁**：删 `IndexLock`，索引读写改用 `xiao-lock`；裁定并实现
   §四 的只读副作用问题。
3. **引用扫描**：实现从项目锁文件与 `.xar` 归档求引用集，含 §2.6 的保守规则。
4. **两阶段清理**：`plan` / `apply`，含损坏对象隔离。
5. **16.11 离线读取**：确认离线路径不绕过验证（复用 `read_checked`）。
6. **16.12 测试**：多进程、终止恢复、只读权限、跨平台路径。
7. **§2.9 架构测试改造** + 文档与主表登记。

## 七、最可能翻车的地方

1. **迁移时改了对外错误码或文案**（§2.2）——`sync.rs:182` 直接读 `error.code`，最容易漏。
2. **把 `pub(crate)` 改成 `pub` 时漏了导出**，或忘了 `xiao-lock` 的 `missing_docs` 接线。
3. **顺手实现了 LRU**（§2.3）——"既然都扫描了，不如记个时间戳"，这一步就把待定决策冻了。
4. **引用来源读取失败按"无引用"处理**（§2.6）——最危险的一条，直接导致误删。
5. **只读场景仍要求写权限**（§四）——`read_archive` 的老问题。
6. **清理真的删了正在用的对象**——见 4。
7. **为省事引入 `fs2`/`fs4`**（§2.8），破坏 11A-CONC 已冻结的零新依赖。
8. **`xiao-lock` 反向依赖 `xiao-*`**（§五 第 3 条）——形成环。
9. **`process_alive` 返回 `None` 时当成"已死"回收**（§五 第 4 条）——会误删活跃锁。
10. **忘了 `core/rust/Cargo.lock` 的同步**——`check:lock` 会红（16A 栽过一次）。
11. **改了 16A 的索引 wire 编码**（§2.7）——违反 16A-FIX §五。
12. **架构测试仍是字符串匹配**（§2.9）。

## 八、验收

1. **锁单一来源**：全仓只有一份跨进程锁实现；
   `grep -rn "create_new(true)" core/rust/crates/*/src` 不再出现第二处锁创建；
2. **对外行为不变**：`xiao-package` 既有测试**一条未改**且全绿；`sync.rs` 的
   错误码仍为 `X05-SOURCE-008`、文案仍为 `缓存锁 {path}：{error}`；
3. **零新依赖**：`xiao-lock` 的 `[dependencies]` 为空（或仅内部 crate 之外的既有项）；
   MSRV 仍为 `1.85`；
4. **陈旧回收有效**：真起子进程 kill 后，下一个进程能在超时内取得锁（16.12）；
5. **两阶段**：`plan` 不产生任何文件系统修改（用前后目录快照断言）；
6. **保守性**：引用来源损坏或缺失时，`plan` **不把**该来源的对象列入删除集；
7. **只读可用**：只读目录下读取索引与对象**不因无法建锁而失败**；
8. **不重开格式**：16A/16A-FIX 的索引与对象测试全绿且未改；
9. **架构测试**：改为行为断言（§2.9）；
10. **门禁全绿**：`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、
    `bun run check` 退出码 0（含 `check:lock`，两个 lock 文件都已提交）。

## 九、不负责与不要重复做的事

- **不做 LRU 与自动淘汰**（§2.3）——待定决策，不得擅自冻结；
- **不做引用记录的持久化**（§2.5）——用户已决策现算；
- **不做 CLI 命令**（`prune` 的子命令归 `18`）——本批只出库层 API；
- **不做 `.xar` 归档写入**——归 `17`（`xiao-xar` crate 已存在但本批不接）；
- **不做数字签名/信任链**——规范待定决策 4；
- **不做原生二进制与 `.xar` 自身的内容寻址命名**——规范待定决策 3；
- **不重开** 16A 的索引格式、16A-FIX 的 `validate_archive_index` 与契约常量；
- **不改** 11A-E1 的 `~/.xiao/` 与 `XIAO_HOME` 决策。

## 本批实现记录

`xiao-lock` 已成为全仓唯一的跨进程缓存锁，迁移了原 `EntryLock` 的 owner、超时、
陈旧回收和二次比对语义；`xiao-package` 的错误码与既有文案保持不变，`xiao-artifacts`
的 `IndexLock` 与旧 `entry_lock.rs` 已删除。索引读取不再创建锁文件，写入仍使用统一锁，
因此只读缓存目录可以离线读取。

`ArtifactStore::collect_references` 从项目锁文件、归档索引和显式引用现算保护集合；任何
来源读取失败都返回错误，不把它当作无引用。`plan_cleanup` 只读扫描并生成删除计划，
`apply_cleanup` 接收执行前重新扫描的引用集合，重新验证后才删除；期间发现损坏对象会沿用
16A 隔离路径。对象写入与清理执行共用维护锁，避免发布对象和删除对象交错。实现只提供
显式清理，不记录访问时间、不做 LRU，也不接入 CLI。索引读取不创建锁文件，持有写锁时仍
可读取已原子提交的快照；测试覆盖子进程终止后的陈旧锁回收、归档引用保护和新引用保护。

## 相关页面

- [16. SHA-256 内容寻址与二进制索引](16-content-addressed-artifacts.md) —— **权威规范**；§「缓存维护」`16.9`–`16.12`、`:21` 的锁可清理要求、`:70` 的不写访问时间
- [16A. 内容寻址对象与二进制索引](16a-content-addressed-objects-and-indexes.md) —— 对象与索引核心；§十 批次边界
- [16A-FIX. 内容寻址边界的编解码对称与单一来源修复](16a-fix-content-addressed-boundaries.md) —— §二 2.3 锁残留的来源、§六 第 3 条架构测试
- [11A-E3B. 快速解析与远程缓存](11ae3b-fast-resolution-and-cache.md) —— **`EntryLock` 的原始交付批次**
- [11A-CONC. 并发模型边界](11a-concurrency-model.md) —— §1.2 零新依赖（本批 §2.8 的依据）
- [11A-E1. 本地依赖与共享缓存](11ae1-local-dependencies-and-cache.md) —— `~/.xiao/`、`XIAO_HOME` 与损坏条目隔离
- [13A. 优化契约与统一管线的实施](13a-optimization-boundary-implementation.md) —— 快照回滚与验证阻断（§2.4 两阶段清理的同源做法）
