//! B1-IMP 产品清单，由契约测试及 VM/native 精确入口共同消费。

/// 已落地产品的冻结导出；新增源码后由独立的 20 入口契约表核对。
pub const PRODUCTS: &[(&str, &[&str])] = &[
    ("math", &["abs", "min", "max", "clamp", "gcd", "pow"]),
    ("statistics", &["sum", "min", "max"]),
    ("fmt", &["pad_left", "pad_right", "format_int"]),
];

/// 辅助回归不计入产品模块或公开入口数。
const AUXILIARY_CASES: &[&str] = &["aliases", "string-index"];

/// 精确入口始终执行全部活跃产品及辅助样例，不另维护一份产品名称数组。
pub fn case_names() -> impl Iterator<Item = &'static str> {
    PRODUCTS
        .iter()
        .map(|&(module, _)| module)
        .chain(AUXILIARY_CASES.iter().copied())
}
