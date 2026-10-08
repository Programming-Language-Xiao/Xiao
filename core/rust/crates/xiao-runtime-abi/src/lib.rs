//! Xiao 原生程序使用的稳定 C ABI 边界。
//!
//! 本 crate 只定义固定布局和外部符号声明。对象头、引用计数、字符串内容以及容器字段
//! 全部由 `xiao-runtime` 实现；这里不能依赖 Runtime，也不能把 Rust enum 或对象字段
//! 暴露成语言契约。所有拥有句柄的入口都在注释中写明了空指针和释放责任。
//!
//! 句柄参数还有一条不可省略的调用约定：只能传入 Runtime 返回、尚未归还的 live
//! 句柄；`release`/`weak_release` 之后不得再次使用同一地址。Runtime 盒子中的魔数和
//! 种类标记只用于拦截仍可读的明显类型错配，不能把任意外部地址或释放后的悬空指针
//! 变成安全输入。
//!
//! N0-B 句柄继承 06B 的单线程、非原子引用计数约束，不得跨线程传递或并发调用；并发
//! 版本必须另行冻结原子存储和对应的 ABI 生命周期规则。

/// 当前 ABI 的主版本。
pub const ABI_MAJOR_VERSION: u32 = 1;
/// 当前 ABI 的次版本；新增兼容入口只递增此字段。
pub const ABI_MINOR_VERSION: u32 = 8;
/// 兼容旧调用方的主版本常量。
pub const ABI_VERSION: u32 = ABI_MAJOR_VERSION;
/// ABI 版本编码的高位宽度。
pub const ABI_VERSION_MINOR_BITS: u32 = 16;

/// 返回一个可比较的主/次版本编码。
#[must_use]
pub const fn encoded_abi_version(major: u32, minor: u32) -> u64 {
    ((major as u64) << ABI_VERSION_MINOR_BITS) | (minor as u64)
}

/// 当前 ABI 的可比较版本编码。
pub const ABI_ENCODED_VERSION: u64 = encoded_abi_version(ABI_MAJOR_VERSION, ABI_MINOR_VERSION);

/// C ABI 函数的稳定状态码。
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum XiaoAbiStatus {
    /// 调用成功。
    Ok = 0,
    /// 必需的指针为空或长度不合法。
    Null = 1,
    /// 标签或描述符字段不受支持。
    InvalidArgument = 2,
    /// 句柄类型与入口要求不一致，或对象已释放。
    InvalidHandle = 3,
    /// UTF-8 输入无法解码。
    InvalidUtf8 = 4,
    /// 目标索引或键不存在。
    OutOfBounds = 5,
    /// Runtime 内部操作失败。
    RuntimeError = 6,
    /// 请求的 ABI 主版本不兼容。
    VersionMismatch = 7,
    /// Runtime 报告了不可恢复故障；调用方不得把它路由到 Xiao `catch`。
    Fatal = 8,
}

impl XiaoAbiStatus {
    /// 返回 C ABI 中的整数状态码。
    #[must_use]
    pub const fn code(self) -> i32 {
        self as i32
    }
}

/// 一个只可由 Runtime 实现解释的不透明强句柄。
///
/// 该类型没有可构造的公开字段。调用方只能把 Runtime 返回的地址原样传回，并且必须
/// 为每个成功获得的强句柄调用一次 `xiao_runtime_release`；释放后不得再次传回任何
/// 句柄入口。
#[repr(C)]
pub struct XiaoOpaqueHandle {
    _private: [u8; 0],
}

/// 一个只可由 Runtime 实现解释的不透明弱句柄。
///
/// 弱句柄不拥有目标载荷；调用方必须用 `xiao_runtime_weak_release` 归还自身的弱引用，
/// 不能把它当作强句柄传给普通 `retain`/`release`，且归还后不得再次传给弱句柄入口。
#[repr(C)]
pub struct XiaoOpaqueWeakHandle {
    _private: [u8; 0],
}

/// C ABI 中使用的不透明强句柄指针类型。
pub type XiaoHandle = *mut XiaoOpaqueHandle;
/// C ABI 中使用的不透明弱句柄指针类型。
pub type XiaoWeakHandle = *mut XiaoOpaqueWeakHandle;

/// 用于 ABI 边界的固定宽度源码区间。
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XiaoAbiSpan {
    /// 起始字节偏移。
    pub start: u64,
    /// 结束字节偏移（不包含）。
    pub end: u64,
}

/// 错误位置是否包含有效源码区间。
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XiaoAbiErrorLocation {
    /// 源码字节区间。
    pub span: XiaoAbiSpan,
    /// 非零表示 `span` 有效。
    pub present: u8,
    /// 为固定布局保留的字节。
    pub _reserved: [u8; 7],
}

impl XiaoAbiErrorLocation {
    /// 创建一个没有源码位置的错误位置。
    #[must_use]
    pub const fn none() -> Self {
        Self {
            span: XiaoAbiSpan { start: 0, end: 0 },
            present: 0,
            _reserved: [0; 7],
        }
    }

    /// 创建一个带源码区间的错误位置。
    #[must_use]
    pub const fn from_span(span: XiaoAbiSpan) -> Self {
        Self {
            span,
            present: 1,
            _reserved: [0; 7],
        }
    }
}

/// 错误报告的机器类别。
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct XiaoErrorClass(u32);

#[allow(non_upper_case_globals)]
impl XiaoErrorClass {
    /// 没有挂起错误。
    pub const None: Self = Self(0);
    /// 可被 Xiao `catch` 捕获的错误。
    pub const Recoverable: Self = Self(1);
    /// 不得被 Xiao `catch` 捕获的致命故障。
    pub const Fatal: Self = Self(2);

    /// 从 ABI 原始值构造类别。
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// 返回 ABI 原始类别。
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// 错误参数的机器值类别。
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct XiaoErrorParamKind(u32);

#[allow(non_upper_case_globals)]
impl XiaoErrorParamKind {
    /// 文本参数。
    pub const Text: Self = Self(1);
    /// 有符号整数参数。
    pub const Integer: Self = Self(2);
    /// 布尔参数。
    pub const Boolean: Self = Self(3);

    /// 从 ABI 原始值构造参数类别。
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// 返回 ABI 原始参数类别。
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// 错误报告的固定宽度摘要；其中的字节视图只在当前 ABI 调用期间有效。
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XiaoAbiErrorSnapshot {
    /// 可恢复、致命或无错误类别。
    pub class: XiaoErrorClass,
    /// Runtime 错误类别的稳定数值。
    pub kind: u32,
    /// 进程内错误事件编号。
    pub error_id: u64,
    /// 稳定错误码。
    pub code: XiaoAbiBytes,
    /// 消息目录键。
    pub message_id: XiaoAbiBytes,
    /// 源码位置。
    pub location: XiaoAbiErrorLocation,
    /// 对应宿主进程退出码；无错误时为零。
    pub exit_code: i32,
    /// 调用栈帧数量。
    pub stack_depth: u32,
    /// 结构化参数数量。
    pub param_count: u32,
}

/// 一个结构化错误参数；文本视图只在当前 ABI 调用期间有效。
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XiaoAbiErrorParam {
    /// 稳定参数名。
    pub key: XiaoAbiBytes,
    /// 参数值类别。
    pub kind: XiaoErrorParamKind,
    /// 整数或布尔参数的固定宽度载荷。
    pub integer: i64,
    /// 文本参数视图；非文本参数为空。
    pub text: XiaoAbiBytes,
}

/// 一个统一堆栈帧摘要；所有字符串视图只在当前 ABI 调用期间有效。
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XiaoAbiStackFrame {
    /// 模块名。
    pub module: XiaoAbiBytes,
    /// 函数名。
    pub function: XiaoAbiBytes,
    /// 源文件名；缺失时为空。
    pub source: XiaoAbiBytes,
    /// 源码位置。
    pub location: XiaoAbiErrorLocation,
    /// 字节码偏移；缺失时为 `-1`。
    pub bytecode_offset: i64,
    /// 原生地址；缺失时为 `-1`。
    pub native_address: i64,
    /// 内联深度；缺失时为 `-1`。
    pub inline_depth: i32,
    /// 用户帧为 `0`，Runtime 帧为 `1`。
    pub frame_kind: u32,
}

/// 原生 Runtime 发出的诊断事件；实现侧转交给既有诊断会话，不负责本地化。
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XiaoAbiDiagnosticEvent {
    /// 稳定事件类型。
    pub event_type: XiaoAbiBytes,
    /// 可选错误码。
    pub code: XiaoAbiBytes,
    /// 可选消息目录键。
    pub message_id: XiaoAbiBytes,
    /// 事件源码位置。
    pub location: XiaoAbiErrorLocation,
}

/// 一个不拥有输入内存的 UTF-8 字节视图。
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XiaoAbiBytes {
    /// 字节起始地址；长度为零时可以为空。
    pub ptr: *const u8,
    /// 字节长度。
    pub len: usize,
}

/// 一个不拥有输入或输出内存的可变字节视图。
#[repr(C)]
#[derive(Debug, Eq, PartialEq)]
pub struct XiaoAbiMutBytes {
    /// 可写缓冲区起始地址；容量为零时可以为空。
    pub ptr: *mut u8,
    /// 缓冲区容量。
    pub capacity: usize,
}

/// `RuntimeValue` 在 ABI 线上的显式标签。
///
/// 数值固定后不得重排。标签是线格式的一部分，不由 LLVM 后端根据 Rust enum 判别式
/// 推断；新增变体只能追加编号，并且要同步更新 Runtime 的穷举转换测试。
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct XiaoValueTag(u32);

#[allow(non_upper_case_globals)]
impl XiaoValueTag {
    /// 空值。
    pub const None: Self = Self(0);
    /// 布尔值，载荷使用 `u8` 的 0/1。
    pub const Bool: Self = Self(1);
    /// 64 位有符号整数。
    pub const Int: Self = Self(2);
    /// 32 位有符号整数。
    pub const Sint: Self = Self(3);
    /// 任意精度整数文本对象。
    pub const Lint: Self = Self(4);
    /// 64 位浮点。
    pub const Float: Self = Self(5);
    /// 32 位浮点。
    pub const Sfloat: Self = Self(6);
    /// 任意精度浮点文本对象。
    pub const Lfloat: Self = Self(7);
    /// UTF-8 字符串对象强句柄。
    pub const Str: Self = Self(8);
    /// 表实例强句柄。
    pub const Table: Self = Self(9);
    /// 析构观察视图弱句柄。
    pub const TableDropView: Self = Self(10);
    /// 数组强句柄。
    pub const Array: Self = Self(11);
    /// 元组强句柄。
    pub const Tuple: Self = Self(12);
    /// 字典表强句柄。
    pub const DictTable: Self = Self(13);
    /// 字典列强句柄。
    pub const DictColumn: Self = Self(14);
    /// 集合强句柄。
    pub const Set: Self = Self(15);
    /// 可恢复错误对象句柄。
    pub const Error: Self = Self(16);

    /// 从 ABI 原始标签构造值；未知标签由 Runtime 入口拒绝。
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// 返回 ABI 原始标签。
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// 判断标签是否属于当前 ABI 版本。
    #[must_use]
    pub const fn is_known(self) -> bool {
        self.0 <= Self::Error.0
    }
}

impl XiaoValueTag {
    /// 返回该标签是否携带一个强拥有句柄。
    #[must_use]
    pub const fn owns_strong_handle(self) -> bool {
        matches!(
            self,
            Self::Str
                | Self::Table
                | Self::Array
                | Self::Tuple
                | Self::DictTable
                | Self::DictColumn
                | Self::Set
                | Self::Lint
                | Self::Lfloat
        )
    }

    /// 返回该标签是否携带一个弱句柄。
    #[must_use]
    pub const fn owns_weak_handle(self) -> bool {
        matches!(self, Self::TableDropView)
    }
}

/// 标签对应的固定 64 位 ABI 载荷。
///
/// 读取 union 字段前必须先检查 [`XiaoValue::tag`]；Runtime 入口会再次校验标签，避免把
/// 一个句柄按数值解释。union 只承载标量位或不透明指针，不暴露 Rust 对象布局。
#[repr(C)]
#[derive(Clone, Copy)]
pub union XiaoValuePayload {
    /// 64 位整数载荷。
    pub i64_value: i64,
    /// 32 位整数载荷。
    pub i32_value: i32,
    /// 64 位浮点载荷。
    pub f64_value: f64,
    /// 32 位浮点载荷。
    pub f32_value: f32,
    /// 布尔载荷，合法值只有 0 和 1。
    pub bool_value: u8,
    /// 强句柄载荷。
    pub handle: XiaoHandle,
    /// 弱句柄载荷。
    pub weak_handle: XiaoWeakHandle,
    /// 用于清零和调试的原始机器字。
    pub raw: usize,
}

impl std::fmt::Debug for XiaoValuePayload {
    /// 只打印载荷地址数值，避免在未知标签下误读 union 字段。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("XiaoValuePayload")
            .finish_non_exhaustive()
    }
}

/// 动态值的稳定 C 布局。
#[repr(C)]
#[derive(Debug)]
pub struct XiaoValue {
    /// 解释 payload 的固定标签。
    pub tag: XiaoValueTag,
    /// 与标签配套的标量位或不透明句柄。
    pub payload: XiaoValuePayload,
}

impl XiaoValue {
    /// 构造空值；不产生 Runtime 所有权。
    #[must_use]
    pub const fn none() -> Self {
        Self {
            tag: XiaoValueTag::None,
            payload: XiaoValuePayload { raw: 0 },
        }
    }

    /// 构造内联 64 位整数值。
    #[must_use]
    pub const fn int(value: i64) -> Self {
        Self {
            tag: XiaoValueTag::Int,
            payload: XiaoValuePayload { i64_value: value },
        }
    }

    /// 构造内联 32 位整数值。
    #[must_use]
    pub const fn sint(value: i32) -> Self {
        Self {
            tag: XiaoValueTag::Sint,
            payload: XiaoValuePayload { i32_value: value },
        }
    }

    /// 构造内联 64 位浮点值。
    #[must_use]
    pub const fn float(value: f64) -> Self {
        Self {
            tag: XiaoValueTag::Float,
            payload: XiaoValuePayload { f64_value: value },
        }
    }

    /// 构造内联 32 位浮点值。
    #[must_use]
    pub const fn sfloat(value: f32) -> Self {
        Self {
            tag: XiaoValueTag::Sfloat,
            payload: XiaoValuePayload { f32_value: value },
        }
    }

    /// 构造内联布尔值。
    #[must_use]
    pub const fn bool(value: bool) -> Self {
        Self {
            tag: XiaoValueTag::Bool,
            payload: XiaoValuePayload {
                bool_value: value as u8,
            },
        }
    }
}

/// 表定义中的字段类型标签；它只描述 ABI 构造所需的有限静态类型。
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct XiaoFieldType(u32);

#[allow(non_upper_case_globals)]
impl XiaoFieldType {
    /// 动态字段，可保存任意 `XiaoValue`。
    pub const Dynamic: Self = Self(0);
    /// 64 位整数。
    pub const Int: Self = Self(1);
    /// 32 位整数。
    pub const Sint: Self = Self(2);
    /// 64 位浮点。
    pub const Float: Self = Self(3);
    /// 32 位浮点。
    pub const Sfloat: Self = Self(4);
    /// 布尔值。
    pub const Bool: Self = Self(5);
    /// UTF-8 字符串。
    pub const Str: Self = Self(6);

    /// 从 ABI 原始字段类型构造值。
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// 返回 ABI 原始字段类型。
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// 判断字段类型是否受当前 ABI 支持。
    #[must_use]
    pub const fn is_known(self) -> bool {
        self.0 <= Self::Str.0
    }
}

/// C ABI 表字段描述符；所有指针只在构造调用期间借用。
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct XiaoTableFieldDescriptor {
    /// 字段名的 UTF-8 字节视图。
    pub name: XiaoAbiBytes,
    /// 字段类型标签。
    pub ty: XiaoFieldType,
    /// `1` 表示公开字段，`0` 表示私有字段。
    pub public: u8,
}

/// C ABI 表定义描述符；Runtime 会复制名称和字段元数据。
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct XiaoTableDescriptor {
    /// 表名的 UTF-8 字节视图。
    pub name: XiaoAbiBytes,
    /// `0` 表示 singleton，`1` 表示 instance。
    pub kind: u32,
    /// 字段描述数组；字段数量为零时可以为空。
    pub fields: *const XiaoTableFieldDescriptor,
    /// 字段描述数量。
    pub field_count: usize,
}

/// 新表描述符的独立布局版本；旧 `XiaoTableDescriptor` 保持原布局。
pub const TABLE_DESCRIPTOR_VERSION: u32 = 1;

/// 方法元数据中的动态类型；其他值使用既有 `XiaoValueTag`，不新增值标签。
pub const TABLE_METHOD_DYNAMIC_TYPE: u32 = u32::MAX;

/// 表方法、字段初始化和生命周期回调的统一 C 调用约定。
///
/// receiver 与 arguments 仅在调用期间借用；需要持有时显式复制。out 指向已初始化
/// 的 none 槽，成功返回唯一拥有的结果，失败保持 none。析构 receiver 为弱只读视图，
/// 不得复活、写入、逃逸或跨线程。回调代码必须在全部相关实例存活期间有效。
pub type XiaoTableCallback = unsafe extern "C" fn(
    receiver: *const XiaoValue,
    arguments: *const XiaoValue,
    argument_count: usize,
    out: *mut XiaoValue,
) -> i32;

/// 注册式表方法描述；名称和参数类型数组由 Runtime 复制。
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct XiaoTableMethodDescriptor {
    /// 含名字空间前缀的规范成员名。
    pub name: XiaoAbiBytes,
    /// 由 `table_method_signature_id` 生成的签名标识。
    pub signature_id: u64,
    /// 不含 receiver 的参数值标签；动态参数使用 `TABLE_METHOD_DYNAMIC_TYPE`。
    pub parameter_types: *const u32,
    /// 固定参数个数，零参数时 parameter_types 可为空。
    pub parameter_count: usize,
    /// 返回值标签，规则同 parameter_types。
    pub return_type: u32,
    /// 1 为公开，0 为私有；后端仍须完成静态可见性检查。
    pub public: u8,
    /// 必须非空的方法实现。
    pub callback: Option<XiaoTableCallback>,
}

/// 独立的表方法描述符；只由新增入口消费，不改变旧描述符的所有权契约。
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct XiaoTableDescriptorV2 {
    /// 本结构的字节大小，用于拒绝截短或未知布局。
    pub struct_size: u32,
    /// 必须等于 `TABLE_DESCRIPTOR_VERSION`。
    pub version: u32,
    /// 原有名称、种类和字段元数据，保持原结构布局。
    pub fields: XiaoTableDescriptor,
    /// 稳定排序的方法数组；数量为零时可为空。
    pub methods: *const XiaoTableMethodDescriptor,
    /// 方法数组长度。
    pub method_count: usize,
    /// 字段初始化回调，执行于 FieldsInitializing 状态，返回 none。
    pub initialize_fields: Option<XiaoTableCallback>,
    /// 零显式参数的 init 回调；须与方法表 ascii:init 一致。
    pub init: Option<XiaoTableCallback>,
    /// drop 回调；须与方法表 ascii:drop 一致，接收弱只读视图。
    pub drop: Option<XiaoTableCallback>,
}

/// 对 ABI 类型标签和参数顺序生成确定性的签名 ID；不依赖主机字节序或地址。
#[must_use]
pub fn table_method_signature_id(parameters: &[u32], result: u32) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in (parameters.len() as u64)
        .to_le_bytes()
        .into_iter()
        .chain(parameters.iter().flat_map(|tag| tag.to_le_bytes()))
        .chain(result.to_le_bytes())
    {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

// ABI 版本兼容检查：新增入口使用次版本，改变布局/标签/所有权契约必须升主版本。
// 这些声明由 `xiao-runtime` 提供实现；ABI crate 本身不定义同名符号，避免出现两份
// Runtime。调用方必须保证 `required_major`/`required_minor` 是编译产物记录的版本。
unsafe extern "C" {
    /// 设置当前线程原生会话的确定性随机种子；接受非负 int/sint/lint，失败不改变状态。
    pub fn xiao_runtime_random_seed(value: *const XiaoValue) -> i32;
    /// 复制 V2 元数据并经既有状态机执行字段初始化、零参数 init 与失败回滚。
    /// out 必须指向已初始化的 null 槽；失败保持 null，成功后调用方唯一拥有表强句柄。
    pub fn xiao_runtime_table_new_v2(
        descriptor: *const XiaoTableDescriptorV2,
        out: *mut XiaoHandle,
    ) -> i32;
    /// 带初始化参数的新表入口；参数数组在调用期间借用，失败保持 null。
    pub fn xiao_runtime_table_new_v2_with_args(
        descriptor: *const XiaoTableDescriptorV2,
        arguments: *const XiaoValue,
        argument_count: usize,
        out: *mut XiaoHandle,
    ) -> i32;
    /// 调用静态检查过的方法；校验名称、签名 ID、参数个数/标签，失败输出 none。
    /// receiver 和参数借用，返回结果唯一拥有；drop 只能由生命周期状态机触发。
    pub fn xiao_runtime_table_call(
        receiver: *const XiaoValue,
        method: XiaoAbiBytes,
        signature_id: u64,
        arguments: *const XiaoValue,
        argument_count: usize,
        out: *mut XiaoValue,
    ) -> i32;
    /// 读取普通表或析构弱只读视图字段；字段访问已经过静态可见性检查。
    pub fn xiao_runtime_table_get_value(
        receiver: *const XiaoValue,
        field: XiaoAbiBytes,
        out: *mut XiaoValue,
    ) -> i32;
    /// 写入普通表字段；拒绝析构弱只读视图。
    pub fn xiao_runtime_table_set_value(
        receiver: *const XiaoValue,
        field: XiaoAbiBytes,
        value: *const XiaoValue,
    ) -> i32;
    /// 归还回调中可能是强值或析构弱只读视图的临时拥有值，并清空槽。
    pub fn xiao_runtime_value_release_any(value: *mut XiaoValue);
    /// 返回 Runtime 支持的 ABI 主版本。
    pub fn xiao_runtime_abi_version() -> u32;
    /// 返回 Runtime 支持的 ABI 次版本。
    pub fn xiao_runtime_abi_minor_version() -> u32;
    /// 判断一个生成产物所需版本是否可由当前 Runtime 满足。
    pub fn xiao_runtime_abi_is_compatible(required_major: u32, required_minor: u32) -> i32;

    /// 清除当前线程的挂起错误；普通成功路径必须在进入新一轮执行前调用。
    pub fn xiao_runtime_error_clear();
    /// 返回当前线程挂起错误的机器类别：`0` 无错误、`1` 可恢复、`2` 致命。
    pub fn xiao_runtime_error_class() -> u32;
    /// 为挂起的 Runtime 错误补上触发它的源码位置。
    pub fn xiao_runtime_error_attach_span(start: u64, end: u64) -> i32;
    /// 创建一个可恢复错误值；`FatalError` 或未知错误类型不会伪装成可恢复值。
    pub fn xiao_runtime_error_new(
        type_name: XiaoAbiBytes,
        code: XiaoAbiBytes,
        message: XiaoAbiBytes,
        location: *const XiaoAbiErrorLocation,
    ) -> XiaoValue;
    /// 从借用的 Runtime 文本值构造可恢复错误；缺省参数传空指针，调用方保留值所有权。
    pub fn xiao_runtime_error_new_values(
        type_name: XiaoAbiBytes,
        code: *const XiaoValue,
        message: *const XiaoValue,
        location: *const XiaoAbiErrorLocation,
    ) -> XiaoValue;
    /// 创建一个可恢复错误并放入当前线程的挂起错误槽。
    pub fn xiao_runtime_error_raise_type(
        type_name: XiaoAbiBytes,
        code: XiaoAbiBytes,
        message: XiaoAbiBytes,
        location: XiaoAbiErrorLocation,
    ) -> i32;
    /// 将 `XiaoValueTag::Error` 值复制为当前线程的挂起错误。
    pub fn xiao_runtime_error_raise_value(
        value: *const XiaoValue,
        location: XiaoAbiErrorLocation,
    ) -> i32;
    /// 判断挂起的可恢复错误是否匹配给定的语言错误类型名。
    pub fn xiao_runtime_error_matches(error_type: XiaoAbiBytes) -> i32;
    /// 把挂起的可恢复错误取成拥有的 `XiaoValueTag::Error` 值；`out` 必须是已初始化且
    /// 不含弱表观察值的槽，弱表观察值只能由 `xiao_runtime_value_copy` 覆盖；失败时保留挂起错误。
    pub fn xiao_runtime_error_take(out: *mut XiaoValue) -> i32;
    /// 读取当前线程挂起错误的机器字段。
    pub fn xiao_runtime_error_snapshot(out: *mut XiaoAbiErrorSnapshot) -> i32;
    /// 读取当前线程挂起错误的第 `index` 个结构化参数。
    pub fn xiao_runtime_error_param(index: usize, out: *mut XiaoAbiErrorParam) -> i32;
    /// 读取当前线程挂起错误的第 `index` 个堆栈帧。
    pub fn xiao_runtime_error_stack_frame(index: usize, out: *mut XiaoAbiStackFrame) -> i32;
    /// 返回当前线程挂起错误对应的稳定宿主退出码。
    pub fn xiao_runtime_error_exit_code() -> i32;
    /// 将当前挂起错误写成既有诊断路径可消费的机器摘要。
    pub fn xiao_runtime_error_report();
    /// 将一个诊断事件交给已建立的诊断会话；无会话时仍保留确定性失败状态。
    pub fn xiao_runtime_diagnostic_event(event: *const XiaoAbiDiagnosticEvent) -> i32;
    /// 为动态原生产物建立本机诊断会话；失败时返回稳定的启动失败退出码。
    pub fn xiao_runtime_diagnostic_prepare() -> i32;
    /// 等待独立诊断进程完成握手；成功前不得执行用户代码。
    pub fn xiao_runtime_diagnostic_ready() -> i32;
    /// 发送最终指标并关闭原生诊断会话。
    pub fn xiao_runtime_diagnostic_finish();
    /// 设置当前原生入口使用的不可变语言上下文。
    pub fn xiao_runtime_language_context_set(locale: XiaoAbiBytes) -> i32;
    /// 按 `XIAO_RUNTIME_RELEASE_TRACE_PATH` 开启当前线程的释放事件追踪。
    pub fn xiao_runtime_release_trace_begin();
    /// 将当前线程的释放事件写入追踪文件并清空事件缓冲。
    pub fn xiao_runtime_release_trace_flush();
    /// 处理 ABI/平台边界致命故障；该函数不会返回，也不会进入 Xiao `catch`。
    pub fn xiao_runtime_fatal_abi() -> !;

    /// 保留一个强句柄并返回同一 ABI 盒子地址；每次成功调用都必须配对一次 release。
    pub fn xiao_runtime_retain(handle: XiaoHandle) -> XiaoHandle;
    /// 释放一个强句柄；空指针安全，无需调用方先判断。
    pub fn xiao_runtime_release(handle: XiaoHandle);
    /// 从强句柄创建一个弱句柄；失败时返回空指针。
    pub fn xiao_runtime_weak(handle: XiaoHandle) -> XiaoWeakHandle;
    /// 保留一个弱句柄并返回同一 ABI 盒子地址；每次成功调用都必须配对一次 release。
    pub fn xiao_runtime_weak_retain(handle: XiaoWeakHandle) -> XiaoWeakHandle;
    /// 释放一个弱句柄；它不会释放目标载荷，空指针安全。
    pub fn xiao_runtime_weak_release(handle: XiaoWeakHandle);
    /// 尝试把弱句柄升级为新的强句柄；目标已销毁时返回空指针。
    pub fn xiao_runtime_weak_upgrade(handle: XiaoWeakHandle) -> XiaoHandle;

    /// 按标签复制一个 ABI 值并增加其句柄引用；标量只做位复制。
    ///
    /// `out` 必须指向已经初始化的 `XiaoValue` 槽（通常先写入 `none`），成功时会先
    /// 释放槽中原有的拥有值再写入副本；失败时保留原值。`value` 与 `out` 不得重叠。
    pub fn xiao_runtime_value_copy(value: *const XiaoValue, out: *mut XiaoValue) -> i32;
    /// 释放一个 ABI 值拥有的句柄；标量和空值无操作。
    pub fn xiao_runtime_value_release(value: *mut XiaoValue);
    /// 只释放 ABI 值中的强句柄；弱值保持不变，标量重置为空值。
    pub fn xiao_runtime_value_release_strong(value: *mut XiaoValue);
    /// 只释放 ABI 值中的弱句柄；强值或标量返回错误码且不改变值。
    pub fn xiao_runtime_value_release_weak(value: *mut XiaoValue) -> i32;
    /// 从弱句柄构造析构观察值并增加一次弱引用。
    pub fn xiao_runtime_value_weak(handle: XiaoWeakHandle) -> XiaoValue;
    /// 从错误强句柄构造 `XiaoValueTag::Error`；句柄所有权转移到返回值。
    pub fn xiao_runtime_value_error(handle: XiaoHandle) -> XiaoValue;
    /// 构造内联 64 位整数值。
    pub fn xiao_runtime_value_int(value: i64) -> XiaoValue;
    /// 构造内联 32 位整数值。
    pub fn xiao_runtime_value_sint(value: i32) -> XiaoValue;
    /// 构造内联 64 位浮点值。
    pub fn xiao_runtime_value_float(value: f64) -> XiaoValue;
    /// 构造内联 32 位浮点值。
    pub fn xiao_runtime_value_sfloat(value: f32) -> XiaoValue;
    /// 构造内联布尔值；非零输入变成 `true`。
    pub fn xiao_runtime_value_bool(value: u8) -> XiaoValue;
    /// 构造空值。
    pub fn xiao_runtime_value_none() -> XiaoValue;
    /// 按 Runtime 唯一算子表执行二元运算；`op` 使用稳定 ASCII 名称，`out` 为已初始化值槽。
    pub fn xiao_runtime_value_binary(
        op: XiaoAbiBytes,
        left: *const XiaoValue,
        right: *const XiaoValue,
        out: *mut XiaoValue,
    ) -> i32;
    /// 按 Runtime 唯一算子表执行一元运算；`out` 为已初始化值槽。
    pub fn xiao_runtime_value_unary(
        op: XiaoAbiBytes,
        value: *const XiaoValue,
        out: *mut XiaoValue,
    ) -> i32;
    /// 执行显式标量转换；`target` 使用 `ScalarType::as_str()` 的稳定名称。
    pub fn xiao_runtime_value_cast(
        target: XiaoAbiBytes,
        value: *const XiaoValue,
        out: *mut XiaoValue,
    ) -> i32;
    /// 执行一个前端登记的动态检查；检查失败写入统一 Runtime 错误槽。
    pub fn xiao_runtime_dynamic_check(kind: XiaoAbiBytes, value: *const XiaoValue) -> i32;
    /// 返回动态可迭代值的元素数量。
    pub fn xiao_runtime_value_iter_len(value: *const XiaoValue, out: *mut usize) -> i32;
    /// 复制动态可迭代值的指定元素。
    pub fn xiao_runtime_value_iter_get(
        value: *const XiaoValue,
        index: i64,
        out: *mut XiaoValue,
    ) -> i32;

    /// 从 UTF-8 字节复制字符串对象并返回强句柄值。
    ///
    /// `out` 必须指向已初始化的句柄槽（空指针或有效强句柄）；成功时会释放旧句柄并
    /// 写入新句柄，失败时保留旧槽位。
    pub fn xiao_runtime_string_new(bytes: XiaoAbiBytes, out: *mut XiaoHandle) -> i32;
    /// 返回字符串 Unicode 标量数量；失败时写零并返回错误码。
    pub fn xiao_runtime_string_len(handle: XiaoHandle, out: *mut usize) -> i32;
    /// 把字符串 UTF-8 字节复制到调用方缓冲区；容量不足时返回 `OutOfBounds`。
    pub fn xiao_runtime_string_copy(
        handle: XiaoHandle,
        buffer: XiaoAbiMutBytes,
        written: *mut usize,
    ) -> i32;
    /// 从字符串强句柄构造 `XiaoValueTag::Str`。
    pub fn xiao_runtime_value_str(handle: XiaoHandle) -> XiaoValue;
    /// 消费字符串强句柄并构造 `XiaoValueTag::Str`；调用成功后不得再次释放句柄。
    pub fn xiao_runtime_value_str_owned(handle: XiaoHandle) -> XiaoValue;
    /// 从字符串强句柄构造任意精度整数文本值。
    pub fn xiao_runtime_value_lint(handle: XiaoHandle) -> XiaoValue;
    /// 消费字符串强句柄并构造任意精度整数文本值；调用成功后不得再次释放句柄。
    pub fn xiao_runtime_value_lint_owned(handle: XiaoHandle) -> XiaoValue;
    /// 从字符串强句柄构造任意精度浮点文本值。
    pub fn xiao_runtime_value_lfloat(handle: XiaoHandle) -> XiaoValue;
    /// 消费字符串强句柄并构造任意精度浮点文本值；调用成功后不得再次释放句柄。
    pub fn xiao_runtime_value_lfloat_owned(handle: XiaoHandle) -> XiaoValue;

    /// 从值数组构造数组对象；Runtime 会复制每个输入值的所有权。`out` 遵循字符串构造
    /// 入口的已初始化句柄槽契约。
    pub fn xiao_runtime_array_new(
        values: *const XiaoValue,
        length: usize,
        out: *mut XiaoHandle,
    ) -> i32;
    /// 返回数组长度；`out` 非空时失败会写零。
    pub fn xiao_runtime_array_len(handle: XiaoHandle, out: *mut usize) -> i32;
    /// 复制数组指定元素到调用方；`out` 必须是已初始化且不含弱表观察值的槽，成功时替换并释放旧值。
    pub fn xiao_runtime_array_get(handle: XiaoHandle, index: usize, out: *mut XiaoValue) -> i32;
    /// 从数组强句柄构造 `XiaoValueTag::Array`。
    pub fn xiao_runtime_value_array(handle: XiaoHandle) -> XiaoValue;
    /// 消费数组强句柄并构造 `XiaoValueTag::Array`；调用成功后不得再次释放句柄。
    pub fn xiao_runtime_value_array_owned(handle: XiaoHandle) -> XiaoValue;

    /// 从值数组构造元组对象；`out` 遵循已初始化句柄槽契约。
    pub fn xiao_runtime_tuple_new(
        values: *const XiaoValue,
        length: usize,
        out: *mut XiaoHandle,
    ) -> i32;
    /// 返回元组长度；`out` 非空时失败会写零。
    pub fn xiao_runtime_tuple_len(handle: XiaoHandle, out: *mut usize) -> i32;
    /// 复制元组指定元素到调用方；`out` 必须是已初始化且不含弱表观察值的槽，成功时替换并释放旧值。
    pub fn xiao_runtime_tuple_get(handle: XiaoHandle, index: usize, out: *mut XiaoValue) -> i32;
    /// 从元组强句柄构造 `XiaoValueTag::Tuple`。
    pub fn xiao_runtime_value_tuple(handle: XiaoHandle) -> XiaoValue;
    /// 消费元组强句柄并构造 `XiaoValueTag::Tuple`；调用成功后不得再次释放句柄。
    pub fn xiao_runtime_value_tuple_owned(handle: XiaoHandle) -> XiaoValue;

    /// 按表或列形态从键值数组构造字典；`out` 遵循已初始化句柄槽契约。
    pub fn xiao_runtime_dict_new(
        kind: u32,
        keys: *const XiaoAbiBytes,
        values: *const XiaoValue,
        length: usize,
        out: *mut XiaoHandle,
    ) -> i32;
    /// 返回字典条目数量；`out` 非空时失败会写零。
    pub fn xiao_runtime_dict_len(handle: XiaoHandle, out: *mut usize) -> i32;
    /// 按 UTF-8 键复制字典值；`out` 必须是已初始化且不含弱表观察值的槽，成功时替换并释放旧值。
    pub fn xiao_runtime_dict_get(handle: XiaoHandle, key: XiaoAbiBytes, out: *mut XiaoValue)
    -> i32;
    /// 从字典强句柄构造对应标签的 ABI 值。
    pub fn xiao_runtime_value_dict(handle: XiaoHandle, kind: u32) -> XiaoValue;
    /// 消费字典强句柄并构造对应标签的 ABI 值；调用成功后不得再次释放句柄。
    pub fn xiao_runtime_value_dict_owned(handle: XiaoHandle, kind: u32) -> XiaoValue;

    /// 从值数组构造集合并执行可哈希与去重检查；`out` 遵循已初始化句柄槽契约。
    pub fn xiao_runtime_set_new(
        values: *const XiaoValue,
        length: usize,
        out: *mut XiaoHandle,
    ) -> i32;
    /// 返回集合元素数量；`out` 非空时失败会写零。
    pub fn xiao_runtime_set_len(handle: XiaoHandle, out: *mut usize) -> i32;
    /// 判断集合是否包含给定值。
    pub fn xiao_runtime_set_contains(
        handle: XiaoHandle,
        value: *const XiaoValue,
        out: *mut u8,
    ) -> i32;
    /// 从集合强句柄构造 `XiaoValueTag::Set`。
    pub fn xiao_runtime_value_set(handle: XiaoHandle) -> XiaoValue;
    /// 消费集合强句柄并构造 `XiaoValueTag::Set`；调用成功后不得再次释放句柄。
    pub fn xiao_runtime_value_set_owned(handle: XiaoHandle) -> XiaoValue;

    /// 按静态字段描述构造一个表实例或单例表；`out` 遵循已初始化句柄槽契约。
    pub fn xiao_runtime_table_new(
        descriptor: *const XiaoTableDescriptor,
        out: *mut XiaoHandle,
    ) -> i32;
    /// 按字段名读取表字段并复制值；`out` 必须是已初始化且不含弱表观察值的槽，成功时替换并释放旧值。
    pub fn xiao_runtime_table_get(
        handle: XiaoHandle,
        field: XiaoAbiBytes,
        out: *mut XiaoValue,
    ) -> i32;
    /// 按字段名写入表字段；Runtime 会复制输入值。
    pub fn xiao_runtime_table_set(
        handle: XiaoHandle,
        field: XiaoAbiBytes,
        value: *const XiaoValue,
    ) -> i32;
    /// 从表强句柄构造 `XiaoValueTag::Table`。
    pub fn xiao_runtime_value_table(handle: XiaoHandle) -> XiaoValue;
    /// 消费表强句柄并构造 `XiaoValueTag::Table`；调用成功后不得再次释放句柄。
    pub fn xiao_runtime_value_table_owned(handle: XiaoHandle) -> XiaoValue;

    /// 把一个固定宽度整数写到标准输出，返回 C 风格状态码。
    pub fn xiao_runtime_write_i64(value: i64) -> i32;
    /// 把一组值按空格分隔并追加换行后写到标准输出。
    pub fn xiao_runtime_print_values(values: *const XiaoValue, count: usize) -> i32;
    /// 可选提示后从标准输入读取一行；失败时返回 `none` 并写入挂起错误槽。
    pub fn xiao_runtime_input(prompt: *const XiaoValue, has_prompt: u8) -> XiaoValue;
}

#[cfg(test)]
/// ABI 布局和版本策略的单元测试。
mod tests {
    use super::{ABI_ENCODED_VERSION, ABI_MAJOR_VERSION, ABI_MINOR_VERSION, XiaoValue};

    #[test]
    /// 标签/载荷结构保持两个机器字大小，便于 LLVM 以固定布局传递。
    fn value_layout_is_two_machine_words() {
        assert_eq!(std::mem::size_of::<XiaoValue>(), 16);
        assert_eq!(std::mem::align_of::<XiaoValue>(), 8);
    }

    #[test]
    /// 版本编码能区分主版本并保留次版本比较空间。
    fn version_encoding_is_stable() {
        assert_eq!(ABI_ENCODED_VERSION, 0x0001_0008);
        assert_eq!(ABI_MAJOR_VERSION, 1);
        assert_eq!(ABI_MINOR_VERSION, 8);
    }

    #[test]
    /// 新布局与旧描述符分别锁定，避免新增回调意外改变旧 ABI。
    fn table_callback_layout_is_independent() {
        use super::{XiaoTableDescriptor, XiaoTableDescriptorV2, XiaoTableMethodDescriptor};
        use std::mem::{align_of, offset_of, size_of};
        assert_eq!(size_of::<XiaoTableDescriptor>(), 40);
        assert_eq!(size_of::<XiaoTableMethodDescriptor>(), 56);
        assert_eq!(offset_of!(XiaoTableMethodDescriptor, callback), 48);
        assert_eq!(size_of::<XiaoTableDescriptorV2>(), 88);
        assert_eq!(align_of::<XiaoTableDescriptorV2>(), 8);
        assert_eq!(offset_of!(XiaoTableDescriptorV2, fields), 8);
        assert_eq!(offset_of!(XiaoTableDescriptorV2, methods), 48);
        assert_eq!(offset_of!(XiaoTableDescriptorV2, drop), 80);
    }

    #[test]
    #[ignore = "需要 XIAO_CLANG；准备方式见 10D §4，交叉验证 MSVC/Linux/macOS C ABI 布局"]
    /// 使用实际 clang 三目标布局规则，回调只返回 i32，所有聚合值通过指针传递。
    fn table_callback_c_layout_matches_three_targets() {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let clang = std::env::var_os("XIAO_CLANG").expect("必须配置 XIAO_CLANG，见 10D §4");
        let source = r#"
typedef __SIZE_TYPE__ usize;
typedef unsigned int u32;
typedef unsigned long long u64;
typedef struct { u32 tag; u64 payload; } Value;
typedef struct { const unsigned char *ptr; usize len; } Bytes;
typedef int (*Callback)(const Value *, const Value *, usize, Value *);
typedef struct { Bytes name; u32 kind; const void *fields; usize count; } Old;
typedef struct { Bytes name; u64 signature; const u32 *types; usize count;
                 u32 result; unsigned char visible; Callback callback; } Method;
typedef struct { u32 size; u32 version; Old fields; const Method *methods;
                 usize count; Callback fields_init; Callback init; Callback drop; } New;
_Static_assert(sizeof(Value) == 16, "value");
_Static_assert(sizeof(Old) == 40, "old descriptor");
_Static_assert(sizeof(Method) == 56, "method");
_Static_assert(__builtin_offsetof(Method, callback) == 48, "callback offset");
_Static_assert(sizeof(New) == 88, "new descriptor");
_Static_assert(_Alignof(New) == 8, "alignment");
_Static_assert(__builtin_offsetof(New, fields) == 8, "old fields offset");
_Static_assert(__builtin_offsetof(New, methods) == 48, "methods offset");
_Static_assert(__builtin_offsetof(New, drop) == 80, "drop offset");
int invoke(Callback fn, const Value *receiver, const Value *args, usize count, Value *out) {
    return fn(receiver, args, count, out);
}
"#;
        for target in [
            "x86_64-pc-windows-msvc",
            "x86_64-unknown-linux-gnu",
            "aarch64-apple-darwin",
        ] {
            let mut child = Command::new(&clang)
                .args([
                    "-target",
                    target,
                    "-x",
                    "c",
                    "-std=c11",
                    "-fsyntax-only",
                    "-",
                ])
                .stdin(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("运行 clang");
            child
                .stdin
                .take()
                .expect("stdin")
                .write_all(source.as_bytes())
                .expect("写入 C 布局探针");
            let output = child.wait_with_output().expect("等待 clang");
            assert!(
                output.status.success(),
                "{target}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    /// 参数数量、顺序及返回标签均进入稳定签名，调用方不能用错误签名调用回调。
    fn table_signature_distinguishes_argument_and_result_types() {
        use super::table_method_signature_id as signature;
        assert_ne!(signature(&[1, 2], 3), signature(&[2, 1], 3));
        assert_ne!(signature(&[1], 2), signature(&[1, 2], 0));
        assert_ne!(signature(&[1], 2), signature(&[1], 3));
    }
}
