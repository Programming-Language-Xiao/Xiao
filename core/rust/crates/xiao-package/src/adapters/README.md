# `xiao-package/src/adapters`

本目录放置 11A-E3C 的同步联网源适配器，不执行任何包代码。

工程期：11A-E3C；远程求解与安装归 11A-E3D。

- `http_static.rs`：受限 HTTP(S) 静态索引与校验后的 Range 续传。
- `git_refs.rs`：smart HTTP v1 的 pkt-line/不可变提交解析。
- `github.rs`：GitHub raw 内容的稀疏查询、标签防改写与快照钉住。
- `multi.rs`：同一 `SourceResolver` 内按源种类分派。

所有解析与摘要语义由父模块统一提供；远程版本求解与包安装归 E3D。
