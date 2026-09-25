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

/// 包身份冲突诊断编号的简短别名。
pub const IDENTITY_CONFLICT_CODE: &str = PACKAGE_IDENTITY_CONFLICT_CODE;
/// 缺失依赖诊断编号的简短别名。
pub const MISSING_DEPENDENCY_CODE: &str = PACKAGE_MISSING_DEPENDENCY_CODE;
/// 依赖环诊断编号的简短别名。
pub const DEPENDENCY_CYCLE_CODE: &str = PACKAGE_DEPENDENCY_CYCLE_CODE;
