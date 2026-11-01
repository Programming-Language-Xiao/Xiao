# 20O-B1-IMP. 七库产品实现规划

> **这是 B1-IMP 的实现前规划，不是产品代码。**
> 精确契约的唯一正文仍是 [B1 冻结草案 §三～§七](20o-b1-freeze-draft.md#三七库已确认签名与调用边界f2f8f12)；
> 本页只把七个模块、20 个入口、资源/错误/性能目标和落地顺序拆成可执行步骤。
> 不重问 F1～F13/E-2，不新增 intrinsic、ABI、私有语法或公开 `dynamic`。
>
> 状态：**FIX-B 已验收，12 入口草稿已落地，B1-IMP 因前置缺口暂停（2026-11-02）**。
> math/statistics/fmt 已有 VM 草稿验证；原生命名调用与字符串遍历成本尚待处理，见
> [实测与暂停记录](20o-b1-imp-blockers.md)。规划基线为 `48a47440`，
> 实施开工提交为 `6a23eabc`；已发现并单独处理既有 `std.` 别名接线前置，见 §2.2。
> 当前 Runtime ABI 为 1.18，七库 registry 条目仍为 `planned`，七库教程仍为 `planned`。
> B1-IMP 完成后才进入 [B1-CLOSE](20o-b1-implementation-plan.md#b1-close证据收口)；
> 本批不把教程升级为 `verified`。

## 一、批次目标与不可变边界

在 `stdlib/` 新增七个纯 Xiao 模块，恰好交付以下 20 个公开名称：

| 模块 | 文件 | 公开名称 |
| --- | --- | --- |
| math | `stdlib/math.xiao` | `abs`、`min`、`max`、`clamp`、`gcd`、`pow` |
| hex | `stdlib/hex.xiao` | `encode`、`decode` |
| base64 | `stdlib/base64.xiao` | `encode`、`decode`；两者 `url_safe` 必填 |
| url | `stdlib/url.xiao` | `encode_component`、`decode_component` |
| csv | `stdlib/csv.xiao` | `parse_record`、`format_record` |
| fmt | `stdlib/fmt.xiao` | `pad_left`、`pad_right`、`format_int` |
| statistics | `stdlib/statistics.xiao` | `sum`、`min`、`max` |

冻结签名：

```text
abs(int value) -> int
min(int left, int right) -> int
max(int left, int right) -> int
clamp(int value, int lower, int upper) -> int
gcd(int left, int right) -> int
pow(int base, int exponent) -> int

encode([int] bytes) -> str
decode(str text) -> [int]

encode([int] bytes, bool url_safe) -> str
decode(str text, bool url_safe) -> [int]

encode_component(str text) -> str
decode_component(str text) -> str

parse_record(str record) -> [str]
format_record([str] fields) -> str

pad_left(str text, int width, str fill) -> str
pad_right(str text, int width, str fill) -> str
format_int(int value, int radix) -> str

sum([int] values) -> int
min([int] values) -> int
max([int] values) -> int
```

不做：

- 不新增 `stdlib` 辅助模块、私有可见性、嵌套辅助函数或下划线私有名；
- 不在新模块顶层 `from`/`import`，也不新增顶层辅助变量；
- 不新增 intrinsic/ABI/值类型，不修改 FIX-B 错误读取通路；
- 不实现七库以外的 mean/median/variance、全文 CSV、URL 解析、浮点格式化等；
- 不把教程/registry 升为 verified；本批只交付产品实现与初步证据。

## 二、开工前基线与关键陷阱

| 项 | 当前状态 | 依据 |
| --- | --- | --- |
| FIX-B | id 32、ABI 1.18、getter 与三平台验收完成 | [FIX-B 收口](20o-b1-fix-b-closeout.md) |
| 七库源码 | 不存在；registry 七项 `code=[]`、status `planned` | `docs/module-registry.json` |
| 现有纯 Xiao 风格 | 顶层 `def`、无顶层 import、无 helper | `stdlib/text.xiao`、`path.xiao`、`set.xiao` |
| 标准库加载 | `stdlib.<module>` 固定身份，`std.<module>` 用户别名 | `xiao-driver/src/frontend.rs` |
| 顶层导入 | 会被 `resolver.rs` 加入模块导出接口，可能泄漏辅助名 | `xiao-modules/src/resolver.rs:422-514` |
| 既有产品测试 | 20M 的 VM/native、移除、归档、别名和教程一致性模式 | `tests/20m_text_and_path.rs`、`20c_first_modules.rs` |
| 精确执行点 | `20o_b1_probe` 的 VM 与三平台原生目标已被维护回归精确执行 | `.github/workflows/maintenance-regression.yml`、`tools/gates/b1-probe-guards.ps1` |

### 2.1 两个必须先遵守的约束

1. **顶层导入会变成导出接口。** 七个模块不得在文件顶层写 `from stdlib.text import ...`；
   需要复用既有标准库时，把导入放在使用它的公开函数体内（现有用户模块支持函数内导入），
   或者直接把逻辑写在函数体里。最终导出集合必须从真实模块接口验证，不能只看源码 `def`。
2. **没有私有 helper。** 所有辅助步骤必须内联在公开函数体中；可以在函数体内使用局部变量、
   数组、循环和 `array_push`，但不能新增顶层 `def`，也不能用 `_` 前缀绕过导出。

`error_code` 已可用；产品代码本身不需要调用它，测试可以用它核对错误身份。

### 2.2 经星崽确认的既有 std 别名接线前置

首次执行公开 `std.math` 产品样例时发现：前端已识别别名并链接标准库，但 VM 将它作为
`project:std.math` 加载，以 `X06-RUNTIME-012` 失败。既有 `from std.text import length`
也可独立复现，说明这是七库之前的加载接线缺口。星崽已确认先独立修复再继续七库验收。

修复只统一身份：标准库元数据 crate 提供 `canonical_module_identity`，VM、LLVM 与归档调用图
共同将 `std.` 送入 `stdlib.` 固定命名空间；虚拟并发特例先按原有路径处理。
LLVM 的 CODEGEN_VERSION 升至 16 以隔离修正后的初始化/绑定指纹，Runtime ABI 仍为 1.18。
不新增语法、intrinsic、ABI 或官方导出。四种导入形态分别验证 VM、规范身份只初始化一次、
真实 `.xar` 的宿主要求与删除应用源码后重放；原生复用既有 20C 差分入口执行选择导入与
函数内导入。原生模块值/限定调用在规范 `stdlib.` 路径也会拒绝，新增配对回归确认别名的
失败边界与规范路径相同；本前置不扩大这一既有后端能力边界。
七库实现另行提交，教程与七库 registry 条目保持 planned。

本机 Windows 初步验证：20C 默认目标 8 passed / 3 ignored，含 VM、真实归档和原生配对
降低边界；既有原生差分目标显式 `--exact` 执行 1 passed / 0 failed / 0 ignored。
这仍是本机证据，最终七库交付时另记录真实三平台运行号。

## 三、实施顺序与每模块要点

本批采用的落地顺序为 **math → statistics → fmt → hex → base64 → url → csv**。
这是对高层规划中“math→hex→base64→url→fmt→statistics→csv”建议的顺序调整：先把最简单的
标量/资源模板固定，再做字节与 UTF-8，CSV 状态机和预算仍最后做；模块契约、错误、限额和
交付集合不变。若星崽要求沿用原顺序，实现 Agent 只需调整提交顺序，不影响最终产品。
每完成一个模块就运行定向 VM 测试和导出接口检查，不要等七个全部写完才第一次执行。

### 3.1 math

- `abs`：0 返回 0；`MIN` 不能取正绝对值，明确报 `ArithmeticError / X06-RUNTIME-009`；
  其它负数返回 `-value`。
- `min`/`max`：二元比较；相等返回同一数值。
- `clamp`：先检查 `lower > upper`，报 `Error / MATH_INVALID_BOUNDS`；再比较钳位。
- `gcd`：结果非负；`gcd(0,0)=0`；`gcd(MIN,2)=2`、`gcd(MIN,-1)=1`；
  `gcd(MIN,0)`、`gcd(MIN,MIN)` 的数学结果 2^63 不可表示，报 `X06-RUNTIME-009`。
  禁止先做 `abs(MIN)`；实际实现将两数归一到包含 MIN 的非正域做余数迭代，遇 -1 直接返回 1，
  只在最终取正结果时产生应有溢出，无需对 MIN 求正绝对值。
- `pow`：`exponent < 0` 报 `Error / MATH_NEGATIVE_EXPONENT`；`0^0=1`；
  `0^正数=0`；`MIN^1=MIN`；使用平方求幂，按二进制位决定是否乘，最后一步之后不再计算
  无必要的平方；`pow(2,63)` 必须溢出；指数可到 `MAX`，不得人为限为 63。
- 算术溢出使用既有 `X06-RUNTIME-009`，不改成浮点、lint 或饱和。

### 3.2 statistics

- `sum([])=0`；从头到尾 checked int 累加；`[MAX,1,-1]` 在第二项立即溢出；
  不排序、不重排、不修改输入数组。
- `min`/`max` 空数组报 `Error / STATISTICS_EMPTY`；直接比较，不做差值；
  两端极值和重复值均合法。
- 数组元素超过 65,536 项时，在遍历前或遍历到上限+1时拒绝，报
  `ResourceError / STATISTICS_LIMIT_EXCEEDED`；不要先复制整个数组。

### 3.3 fmt

- `pad_left`/`pad_right`：`width < 0` 报 `Error / FMT_INVALID_WIDTH`；
  `fill` 必须恰好一个 Unicode 码点（即使 `width` 不大于原文也必须检查），否则
  `Error / FMT_INVALID_FILL`；宽度按码点数而非 UTF-8 字节；不截断原文。
- 输入 text 最多 65,536 UTF-8 字节，width 0～65,536，结果最多 262,144 字节；
  超限报 `ResourceError / FMT_LIMIT_EXCEEDED`。
- 填充可用二进制翻倍或平衡块拼接；不要逐字符反复追加导致二次复制。
- `format_int`：radix 必须在 2～36；数字表为
  `0123456789abcdefghijklmnopqrstuvwxyz`；无前缀、不补零、负号在最前；
  `0` 在每种进制为 `"0"`；`MIN` 全进制必须成功，不能先取正绝对值；结果最多 65 字节。
  非法 radix 报 `Error / FMT_INVALID_RADIX`。

### 3.4 hex

- `encode`：输入 `[int]` 每项必须在 0～255，否则 `Error / HEX_INVALID_BYTE`；
  每字节输出两位小写十六进制，无 `0x`；空输入返回空串。
- `decode`：输入是字符串，不是字节数组；先检查 UTF-8 字节数和偶数长度，
  奇数报 `Error / HEX_INVALID_LENGTH`，然后逐字符校验十六进制；大写/小写都接受，
  前缀、分隔符、空白和非 ASCII 报 `Error / HEX_INVALID_CHARACTER`；空串返回 `[]`。
- 输入/输出上限按 B1 §四：encode 最多 65,536 项、结果 131,072 字节；
  decode 最多 131,072 UTF-8 字节、结果最多 65,536 项；超过报
  `ResourceError / HEX_LIMIT_EXCEEDED`。
- 输入数组只读；结果数组新建；不要修改调用方元素。

### 3.5 base64

- 两个入口都带必填 `bool url_safe`：false 使用 `A-Za-z0-9+/`，true 使用 `A-Za-z0-9-_`；
  共享字母相同，模式专用字符严格隔离，不自动探测，不新增无模式入口。
- 两种模式都补位到 `4*ceil(n/3)`，不换行，不缺省 `=`。
- `encode` 每项 0～255，否则 `Error / BASE64_INVALID_BYTE`；上限 65,536 项、
  结果最多 87,384 字节。
- `decode` 必须长度是 4 的倍数；`=` 只在最后一组末尾，0/1/2 个；
  双补位时第二索引低 4 位为零，单补位时第三索引低 2 位为零；拒绝空白、模式外字符、
  非 ASCII、中间/过量/缺少补位和非零尾位；统一报
  `Error / BASE64_INVALID_ENCODING`。
- decode 输入最多 87,384 UTF-8 字节，输出实际字节数最多 65,536；**仅文本长度合规
  不代表解码长度合规**，要按补位计算真实长度；超限报
  `ResourceError / BASE64_LIMIT_EXCEEDED`。
- 可用字符码点算术实现字母表映射，避免顶层导入；不得用同一算法往返替代独立向量。

### 3.6 url

- `encode_component`：只保留 ASCII 字母、数字、`-._~`；其余按字符的 UTF-8 字节输出
  大写 `%HH`；空格为 `%20`，`+` 为 `%2B`，`/` 为 `%2F`。
- `decode_component`：接受 `%HH` 两种十六进制大小写；普通 Unicode 原文保留，`+`
  不变空格；不正规化；先查转义语法，再校验还原字节的 UTF-8。
- 非法 `%` 序列报 `Error / URL_INVALID_ESCAPE`；还原字节非法 UTF-8（截断、
  孤立续字节、非最短形式、代理区、>U+10FFFF）报 `Error / URL_INVALID_UTF8`；
  `%00` 可还原 NUL，BOM、组合字符原样保留。
- 输入/输出上限：encode 输入 65,536 UTF-8 字节、输出 196,608 字节；
  decode 输入 196,608 字节、输出 65,536 UTF-8 字节；超过报
  `ResourceError / URL_LIMIT_EXCEEDED`。
- 用码点算术产生 UTF-8 字节；用局部字节数组和严格 UTF-8 校验，不要依赖宿主或
  新增 intrinsic。合法输入满足 `decode_component(encode_component(s)) == s`。

### 3.7 csv

- 只实现记录子集：逗号分隔、逻辑记录无结束符；引号只能在字段起始；引号内 `""`
  表示双引号；闭引号后只许逗号或结束；引号外 CR/LF、未闭合引号、未引号字段中的引号、
  闭引号后多余文字报 `Error / CSV_INVALID_RECORD`；引号内 CR/LF/CRLF 原样保留。
- `parse_record("")` 返回 `[""]`；不剥 BOM、不裁剪空白、不做数字/日期转换。
- `format_record([])` 报 `Error / CSV_EMPTY_FIELDS`；仅对含逗号/双引号/CR/LF 的字段
  加外引号并双写内部引号；不追加结束符；`format_record([""]) == ""`。
- 资源上限与预算必须按 B1 §4.1：记录 ≤65,536 UTF-8 字节；字段数 1～1,024；
  单字段 ≤16,384 字节；原始总量 S ≤65,536 只是预检；精确
  `E = S + Q + 2*W + (k-1)` 必须 ≤65,536；任一超限统一
  `ResourceError / CSV_LIMIT_EXCEEDED`，无部分结果。
- 检查顺序：类型 → 字段数/单字段/S 预检 → 扫描 Q/W 计算 E → 构造结果；
  多个超限维度同一 code。CSV 语法错误与预算错误并存时按 B1 §6.1 固定顺序。
- 解析和格式化都要单次有界扫描；格式化在分配结果前算 E；不要先构造超限字符串。
- 这是本批最高风险模块：B0 的全量大字段局部探测曾在 60 秒看门狗返回
  `X09-DRIVER-002`。先用最大合法输入做局部性能测试，再做三平台证据；若不达标，
  停下来报告，不缩小公开限额、不放宽预算、不新增 intrinsic。

## 四、错误与限额总表

当前逐模块进度：math 六项、statistics 三项、fmt 三项有纯 Xiao 草稿、完整产品样例与
独立黄金，三组真实导出与 VM 黄金已通过；math 另有移除、归档、普通包和教程回归。
三组已接入原有精确目标，原生命名调用尚被前置缺口阻塞，不能宣称完整原生验收通过。
其余四库暂停，F10 只有 fmt 的部分 VM 数据；七库最终三平台证据和 CLOSE 均未完成，状态 planned。

错误 code、类别和检查顺序以 [B1 §六](20o-b1-freeze-draft.md#六统一错误表f11已确认) 为唯一正文。
实现时至少核对：

| 模块 | 内容错误 | 资源错误 | 算术错误 |
| --- | --- | --- | --- |
| math | `MATH_INVALID_BOUNDS`、`MATH_NEGATIVE_EXPONENT` | — | `X06-RUNTIME-009` |
| hex | `HEX_INVALID_BYTE`、`HEX_INVALID_LENGTH`、`HEX_INVALID_CHARACTER` | `HEX_LIMIT_EXCEEDED` | — |
| base64 | `BASE64_INVALID_BYTE`、`BASE64_INVALID_ENCODING` | `BASE64_LIMIT_EXCEEDED` | — |
| url | `URL_INVALID_ESCAPE`、`URL_INVALID_UTF8` | `URL_LIMIT_EXCEEDED` | — |
| csv | `CSV_INVALID_RECORD`、`CSV_EMPTY_FIELDS` | `CSV_LIMIT_EXCEEDED` | — |
| fmt | `FMT_INVALID_WIDTH`、`FMT_INVALID_FILL`、`FMT_INVALID_RADIX` | `FMT_LIMIT_EXCEEDED` | — |
| statistics | `STATISTICS_EMPTY` | `STATISTICS_LIMIT_EXCEEDED` | `X06-RUNTIME-009` |

- 内容错误用 `Error(code=...)`；资源上限用 `ResourceError(code=...)`；
  算术不可表示用 `ArithmeticError / X06-RUNTIME-009`（或让现有 checked 算术产生同一身份）。
- 共同静态参数/类型错误沿用 `X04-TYPE-002`，动态边界沿用 `X06-RUNTIME-002`；
  不把类型错误混入内容错误。
- 失败不修改输入、不返回部分结果；输入数组只读、结果数组新建。

## 五、测试与证据

### 5.1 分层测试

| 层次 | 内容 |
| --- | --- |
| 模块单测/VM | 每个入口成功、边界、错误、上限+1、输入不变、失败无部分结果 |
| 导出接口 | 真实模块接口恰好 20 个名称；无顶层 import/helper/变量泄漏；`std.` 与 `stdlib.` 两路径 |
| 别名/重名 | hex/base64 的 `encode`/`decode`、math/statistics 的 `min`/`max` 别名导入结果 |
| 移除 | 逐个移除公开函数后，VM 加载失败或 LLVM 静态绑定给出带 span 的明确拒绝 |
| 归档 | 真实 `.xar` 只记录实际 stdlib_modules，不复制函数体；删除源码后可重放 |
| 普通包隔离 | `stdlib` 保留根不被普通包占用，不进入 PackageRegistry/锁文件 |
| 教程 | 产品示例与 fixture 逐字一致；七库页面保持 planned |
| 资源/性能 | F9 上限、F10 翻倍比/最大输入/内存；CSV 预算与全量扫描另测 |

产品运行时夹具建议放在 `tests/fixtures/20o-b1/imp/<module>/product.xiao`，每个模块一份
完整示例和独立 `golden.json`；B0 的 `tests/fixtures/docs/<module>/probe.xiao` 保留，不改成产品
夹具。错误夹具通过 `error_code(err)` 打印稳定 code，不能只打印分支标签。

### 5.2 精确执行点

优先**复用现有两条精确目标**，不新增 workflow 路径或计数守卫：

- 在 `core/rust/crates/xiao-driver/tests/20o_b1_probe.rs` 增加 `B1_IMP_CASES`，加入
  `b1_capabilities_have_vm_golden_outputs` 与
  `b1_capabilities_match_native_golden_outputs`；
- 每个模块打印独立 `20O-B1-IMP-PROBE case=<module> backend=vm/native optimization=<0|3>`
  标记；原生 O0/O3 各执行一次；
- 现有 `tools/gates/b1-probe-guards.ps1` 保持四路径 28/28；不得静默减少路径。
- 导出/移除/归档/普通包/教程一致性放在普通 workspace 测试；可新增
  `20o_b1_imp_contract.rs`，但不要新增无守卫的 ignored 精确入口。
- 如果实现确实需要新的精确目标，必须同步更新维护回归和守卫（路径、场景总数、
  文档），不能只加 workflow 步骤。

精确目标仍须满足：恰好 `1 passed; 0 failed; 0 ignored`，原生退出码 0、stderr 为空；
不能把 workspace 批量通过当作三平台证据。

### 5.3 证据与文档

- 本批完成后先记录一次真实维护回归/工作区门禁的运行号与 job；证据目录建议
  `docs/DevDocs/assets/20o-b1-imp/<run-id>/`，包含 README、机器清单和四份精确日志。
- 完整源日志哈希、字节数、摘录行号按 FIX-A/FIX-B 的方式登记；摘录不能代替源哈希。
- B1-IMP 不把教程/registry 升 `verified`；B1-CLOSE 再按最终三平台证据升级。
- 更新 `docs/DevDocs/20o-b1-implementation-plan.md`、`docs/DevDocs/README.md`、
  `docs/DevDocs/20-builtins-and-standard-library.md` 的 IMP 状态与链接。

## 六、文档与 registry 落点

实现时同时更新：

```text
stdlib/<module>.xiao                         七个产品模块
stdlib/<module>.md                           七个模块 README（接口/错误/限额/不新增 intrinsic）
stdlib/README.md                             模块清单与接口身份
docs/UseDocs/library/<module>.md             保留 planned，增加产品示例段
docs/module-registry.json                    七项 code/tests 路径；status 仍 planned
docs/DevDocs/20o-b1-imp-seven-libraries.md   本规划
docs/DevDocs/20o-b1-implementation-plan.md   状态/链接
docs/DevDocs/README.md                       索引
docs/DevDocs/20-builtins-and-standard-library.md  阶段表
tests/fixtures/20o-b1/imp/<module>/           产品源码与独立黄金
core/rust/crates/xiao-driver/tests/20o_b1_probe.rs  精确目标扩展
core/rust/crates/xiao-driver/tests/20o_b1_imp_contract.rs  可选：契约/移除/归档测试
```

`docs/module-registry.json` 七项的 `code` 要加真实 `stdlib/<module>.xiao`；`tests` 要加产品
fixture 和 contract 测试；`status` 保持 `planned`。不要只更新 README 或只改 registry。

## 七、性能与资源验收（F9/F10）

- 每个入口的上限按 B1 §四；等于上限合法，上限+1 失败；不得截断、丢弃或部分输出。
- 预算运算本身不能先溢出；长度扫描最多到上限+1；不要先复制全部超大输入再检查。
- 结果构造优先使用块构造/平衡拼接；避免 `result = result + character` 造成二次复制。
  该优化只能写在公开函数体内，不能新增 helper。
- 按 F10 协议测量：4,096→8,192→16,384 翻倍比 ≤3；最大合法输入 VM ≤10s、
  原生 ≤2s；峰值增量内存 ≤64 MiB；O0/O3 分别记录；预热/中位数/固定批次。
- CSV 最大记录（含全引号/多换行）和 URL 非 ASCII、Base64 全模式、fmt 最大填充必须
  单独测量；若性能目标未达，停止并报告，不通过缩小公开限额或放宽预算解决。

## 八、退出条件

1. 七个模块存在于 `stdlib/`，恰好 20 个公开名称；无顶层 import/helper/变量泄漏；
2. 所有签名、参数名、错误 code/类别/优先级和 F9 限额符合 B1；
3. 每模块 VM/native 成功、边界、错误、失败无部分结果、输入只读；
4. 导出/别名/重名、逐函数移除、真实 `.xar` 重放、普通包隔离和教程源码一致性通过；
5. 默认 workspace、VM 精确、三平台原生精确和完整门禁通过；证据带 run/job；
6. F10 测量完成或明确记录未达标；CSV 全量性能不再是未验证的历史债；
7. 七库教程/registry 仍为 `planned`，B1-CLOSE 才升级；
8. 不新增 intrinsic/ABI/公开 dynamic，不修改 FIX-B 或 19/11/22 债务。

## 九、交接给实现 Agent

按顺序读：

1. [B1 冻结草案 §三～§七](20o-b1-freeze-draft.md#三七库已确认签名与调用边界f2f8f12)；
2. [FIX-A §1.1 导出面](20o-b1-fix-a-contract-closeout.md#11-helper-与导出面)；
3. [FIX-B 收口](20o-b1-fix-b-closeout.md)（只作为前置，不重做）；
4. 本页；
5. 现有 [`stdlib/text.xiao`](../../stdlib/text.xiao)、[`20m_text_and_path.rs`](../../core/rust/crates/xiao-driver/tests/20m_text_and_path.rs)
   和 [`20c_first_modules.rs`](../../core/rust/crates/xiao-driver/tests/20c_first_modules.rs) 的实现/测试风格。

硬规则：

- 本批只写七库产品和对应测试/文档；不要顺手新增语言能力、intrinsic 或发布收口；
- 遇到契约未写明的语义，先停下来问星崽；不要用近似算法、浮点、snprintf 或新 intrinsic 兜底；
- 每个模块先跑定向 VM/导出测试，再做 native；不要七个模块全写完才第一次执行；
- 不能用同一算法往返替代独立黄金，不能把本地通过写成三平台结论；
- CSV 若在最大输入上仍超时，保留记录并停止，不要删测试或缩小限额；
- 提交前核对实际导出集合、错误 code、精确目标恰好一项通过，以及守卫仍覆盖原有路径。
