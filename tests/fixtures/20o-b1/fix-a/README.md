# B1-FIX-A 边界探测

工程期：20O-B1-FIX-A；这里是探测入口，不是可导入的官方库源码。

- math-extremes：负域 gcd、MIN/MAX、0^0、高指数和不计算多余平方。
- hex-all-bytes：全部 256 字节的独立完整黄金、大小写与坏长度/字符。
- base64-tail-bits：标准/URL-safe 的必填模式、四字符分组、规范尾位与混用拒绝。
- utf8-boundaries：Unicode 0～10FFFF 临界、非法/非最短/代理/截断字节与百分号对。
- csv-budget：短记录 UTF-8/转义计数，加精确 E 的 64 KiB 边界算术；不是全量扫描性能。
- fmt-int-min：MIN 的 2～36 全进制，不先取正绝对值。
- statistics-overflow：中途溢出与失败后输入别名不变。

golden.json 保存独立固定 stdout，不从 Xiao 探测结果自动生成。全部七份由既有 VM/native
精确测试执行，与 B0 七份合计 14 个场景；每条精确 Rust 测试仍须恰好一项通过。
本组看门狗为 60 秒，以容纳多个边界调用；不是单次产品 API 的性能预算。

局部 probe 函数可以分解探测步骤，但它们不登记官方导出；产品辅助逻辑遵守星崽已确认的
内联策略。完整记录见 [FIX-A 收口](../../../../docs/DevDocs/20o-b1-fix-a-contract-closeout.md)。
