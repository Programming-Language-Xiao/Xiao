---
id: tooling.cli.package-sources
title: 包源声明与离线索引
status: verified
audience: learner
module: rust.xiao-package
stage: "11A-E3C/11A-E3D1"
version: "0.1.0"
related:
  - README.md
  - ../config/syntax.md
  - package-lockfile.md
---

# 包源声明与远程索引

11A-E3C 在 E3A 的离线契约和 E3B 的缓存上增加同步 HTTP 与 GitHub 适配器；
E3D1 将联邦求解接入 `sync`，远程源码正文校验后进入 E1 目录缓存。
单独声明 `[sources]` 不会安装包；需在 `[dependencies]` 中声明版本需求。

## 多源声明

```xiao
[sources]
primary = { kind = "static", location = "https://example.org/index", display = "主源" }
mirror = { kind = "git-index", location = "https://github.com/team/index.git", alias = "team" }
release = { kind = "git-index", location = "https://github.com/team/index.git", tag = "v1" }
extra = { list = "https://example.org/sources.json", digest = "<源列表 JSON 规范化后的 64 位小写 SHA-256>" }

[dependencies]
utils = { path = "../utils", source = "team" }
```

每个直接源允许 `kind`、`location`、`alias`、`display`、`protocol`；前两者必填，
`alias` 默认取条目名，`protocol` 默认 1。支持 `path`、`registry`、`static`、
`git-index`；静态源使用 `http://`/`https://`。早期协议接受 `ssh://` 源身份，
当前联网适配器**不支持 SSH 传输**，须使用 HTTP(S)。
本地 `path` 源需给出不含 `.`/`..` 点段的绝对目录路径；相对路径缺少项目根上下文，
不能安全地生成跨项目稳定源身份。
`source` 绑定只能填 alias 或规范化后的 `source_id`，不能填显示名。
`source_id` 含来源种类、协议、规范主机和路径；修改 alias/显示名不会改变源身份。

导入项只能有 `list` 和 `digest`。源列表是一份独立 JSON，至少包含
`protocol_version: 1` 与有序 `sources` 数组，每项含 `kind`、`location`，
可带 `alias`、`display`、`protocol_version`；Git 索引项还可带唯一的
`rev`、`tag` 或 `branch`。先处理全部直接源（按书写顺序），
再按导入项书写顺序及各列表内部顺序展开；清单摘要不匹配、别名冲突即失败，
不会静默改写配置。本批不支持保留注释的结构化写回；它属于 E3D。

## 本地索引结构

对 `path` 源，本地目录可提供 `snapshot.json`，其中 `protocol_version` 固定为 1，
`source_id` 与配置一致，`snapshot_id` 标识快照，`shards` 把规范包名映射到各分片的
JCS SHA-256；可选 `mirrors`、`expires_at`、`signature` 仅保留元数据，本批不做信任或过期判定。每个 `index/<包名>.json` 分片携带同一个协议版本、源身份和快照标识，
`packages` 提供该包的全部版本元数据（版本、撤回状态、目标/ABI/兼容性条件、直接依赖、
源码对象引用和可为空的预编译对象引用）。正文引用由相对路径、字节长度与摘要组成，
只有需要包正文时才单独读取并验证。索引解析不会执行项目或包代码。

源清单、分片和源列表摘要都针对 UTF-8/JCS 规范字节计算，不针对原始空白或键顺序计算。
源不存在、版本不识别、分片与清单不符、路径逃逸、正文摘要不匹配都会报错；
不可用源不能伪装为空索引。

## HTTP 与 Git 稀疏索引

静态源把基址与 `snapshot.json`、`index/<包名>.json` 或正文相对路径拼接；
会拒绝 `..`、编码路径、查询/片段及含用户信息的地址。重定向**不跟随**，
无论是否跨主机；3xx、4xx/5xx 和真正的连接、TLS、超时或截断错误作为
`X05-SOURCE-003` 上报。正文在内存中接续 `Range: bytes=<偏移>-`，只接受匹配
长度与起点的 206 响应，完成后校验长度和 SHA-256；中断的字节不写入缓存。

`git-index` 必须是遵守相同分片规范的仓库，不扫描任意项目。通过标准 Git smart HTTP
`info/refs?service=git-upload-pack` 获取分支/标签的提交号，然后从 raw 服务的
`<commit>/snapshot.json` 和对应分片读取；默认发现 HEAD，也可在源声明中选择
唯一的 `branch`、`tag` 或 `rev`。`rev` 是完整小写 Git 哈希时直接固定提交，
否则作为标签名。Git 快照的 `snapshot_id` 必须等于该完整提交哈希；
锁文件 `source_snapshots` 钉住提交与清单摘要。在线解析已锁定标签时发现指向
新提交，或相同提交的清单规范内容变了，都会拒绝。锁定包的正文即使缓存命中，
同步时也会在线复核已锁定的 Git 标签；无法复核时不会静默改用旧缓存。
解析器的显式离线模式只复用已经验证的旧缓存。
仓库中的脚本不会在元数据读取或正文下载时执行。

`[dependencies]`/`[devdependencies]` 也允许独立的
`{ git = "https://github.com/team/lib.git", rev = "v1" }`；必须显式提供且仅提供
`rev`/`tag`/`branch` 一项，不得与 `path`/`source` 并用。`version` 仍只保存为约束文本。
独立的 `{ git = "...", tag = "..." }` 单仓库直接依赖仍会明确失败；
E3D1 闭环针对 `[sources]` 中的 `git-index` 与其他索引源，不擅自将两种声明合并。

## 选择与诊断

显式 `source` 只考虑绑定的源；否则按最终配置顺序逐源尝试。前面的源不可用且没有
可验证缓存时必须报错，不可跳到后面的源。选到一个有满足条件候选的源后，
不跨源比较版本；源内版本求解归 E3D。
SemVer 求解器按 `1` 等于 `1.*`、`1.2` 等于 `1.2.*` 解释，
同源内按版本优先级回溯；`sync` 首次解析及显式 `update` 使用同一入口，
普通 `sync`/`install` 有锁时复用已锁定候选，不隐式升级。
在缺少目标、特性和工具链兼容性上下文时，条件候选暂不参与内部求解；不能将其
当成通用包安装。
无可行候选时诊断会说明条件候选为何被保守排除；这些兼容维度仍待独立冻结。
`X05-SOURCE-001` 表示未知协议版本，
`002` 为别名冲突，`003` 为源不可用，`004` 为摘要不匹配，`005` 为未知源引用，
`006` 为歧义，`007` 为源描述或索引格式错误；`008` 为缓存或锁的本地基础设施故障，
`009` 为缓存对象损坏，`010` 为快照归属不匹配。三种缓存错误不会伪装为源不可用。

## 正文与凭据

远程源码正文必须是无压缩 TAR，仅展开普通文件及目录；拒绝链接、设备、越界或
平台不安全路径，并限制条目数及展开大小。正文还必须包含可解析的 `config.xiao`，
其中的包名和版本须与索引声明一致；身份不符报 `X05-TRUST-001`。
不运行归档中的脚本。
源列表按声明的 JCS 摘要缓存；缓存完整命中时无需重取源列表或索引。

源别名 `team` 的令牌优先从 `XIAO_SOURCE_TOKEN_TEAM` 读取；缺失时查找
`~/.xiao/credentials`，格式为 JSON 对象（如 `{"team":"令牌"}`）。
环境变量令牌仅适用于小写 ASCII 字母、数字和下划线组成的别名，避免大小写不同的
来源共享同一个变量；其他合法别名仍可匿名访问，Unix 也可使用凭据文件中的原始别名。
Unix 回退文件必须为普通文件、权限严格为 `0600`；权限过宽即拒绝，
Windows 目前无法验证 POSIX `0600`，因此**只接受环境变量令牌，不读取回退文件**。
带令牌请求仅允许 HTTPS 或本机测试地址，不自动跟随跨主机重定向；
Git raw 地址仅在同源或受信的 GitHub raw 域使用对应令牌。
令牌不会写入锁文件或诊断，也不通过 `Debug` 输出。
