# 20O-B1-FIX-A. 契约收口与新增边界探测

> **本批只写文档/探测。** 开工基线 819ec06，日期 2026-11-01，工作区开工时干净。
> F1～F13/E-2 已在本次会话逐条询问并收到星崽答复；没有默认选择或代为延期。
> 精确 API、错误/限额/性能与 error_code 契约的唯一正文仍是
> [B1 契约](20o-b1-freeze-draft.md)。FIX-A 取证时 ABI 为 1.17，七库产品尚未实现。
> 后续独立 FIX-B 已实现 ABI 1.18；当前状态见 [FIX-B 收口](20o-b1-fix-b-closeout.md)，
> 本页只保留 FIX-A 历史证据。
> **远端证据已补齐**：提交 4d347670cdfa05fac2f01888f9d5ff6e3a5db37e 的 maintenance
> [41285853958](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41285853958)
> 与工作区门禁 [41285853953](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41285853953)
> 全绿；默认 VM 与三平台原生的新增七项均实际执行，逐作业核对见 §3.1。此结果仅关闭探测取证。

## 一、裁定与范围

| 事项 | 本轮实际裁定 |
| --- | --- |
| F1 | 全局 error_code(Error value, /) -> str，借读、不消费；Error 仅为入口参数类别；其他访问器后置 |
| F13 | 独立 FIX-B 新增 intrinsic 规划号 32 与 ABI 1.18 只读 C getter；借用字节视图后复制为拥有的 str |
| F13 精确补充 | xiao_runtime_error_code(const XiaoValue*, XiaoAbiBytes*)；0/1/2/3 状态、失败 out 不变、裸读取不触碰挂起槽与源级错误表均已确认 |
| F13 类别勘误 | 已核现有分配失败 011 的类别为 ResourceError；星崽确认沿用，修正旧草案的 MemoryError 写法，Fatal 003 与句柄 MemoryError 不变 |
| E-2 | B1 辅助逻辑内联，只有冻结 20 个入口；私有可见性另立语言批次 |
| F4 | 用户明确要求首批加入 URL-safe；两个入口增加必填 bool url_safe，仍共 20 个公开入口 |
| F2～F12 其余项 | math/hex/url/csv/fmt/statistics 边界、全部资源数值、完整错误表/顺序、共同调用与性能验收目标已分别确认 |

此轮不增加 intrinsic 声明、C ABI 声明/符号、诊断常量、产品函数或可导入内部模块。
“文档规划号已确认”不能写成 id 已在运行时激活；旧 Runtime 尚不提供 error_code。

### 1.1 helper 与导出面

E-2 采用内联：新增辅助步骤写在公开函数体中，允许调用已交付的 text 等既有接口，
不在官方模块新增辅助顶层函数，不靠下划线、缺少注解、嵌套函数或 stdlib._b1 规避导出。
不增加 private 语法、保留内部命名空间或新的模块可见性规则。

| 模块 | 唯一新增公开名称 |
| --- | --- |
| math | abs、min、max、clamp、gcd、pow |
| hex | encode、decode |
| base64 | encode、decode；两者的 url_safe 必填 |
| url | encode_component、decode_component |
| csv | parse_record、format_record |
| fmt | pad_left、pad_right、format_int |
| statistics | sum、min、max |

FIX-A 夹具中的 probe 函数不是官方模块导出，不受“产品不能新增 helper 导出”的误读限制。
IMP 阶段必须从真实模块签名表和用户导入验证上表精确集合，不能只靠 README 计数。

## 二、新增真实源码

执行入口仍为 [20o_b1_probe.rs](../../core/rust/crates/xiao-driver/tests/20o_b1_probe.rs)，
源码与独立固定黄金位于 [fix-a 目录](../../tests/fixtures/20o-b1/fix-a/README.md)。
没有新建第三条无人执行的原生 ignored 测试；新增场景进入已有两条精确执行点。

| 场景 | 实测范围 | 明确未外推的能力 |
| --- | --- | --- |
| math-extremes | gcd(MIN,2/-1/0/MIN)、0^0、极大指数的 0/1/-1、MIN^1、2^62/2^63、abs MIN、倒置 clamp | 不是官方模块导入、移除或资源性能验收 |
| hex-all-bytes | 256 字节完整独立黄金、校验和、大小写、坏长度/字符、256 拒绝 | 未实现完整数组解码与所有资源上限 |
| base64-tail-bits | 两种字母表、混用/交叉模式拒绝、单/双补位的非零尾位、坏长度与空白 | 仅四字符分组判据与编码原语，不冒充完整多组编解码器 |
| utf8-boundaries | 0/7F/80/7FF/800/D7FF/E000/FFFF/10000/10FFFF、孤立/非最短/代理/截断/超范围字节、坏百分号对 | 未实现完整组件编码器及大输入资源策略 |
| csv-budget | 短记录 UTF-8 字节/引号/换行计数、E=65536/65537、全引号/多换行膨胀、零字段拒绝 | 大规模用计数驱动算术判定，不是全量记录扫描或解析性能证据 |
| fmt-int-min | MIN 的 2～36 全进制独立黄金，零、非法进制 | 仍须 IMP 验证正式 padding 与完整资源策略 |
| statistics-overflow | MAX,1,-1 中间溢出，失败后别名三项不变，边界成功与空 sum | 不外推为所有统计资源/生命周期已验收 |

原 B0 七份教程完整示例继续运行，合计 **14 场景**。两路使用同源 IR，各自逐字比较固定
黄金 stdout；原生还要求退出码 0、stderr 为空。新增组有 60 秒看门狗以容纳多次调用，
不是放宽单次产品的 VM≤10s/原生≤2s 目标。

### 2.1 黄金与错误证据

golden.json 的整数/进制来自独立宿主精确格式，UTF-8 是固定临界码点及人工非法字节，
Base64 是模式/尾位固定向量，CSV 是已确认 E 的数学预算。黄金不从 Xiao 运行结果更新，
也不只用同一算法往返来证明正确。

catch 中的文本只是分支标签，不是从错误对象读取的 code。既有独立 VM 错误身份夹具继续
运行；新增场景不冒充全部未捕获错误、错误字段与清理的原生差分。error_code 本体及其
类型/ABI/挂起槽保护测试属于 FIX-B，不能在此伪造通过。

### 2.2 CSV 全量扫描超时的保留记录

开发中初版对四组实际大字段逐字符计数的局部 VM 探测在 60 秒看门狗下返回
X09-DRIVER-002；这是**本机开发观察**，不是已冻结产品 API 的性能测量，未保留该临时版本
为可复跑验收材料。本轮没有通过延长预算或缩小公开上限消除这条限制。

当前将短记录计数与大规模 E 算术边界分开：64 KiB/65537、引号/换行膨胀的预算判定已测，
但完整大输入扫描性能仍未证明。F10 的验收数字保持星崽确认值；IMP 必须补全量性能/内存
证据，未达标时优化或再提契约变更，不能拿此算术探测宣称达标。

## 三、本地与远端证据

| 层级 | 本轮状态 |
| --- | --- |
| VM 精确目标 | 七旧+七新场景通过，恰好 1 passed/0 failed/0 ignored；[本机日志](assets/20o-b1-fix-a/vm-windows.log) |
| Windows 原生精确目标 | vcvars64 + 显式 LLVM + 当前 Runtime staticlib；14 场景通过，恰好 1 passed/0 failed/0 ignored；[本机日志](assets/20o-b1-fix-a/native-windows.log) |
| 新增组独立黄金 | 七项齐全，缺项/多项或 stdout 不同均失败 |
| 全量 workspace | 170 组，1522 passed、0 failed、49 ignored；环境门控不算执行 |
| fmt/Clippy/文档 | fmt、全目标 Clippy -D warnings、docs/usedocs/layout 全部通过 |
| 覆盖率/锁文件 | 文档覆盖率 90.20%、公共 API 100%；workspace/benchmarks cargo check 与两个锁文件无差异检查通过 |
| 计数守卫 | 仓库脚本四条路径合计 28/28，本机复跑；不冒充真实三平台执行 |
| ad176c3 的三平台运行 | maintenance 41275847931 与 gates 41275847899，适用旧七项，不含本轮新增夹具 |
| 本轮新边界的三平台远端 | 已取得：4d34767 的 maintenance 41285853958，默认 VM 和 Windows/Linux/macOS 原生均含七旧+七新场景，各恰好 1 passed |

本轮同名精确目标保留 --exact 及一项通过断言；14 场景是这一个 Rust 测试内部的场景数，
不能把 1 passed 写成仅跑一份夹具，也不能把 14 个场景写成 14 条 Rust 测试。

### 3.1 同一提交的远端逐作业证据

已从 GitHub Actions 的 run/jobs/logs 接口核对 head SHA、attempt 1、作业 conclusion 及实际
测试输出；maintenance 六个作业均为 success，workspace gates 同提交 success。

| 路径 | 实际作业 | 日志核对 |
| --- | --- | --- |
| 默认 VM（Linux） | [124786889391](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41285853958/job/124786889391) | b1_capabilities_have_vm_golden_outputs；七旧+七新；1 passed/0 failed/0 ignored |
| Windows 原生 | [124786889338](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41285853958/job/124786889338) | b1_capabilities_match_native_golden_outputs；14 个同源 VM/native 场景；1 passed/0 failed/0 ignored |
| Linux 原生 | [124786889309](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41285853958/job/124786889309) | 同上，实际新增七项逐条 success |
| macOS 原生 | [124786889367](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41285853958/job/124786889367) | 同上，实际新增七项逐条 success |

[证据清单与摘录](assets/20o-b1-fix-a/maintenance-41285853958/README.md) 已入库：保留每个目标从
Rust 测试启动到对应汇总的原始时间戳、源日志行号和摘要，避免仅靠工作流结论推断测试执行。
CSV 的通过项仍是预算算术/短记录计数，不关闭 §2.2 的全量扫描性能限制；也未执行尚不存在的
error_code、ABI 1.18 或七库产品。七库教程与七库 registry 条目保持 planned。

## 四、FIX-A 与下一批的交接

| FIX-A 退出项 | 对账 |
| --- | --- |
| F1～F13 逐条答复或明确延期 | 全部已直接确认，无代为选择或延期 |
| error_code 名称/位置/所有权/错误/通路 | 冻结于 B1 §二；id 32、ABI 1.18、借用后复制和失败边界明确 |
| 20 入口、限额、错误、性能目标 | 冻结于 B1 §三～§六；Base64 模式扩展已同步，数量仍为 20 |
| helper 导出策略 | 星崽确认内联；精确导出集合见 §1.1，私有可见性独立后置 |
| 补 B0 漏测边界 | 新增七项本地和 41285853958 的三平台探测通过；完整资源/性能与产品证据仍未完成 |
| 没有产品函数 | 本批只有文档/测试/夹具；不激活 intrinsic 32，不改实际 ABI 1.17 |

当前已完成契约收口和新增边界的本地/三平台探测取证。下一批仍是**独立 FIX-B**，不是
将 error_code 偷放进七库：类型/IR/VM/LLVM/Runtime 与 ABI 1.18、版本拒绝、挂起槽不变、
借用视图生命周期和释放证据必须一起交付。然后才按 math→hex→base64→url→fmt→statistics→csv
做 IMP，再做 CLOSE 的产品三平台、归档/移除和教程升级。七库教程与七库 registry 条目在 FIX-A 时点继续 planned。
