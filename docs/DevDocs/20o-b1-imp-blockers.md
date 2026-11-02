# B1-IMP 返工结果与剩余阻塞

> 本页保留首批 math/statistics/fmt 三模块、12/20 入口的返工历史。
> 当前 REM 已有七模块 20/20 产品草稿及本机功能/资源回归，见 [REM 进度](20o-b1-imp-remainder.md)。
> 生成器20已关闭六组已知释放差异，完整F10四报告721条与三平台功能/释放均已取证；REM退出对账完成。七库教程和七库registry条目保持planned，B1-CLOSE独立后续。

首批按六项反馈返工，没有撤掉 F12 命名调用、缩小输入限额或放宽性能预算。
完整本机日志、机器信息与源摘要见 [返工证据](assets/20o-b1-imp/review-local-20261102/README.md)。
[旧诊断目录](assets/20o-b1-imp/local-20261102/README.md) 保留原始失败，属于历史状态。

同代码的 [维护回归 41372968883](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41372968883)
六个作业全绿，[工作区门禁 41372968916](https://github.com/Programming-Language-Xiao/Xiao/actions/runs/41372968916)
通过。三模块与辅助回归已有真实 VM/三平台精确证据，逐作业与源日志摘要见
[CI 证据目录](assets/20o-b1-imp/maintenance-41372968883/README.md)；这不代表七库验收完成。

## 一、独立前置

| 提交 | 修复 | 边界 |
| --- | --- | --- |
| `4011060f` | std 别名的 VM/LLVM/归档身份 | 已推送；维护回归 41331047226、门禁 41331047202 通过 |
| `8bd9205b` | 原生固定参数命名降低 | 源码顺序求值、形参顺序绑定、后续实参失败清理；生成器 17 |
| `5a8ab776` | 字符串码点索引及精确负下标 | ASCII 不分配索引，Unicode 共享起点表；生成器 18 |
| `35becfb1` | 同名模块绑定及 std 静态签名 | 无别名导入、模块内调用和真实移除拒绝；生成器 19 |

本轮前置说明见 [命名调用前置](20o-b1-native-named-prerequisite.md) 与
[字符串成本前置](20o-b1-string-index-prerequisite.md)。Runtime ABI 保持 1.18，
不新增 intrinsic、语法、公开 dynamic 或 helper。

原生最初在 `abs(value = ...)` 拒绝。完整复测又暴露 `statistics.min` 错绑到 `math.min`；
逐形参负例则暴露公共 `std.` 路径缺少静态接口，接受了 `abs("bad")`。这些缺口均已修复，
保留触发用例，不以规范路径或位置调用替换公共路径/命名调用。

## 二、原生证据和契约覆盖

此前“位置调用原生 O0/O3 已通过”没有可复核的成功日志，已撤回。
新日志执行完整产品 fixture，包含位置及命名调用；math/statistics/fmt、同名别名和
Unicode 下标均实际经过 O0/O3。目标要求恰好 `1 passed; 0 failed; 0 ignored`，
逐字核对 stdout、退出码和空 stderr。以返工目录的最终成功日志为依据，不再引用早期口头记录。

三模块都有真实导出集合、逐函数移除、真实 .xar 重放、普通包隔离、公开双路径及教程一致性。
新增独立的 20 入口参数表：当前对真实落地的 12 项执行两条路径下的逐形参错误类型、缺参、
重复参数、未知名称、过多参数、未导入拒绝，以及合法逆序命名降低。
另核对源码、registry 和活跃矩阵一致，新增模块后不能继续只测旧模块而静默通过。

**本节首批证据不是 20/20 产品证据。** 后续 REM 已实现 hex/base64/url/csv 八项，补齐
调用/资源负例与 hex/base64 encode/decode 同名碰撞；本轮三平台证据与 F10 另行记录。

## 三、字符串成本与 fmt 最大输出

旧 Runtime 的 len 使用 chars().count()，下标/原生迭代使用 chars().nth(index)，导致
反复扫描前缀；旧 debug 初测超过 60 秒看门狗，release 1.459 秒只能算旧输入扫描的局部成绩。
现在不可变字符串构造时建立索引，len/单码点读取为 O(1)，构造为 O(n)。
Unicode 索引容量计入 Runtime 对象拥有内存，克隆共享载荷，不改变 UTF-8 或负下标语义。

性能测试继续默认执行，未加 ignore；同一测试二进制用局部互斥串行测量，工作区的 jobs=4、
incremental=false 和 RUST_TEST_THREADS=4 不变。预热一次，五次中位数，编译、初始化、
输入生成、打印与黄金比较在外；未改 10 秒和翻倍比≤3 的断言。

| 案例 | 输入/输出 UTF-8 字节 | 本机 debug 五次中位数 |
| --- | --- | --- |
| 原文扫描 pad_left(text, 0, ".") | 65,536 / 65,536 | 3.8304936 秒 |
| 左填充最大输出 | 65,536 / 262,144 | 1.268827 秒 |
| 右填充最大输出 | 65,536 / 262,144 | 1.2552479 秒 |

最大输出使用 16,384 个 🚀 作为原文、😀 作为 fill、width=65,536，真实构造 49,152 个
填充字符。左右结果由独立预期串逐字比较。Unicode 填充翻倍比为 1.8252 / 1.8650，
ASCII 扫描为 2.6792 / 1.9977；完整样本保存在日志，release 另记。

当 width≤text_points 时返回原文，结果≤65,536 字节；当 width>text_points 时，
结果字节数=text_bytes+fill_bytes*(width-text_points)≤4*width≤262,144。
因此合法 text/fill/width 无法单独越过结果上限。保留产品的结果预算检查，
测恰好上限与 text/width 上限+1、fill 校验先于资源拒绝；不伪造“合法输入进入结果超限”。
Runtime 对象峰值含 Unicode 索引及输出，最大观测约 1.90 MB；它不包含全部 VM 分配或
进程 RSS，不能替代完整 F10 的 64 MiB 峰值增量内存证据。

## 四、额外释放差异与后续门槛

扩展 Unicode 诊断时，完整下标样例虽然 VM/native stdout 一致，20C 销毁计数为
native 14 / VM 16；去掉越界部分仍为 native 12 / VM 13。原生轨迹中，完整样例的
输入文本和末次循环字符有 strong_release、没有 destroy；纯成功样例剩下末次循环字符。
这属于实际未收口的释放差异，不能因为语义精确目标通过而声称生命周期验收通过。
REM 已用显式工具复现并定位普通 for 正常出口与 selector 失败边的引用遗漏，
星崽已授权独立生成器20前置，本机与三平台六组同源销毁顺序均已匹配；详见 [释放诊断](20o-b1-rem-release-prerequisite.md)。

完整样例保存在 tests/fixtures/20o-b1/imp/string-index/，VM/native 精确目标仍执行所有
正负下标、越界、最小 int 和 Unicode 码点。原 20C 释放断言及原有场景保持原样；
未把新增释放失败算作通过，也没有扩大修改 19/11/22 发布债务。相关 trace 和本机记录入库，
后续REM已补齐这一已知缺口的三平台释放证据，B1-CLOSE复核记录并独立作状态升级。

当前REM门槛已由41433194454第二次尝试 / 41433190291收齐，见 [最终证据](20o-b1-rem-evidence.md)。
后续是独立B1-CLOSE；四库、编码别名碰撞、已知释放修复及完整F10均已取证，七库教程保持planned，
error_code 的独立 verified 状态不变。

## 五、复跑

```text
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_imp_contract -- --nocapture
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_imp_vm_performance -- --nocapture
cargo test --manifest-path core/rust/Cargo.toml --release -p xiao-driver --test 20o_b1_imp_vm_performance -- --nocapture --test-threads=1
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- b1_capabilities_have_vm_golden_outputs --exact --nocapture
cargo test --manifest-path core/rust/Cargo.toml -p xiao-driver --test 20o_b1_probe -- --ignored b1_capabilities_match_native_golden_outputs --exact --nocapture
```

原生命令需按 10D 配置受控 LLVM、MSVC/平台工具链和真实 Runtime staticlib；旧 B0/FIX-A/FIX-B
场景及四路径 28/28 计数守卫保留。本机通过不冒充本轮三平台产品验收。
