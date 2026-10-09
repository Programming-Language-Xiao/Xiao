# `xiao-package`

## 目录职责

提供多包源协议、联邦源索引、依赖求解、来源优先级、锁文件和环境物化的核心逻辑。显式源优先，其次按配置顺序，无法唯一确定时报告歧义。

D1 首批只实现 `config.xiao` 本地路径依赖的静态读取和包身份图；E1 在此图上导入不可变
源码对象并保存环境逻辑映射；E2A 使用 E0 配置指纹、D1 完整图和 E1 对象摘要生成可审计的
`xiao.lock.json`，并支持既有环境映射原子更新。
11A-E2B 的 `src/sync.rs` 则编排三档环境目标、锁模式、本地缓存和安装映射。图节点独立于
`xiao-modules::ModuleGraph`，使用 `source_id`、`alias`、展示名三个分开的来源字段，
并按依赖优先顺序生成内存解析计划；版本约束只作为结构化输入保留。
E3A 新增静态多源声明、钉住摘要的独立源列表、JCS 规范字节、联邦记录与跨源优先级，
本地目录源通过相同的适配器接口分离读取索引和正文。E3B 增加离线快速解析、
有界并行读源与四类独立缓存，联邦视图按配置与源顺序独立存储，并为锁文件钉住快照；
E3C 用同步 HTTP 与 Git 稀疏索引接入现有解析器，远程求解与 CLI 接线归 E3D。
E3D1 将直接与传递版本需求送入联邦求解器；远程源码 TAR 先按锁定字节摘要校验，
再安全展开并导入 E1 源码目录对象。`lock_version: 2` 同时记录 TAR 引用、目录摘要
和源快照，保留读取 v1 本地锁文件；环境映射复用同一包图。Bearer 凭据只在
请求阶段读取且不出现在诊断与 `Debug` 中，索引缓存命中时不会重新联网。

## 工程期

11A；16 接入内容寻址缓存；18 由 TypeScript CLI 调用管理命令。

## 模块放置

`src/model.rs` 放置包身份、来源和图模型，`src/resolver.rs` 读取本地包配置并解析图，
`src/environment.rs` 消费规范化配置文档并生成环境布局、指纹、v2 元数据和环境物化，
`src/cache.rs` 放置 `XIAO_HOME` 布局、SHA-256 源码对象、只读校验与损坏隔离，
`src/mapping.rs` 放置包身份到对象引用的确定性逻辑映射，`src/lockfile.rs` 保存完整图、
稳定诊断与跨平台原子文件替换，`src/sync.rs` 编排已有接口，`src/diagnostics.rs` 放置稳定编号。
`src/source.rs` 处理规范源身份与有序配置，`src/jcs.rs` 处理规范 JSON，
`src/federation.rs` 和 `src/selection.rs` 处理联邦记录与优先级，`src/adapters.rs` 放置
元数据/正文分离接口和离线目录源。`src/snapshot_store.rs` 保存不可变快照及当前指针，
`src/federation_cache.rs` 保存联邦索引与内容寻址元数据，`src/fastpath.rs` 编排离线命中、
三态报告与确定性并行读取，`src/entry_lock.rs` 提供跨进程条目锁。
`src/adapters/http_static.rs` 是无重定向、带 TLS 与 Range 校验的静态传输，
`src/adapters/git_refs.rs` 校验 pkt-line，`src/adapters/github.rs` 将引用解析成不可变提交，
`src/adapters/multi.rs` 按源种类统一分派；版本求解归 E3D，命令参数和提示符放在 CLI。
`src/remote.rs` 负责完整图合并与锁定复用，`src/fetch.rs` 安全展开 TAR，
`src/credentials.rs` 管理权限检查和脱敏令牌，`src/source_lists.rs` 保存钉摘要源列表。

## 禁止事项

不执行包安装脚本或包代码，不绘制提示符，不把不同源的版本简单按高低混选；E2A 不实现
远程下载、缓存清理或 CLI 命令；E2B 只编排本地依赖的 `sync` 与 `install`/`i`。
E3B 的本地目录源测试不验证 HTTP Range；E3C 的本机服务测真实 206 范围续传。
E3C 仅提供联网适配器；E3D1 才接通远程索引依赖的锁定、下载与安装。
