# 20O-B1. 七库冻结契约与 error_code 前置

> **状态：B1-FIX-A 契约已逐条确认，本地新增探测完成；未开始产品实现。** 日期：2026-11-01。
> 原草案基线为 74f1c60，本轮开工基线为 819ec06。文件名保留 draft，确认状态以 §一为准。
> 本批只写文档和探测，不注册新 intrinsic、不改 ABI 常量或产品函数，不认领 19/11/22 债务。
>
> F1～F13 与 E-2 已由星崽逐条答复：error_code 采用全局自由函数、intrinsic 规划号 32、
> ABI 1.18 的只读借用视图后复制通路；七库的辅助逻辑内联。当前代码仍是 ABI 1.17。
> 必须先在独立 B1-FIX-B 实现并验收前置，再进入纯 Xiao 七库。
>
> ad176c3 的 maintenance 41275847931 与 workspace gates 41275847899 是旧七项探测的三平台
> 历史证据，不能外推本轮新增边界、error_code 或产品验收。分批遵循
> [实现规划](20o-b1-implementation-plan.md)，本轮证据见 [FIX-A 收口](20o-b1-fix-a-contract-closeout.md)。

## 一、已确认范围与逐条裁定

来源：星崽在本次 B1-FIX-A 会话逐条直接答复；此前 D-23～D-26 和 CSV P3-1 的确认继续有效。
七库一起冻结、CSV 只做记录子集、全局自由函数 error_code、精确 CI 与 planned 教程门槛不重问。

| 编号 | 已确认裁定 | 当前状态 |
| --- | --- | --- |
| F1 | error_code(Error value, /) -> str，全局、借读、不消费；Error 仅为入口参数类别，error_cause 后置 | 契约已确认，未实现 |
| F2 | math 六项的 MIN/MAX、gcd、0^0=1、负指数拒绝与无多余平方溢出 | 已确认 |
| F3 | hex 的 [int] 字节接口、严格大小写解码、空输入及非法长度/字符 | 已确认 |
| F4 | **首批同时支持标准与 URL-safe**；两个入口都增加必填 bool url_safe；严格补位/尾位/拒绝空白，不混模式字母表 | 已确认，入口总数仍为 20 |
| F5 | UTF-8 组件、-._~、大写百分号、保留 +/普通 Unicode、严格解码与不规范化 | 已确认 |
| F6 | 无结束符的严格 CSV 记录；引号状态、BOM/空白/字段内换行保留、空记录与零字段区别 | 已确认 |
| F7 | fmt 码点宽度、负宽度拒绝、始终校验单码点 fill、2～36 进制与最小 int 全范围 | 已确认 |
| F8 | statistics 顺序 int 累加，中途溢出即失败，输入不排序/修改 | 已确认 |
| F9 | §四全部数值与首版无放宽开关；CSV 延续 S 预检和精确 E≤64 KiB | 已确认；产品未验证 |
| F10 | §五复杂度、翻倍比、VM/原生时间、增量内存与测量协议 | 已确认验收目标；未宣称性能达标 |
| F11 | §六完整错误表与优先级；Base64 模式不符归 BASE64_INVALID_ENCODING | 已确认 |
| F12 | 20 入口参数必填、现有位置/命名调用，无新增方法别名；只读输入/新数组；CSV 引号和 URL NUL/BOM/组合字符规则 | 已确认 |
| F13 | 独立 FIX-B：intrinsic 32、ABI 1.18、只读字节视图后复制；C 状态 0/1/2/3 与 §2.2 源级错误表 | 编号、符号、读取及失败边界均已确认，尚待实现 |
| E-2 | B1 辅助逻辑内联，只导出冻结的 20 项，不新增内部模块或私有语法 | 已确认；通用私有可见性另立语言批 |

上述确认是契约冻结，不等于能力已交付。FIX-B 的真实读取通路/产物兼容仍是七库实现前置；
产品的资源/性能、导出/移除/归档和三平台证据仍须在后续阶段补齐。教程与 registry 保持 planned。

## 二、error_code 的名称、位置与签名

### 2.1 已确认公开契约（F1/F13）

```text
error_code(Error value, /) -> str
```

| 字段 | 已确认契约 |
| --- | --- |
| 名称/位置 | 全局内建自由函数 error_code，无需 import；FIX-B 注册，不建立 std.errors 模块 |
| 参数 | 恰好一个，仅位置；接受现有可恢复错误对象及子类；不同时开放普通函数的 Error 注解 |
| 返回 | 拥有的不可变 str，逐字等于传入对象存储的原始 code，包括自定义大小写/Unicode；不返回 message/message_id/类别 |
| 所有权 | 借读，不消费、不重新抛出；结果脱离 catch 绑定仍有效，原错误仍可保存、重复读取和重新抛出 |
| 状态 | 读取本身不动挂起槽，不改 error_id、location、stack、cause、suppressed 或语言环境 |
| 资源/复杂度 | 裸 getter 不分配，公开结果复制为 O(L) 时间/结果存储；不截断已有 code，分配失败沿用既有内存边界 |
| 能力/并发 | 不需宿主能力，不放宽错误对象的线程边界，不开放 FatalError 捕获 |
| 后置范围 | error_cause/message/suppressed 访问器、error.code、通用反射、普通函数 Error 注解 |

Error 是内建入口的规格参数类别；当前 catch 绑定仍为 Type::Dynamic，类型层没有普通 Error
注解。FIX-B 可在 intrinsic 契约中增加 Error 类别和相应检查，不把内部 dynamic 扩成公开 API。

### 2.2 error_code 的源级错误表

本表与 §2.4 的裸 C 状态已经一起提交星崽并确认。正常错误传播与“通过挂起槽提取 code”是
不同路径：参数/复制失败走既有异常机制，读取已有错误时绝不 take/snapshot/clear 或临时 raise。

| 情况 | 阶段/身份 | 行为 |
| --- | --- | --- |
| 参数数不为 1、关键字参数、静态已知非错误值 | 前端 X04-TYPE-002 | 执行前拒绝 |
| 内部动态边界实际为 none/标量/容器等非错误对象 | TypeError，X06-RUNTIME-002 | getter 前拒绝，保留期望/实际类型的结构化诊断 |
| 合法可恢复错误对象 | 成功 | 返回该对象原始 code，不因已被 catch 处理而失败 |
| 构造/捕获 FatalError | 沿用 X07-TYPE-003 等已有 Fatal 边界 | 不开可恢复通道 |
| 结果字符串受控分配失败 | ResourceError，X06-RUNTIME-011 | 沿用已核现有分配类别；无部分字符串，原错误对象不变，须失败注入验证 |
| 无法安全建立错误对象的内存耗尽 | Fatal，X07-FATAL-003 | 沿用不可恢复边界 |
| 既有内存层报告无效/已释放句柄 | MemoryError，X06-RUNTIME-001 / X06-RUNTIME-004 | 内部防御路径保留具体身份，不新增源级裸句柄访问 |

这些均引用现有诊断身份，本轮不加诊断常量。当前代码仍未实现 error_code，不能把本表当作
运行结果；源码访问能力在 FIX-B 交付。

### 2.3 现状与必要前置

- catch 绑定在 xiao-types/src/control_checker.rs 中仍登记为 Type::Dynamic；
- xiao_runtime_error_take 把线程挂起错误取为拥有的 Error 值；
- xiao_runtime_error_snapshot 只接受线程挂起槽，不接受指定对象；
- 当前 xiao_runtime_dynamic_member_get 仍只处理表成员，error.code 边界探测继续返回 X06-RUNTIME-002；
- Rust 错误对象已有 code()，但当前 Xiao/LLVM 没有直接对象读取通路。

F13 已选择新增独立 intrinsic 和 C ABI，故 FIX-B 是必要批次，不再写成“是否需要待定”。
本批只冻结以下声明，不修改 intrinsics.json、Runtime ABI 常量、Runtime 或编译器实现。
七库批在 FIX-B 之后，不能把前置混入纯 Xiao 库提交。

### 2.4 intrinsic / ABI 精确读取通路

现表活动声明到 31，100 是墓碑。规划号 **32** 专用于 error_code；它与并发诊断
X06-RUNTIME-032 属于不同编号空间，任何既有并发错误与布局均不改。

| 项目 | FIX-B 冻结值 |
| --- | --- |
| intrinsic | id=32，public_name=error_code，kind=ErrorCode，arity=One，参数类别 Error，返回 Str |
| 效果/能力 | 公开操作 Allocates、capability=None；分配来自结果复制，裸 getter 不分配 |
| IR/VM | 现有 IntrinsicCall 携带稳定 id；VM 匹配 RuntimeValue::Error 并把 code 复制为拥有的 str |
| LLVM | 源级参数检查 → 新 getter 的借用视图 → 已有 string_new 复制 → str_owned 接管结果 |
| ABI/布局 | 1.17→1.18；新增下述符号；XiaoValue/XiaoAbiBytes 与 32B/16B 布局不变 |
| 产物 | FIX-B 更新 .xiaoc/.xar/原生的 ABI 要求及指纹；1.18 要求在旧 Runtime 上执行前拒绝，保留旧产物兼容测试 |

```c
int32_t xiao_runtime_error_code(
    const XiaoValue *value,
    XiaoAbiBytes *out
);
```

getter 借读有效 Error 对象，返回 code 的 UTF-8 指针/字节数；不分配、不 retain/release，
不读取或写入挂起槽。out 与 value 不重叠；失败保留 out 原值；空 code 的 len=0，ptr 可为空。

| C 状态 | 语义 |
| --- | --- |
| 0 Ok | 成功写入借用视图，挂起槽不变 |
| 1 Null | value 或 out 指针为空，out/挂起槽不变 |
| 2 InvalidArgument | 输入不是 Error 标签，out/挂起槽不变 |
| 3 InvalidHandle | 没有有效错误句柄，out/挂起槽不变 |

调用方必须提供合法、存活的内存，不能承诺任意悬空指针都能被安全识别。输入强引用存活到
复制完成：LLVM 用既有 xiao_runtime_string_new 复制视图，再由 xiao_runtime_value_str_owned
接管结果句柄；借用指针不泄漏给 Xiao，也不得在释放源对象后继续使用。
源级动态类型错误先于 getter 走正常 TypeError 传播；字符串复制失败沿用既有内存通路。

FIX-B 验收包括：默认/自定义 code、同类不同码、内外层嵌套捕获、重复读取后重抛、
结果脱离绑定、cause/suppressed/error_id 不变、源级参数负例、C 失败输出槽不变及挂起槽
保护、VM/native 同输出/错误/清理、ABI 1.18 版本拒绝和三平台证据。

## 三、七库已确认签名与调用边界（F2～F8/F12）

公共路径为 `std.<module>`，固定身份为 `stdlib.<module>`。以下 20 个入口全为自由函数，
参数名属于契约：全部必填，可按现有位置或命名参数规则调用；无默认值、可变参数、重载或
新增方法别名。`error_code` 的仅位置规则单独见 §二。

所有数组/文本输入只读且不消费调用方引用；结果数组新建，不与输入数组共享可变存储；
不可变字符串可按现有实现共享，但结果必须有独立的有效生命周期。失败不修改输入、没有
可见的部分输出。异构/类型不符参数遵守共同前端/运行时检查，不隐式改用浮点、lint 或 dynamic。

### 3.1 math：六项

```text
abs(int value) -> int
min(int left, int right) -> int
max(int left, int right) -> int
clamp(int value, int lower, int upper) -> int
gcd(int left, int right) -> int
pow(int base, int exponent) -> int
```

| 边界 | 精确提案 |
| --- | --- |
| 数值域 | 固定 64 位有符号 int；不另做 sint/lint/float 重载 |
| abs | 0 返回 0；最小 int 的正绝对值不可表示，报现有溢出 |
| min/max | 二元数值最小/最大；相等返回同一数值，无排序/状态 |
| clamp | 闭区间 [lower, upper]；先检查 lower<=upper，再钳位；相等边界合法 |
| gcd | 结果非负；gcd(0,0)=0；负输入按数学绝对值计算；只在**最终 gcd**不可表示时溢出 |
| gcd 极值 | gcd(MIN,2)=2、gcd(MIN,-1)=1；gcd(MIN,0)/gcd(MIN,MIN) 的结果 2^63 报溢出；禁止先 abs(MIN) 导致误拒绝 |
| pow | exponent 必须非负，0^0=1；0^正数=0；结果必须精确可表示，不回绕/饱和 |
| pow 极值 | pow(MIN,1)=MIN，不计算无必要的后续平方；pow(2,63) 溢出；指数可到 int 最大值，不人为限为 63 |

### 3.2 hex：两项

```text
encode([int] bytes) -> str
decode(str text) -> [int]
```

字节载体为 [int]，每项 0～255。encode 每字节恰好两位小写 ASCII，不带 `0x`；
decode 接受大小写，但不接受前缀、分隔符或任何空白；先核对长度为偶数，再从左到右校验
十六进制字符。空数组与空串互转。字符串不是字节数组，不把 Unicode 码点直接当字节。

### 3.3 base64：两项

```text
encode([int] bytes, bool url_safe) -> str
decode(str text, bool url_safe) -> [int]
```

url_safe **必填**：false 为标准字母表 A-Z/a-z/0-9/+ /，true 为尾部 -_ 的 URL-safe 字母表。
共享字符照常接受，模式专用字符严格隔离；不能混用，不自动探测，不新增入口。
两种模式都按 4*ceil(n/3) 长度强制补位且不换行，URL-safe 不代表允许省略补位。

解码长度为 4 的倍数；= 只能在最后一组末尾，个数为 0/1/2；双补位时第二索引低 4 位为零，
单补位时第三索引低 2 位为零。拒绝空白、模式外字符、非 ASCII、中间/过量/缺少补位
及非零尾位；空输入互转，错误整次失败。模式不符归 BASE64_INVALID_ENCODING。

### 3.4 url：两项

```text
encode_component(str text) -> str
decode_component(str text) -> str
```

只保留 ASCII 字母、数字、`-._~`，其他内容按 UTF-8 字节转大写 `%HH`；空格为 `%20`，
加号为 `%2B`，斜杠为 `%2F`。解码接受 `%HH` 的两种十六进制大小写；普通 Unicode 原文
原样保留，`+` 不变为空格。不进行大小写、Unicode 正规化、路径归一化或完整 URL 分解。

拒绝残缺/坏百分号和非法 UTF-8（孤立续字节、截断、非最短形式、代理区、超过 U+10FFFF）。
`%00` 可还原 NUL，BOM 及组合字符均保留；后续宿主路径/HTTP 层自行执行其字段约束。
空串往返为空串。合法输入满足 decode_component(encode_component(s))=s。

### 3.5 csv：两项，已确认记录子集

```text
parse_record(str record) -> [str]
format_record([str] fields) -> str
```

记录是逻辑记录，无记录结束符。逗号分隔字段；引号只能在字段起始出现；带引号字段内部的
双引号写为 `""`，单个闭引号之后只许逗号或输入结束。引号外 CR/LF、未闭合引号、未引号字段
中的引号及闭引号后的多余文字均拒绝。引号内 CR、LF、CRLF 原样保留，不规范化换行。

format_record 仅对含逗号/双引号/CR/LF 的字段加外引号，并将内部双引号双写，不追加结束符；
不隐式解析数字/日期，不裁剪空白，不剥除 BOM，也不自动添加表格软件专用转义。
空文本解析为 `[""]`，`format_record([""])=""`；零字段 `[]` 格式化报错，避免与一个空字段混同。
所有合法且未超限的 fields 满足 parse_record(format_record(fields))=fields。
全文文件/多记录、表头、方言探测、流式解析及 `[[str]]` 仍后置。

### 3.6 fmt：三项

```text
pad_left(str text, int width, str fill) -> str
pad_right(str text, int width, str fill) -> str
format_int(int value, int radix) -> str
```

width 按 Unicode 标量/码点计数，不按 UTF-8 字节、字素簇或终端列宽；必须非负。
fill 必须恰好一个码点，即使 width 不大于原文长度仍须校验。宽度不足时补齐，否则原文不截断。
format_int 只接受 2～36 进制，使用 `0123456789abcdefghijklmnopqrstuvwxyz`，无前缀、不补零，
负号始终位于最前；0 在每种进制均为 `"0"`。最小 int 全进制必须成功，不先取正绝对值。
最大输出为 65 个 ASCII 字符（最小 int 的二进制负号加 64 位）；浮点/locale/模板格式化后置。

### 3.7 statistics：三项

```text
sum([int] values) -> int
min([int] values) -> int
max([int] values) -> int
```

D-5/F8 已确认：sum([])=0，空 min/max 报错，溢出使用现有身份；sum 从左到右从 0
开始 checked int 累加；例如 `[MAX,1,-1]` 在第二项立即溢出，不重排、不自动提升精度。
min/max 比较不做差值相减，允许 int 两端极值；重复数值没有特殊规则。不排序或修改数组。
mean/median/variance/stddev、浮点、NaN 与高精度策略仍归 B5。

## 四、资源限额（F9；已确认，产品未验证）

KiB=1,024 字节。字符串大小均按 **UTF-8 字节数**，width 单独按码点数，数组限额按元素项数。
等于上限合法，超过即失败；不得截断、丢弃字段、返回部分结果或隐式放宽。首版不提供限额开关。

| 入口 | 输入限制 | 结果限制 |
| --- | --- | --- |
| math 六项 | 参数为 int 合法范围；pow 的非负指数可到 MAX | 结果须在 int 范围；无可变长度输出 |
| hex.encode | 最多 65,536 字节整数 | 最多 131,072 ASCII 字节 |
| hex.decode | 最多 131,072 UTF-8 字节，且必须是合法偶数长 ASCII 十六进制 | 最多 65,536 项 |
| base64.encode | 最多 65,536 字节整数 | 最多 87,384 ASCII 字节 |
| base64.decode | 最多 87,384 UTF-8 字节；需通过全部严格格式校验 | 最多 65,536 项；仅满足文本长度上限仍可能超限，必须按补位核对真实长度 |
| url.encode_component | 最多 65,536 UTF-8 字节 | 最多 196,608 ASCII 字节 |
| url.decode_component | 最多 196,608 UTF-8 字节 | 最多 65,536 UTF-8 字节 |
| csv.parse_record | 整条记录最多 65,536 UTF-8 字节 | 最多 1,024 字段；每字段解码后最多 16,384 UTF-8 字节 |
| csv.format_record | 1～1,024 字段，单字段≤16,384 字节；S≤65,536 仅预检，还须精确 E≤65,536 | 编码后≤65,536 字节；预算超限统一 CSV_LIMIT_EXCEEDED，无部分结果 |
| fmt.pad_left/right | text 最多 65,536 UTF-8 字节；0<=width<=65,536；fill 一个码点 | 最多 262,144 UTF-8 字节 |
| fmt.format_int | int 与 2～36 进制 | 最多 65 ASCII 字节 |
| statistics 三项 | 最多 65,536 项 | 一个 int |
| error_code | 既有错误对象；不为访问器新增 code 截断上限 | 完整原始 code，资源受既有分配边界约束 |

预算检查本身也需有界：无法常量取得长度时，最多扫描到上限+1 即拒绝，不先扫描/复制全部超大输入。
不能调用已知会反复扫描的码点下标来“测量一次长度”。结果长度先用安全整数计算；乘加前做除法式
上限判定，不能让预算运算先溢出。CSV 计数/转义后长度以及 Base64 的解码长度需独立检查。
结果对外可见之前完成校验；私有暂存若失败须按原有清理机制释放。

### 4.1 CSV 编码预算

原表把原始字段总量≤65,536 与编码结果≤65,536 并列，却未写完整接纳条件，容易把预检
通过误读为保证可格式化成功。双引号会双写，外引号和逗号也占字节；CR/LF 自身不双写，
但会使该字段需要外引号。

**P3-1 已由星崽直接确认：保留 64 KiB 编码后预算，显式校验精确 E。** 原始总量限制只是
预检；它与 E 预算共同决定资源接纳条件，不承诺每个结构/类型合法且 S≤65,536 的输入都会成功。
F9 的其余限额也已确认；FIX-B 与产品验收仍未完成，不能据预算冻结标为产品可用。

设 k 为字段数、S 为所有字段的 UTF-8 字节总量、Q 为双引号字符总数、W 为需要外引号的
字段数（含逗号/双引号/CR/LF 的字段）。非空 fields 的精确编码长度：

```text
E = S + Q + 2*W + (k-1)
```

没有记录结束符或 BOM 的额外添加。按已确认的单字段与字段数限额，S=65,536、k=1,024 时，
最坏 E=134,143；例如四个各含 16,384 个双引号的字段已经满足原始总量限制，但 E=131,083。
因此 **S 上限不是完整的合法输入判据**。

已确认的预算检查顺序：

1. 共同类型检查优先；fields=[] 报 `CSV_EMPTY_FIELDS`；
2. 字段数、单字段字节数、原始总量的预检超限报 `CSV_LIMIT_EXCEEDED`；
3. 扫描统计 Q/W，在分配结果前用有界加法核对精确 E；E>65,536 也报 `CSV_LIMIT_EXCEEDED`；
4. 全部通过才构造结果；输入不变，错误无部分输出。多个资源维度同时超限仍是同一 code，
   不得按本地化文本区分超限身份。

若类型、空字段数组与预算问题同时存在，按上述阶段确定身份。format_record 接受的字段
本身就是文本数据，没有 CSV 引号语法错误；引号语法错误只属于 parse_record。保留 64 KiB
预算的结果能进入当前同上限的 parse_record；在记录子集格式/其他限额都确认并满足时，才承诺
往返。实施前补 E 恰好上限、上限+1、全引号、多换行和多字段向量，不把此次文档裁定冒充实现证据。

按已确认单字段阈值构造的预算验收例（只规定数学预期，产品尚未实现）：

| 字段内容/长度 | S / Q / W / k | E | 编码预算预期 |
| --- | --- | --- | --- |
| 四个不需引号的字段，长度 16,384/16,384/16,384/16,381 | 65,533 / 0 / 0 / 4 | 65,536 | 预算通过 |
| 上例末字段增加一个普通 ASCII 字符 | 65,534 / 0 / 0 / 4 | 65,537 | CSV_LIMIT_EXCEEDED |
| 四个字段各含 16,384 个双引号 | 65,536 / 65,536 / 4 / 4 | 131,083 | CSV_LIMIT_EXCEEDED |
| 四个字段各含 16,384 个 LF | 65,536 / 0 / 4 / 4 | 65,547 | CSV_LIMIT_EXCEEDED |

## 五、性能预算与测量协议（F10）

以下预算与测量协议已由星崽确认，**是待实测验收目标，不是已有性能成绩或产品超时语义**。n/m 为输入/输出实际规模。

| 算法 | 允许的成本目标 |
| --- | --- |
| math abs/min/max/clamp | 常量时间、常量状态 |
| math gcd/pow | gcd 为对数次数的余数迭代，pow 为对数指数步数；禁止按指数值逐次乘法 |
| statistics | 线性遍历，算法状态常量；Runtime 迭代快照如复制数组，必须计入 O(n) 实际存储 |
| hex/base64/url/csv | 有界次数的顺序扫描；结果构造至多 O((n+m) log(n+m))，总辅助空间 O(n+m) |
| fmt padding | O(n+m) 扫描加至多 O((n+m) log(n+m)) 的构造；format_int 至多 64 次数字提取 |
| error_code | 与 code 字节数成线性；不遍历 cause、suppressed 或堆栈 |

现有字符串逐次 `result + character` 与循环 `text[index]` 可能造成二次复制/扫描；B0 的短示例
不能证明它们满足预算。优先评估块构造/均衡拼接与一次迭代；不为达标擅自新增 intrinsic。

已确认的可复跑测量协议：

1. 使用同一提交、同一真实源码与固定输入，VM 与原生分别测；记录目标三元组、CPU、工具链、
   编译配置/优化级别；原生固定 O0、O3 分别登记，不能拿 O3 代替 O0 正确性证据。
2. 编译、进程/VM 初始化、输入生成、打印和结果黄金比较在计时外；先预热 1 次，再采 5 次取中位数。
   对太短的输入批量重复到至少 50ms，按调用数归一化，固定批次数以便复跑。
3. 规模 4,096→8,192→16,384，固定合法内容/转义比例，归一化耗时翻倍比每档不超过 3；
   编解码、全部引号 CSV、非 ASCII URL/填充分别取样，不能只测 ASCII 或空输入。
4. 每入口最大合法输入的单次已编译调用：VM 不超过 10s，原生不超过 2s；这是受控验收目标，
   不是 API 运行时 timeout 或对任意硬件的保证。进程看门狗另留启动/采样余量。
5. 单次最大输入的峰值增量内存目标不超过 64 MiB；以同进程已预热、输入已准备的基线扣除
   固定 Runtime/VM/输入成本，包含算法暂存、迭代快照及输出；三平台记录各自测量方法。
6. 超限输入只验证拒绝所需的上限+1 范围；回归包含边界输入、最坏转义和失败清理。
   未测或未达标明确登记，不因缩小公开限额、放宽预算、只贴小样本时间而宣称通过；修改预算再问星崽。

## 六、统一错误表（F11，已确认）

F11 已确认下表全部 code、类别与顺序，本批不在产品注册。已有 Error/ResourceError/ArithmeticError
类别保持不变；不新增错误类型，也不占用已冻结 027–032。

| code | 类别 | 唯一触发条件 |
| --- | --- | --- |
| `MATH_INVALID_BOUNDS` | Error | clamp 的 lower>upper |
| `MATH_NEGATIVE_EXPONENT` | Error | pow 的 exponent<0 |
| `X06-RUNTIME-009` | ArithmeticError（已有） | abs/gcd/pow 的不可表示结果、statistics 顺序累加溢出 |
| `HEX_INVALID_BYTE` | Error | encode 元素不在 0～255 |
| `HEX_INVALID_LENGTH` | Error | decode 的字符数为奇数 |
| `HEX_INVALID_CHARACTER` | Error | decode 非十六进制字符，含空白、前缀或非 ASCII |
| `HEX_LIMIT_EXCEEDED` | ResourceError | §四该模块任一大小上限被超过 |
| `BASE64_INVALID_BYTE` | Error | encode 元素不在 0～255 |
| `BASE64_INVALID_ENCODING` | Error | 解码长度、所选模式/字母表、补位位置/数量或尾位不规范 |
| `BASE64_LIMIT_EXCEEDED` | ResourceError | 输入或实际解码长度超过上限 |
| `URL_INVALID_ESCAPE` | Error | 不完整或非十六进制的 %HH |
| `URL_INVALID_UTF8` | Error | 转义还原的字节不构成规范 UTF-8 |
| `URL_LIMIT_EXCEEDED` | ResourceError | 输入或输出 UTF-8/编码字节数超限 |
| `CSV_INVALID_RECORD` | Error | 非法引号位置、未闭合引号、闭引号后多余文本、引号外 CR/LF |
| `CSV_EMPTY_FIELDS` | Error | format_record 的 fields 为 [] |
| `CSV_LIMIT_EXCEEDED` | ResourceError | 记录字节、字段数、单字段、原始总量或精确编码结果超限，阈值按已确认 §四 |
| `FMT_INVALID_WIDTH` | Error | padding 的 width<0 |
| `FMT_INVALID_FILL` | Error | padding 的 fill 不是一个 Unicode 码点 |
| `FMT_INVALID_RADIX` | Error | format_int 的 radix 不在 2～36 |
| `FMT_LIMIT_EXCEEDED` | ResourceError | text/width/结果大小超过上限 |
| `STATISTICS_EMPTY` | Error | min/max 输入为空 |
| `STATISTICS_LIMIT_EXCEEDED` | ResourceError | 统计数组超过 65,536 项 |

共同静态参数错误沿用 `X04-TYPE-002`；内部动态边界类型不符沿用 `X06-RUNTIME-002`，
不把类型错误混入上述内容错误。message 可本地化，不参与身份；不把输入全文拼入错误。
恢复方式：修正非法参数/编码，按预算缩小输入；算术溢出不能静默转为浮点或截断。

### 6.1 已确认检查优先级

1. 共同调用/类型检查；随后标量参数域（math 区间/指数，fmt width/fill/radix）；
2. 有界输入大小预检；CSV 空字段数组、statistics 空 min/max 在非超限空输入时处理；
3. 结构校验：hex 偶数长度、Base64 4 的倍数，再从左到右校验内容；URL 每个转义先查语法再查 UTF-8，
   CSV 按状态机位置判断语法与字段计数，不回头改变已经发现的首个错误；
4. 校验/构造中核对实际结果大小，在达到上限+1 时失败；无效内容与超限并存时依此固定顺序；
5. 已有算术异常只在算法真正执行到的运算出现；不要为未使用的平方/绝对值制造额外溢出。

上表与顺序已确认，实施批仍须完整反例验证；B0 的 B0_* 是探测数据，不能静默映射为产品 code。

## 七、联合冻结的证据清单

| 模块 | 必补的区分度/边界向量 |
| --- | --- |
| math | MIN/MAX、gcd(MIN,2/-1/0/MIN)、clamp 相等/倒置、0^0、负指数、MIN^1、2^63、极大指数的 0/1/-1 |
| hex | 全部 256 个字节、空输入、大小写、奇数、空白/0x/非 ASCII、长度上限及上限+1、输入别名不变 |
| base64 | 两种模式的 RFC 独立向量、任意字节、严格补位/尾位、交叉/混用模式与空白拒绝、真实解码长度超限 |
| url | ASCII/2/3/4 字节、0/7F/80/7FF/800/D7FF/E000/FFFF/10000/10FFFF、+、坏百分号、过长编码/代理/截断/孤立续字节、NUL/BOM、UTF-8 大小上限 |
| csv | 空记录/零字段/尾空字段，逗号/双引号/各类换行，空白/BOM，坏引号四状态，转义后大小恰好上限和超限，字段数/字段大小，合法往返 |
| fmt | 码点与字节/字素宽度区别，width=0/负数/等长/更小/上限+1，即使无填充也检查 fill，2/10/16/36 与非法进制，MIN 的所有进制 |
| statistics | 空数组、单项、重复、两端极值、[MAX,1,-1] 中途溢出、输入未重排、最大项数与+1 |

每项需要真实源码、独立黄金 VM/native、同一提交的 Windows/Linux/macOS 结果；共享同源 IR，
成功退出码/stdout/stderr、未捕获错误 code/类别/位置及清理证据分开登记。不能以同一算法的往返
代替独立标准向量；不能因为源码含候选名就认定已测试。

契约收口：F1～F13、E-2 和精确读取通路已确认，本轮只写文档/探测。进入纯库实现的
执行前置仍为独立 FIX-B 的真实能力、ABI 1.18 与版本拒绝证据，不能把文档冻结当作能力已实现。
资源/性能、公开导入、逐函数移除、真实归档与产品教程在 FIX-B/IMP/CLOSE 相应阶段验收，
不能在产品不存在时伪造完成，也不能用旧探测替代。全部通过才按 D-21 升级 verified。

## 八、本轮 CI 接线与验证

本节保留 B0/P3 的历史适用证据。819ec06 之后的 FIX-A 新边界与本地结果见
[本轮收口记录](20o-b1-fix-a-contract-closeout.md)，不能用下表旧七项的三平台绿色代替新探测。

### 8.1 两个精确执行点

文件：[maintenance-regression.yml](../../.github/workflows/maintenance-regression.yml)。

| 路径 | 精确目标 | 批量路径与计数规则 |
| --- | --- | --- |
| 默认 workspace 作业 | `b1_capabilities_have_vm_golden_outputs`，目标 20o_b1_probe | 默认全量跳过该名字，再独立执行 --exact；恰好一条结果汇总、1 passed/0 failed/0 ignored |
| Windows/Linux/macOS 原生作业 | `b1_capabilities_match_native_golden_outputs`，目标 20o_b1_probe | 三平台 ignored 批量均跳过该名字，再 --ignored --exact；同样校验恰好一条成功且未忽略 |

教程一致性、状态迁移回归与四份既有边界夹具仍由默认 workspace 的其余测试执行。只修改
`docs/UseDocs/library/**` 现在也触发 push/pull_request 维护回归；保留原有源码/夹具触发路径。
未新增作业、未改变 runner、release 矩阵或既有并发/发布测试清单。

计数守卫现已落在 [b1-probe-guards.ps1](../../tools/gates/b1-probe-guards.ps1)，由同一默认作业
执行；该脚本改动也触发维护回归。它直接提取工作流的实际判断代码，不保留一份测试专用判断。
运行方法见 [门禁 README](../../tools/gates/README.md)，脚本不依赖本机私有路径或 Python 包。

```powershell
# Linux/macOS：PowerShell 7 + PATH 中的 Bash
pwsh -NoProfile -File tools/gates/b1-probe-guards.ps1
# Windows：显式选择已安装的 MSYS Bash
pwsh -NoProfile -File tools/gates/b1-probe-guards.ps1 -BashPath D:/msys64/usr/bin/bash.exe
```

可用 `-ReportPath <已有目录中的文件>` 输出含工作流 SHA-256、宿主版本和 28 个逐例结果的 JSON。
计数脚本通过不等于其模拟的平台实际运行了产品；三平台运行号仍单独登记。

### 8.2 教程状态测试的生命周期规则

测试读取对应 `stdlib.<module>` 的 registry 状态，并要求教程 front matter 与之相同，
不再固定要求 planned。planned→verified 只需按既有验收流程同步 registry/页面，测试代码
无需为状态升级修改。另有 planned/verified、正文伪造状态及重复字段的回归。
这只消除测试与合法升级的冲突，**不授权本批把教程升级为 verified**；三平台及产品证据仍是硬门槛。

### 8.3 结果登记

| 验证 | 本轮证据 |
| --- | --- |
| 9df91bb 的“28/28”历史声明 | 当时只有本机临时脚本/日志，没有入库的复跑入口；仅算本机记录，不作为远端或可交接证据 |
| P3 后的可复跑计数守卫 | 仓库脚本已在本机复跑，四条路径各七项、28/28；[逐例本机报告](assets/20o-b1-p3/guards-windows.json) 绑定工作流/脚本摘要；只验证提取脚本语法和计数，不是 YAML 全量语义或真实三平台证据 |
| P3 教程状态回归 | 20o_b1_probe 默认 5 passed、0 failed、1 ignored；含 planned→verified、正文/重复字段防伪与 registry 一致性；当前页面/registry 仍为 planned |
| 默认 workspace（批量排除 VM 探测） | 本机 Windows 按 CI 同一命令运行：170 组，1520 passed、0 failed、49 ignored；日志 xiao-b1-default-workspace.log；不是 Linux runner 证据，原生 ignored 不计作执行 |
| VM 精确探测 | 本机 Windows/MSYS 运行从工作流提取的 Bash 入口，七模块黄金通过；1 passed/0 failed/0 ignored，日志 xiao-b1-vm-exact.log |
| Windows 原生精确探测 | 当前 Runtime release + vcvars64/显式 LLVM，执行从工作流提取的 PowerShell 入口；七项同源 VM/native 通过，1 passed/0 failed/0 ignored；日志 xiao-b1-native-exact.log |
| 教程完整代码一致性 | 七页源码嵌入一致；专用测试精确运行，1 passed/0 failed/0 ignored |
| docs/usedocs/layout、格式/Clippy/覆盖率/锁文件 | 全部通过；覆盖率 90.20%、公共 API 100%；workspace/benchmarks cargo check 通过，两个锁文件无差异；按 Git 必须经 cmd 的规则分项完成聚合门禁 |
| 提交范围 | 仅维护回归、探测测试说明、文档与登记；没有修改产品函数、intrinsic、ABI 或 19/11/22 文档/发布债务 |
| Linux/macOS 原生、远端三平台运行号 | **未取得**，不外推本地结果，不升级教程 |

P3 的教程状态/文档门禁和守卫脚本结果在本轮提交前复核；此前 workspace 与原生日志保持
其历史适用提交，不用新增状态回归之后的测试数回写历史。

### 8.4 P3 复核对账

| 反馈 | 本次处置 | 仍保留的限制 |
| --- | --- | --- |
| CSV 原始/编码上限矛盾 | P3-1 已确认原始预检加精确 E≤64 KiB；给出优先级和四组预算向量，统一超限 code | 文档裁定，尚无产品实现；其余数值本轮 F9 已确认，产品仍未实现 |
| error_code 非纯 Xiao | 页首和 F1/F13 明记硬阻塞，P3 时标为候选；本轮位置/通路按 §二冻结 | 此为 P3 历史状态；本轮契约已确认，FIX-B 实现仍是前置 |
| 28/28 无复跑入口 | 仓库脚本、README、逐例本机报告均入库；维护回归配置执行并监视脚本路径 | 28/28 只证明模拟计数守卫；真实三平台与远端运行号未取得 |
| 测试固定 planned | 改为与 registry 状态相同；添加 verified 与元数据防伪回归 | 当前教程仍为 planned；升级所需产品/三平台证据门槛未变 |
| 20N 旧建议冲突 | 页首标历史背景并列出新旧裁定对照；20O 阅读清单和阶段索引同步 | 旧正文只保留研究语境，不再作为实施或重新询问已确认方向的依据 |

本次本地验证：默认探测目标 5 passed/0 failed/1 ignored、计数守卫 28/28、工作流 YAML 与
执行/触发路径、报告摘要匹配、fmt、全目标 Clippy、docs/usedocs/layout 均通过。

## 九、交接

先读 [20O](20o-official-library-program.md)、[20P](20p-day1-library-architecture.md)、
[B0](20o-b0-baseline-and-b1-draft.md)，再读本页与 [FIX-A 记录](20o-b1-fix-a-contract-closeout.md)。
D-23～D-26、P3-1、F1～F13/E-2 均已有实际答复，不重问同一裁定；新增需求或契约变更再问。
本轮不自动开始 FIX-B：其编号/符号/版本已在文档冻结，真实产品代码仍是 ABI 1.17。
FIX-B 的真实前置完成后，再按 [实现规划](20o-b1-implementation-plan.md) 进入纯库与证据收口；
不认领 19/11/22 发布债务。
