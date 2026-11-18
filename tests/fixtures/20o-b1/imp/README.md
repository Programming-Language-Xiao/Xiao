# B1-IMP 产品夹具

工程期：20O-B1-IMP。目录职责：保存从公开 `std.`/`stdlib.` 路径导入的七库产品示例和独立黄金。
每个模块的 product.xiao 与 golden.json 成对登记；B0 与 FIX-A/FIX-B 夹具保持各自职责。

产品契约、真实导出和移除/归档回归在 `20o_b1_imp_contract.rs`，精确 VM/native 入口复用
`20o_b1_probe.rs`。教程状态保持 planned；三平台与最终 CLOSE 证据齐备后才升级。

当前产品为 math/statistics/fmt，共 12/20 项；aliases 核对已落地同名函数，string-index
核对前置读取语义。其余四库及 hex/base64 同名入口仍待加入，不把辅助回归计作产品模块。
