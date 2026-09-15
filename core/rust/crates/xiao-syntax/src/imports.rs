//! 05-A 导入语句的语法数据结构。
//!
//! 本模块只保存导入路径、别名和源码区间，不解析文件、不建立模块图，也不
//! 执行名称绑定。文件系统和跨文件解析由 `xiao-modules` 独立负责。

use xiao_source::SourceSpan;

use crate::ast::Name;

/// 一个由点号分隔的绝对模块路径。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportPath {
    /// 按源码顺序保存的路径段。
    pub segments: Vec<Name>,
    /// 覆盖全部路径段的源码区间。
    pub span: SourceSpan,
}

impl ImportPath {
    /// 创建一个模块路径。
    #[must_use]
    pub const fn new(segments: Vec<Name>, span: SourceSpan) -> Self {
        Self { segments, span }
    }

    /// 返回路径的源码区间。
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// 判断路径是否为空。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

/// `import module.path [as alias]` 的一个导入项。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleImport {
    /// 被导入的绝对模块或命名空间路径。
    pub path: ImportPath,
    /// 可选的本地别名。
    pub alias: Option<Name>,
    /// 覆盖路径和别名的源码区间。
    pub span: SourceSpan,
}

impl ModuleImport {
    /// 返回导入项的源码区间。
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// `from module.path import name [as alias]` 的一个选择项。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedImport {
    /// 目标模块或命名空间中的名称。
    pub name: Name,
    /// 可选的本地别名。
    pub alias: Option<Name>,
    /// 覆盖名称和别名的源码区间。
    pub span: SourceSpan,
}

impl SelectedImport {
    /// 返回选择导入项的源码区间。
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
}

/// 一条完整导入语句。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportStatement {
    /// 一个或多个模块/命名空间导入项。
    Modules {
        /// 按源码顺序保存导入项。
        imports: Vec<ModuleImport>,
        /// 覆盖整条语句的源码区间。
        span: SourceSpan,
    },
    /// 从一个模块或命名空间选择一个或多个名称。
    From {
        /// 目标模块或命名空间路径。
        module: ImportPath,
        /// 按源码顺序保存选择项。
        imports: Vec<SelectedImport>,
        /// 覆盖整条语句的源码区间。
        span: SourceSpan,
    },
}

impl ImportStatement {
    /// 返回整条导入语句的源码区间。
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        match self {
            Self::Modules { span, .. } | Self::From { span, .. } => *span,
        }
    }
}
