# 20O-B1-FIX-B. `error_code` 前置实现规划

> **这是 FIX-B 的实现前规划，不是产品代码。**
> 精确契约的唯一正文仍是 [B1 冻结草案 §二](20o-b1-freeze-draft.md#二error_code-的名称位置与签名)；
> 本页只把已确认的签名、C 状态和验收表拆成可执行步骤。不得在本批重新选择
> `std.errors`、成员访问、文本匹配或 snapshot/clear 模拟读取。
>
> 状态：**实现已落地，本地门禁/三平台取证中（2026-11-01）**。实际实现、生命周期修正和
> 验收记录见 [FIX-B 收口](20o-b1-fix-b-closeout.md)。FIX-A 契约与三平台探测已收口，
> 证据提交 `3b78363` 已推送，工作区门禁 [41296123514](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41296123514)
> 成功；该提交只改文档，未触发维护回归，不能代替 FIX-B 证据。
> 当前代码为 ABI 1.18，intrinsic 32 与 C getter 已实现；下表保留开工前 1.17 基线，
> 不作为当前实现状态。三平台验收齐备前七库 IMP 继续阻塞。

## 一、目标与边界

把“从已捕获错误对象读取原始 code”做成真实通路：

```text
error_code(Error value, /) -> str
```

本批只交付：

1. intrinsic 32、`error_code` 的类型检查/IR/VM/LLVM/Runtime 全链路；
2. 只读 C getter `xiao_runtime_error_code(const XiaoValue*, XiaoAbiBytes*)`；
3. Runtime ABI 1.17 → 1.18，以及 `.xiaoc`/`.xar`/原生产物的版本拒绝；
4. 默认/自定义 code、嵌套 catch、重复读取、重抛、结果脱离绑定、类型负例、
   挂起槽保护、失败清理和三平台差分证据。

明确不做：

- 不新增 `std.errors` 或其它模块，不开放 `error.code` 成员访问；
- 不实现 `error_cause`/`error_message`/`error_suppressed` 或通用反射；
- 不实现错误码模式匹配、`Result<T, E>` 或 `?`；
- 不实现七库任何函数体，不把 FIX-B 混入 B1-IMP；
- 不改变 32B `XiaoSharedHeader`、16B `XiaoSharedHandle`、`XiaoValue` 布局或并发规则。

## 二、开工前源码基线（历史记录）

| 检查项 | 现状 | 落点 |
| --- | --- | --- |
| intrinsic 表 | 活动声明到 31，100 是墓碑；没有 32 | `core/rust/crates/xiao-intrinsics/intrinsics.json` |
| 契约枚举 | 没有 `ErrorCode`/`Error` 类别/对应 VM、ABI 绑定 | `core/rust/crates/xiao-intrinsics/src/lib.rs` |
| Runtime ABI | `ABI_MINOR_VERSION = 17`，编码断言 `0x0001_0011` | `core/rust/crates/xiao-runtime-abi/src/lib.rs:26`、`:1330` |
| C getter | 没有 `xiao_runtime_error_code` 定义或声明 | `core/rust/crates/xiao-runtime/src/abi.rs` |
| 错误对象 | `AbiError` 盒子、`clone_error`、`value_from_owned_error` 已有 | `abi.rs:245`、`:443`、`:705` |
| 现有读取 | 只有挂起槽的 `error_take`/`error_snapshot` | `abi.rs:1220`、`:1260` |
| VM | `execute_intrinsic` 没有 ErrorCode 分支 | `core/rust/crates/xiao-vm/src/semantics/exec.rs:1826` |
| LLVM | `emit_intrinsic` 没有 ErrorCode 分支；动态判定白名单也缺 | `core/rust/crates/xiao-codegen-llvm/src/dynamic/expression.rs:508`、`predicate.rs:209-224` |
| 原生产物 | 元数据文档写死 1.17，校验使用精确常量 | `xiao-codegen-llvm/src/native_metadata.rs:32`、`:246-256` |
| 兼容矩阵 | `1.17` 为 Direct，`1.16` 为 Reject | `xiao-driver/src/compatibility.rs:373-395` |
| 生成器版本 | `CODEGEN_VERSION = 14` | `xiao-codegen-llvm/src/lib.rs:95` |
| 教程现状 | 错误控制流页把错误码读取写成后续阶段 | `docs/UseDocs/language/control-flow/error-handling.md:56` |
| 精确执行 | `20o_b1_probe` 的 VM/原生目标已被维护回归精确执行，守卫 28/28 | `.github/workflows/maintenance-regression.yml`、`tools/gates/b1-probe-guards.ps1` |

FIX-A 的三平台绿色只证明七库探测，不解除上述阻塞；本规划的前提是 B1 §二已冻结，
而不是重新讨论“要不要做”。

## 三、冻结契约摘要（实现时逐字遵守）

| 项目 | 值 |
| --- | --- |
| 名称/位置 | 全局内建自由函数 `error_code`，无需 import；仅位置参数 |
| 签名 | 恰好一个 `Error` 参数，返回拥有的不可变 `str` |
| intrinsic | id `32`，`kind=ErrorCode`，`arity=One`，参数 `Error`，返回 `Str` |
| 效果/能力 | `Allocates`、`capability=None`；裸 getter 不分配 |
| 所有权 | 借读；不消费、不重抛、不 take/snapshot/clear；结果脱离 catch 绑定仍有效 |
| 状态 | 读取不改挂起槽、error_id、location、stack、cause、suppressed 或语言环境 |
| 资源 | 裸 getter 不分配；公开结果是 O(L) 复制；不截断 code |
| C 入口 | `int32_t xiao_runtime_error_code(const XiaoValue *value, XiaoAbiBytes *out)` |
| C 状态 | `0 Ok`、`1 Null`、`2 InvalidArgument`、`3 InvalidHandle` |
| 失败约束 | `out` 与挂起槽逐字不变；空 code 的 `len=0`，`ptr` 可为空 |
| 源级类型错误 | 静态已知非错误值 `X04-TYPE-002`；动态非错误值 `X06-RUNTIME-002` |
| 分配失败 | 结果复制失败沿用 `X06-RUNTIME-011`；无部分字符串 |
| Fatal | 沿用既有构造/捕获边界，不新增可恢复通道 |

## 四、实现设计

### 4.1 契约表与 Rust 枚举

在 `intrinsics.json` 的 31 与墓碑 100 之间加入：

```json
{"id":32,"name":"error_code","kind":"error_code","arity":"one","parameter_type":"error","return_type":"str","vm_binding":"error_code","abi_binding":"error_code","effect":"allocates","capability":"none","error":"runtime.type_mismatch","status":"active"}
```

`error` 字段目前是声明性记录，不参与运行分派；这里填与实际源级失败一致的消息键，
不要复制成成员访问或 snapshot 方案。

同步修改：

- `IntrinsicKind::ErrorCode`、`ValueType::Error`、`VmBinding::ErrorCode`、`AbiBinding::ErrorCode`；
- `build.rs` 增加 `error_code` 分支：一元、`error` 参数、`str` 返回、两个绑定一致、
  `allocates`、`none` 能力；
- `validate_table()` 增加同一组精确校验；
- 契约单测：id 32 活动、名称唯一、墓碑 100 仍不可执行、声明总数比 FIX-A 多 1。

不要复用或重排既有 id；100 仍是墓碑。

### 4.2 类型检查

在 `xiao-types/src/checker/expression.rs` 的 `check_call` 中，沿用 `input`/`print`
的登记顺序加入 `ErrorCode` 分支（放在普通函数值调用之前）：

1. 记录 `record_intrinsic_call(callee, span)`；
2. 检查实参形态：必须恰好一个，且 `kind == positional`、`name.is_none()`；
   关键字、命名参数、展开实参和零/多实参都报 `X04-TYPE-002`；
3. 对实参调用 `check_expression`，保留类型节点的旁路信息；
4. 若实际类型是 `Type::Dynamic` 或尚未统一的 `Type::Variable`，留给运行时；
   若已是任何具体类型（`int`/`str`/`none`/容器/资源/线程句柄等），立即报
   `X04-TYPE-002`，并带上 `expected=Error`、`actual=<实际类型>` 的结构化参数；
5. 返回 `Type::scalar(ScalarType::Str)`，错误恢复时也不要返回 `Dynamic` 伪装成功。

本批**不需要新增 `RuntimeCheckKind`**，也不改 IR/字节码的检查计划：静态边界在类型层；
动态边界由 VM intrinsic 和 LLVM 的 getter 前检查产生，二者都保留 `expected/actual`，
并且都发生在 C getter 之前。若实现中发现必须新增检查类别，先停下来核对契约，不要
顺手扩 IR 编码。

### 4.3 Runtime C getter

在 `xiao-runtime/src/abi.rs` 的错误入口区新增：

```rust
#[unsafe(no_mangle)]
pub extern "C" fn xiao_runtime_error_code(
    value: *const XiaoValue,
    out: *mut XiaoAbiBytes,
) -> i32
```

严格按顺序：

1. `value` 或 `out` 为空 → `XiaoAbiStatus::Null`，不写 `out`；
2. `(*value).tag != XiaoValueTag::Error` → `InvalidArgument`，不写 `out`；
3. 取出 `payload.handle`；空句柄、魔数/种类不符、`inners` 为空 →
   `InvalidHandle`，不写 `out`；
4. 取 `inners.first()` 的 `code()`，把 `ptr/len` 写入 `out`；空 code 写
   `ptr=null, len=0`；
5. 全程不 retain/release、不 clone、不分配、不调用 `PENDING_ERROR`。

注意第 3 步：Error 标签下的空载荷必须返回 `InvalidHandle`，不能误报成 `Null`；
`Null` 只表示调用方传入的 `value`/`out` 指针为空。`error_ref` 的 `Null` 也要在这里
转成 `InvalidHandle`，因为 C 状态 1 的语义已冻结为“入口指针为空”。

实现复核修正：原规划取 last 与已冻结借用期冲突。AbiError retain 向尾部加入深拷贝，
release 弹出尾部；释放另一个别名会销毁 last 的 code，尽管输入强引用仍活着。
必须从直到最后引用才销毁的 first 借出；此为遵守既有生命周期契约的实现修正，不改公开签名/状态。
回归先比较视图地址，失败时不解引用可能悬空的旧指针，再验证复制后结果独立拥有。

在 `xiao-runtime-abi/src/lib.rs` 的 `extern "C"` 块加入声明与文档，说明：
借用视图只在输入 Error 值仍存活且未释放期间有效；getter 不改变挂起槽；失败时
`out` 原值不变；调用方复制完成后才能释放输入。

### 4.4 VM 路径

在 `xiao-vm/src/semantics/exec.rs` 的 `execute_intrinsic` 中增加：

- 校验 `Arity::One`、一个位置参数、无关键字；
- 仅接受 `RuntimeValue::Error(error)`；其它值返回
  `XiaoError::type_mismatch("error", value.type_name())`，并附加当前 span；
- 读取 `error.code()` 后复制为 `RuntimeValue::new_string(...)`；
- 不移动、不替换、不释放原 `RuntimeValue::Error`，不读挂起槽；
- 分配失败按 Runtime 既有 `StringHandle::new` 错误路径返回 `ResourceError`
  （`X06-RUNTIME-011`），不返回部分字符串。

补 VM 单元/集成测试：同对象重复读取、读取后重抛、读取后保存、非错误动态值、
结果脱离绑定后继续分配。VM 的错误对象是 `Box<XiaoError>`，实现时不要为了“省一次
clone”改成取走对象；owned 结果只复制 code。

### 4.5 LLVM 动态路径

1. `dynamic/predicate.rs:209-224` 把 `IntrinsicKind::ErrorCode` 加入必须在动态
   后端处理的类别；否则静态路径会把它当未知 intrinsic 拒绝或错误降低。
2. `dynamic/expression.rs` 的 `emit_intrinsic` 增加 `VmBinding::ErrorCode`：
   - 校验一元位置参数；
   - 求值参数并存入 `alloca %xiao.value`，**在复制完成前保持强引用存活**；
   - 调用既有的 `xiao_runtime_dynamic_check("error_value", ptr arg_slot)`，
     在 mismatch 时产生 `X06-RUNTIME-002`；失败分支必须先 release 参数再进入
     统一错误路由，不能泄漏临时值；
   - 准备 `alloca %xiao.bytes` 并清零，调用
     `xiao_runtime_error_code(ptr arg_slot, ptr out)`；
   - 成功后读取 `%xiao.bytes`，用既有 `xiao_runtime_string_new` 复制，
     再 `xiao_runtime_value_str_owned` 接管结果；
   - 复制成功后再 release 输入；复制失败时同样先 release，再走错误路由，
     不得留下半成品或重复释放；
   - getter 的 1/2/3 状态都在动态检查之后：3 按冻结错误表构造
     `MemoryError`/`X06-RUNTIME-004` 后路由；1/2 属于编译器生成指针和预检查下
     不应出现的内部不变量失败，明确走 fatal 边界，不要伪装成成功或静默继续。
3. 在 `dynamic/runtime_abi.rs` 的声明集合加入
   `declare i32 @xiao_runtime_error_code(ptr, ptr)`；`xiao_runtime_dynamic_check`
   已有声明，只新增 `"error_value"` 操作名。
4. 在 `xiao-runtime/src/abi_dynamic.rs` 的 `xiao_runtime_dynamic_check` 中：
   - 解析 kind 后，`error_value` 直接检查 `XiaoValueTag::Error`，成功不 clone、
     不分配；失败用 `RuntimeError::type_mismatch("error", 实际类型名)`；
   - 该分支必须在现有 `value_to_runtime` 之前返回，避免为了检查而深拷贝错误对象；
   - 旧 Runtime 不认识 `"error_value"` 也无妨：新产物要求 ABI 1.18，旧 Runtime
     在执行前已被版本检查拒绝。

LLVM 文本测试至少断言：getter、string_new、value_str_owned 的先后关系；
没有 `error_take`/`error_snapshot`/`error_clear` 被用于读取；输入 release 在复制之后。

### 4.6 ABI 与产物版本

- `ABI_MINOR_VERSION` 17 → 18；编码断言改为 `0x0001_0012`；
- 兼容矩阵改为 `1.18` Direct、`1.17`/`1.16` Reject（三种产物轴都覆盖）；
- `native_metadata.rs` 的精确范围注释、校验和测试跟随常量；`generator_version < 14`
  的下限保留，但新产物使用新生成器版本；
- `CODEGEN_VERSION` 14 → 15：新增 intrinsic 和 LLVM 符号改变了生成器能力与产物
  指纹，必须隔离旧缓存；不要用“未使用 error_code 的程序文本没变”作为不升版本的理由；
- `.xiaoc` 的 `runtime_abi_version` 精确校验、`.xar` 的 `runtime_abi_min/max`、
  原生元数据三者都要有“1.17 旧产物拒绝、1.18 新产物接受”的代表性测试；
- 保留既有旧产物拒绝测试，不删减为只测新版本；三平台证据必须来自实际执行，
  不能用本地编译成功冒充。

### 4.7 测试与证据

新增夹具建议放 `tests/fixtures/20o-b1/fix-b/`，与 FIX-A 分开：

| 场景 | 断言 |
| --- | --- |
| 默认 code | 内建错误（如算术/类型错误）catch 后返回默认 code |
| 自定义 code | 大小写/Unicode 逐字一致；同类不同码分别读取 |
| 嵌套 catch | 内外层各自读到自己的对象，不串读 |
| 重复读取/重抛 | 同一对象读两次后再 `raise`，外层仍得到同一 code |
| 结果脱离绑定 | 从函数/局部 catch 返回 code，销毁源错误并继续分配后仍正确 |
| 显式对象 + 挂起哨兵 | 读取显式对象不改变当前 catch 的 code/error_id/location/stack |
| 动态类型负例 | 异构动态值进入 `error_code` → `X06-RUNTIME-002`，VM/native 同码 |
| 静态类型负例 | `error_code(1)`、零/多参数、关键字参数 → `X04-TYPE-002` |
| Fatal 边界 | `FatalError` 构造/捕获仍是既有 `X07` 边界，不进入可恢复读取 |
| 清理 | 成功、getter 失败、复制失败三条路径的临时引用恰好释放一次 |

C 级测试放 `xiao-runtime/src/abi_error_code_tests.rs`：

- 状态 0 的 ptr/len、空 code、Unicode/长 code；
- 状态 1（value/out 为空）、2（非 Error 标签）、3（空/错句柄），逐项检查
  `out.ptr`、`out.len` 和挂起快照逐字不变；
- 读取前后 `inners` 长度/强引用轨迹不变；不分配、不 retain/release；
- 出错后哨兵错误仍可 snapshot/take；
- 结果分配失败注入：在 `xiao-runtime` 增加仅在测试 feature 下启用的
  线程本地 failpoint（生产构建不注册），验证 `xiao_runtime_string_new` 失败返回
  `X06-RUNTIME-011`、无部分字符串、源错误与别名不变；VM 源码级注入使用同一
  feature 的 dev-dependency 接线，原生进程不启用生产环境变量注入。

驱动级测试沿用现有 `core/rust/crates/xiao-driver/tests/20o_b1_probe.rs` 的两个精确
目标，新增 `FIX_B_CASES` 与独立 golden，不另开 CI 步骤：

- VM 目标继续由 `b1_capabilities_have_vm_golden_outputs` 精确执行；
- 三平台原生目标继续由 `b1_capabilities_match_native_golden_outputs` 精确执行；
- 每个 FIX-B 场景打印独立 `20O-B1-FIX-B-PROBE case=...` 标记，便于证据摘录；
- 现有计数守卫 28/28 不变，不新增或静默减少执行路径。

这样做的理由：FIX-B 是 B1 的硬前置，复用已经受守卫保护的同一组精确入口，避免再造
一套容易漂移的 workflow 路径；但夹具、golden 和日志标记必须独立，不能把 FIX-B
结果混进七库黄金。

交付证据目录：

```text
docs/DevDocs/assets/20o-b1-fix-b/<run-id>/
```

包含 README、`evidence.json`、默认 VM 与三平台原生日志摘录；`evidence.json` 记录
run id、job id、head sha、场景清单、精确汇总行和完整下载源日志的 SHA-256。
哈希必须针对实际下载的完整日志重新计算，不能用摘录文件代替。

## 五、文档落点

- `docs/UseDocs/language/control-flow/error-handling.md`：把“错误码读取属于后续”
  改为当前可用的 `error_code(error)`，说明借读/不消费/结果可保存，并保留“错误码模式
  匹配仍是后续”的边界；
- `docs/UseDocs/library/*.md` 与 `docs/UseDocs/language/concurrency/README.md`：
  只更新当前基线的 ABI 版本陈述，历史 FIX-A 取证仍写 1.17 的不要改写；
- `docs/DevDocs/20o-b1-freeze-draft.md`：只更新“当前代码仍为 1.17”等状态句，
  不重写已冻结契约；
- `docs/DevDocs/20o-b1-implementation-plan.md`：FIX-B 状态改为“实现中/完成”，
  并链接本页；
- `docs/DevDocs/20-builtins-and-standard-library.md`、`docs/DevDocs/README.md`：
  增加 FIX-B 行；
- 七库 UseDocs 状态保持 `planned`，本批不升级 `verified`。

## 六、验收命令与退出条件

实现完成后至少执行并记录：

```text
cargo fmt --manifest-path core/rust/Cargo.toml --all -- --check
cargo clippy --manifest-path core/rust/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path core/rust/Cargo.toml -p xiao-intrinsics
cargo test --manifest-path core/rust/Cargo.toml -p xiao-runtime
cargo test --manifest-path core/rust/Cargo.toml -p xiao-types
cargo test --manifest-path core/rust/Cargo.toml -p xiao-vm
cargo test --manifest-path core/rust/Cargo.toml -p xiao-codegen-llvm
cargo test --manifest-path core/rust/Cargo.toml -p xiao-bytecode
cargo test --manifest-path core/rust/Cargo.toml -p xiao-xar
cargo test --manifest-path core/rust/Cargo.toml --workspace -- --skip b1_capabilities_have_vm_golden_outputs
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- b1_capabilities_have_vm_golden_outputs --exact --nocapture
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- --ignored b1_capabilities_match_native_golden_outputs --exact --nocapture
bun run check:docs
bun run check:usedocs
bun run check:layout
pwsh tools/gates/b1-probe-guards.ps1
```

退出条件：

1. intrinsic 32 活动且契约、VM、LLVM、Runtime 四处一致；没有额外 intrinsic；
2. C getter 的 0/1/2/3、失败 `out`、挂起槽、借用期和复制顺序均有直接测试；
3. VM 与原生逐字输出/错误码/清理一致，结果脱离绑定后仍正确；
4. ABI 1.18 的 `.xiaoc`、`.xar`、原生旧产物拒绝与新产物接受均有测试；
5. 默认 workspace、VM 精确、三平台原生精确和完整门禁全部通过，并有实际
   run/job 链接；
6. 文档状态与实际证据一致；七库教程仍为 `planned`；
7. 未修改并发 32B/16B 布局、普通句柄规则或 19/11/22 债务。

## 七、交接给实现 Agent

按顺序读：

1. [B1 冻结草案 §二](20o-b1-freeze-draft.md#二error_code-的名称位置与签名)；
2. 本页；
3. [B1 §2.5 借用视图验收表](20o-b1-freeze-draft.md#25-fix-b-借用视图的实现验收)；
4. [B1 实现规划](20o-b1-implementation-plan.md) 的 FIX-B 与 IMP 边界。

硬规则：

- 本批只做 FIX-B；不要顺手实现七库或开放其它错误访问器；
- 遇到契约未写明的行为，先停下来问星崽，不要用 snapshot/clear/member access
  作为“临时方案”；
- 不删除既有测试来换绿色；不接受“文档写了”代替实际运行；
- 提交前核对实际测试数、精确目标和日志；三平台结论必须有 run/job；
- 规划者只审核，不代替实现；实现完成后再由审核者逐条复核。
