# 11A-E3A 离线包源规格

`valid.json` 包含真实 `config.xiao` 文本、规范身份、别名、导入钉住值和完整展开顺序；`errors.json` 包含白名单、缺字段、协议版本和别名冲突反例。由 `core/rust/crates/xiao-package/tests/e3a_source.rs` 实际加载和逐例执行。不会访问网络。
