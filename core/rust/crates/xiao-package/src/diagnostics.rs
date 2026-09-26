//! 11A-D1 包契约与依赖图的稳定诊断编号。
//!
//! 包诊断与 `xiao-modules` 的文件模块诊断分开编号，因为两者的身份粒度、输入来源
//! 和后续处理边界都不同。消息文本只是当前语言预览，上层应优先消费编号和参数。

/// 同一包图中同名包绑定到不兼容身份时使用的编号。
pub const PACKAGE_IDENTITY_CONFLICT_CODE: &str = "X05-PACKAGE-001";
/// 依赖声明指向不存在的本地包配置时使用的编号。
pub const PACKAGE_MISSING_DEPENDENCY_CODE: &str = "X05-PACKAGE-002";
/// 包依赖图出现环时使用的编号。
pub const PACKAGE_DEPENDENCY_CYCLE_CODE: &str = "X05-PACKAGE-003";
/// 根包或依赖包的配置文件无法读取时使用的编号。
pub const PACKAGE_CONFIG_READ_CODE: &str = "X05-PACKAGE-004";
/// 包配置缺少可构造身份的元数据时使用的编号。
pub const PACKAGE_INVALID_METADATA_CODE: &str = "X05-PACKAGE-005";

/// 环境目录已经存在或重复创建时使用的编号。
pub const ENVIRONMENT_ALREADY_EXISTS_CODE: &str = "X05-ENV-001";
/// 环境逻辑名称或目录名称非法时使用的编号。
pub const ENVIRONMENT_INVALID_NAME_CODE: &str = "X05-ENV-002";
/// 环境目录或元数据写入失败时使用的编号。
pub const ENVIRONMENT_WRITE_CODE: &str = "X05-ENV-003";
/// 元数据版本高于当前读取器时使用的编号。
pub const ENVIRONMENT_METADATA_VERSION_CODE: &str = "X05-ENV-004";
/// 缓存根目录或环境元数据无法读取时使用的编号。
pub const CACHE_HOME_UNAVAILABLE_CODE: &str = "X05-CACHE-001";
/// `XIAO_HOME` 或对象摘要格式非法时使用的编号。
pub const CACHE_INVALID_INPUT_CODE: &str = "X05-CACHE-002";
/// 源目录包含不支持的文件类型或路径时使用的编号。
pub const CACHE_INVALID_SOURCE_CODE: &str = "X05-CACHE-003";
/// 缓存对象摘要校验失败时使用的编号。
pub const CACHE_OBJECT_CORRUPT_CODE: &str = "X05-CACHE-004";
/// 包操作开关、项目根或激活路径不合法时使用的编号。
pub const SYNC_INVALID_INPUT_CODE: &str = "X05-SYNC-001";
/// 安装或冻结同步缺少现有锁文件时使用的编号。
pub const SYNC_LOCK_REQUIRED_CODE: &str = "X05-SYNC-002";
/// 激活环境不存在或环境元数据无法写入时使用的编号。
pub const SYNC_ENVIRONMENT_CODE: &str = "X05-SYNC-003";
/// 锁文件格式版本高于当前读取器时使用的编号。
pub const LOCKFILE_UNSUPPORTED_VERSION_CODE: &str = "X05-LOCK-001";
/// 锁文件 JSON、结构或输入图无效时使用的编号。
pub const LOCKFILE_INVALID_CODE: &str = "X05-LOCK-002";
/// 配置解析出的依赖图与锁文件不一致时使用的编号。
pub const LOCKFILE_CONFIG_MISMATCH_CODE: &str = "X05-LOCK-003";
/// 本地包源码内容摘要与锁文件不一致时使用的编号。
pub const LOCKFILE_CONTENT_MISMATCH_CODE: &str = "X05-LOCK-004";
/// 本地包来源身份与锁文件不一致时使用的编号。
pub const LOCKFILE_SOURCE_MISMATCH_CODE: &str = "X05-LOCK-005";
/// 锁文件或原子提交的文件系统操作失败时使用的编号。
pub const LOCKFILE_IO_CODE: &str = "X05-LOCK-006";

/// 包源协议版本不受支持。
pub const SOURCE_UNSUPPORTED_VERSION_CODE: &str = "X05-SOURCE-001";
/// 同一配置内包源别名指向多个源。
pub const SOURCE_ALIAS_CONFLICT_CODE: &str = "X05-SOURCE-002";
/// 源不可用且没有可验证的缓存快照。
pub const SOURCE_UNAVAILABLE_CODE: &str = "X05-SOURCE-003";
/// 源清单或索引快照摘要不匹配。
pub const SOURCE_DIGEST_MISMATCH_CODE: &str = "X05-SOURCE-004";
/// 依赖或导入引用不存在的源。
pub const SOURCE_UNKNOWN_REFERENCE_CODE: &str = "X05-SOURCE-005";
/// 同等优先级或源内存在无法唯一确定的候选。
pub const SOURCE_AMBIGUOUS_CODE: &str = "X05-SOURCE-006";
/// 源描述或索引结构不合法。
pub const SOURCE_INVALID_CODE: &str = "X05-SOURCE-007";
/// 包源缓存、跨进程锁或原子提交的本地基础设施故障。
pub const SOURCE_CACHE_IO_CODE: &str = "X05-SOURCE-008";
/// 已存在的包源缓存对象损坏或缺失。
pub const SOURCE_CACHE_CORRUPT_CODE: &str = "X05-SOURCE-009";
/// 快照缓存声称属于不同的源或快照身份。
pub const SOURCE_SNAPSHOT_OWNER_CODE: &str = "X05-SOURCE-010";

/// 版本或约束不符合冻结的 SemVer 语法。
pub const VERSION_INVALID_CODE: &str = "X05-VERSION-001";
/// 已验证候选无法同时满足版本及来源约束。
pub const VERSION_UNSATISFIED_CODE: &str = "X05-VERSION-002";

/// 锁定的正文摘要与索引、下载字节或缓存目录不一致。
pub const TRUST_ARTIFACT_CODE: &str = "X05-TRUST-001";
/// 来源快照与锁定身份不一致。
pub const TRUST_SOURCE_CODE: &str = "X05-TRUST-002";
/// 正文不是符合首版契约的安全 TAR 包。
pub const TRUST_ARCHIVE_CODE: &str = "X05-TRUST-003";
/// 回退凭据的权限或结构不满足安全要求。
pub const TRUST_CREDENTIALS_CODE: &str = "X05-TRUST-004";

/// 包身份冲突诊断编号的简短别名。
pub const IDENTITY_CONFLICT_CODE: &str = PACKAGE_IDENTITY_CONFLICT_CODE;
/// 缺失依赖诊断编号的简短别名。
pub const MISSING_DEPENDENCY_CODE: &str = PACKAGE_MISSING_DEPENDENCY_CODE;
/// 依赖环诊断编号的简短别名。
pub const DEPENDENCY_CYCLE_CODE: &str = PACKAGE_DEPENDENCY_CYCLE_CODE;
