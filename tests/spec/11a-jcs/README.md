# 11A-E3A JSON 规范化向量

`vectors.json` 为 Rust 与后续 TypeScript 共用的 UTF-8/JCS 测试向量；成功条目提供原始 JSON、规范 UTF-8 文本及规范字节的 SHA-256，失败条目提供原始 JSON、空期望字节/摘要和诊断编号。Rust 执行入口为 `core/rust/crates/xiao-package/tests/e3a_source.rs` 的 `jcs_shared_vectors`（`include_str!` 实际加载）。
