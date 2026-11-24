# 20O-B3-0D. 版本、intrinsic、ABI、C ABI 与 Windows COFF 冻结规划

> **状态（2026-11-24）**　B3-0D 已完成冻结。B3-0C 已收口，六项 B3-0D 裁定均已批准；本批只冻结编号、版本、能力位、协议、C ABI 和 Windows COFF 政策，不写产品。`b3_0d_started=true`、`b3_0d_passed=true`、`b3_0e_entry_ready=true`、`b3_0_passed=false`、`b3_product_started=false`。
> **Windows 产物政策（2026-11-22）**　自 B3 起，所有 Windows 公共/发布产物必须产品级 COFF `Complete`，声明组件=观察组件；B2-C 的同链接 10 份候选只是历史例外，不得继承。B3-0D 只冻结验收方式，不把探测记录写成 B3 产品验收。
> 入口：[B3-0C 收口](assets/20o-b3/entry-20261123-b3-0c/README.md)、[B3-0C 契约](assets/20o-b3/entry-20261123-b3-0c/contract-draft.md)、[B3-0B 契约](assets/20o-b3/entry-20261123-b3-0b/contract-draft.md)、[B3-0 实施规划](20o-b3-0-probe-freeze-plan.md)、[B3 宿主库规划](20o-b3-host-libraries-plan.md)、[20D 能力上下文](20d-capability-context.md)。
> B3-0D 不修改当前产品的 ABI 1.19、生成器 22、接口 1、intrinsic 1～48/ID 100；新增 49～63、ABI 1.20/生成器 23 和协议/C ABI 只进入 B3-0E 交接，产品实现仍未开始。

## 一、入口与范围

| 项 | B3-0D 接收状态 |
| --- | --- |
| B3-0C | `b3_0c_passed=true`、`b3_0d_entry_ready=true`；六模块精确契约、三份教程候选和九项实际答复已封存 |
| 基线 | 固定产品 `e79fab3c`、ABI 1.19、生成器 22、标准库接口 1；两个 Cargo.lock 和 B1 28 守卫可复核 |
| 本批目标 | 冻结 15 个新原语编号、`environment` 能力位、ABI 1.20/生成器 23、C ABI、运行协议和 Windows COFF 验收方式 |
| 本批不做 | 产品实现、B3-0E/B3-1、registry/教程 verified、B4-B9、旧 721、历史债务核销 |
| 版本冻结 | ABI 1.20、生成器 23、标准库接口 1；只作为 B3-0E 实现输入，当前产品仍为 ABI 1.19/生成器 22 |
| COFF | 政策 `complete_required`，实际产品状态保持 `pending` |
| 当前状态 | `b3_0d_started=true`、`b3_0d_passed=true`、`b3_0e_entry_ready=true`、`b3_0_passed=false`、`b3_1_entry_ready=false` |

B3-0D 是 **版本与兼容冻结批**，不是实现批。B3-0C 已冻结“每个模块做什么”；B3-0D 冻结“这些入口以什么编号、版本、能力、ABI 和产物验收身份进入产品”。冻结完成只表示 B3-1 可以按表实现，不表示模块已实现或已验收。

### 1.1 要做

1. 把 B3-0B 的六个原语与 B3-0C 的九个原语合并为 15 个稳定编号，登记名称、效果、能力、所有权和 C 符号；
2. 冻结 `environment` 能力位、已知位掩码和旧请求默认掩码；保证位 0～6 的既有含义不变；
3. 冻结 ABI 1.20、生成器 23、标准库接口 1 的联合增量；确认 `.xiaoc`、`.xar`、索引、IR 和原生元数据是否升格式版本；
4. 冻结旧 1.19/22 产物的拒绝/重生成规则、缓存身份和跨平台产物矩阵；
5. 冻结 15 个 C ABI 符号、签名、输出槽、状态码和所有权；登记 VM/LLVM 绑定；
6. 冻结运行协议的 `arguments` 字段、hello 能力协商、原生注入和协议失配拒绝；
7. 冻结 Windows 产品级 COFF `Complete` 的验收判据、验证器落点、证据路径和负例；
8. 产出 B3-0D 证据、冻结表和 B3-0E 入口；保持教程/registry `planned`。

### 1.2 明确不做

- 不在 B3-0D 写 15 个原语的 Runtime、VM、LLVM、驱动或标准库产品代码；
- 不修改 ABI 1.19、生成器 22、接口 1、intrinsic 1～48/ID 100 或既有能力位含义；
- 不把 `environment` 位加进旧请求默认掩码而不经裁定；
- 不启动 B3-0E/B3-1/B4，不跑旧 721，不关闭 22C/22D 或历史债务；
- 不用 B2-C 同链接补验、`UnverifiedCoffExports` 或导出表抽查替代 B3 Windows 产品级 COFF `Complete`；
- 不在 B3-0D 把“验证器可行性探测”写成“产品验收完成”。

### 1.3 结论字段

```text
b3_0d_passed =
  b3_0d0_decisions_recorded
  and b3_0d1_intrinsics_capabilities_frozen
  and b3_0d2_versions_artifacts_frozen
  and b3_0d3_c_abi_protocol_frozen
  and b3_0d4_windows_coff_policy_frozen
  and b3_0d5_evidence_audit_passed

b3_0e_entry_ready = b3_0d_passed and decisions_approved and no_product_change
```

`b3_0d_passed=true` 只表示编号、版本、能力、ABI、协议和 COFF 政策冻结完成，不表示 15 个原语已实现，也不表示 Windows 产品 COFF 已验收。`windows_product_coff_status=pending` 必须在 B3-1/B3-7 产品验收前保持。

## 二、开工闸门

1. B3-0C 收口审计、自反例和 8 条封存负例通过；`b3_0c_passed=true`、`b3_0d_entry_ready=true`；
2. 固定产品 `e79fab3c`、两个 Cargo.lock、三个平台目标/工具链和 B1 28 守卫可复核；B3-0D 不改产品；
3. B3-0B/C 的 15 个原语名称、签名、效果、能力和资源表可读；
4. 当前常量核对为 ABI 1.19、生成器 22、接口 1；`.xiaoc` 1.0、`.xar` 1.0、索引 1.0、IR 1、原生元数据 1；
5. 20D 已冻结位 0～6 和“未知位拒绝”可复核；`clock` 已有位 1，`stdout` 为位 5，`process` 为位 2；
6. 当前 `verify_artifact` 对不可读 COFF 符号表返回 `UnverifiedCoffExports`，B3 需要把产品级 `Complete` 的判据单列；
7. 三平台探测环境可用；缺平台只能记 `unverifiable`，不能写成通过；
8. 唯一任务根、独立 target/Runtime/缓存、每项裁定/平台/产物独占输出目录；
9. Windows 产物政策已固定：B3 起产品级 COFF `Complete`，同链接/导出表抽查不得替代。

## 三、已批准裁定

| 编号 | 冻结内容 | 来源 |
| --- | --- | --- |
| B3-0D-ID | 15 个原语顺序编号 49～63：time 49～52、args 53～54、fs 55～59、env 60～62、log 63；已发布 ID 不复用，ID 100 保持墓碑；C 符号按 `xiao_runtime_<module>_<op>` | 星崽批准，问题卡 `call_0280c5f4ad294a44b0e41fb136193307` 第 0 项 |
| B3-0D-ENV | 新增 `environment` 位 7；`KNOWN_CAPABILITY_MASK=0xFF`；旧请求省略能力时默认 `0x7F`，不静默获得环境访问；授权 CLI/直接原生入口在应用开始前捕获一次宿主环境，嵌入省略时为空快照，运行内写删共享 | 星崽批准，问题卡 `call_0280c5f4ad294a44b0e41fb136193307` 第 1 项 |
| B3-0D-VER | ABI 1.20、生成器 23、接口 1；`.xiaoc` 1.0、`.xar` 1.0、索引 1.0、IR 1、原生元数据 1；旧 1.19/22 产物执行前拒绝并由源码重生成，缓存按 ABI/生成器/Runtime/目标/能力/标准库身份失配重建 | 星崽批准，问题卡 `call_0280c5f4ad294a44b0e41fb136193307` 第 2 项 |
| B3-0D-CABI | 15 个符号采用借读 `const XiaoValue*`、结果入口独立初始化的 `out` 槽、无结果入口省略 `out`，末尾 `capabilities: u64`；成功 0，结构错误 1/2/3，运行错误 6，Fatal 8；成功替换 `out`，失败保留 `out`，不提供隐式全开旧包装 | 星崽批准，问题卡 `call_0280c5f4ad294a44b0e41fb136193307` 第 3 项 |
| B3-0D-PROTO | `RunOptions.arguments` 为 `{program_name: str, argv: [str]}`，新增 hello 能力 `"arguments"`；省略为已确认的空快照，快照只参与执行，不进入产物或缓存身份；旧核心未声明能力时执行前拒绝，原生启动器注入已校验快照 | 星崽批准，问题卡 `call_0280c5f4ad294a44b0e41fb136193307` 第 4 项 |
| B3-0D-COFF | 产品级 `Complete` 才通过，声明组件=观察组件且无未分类 Runtime 符号；验证器读取保留的真实 COFF 符号表或与产物哈希绑定的受信 linker map/PDB 完整清单；导出表-only、同链接候选、未绑定 sidecar 和 `UnverifiedCoffExports` 均拒绝，当前产品状态仍为 `pending` | 星崽批准，问题卡 `call_0280c5f4ad294a44b0e41fb136193307` 第 5 项 |

六项答复均已登记来源、时点和适用范围。冻结结果只打开 B3-0E 入口，不启动产品代码、registry、教程 verified 或 Windows 产品 COFF 验收。

## 四、分批执行

### B3-0D0：裁定登记与冻结表骨架

1. 建立 B3-0D 决定台账，逐条记录星崽答复、来源、时点和适用范围；
2. 建立冻结表骨架：intrinsic、能力位、版本/格式、C ABI、协议、COFF、平台和教程状态；
3. 固定 B3-0C 的六模块契约、15 个原语名称、九项实际答复和 B3-0D 待定项；
4. 固定当前常量：ABI 1.19、生成器 22、接口 1、`.xiaoc` 1.0、`.xar` 1.0、索引 1.0、IR 1、原生元数据 1；
5. 建立 `entry-20261124-b3-0d/` 证据目录和唯一任务根；
6. 未答问题保持“待裁定”，不得标为冻结。

退出：六项首轮问题有明确答复或逐项待答；冻结表骨架可复核。

### B3-0D1：intrinsic 编号、效果与能力位

1. 为 15 个原语分配稳定 `IntrinsicId`，按模块分组并与 C 符号一一对应；
2. 登记 arity、参数类型、返回类型、所有权、效果和错误身份；
3. 新增 `Capability::Clock`、`Capability::Environment`；`environment` 使用位 7，位 0～6 含义不变；
4. 把 intrinsic 契约从单能力扩展为能力掩码：`log.write` 同时声明 `clock` 与 `stdout`；
5. 把单效果扩展为效果掩码或等价的有序集合：取钟、等待、读参数、读/写环境、读/写文件系统和写 stdout 分别可观察；
6. 冻结 `KNOWN_CAPABILITY_MASK=0xFF` 与旧请求默认掩码；VM、Runtime、驱动和原生入口在触碰宿主前检查全部所需位；
7. 保留 intrinsic 1～48 和 ID 100 的既有语义，不做重编号。

退出：15 个 ID、能力位、默认掩码、效果和所有权的冻结表成立。

### B3-0D2：版本、格式、协议与缓存身份

1. 冻结 Runtime ABI 1.20、生成器 23、标准库接口 1 的联合增量；
2. 冻结 `.xiaoc` 1.0、`.xar` 1.0、索引 1.0、IR 1、原生元数据 1；只有编码字段实际变化才升格式次版本；
3. 冻结 `.xiaoc`/`.xar`/原生元数据写入的 ABI 范围、生成器版本、目标、能力要求和标准库身份；
4. 冻结旧 1.19/22 产物的拒绝/重生成规则、缓存按 ABI/生成器/Runtime 身份失配重建；
5. 冻结 `RunOptions.arguments` 的形状、限制、空快照、CLI/嵌入/原生来源和 hello 能力协商；
6. 冻结参数快照只参与执行、不进入静态产物/缓存身份的规则；
7. 保留 B2-C 1.19/22 历史证据，不把历史例外升级为 B3 默认。

退出：版本/格式/协议/缓存矩阵可复核，旧产物行为明确。

### B3-0D3：C ABI、VM 与 LLVM 绑定

1. 登记 15 个 C 符号、精确签名、参数顺序、输出槽和 `capabilities` 位置；
2. 冻结成功 0、结构输入失败 1/2/3、运行错误 6、Fatal 8；失败保留已初始化输出槽；
3. 冻结借读输入、拥有输出、数组/字典/字符串释放责任和无别名规则；
4. 在 `xiao-runtime-abi` 声明、`xiao-runtime` 实现、`xiao-codegen-llvm` 绑定和 VM 分派表中登记同一份契约；
5. 所有新入口使用带能力掩码的符号；不提供隐式全开的旧包装；
6. 冻结原生启动器注入程序名/argv/环境快照的入口；协议失配在执行用户代码前拒绝。

退出：C ABI/VM/LLVM 绑定表和符号矩阵成立，未写产品实现。

### B3-0D4：Windows 产品级 COFF 验收

1. 探测现有 Windows 产物为何返回 `UnverifiedCoffExports`，记录 COFF 符号表、导出表、PDB/linker map 的实际可读性；
2. 冻结产品级 `Complete` 判据：声明组件=观察组件、无未分类 Runtime 符号、依赖一致、目标/格式一致；
3. 冻结验证器落点：产品验证器读取保留的 COFF 符号表，或读取与产物哈希绑定的受信 linker map/PDB 清单；单独导出表、同链接映射或未绑定哈希的 sidecar 不接受；
4. 冻结证据身份：产物 SHA-256、源码/Runtime/工具/目标、三平台任务根、CI run/job/attempt；
5. 设计负例：`UnverifiedCoffExports`、导出表-only、同链接候选、声明缺失、观察组件多出、未分类符号、哈希/身份失配、旧 sidecar；
6. 只做可行性探测；`windows_product_coff_status` 保持 `pending`，B3-1/B3-7 才产生产品级 `Complete` 证据；
7. 若验证器机制无法读取冻结的任何允许证据源，或无法把证据身份绑定到待验产物，`b3_0d_passed=false`；产品产物的实际 `Complete` 仍由 B3-1/B3-7 产生。

退出：COFF 政策、验证器落点、证据路径和负例冻结；没有把探测写成产品验收。

### B3-0D5：证据、自反例与 B3-0E 入口

1. 生成 B3-0D 证据清单：决定台账、编号表、能力位/默认掩码表、版本/格式矩阵、C ABI 符号表、协议表、COFF 政策；
2. 只读审计核对固定 Git 对象、B3-0C 契约、当前常量、冻结表和用户裁定；
3. 自反例至少覆盖伪造版本、重复 ID、越界能力位、错误默认掩码、伪造 C 符号、旧产物误标为直接接受、导出表-only 冒充 `Complete`；
4. 生成 B3-0E 入口：15 个原语、六模块契约、版本/格式/缓存、C ABI、协议和 COFF 政策；
5. B3-0E/B3-1/B4 未开始；旧 721 和历史债务仍后置；
6. 定稿后单次 docs 门禁；最终 Markdown 由回执覆盖。

退出：B3-0D 证据完整；B3-0E 入口明确；没有产品/registry/教程 verified 变化。

## 五、冻结表候选

### 5.1 intrinsic 编号冻结表

| ID | 名称 | 模块 | 参数 → 返回 | 能力 | 效果 | C 符号 |
| --- | --- | --- | --- | --- | --- | --- |
| 49 | `os_time_utc_now` | time | `str unit` → `int` | `clock` | 读时钟 | `xiao_runtime_time_utc_now` |
| 50 | `os_time_monotonic` | time | `str unit` → `int` | `clock` | 读时钟 | `xiao_runtime_time_monotonic` |
| 51 | `os_time_format_utc` | time | `int unix_time, str unit` → `str` | `none` | 纯计算、分配 | `xiao_runtime_time_format_utc` |
| 52 | `os_time_sleep` | time | `int duration, str unit` → `none` | `clock` | 等待、可取消 | `xiao_runtime_time_sleep` |
| 53 | `os_args_program_name` | args | `()` → `str` | `process` | 读运行快照 | `xiao_runtime_args_program_name` |
| 54 | `os_args_argv` | args | `()` → `[str]` | `process` | 读运行快照、分配 | `xiao_runtime_args_argv` |
| 55 | `os_fs_write_text` | fs | `str path, str text, bool overwrite` → `none` | `filesystem` | 写文件系统 | `xiao_runtime_fs_write_text` |
| 56 | `os_fs_list_dir` | fs | `str path` → `[str]` | `filesystem` | 读文件系统、分配 | `xiao_runtime_fs_list_dir` |
| 57 | `os_fs_remove` | fs | `str path` → `none` | `filesystem` | 写文件系统 | `xiao_runtime_fs_remove` |
| 58 | `os_fs_rename` | fs | `str source, str destination, bool overwrite` → `none` | `filesystem` | 写文件系统 | `xiao_runtime_fs_rename` |
| 59 | `os_fs_metadata` | fs | `str path` → `dict<str, int>` | `filesystem` | 读文件系统、分配 | `xiao_runtime_fs_metadata` |
| 60 | `os_env_get` | env | `str name` → `[str]` | `environment` | 读环境、分配 | `xiao_runtime_env_get` |
| 61 | `os_env_set` | env | `str name, str value` → `none` | `environment` | 写环境 | `xiao_runtime_env_set` |
| 62 | `os_env_unset` | env | `str name` → `bool` | `environment` | 写环境 | `xiao_runtime_env_unset` |
| 63 | `os_log_write` | log | `str level, str message, str target` → `none` | `clock` + `stdout` | 读时钟、写 stdout | `xiao_runtime_log_write` |

编号 1～48 保持原义，100 保持墓碑。49～63 及上述 C 符号名已冻结为 B3-0E 的实现输入，B3-0D 不写入 `intrinsics.json`。

### 5.2 能力位候选

| 位 | 名称 | 当前/候选 | 说明 |
| --- | --- | --- | --- |
| 0 | `filesystem` | 已有 | 不变 |
| 1 | `clock` | 已有 | 不变；time 与 log 使用 |
| 2 | `process` | 已有 | 不变；args 使用 |
| 3 | `network` | 已有 | B3 不使用 |
| 4 | `native_extension` | 已有 | B3 不使用 |
| 5 | `stdout` | 已有 | 不变；log 使用 |
| 6 | `stdin` | 已有 | 不变 |
| 7 | `environment` | **已冻结** | env 的专用授权；位 8+ 仍拒绝 |

冻结默认值：

- `KNOWN_CAPABILITY_MASK = 0xFF`；
- 旧请求省略 `capabilities` 时默认 `0x7F`，不静默获得环境访问；
- 授权 CLI 与直接原生入口在应用开始前捕获一次宿主环境；嵌入请求默认空快照；
- 原生入口沿用同一掩码，缺位报 `X06-RUNTIME-026`。

旧请求默认 `0x7F` 已记录为兼容边界，不能静默获得环境访问。

### 5.3 版本与格式冻结表

| 轴 | 当前 | B3 候选 | 规则 |
| --- | --- | --- | --- |
| Runtime ABI | 1.19 | 1.20 | 新原语和能力掩码使用次版本增量 |
| 生成器 | 22 | 23 | 新 lowering/C ABI 调用改变生成文本 |
| 标准库接口 | 1 | 1 | 首批兼容扩展 |
| `.xiaoc` 格式 | 1.0 | 1.0 | 容器字段不变；ABI/生成器写入头部 |
| `.xar` 格式 | 1.0 | 1.0 | 容器字段不变；索引记录版本/能力 |
| 索引 Schema | 1.0 | 1.0 | 记录类型不变 |
| IR | 1 | 1 | 不新增值类型 |
| 原生元数据 | 1 | 1 | 只更新 ABI/生成器/能力字段 |
| 协议版本 | 1 | 1 | 新增能力 `"arguments"`，不先升协议 |
| Core 版本 | 1 | 1 | 旧核心缺能力时客户端拒绝 |

冻结产物规则：

- 新产物写入 ABI 1.20、生成器 23、精确目标、能力要求和标准库身份；
- 旧 1.19/22 `.xiaoc`/`.xar`/原生产物在 B3 公共路径执行前拒绝，回归从源码重生成；
- B2-C 的 1.19/22 产物只保留历史证据身份，不进入 B3 发布输入；
- 缓存按 ABI、生成器、Runtime 内容、目标、能力和标准库身份失配重建；
- 只有实际编码字段变化时才提升 `.xiaoc`/`.xar`/索引格式次版本。

### 5.4 C ABI 签名冻结表

统一形式为“借读输入 + 可选输出槽 + 末尾 `capabilities`”，`extern "C"`，返回 `i32`：

```text
xiao_runtime_time_utc_now(unit, out, capabilities)
xiao_runtime_time_monotonic(unit, out, capabilities)
xiao_runtime_time_format_utc(unix_time, unit, out, capabilities)
xiao_runtime_time_sleep(duration, unit, capabilities)
xiao_runtime_args_program_name(out, capabilities)
xiao_runtime_args_argv(out, capabilities)
xiao_runtime_fs_write_text(path, text, overwrite, capabilities)
xiao_runtime_fs_list_dir(path, out, capabilities)
xiao_runtime_fs_remove(path, capabilities)
xiao_runtime_fs_rename(source, destination, overwrite, capabilities)
xiao_runtime_fs_metadata(path, out, capabilities)
xiao_runtime_env_get(name, out, capabilities)
xiao_runtime_env_set(name, value, capabilities)
xiao_runtime_env_unset(name, out, capabilities)
xiao_runtime_log_write(level, message, target, capabilities)
```

其中 `out` 是已初始化的 `XiaoValue*`，成功时替换为拥有值，失败时保留原值；无返回值入口省略 `out`。15 个入口的参数顺序、符号名和能力掩码位置按上表冻结。状态码使用 0/1/2/3/6/8；语言级错误对象和取消通道不另开一套。

### 5.5 Windows COFF 验收冻结表

| 项 | 候选判据 |
| --- | --- |
| 适用范围 | Windows x86_64 MSVC 的公共/发布产物；调试、strip 和内部探测分区登记 |
| 必需等级 | 产品验证器返回 `Complete`；`UnverifiedCoffExports` 直接失败；当前状态仍为 `pending` |
| 组件 | 声明组件集合与观察组件集合完全一致；未分类 `xiao_runtime_` 符号为空 |
| 证据 | 产物 SHA-256 + 源码/Runtime/工具/目标 + 任务根 + CI run/job/attempt |
| 机制 | 保留 COFF 符号表的真实产物，或与产物哈希绑定的受信 linker map/PDB 完整清单 |
| 不接受 | 单独导出表、同链接候选、未绑定哈希的 sidecar、B2-C 历史同链接例外 |
| 负例 | 缺声明、额外组件、未分类符号、导出表-only、旧 sidecar、哈希/身份失配、`UnverifiedCoffExports` |

B3-0D 冻结“什么算产品级 `Complete`”和验证器落点；B3-1/B3-7 才产生实际产品证据。2026-11-24 对固定 B3-0B 归档中的四份 Windows O0/O3 产物进行了可行性探测，COFF 符号表指针和符号数均为零，导出表可读，且没有 PDB/map sidecar；因此现有产物仍是 `UnverifiedCoffExports`。探测只确认现有归档不能满足新政策，不是产品级 COFF 验收，状态保持 `pending`。[探测与原始日志](assets/20o-b3/entry-20261124-b3-0d/README.md)

## 六、证据矩阵

| 层 | 必测 |
| --- | --- |
| 基线 | B3-0C 收口、15 个原语、产品/ABI/生成器/接口、两个锁、B1 28 守卫 |
| 编号 | 49～63 唯一性、1～48 原义、ID 100 墓碑、C 符号一一对应 |
| 能力 | 位 0～7、`KNOWN_CAPABILITY_MASK`、旧默认掩码、位 8+ 拒绝、多能力 log |
| 版本 | ABI 1.20、生成器 23、接口 1、`.xiaoc`/`.xar`/索引/IR/原生元数据 |
| 产物 | 旧 1.19/22 拒绝/重生成、缓存身份、`.xiaoc`/`.xar`/原生元数据矩阵 |
| 协议 | `RunOptions.arguments`、hello `"arguments"`、CLI/嵌入/原生来源、失配拒绝 |
| C ABI | 15 个符号、签名、输出槽、状态码、所有权、无别名 |
| COFF | `Complete` 判据、声明=观察、验证器落点、证据身份、全部负例 |
| 冻结审计 | 固定 Git 对象、裁定台账、C 符号、版本表、COFF 负例、只读性 |
| 边界 | 无产品/registry/教程 verified、无 B3-0E/B3-1、无旧 721 |

## 七、退出条件

1. B3-0D0～D5 完成，六项首轮问题有实际答复或逐项待答；
2. 15 个 intrinsic 编号、名称、效果、能力和 C 符号冻结；1～48/ID 100 不变；
3. `environment` 位 7、已知掩码、旧请求默认掩码和多能力检查冻结；
4. ABI 1.20、生成器 23、接口 1 联合候选冻结；`.xiaoc`/`.xar`/索引/IR/原生元数据格式结论明确；
5. 旧 1.19/22 产物的拒绝/重生成、缓存身份和跨平台产物矩阵冻结；
6. 15 个 C ABI 符号、签名、状态、所有权和 VM/LLVM 绑定冻结；
7. `RunOptions.arguments`、hello 能力协商、原生注入和失配拒绝冻结；
8. Windows 产品级 COFF `Complete` 政策、验证器落点、证据路径和负例冻结；实际状态仍为 `pending`；
9. 证据审计与自反例通过；没有产品/registry/教程 verified 变化；
10. B3-0E 入口明确，B3-0E/B3-1/B4 和旧 721 未开始。

## 八、风险与禁止

- 未经星崽批准预分配 intrinsic 编号、能力位或升级 ABI/生成器；
- 重编号 1～48、复用已发布 ID、删除 ID 100 墓碑；
- 把 `environment` 位加进旧请求默认掩码而不显式记录；
- 忽略 `log` 的 `clock` + `stdout` 双能力，或只检查一个位；
- 把 `.xiaoc`/`.xar` 容器格式与 ABI/生成器身份混为一谈；
- 用导出表、同链接映射或未绑定哈希的 sidecar 冒充 COFF `Complete`；
- 把 B3-0D 的验证器可行性探测写成 B3 产品验收；
- B3-0D 写产品、升级 registry/教程、启动 B3-0E/B3-1/B4 或跑旧 721。

## 九、B3-0E 交接

B3-0E 接收 15 个原语、六模块契约、版本/格式/缓存矩阵、C ABI、协议和 COFF 政策。B3-0D 不得自行改 B3-0B/C 已冻结 API；发现冲突回对应批次修订并重新冻结。

交接必须包含：

- 15 个 intrinsic 编号、名称、效果、能力、所有权和 C 符号；
- `environment` 位号、已知掩码、旧默认掩码和多能力检查规则；
- ABI 1.20、生成器 23、接口 1 和格式/缓存结论；
- 旧 1.19/22 产物行为与三平台产物矩阵；
- 15 个 C ABI 签名、状态码、所有权和 VM/LLVM 绑定；
- `RunOptions.arguments`、hello 能力协商和原生注入；
- Windows COFF `Complete` 判据、验证器落点、证据路径和负例；
- B3-0D 证据归档和只读审计回执。

## 十、落点

```text
docs/DevDocs/20o-b3-0d-version-intrinsic-abi-coff-plan.md   本规划
docs/DevDocs/assets/20o-b3/entry-20261124-b3-0d/            执行时建立的证据
docs/DevDocs/20o-b3-0-probe-freeze-plan.md                  B3-0 总规划
docs/DevDocs/20o-b3-host-libraries-plan.md                  B3 宿主库规划
core/rust/crates/xiao-intrinsics/intrinsics.json            执行时的候选登记，不在本规划修改
core/rust/crates/xiao-runtime-abi/src/lib.rs                执行时的 ABI 增量
core/rust/crates/xiao-codegen-llvm/src/artifact.rs          执行时的 COFF 验证落点
docs/module-registry.json                                   只读核对 planned，不升级
stdlib/                                                     本批不改产品
```

本页只做 B3-0D 规划；B3-0E/B3-1 及后续仍未开始。

## 十一、新 Agent 先读

1. [B3-0 探测/冻结规划](20o-b3-0-probe-freeze-plan.md)；
2. [B3-0C 收口](assets/20o-b3/entry-20261123-b3-0c/README.md)；
3. [B3-0C 契约](assets/20o-b3/entry-20261123-b3-0c/contract-draft.md)；
4. [B3-0B 契约](assets/20o-b3/entry-20261123-b3-0b/contract-draft.md)；
5. [B3 宿主库规划](20o-b3-host-libraries-plan.md)；
6. [20D 能力上下文](20d-capability-context.md)；
7. [20P Day-1 架构](20p-day1-library-architecture.md)。
