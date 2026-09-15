//! 04 阶段函数签名与参数匹配模型。
//!
//! 本模块只保存可供类型检查器、统一前端和后续 IR 消费的静态函数信息。
//! 它不执行函数，也不持有 Runtime 闭包或调用栈；参数绑定种类复用语法层
//! 的稳定枚举，但类型值仍完全由 `xiao-types` 管理。

use xiao_source::SourceSpan;
use xiao_syntax::FunctionParameterKind;

use crate::types::Type;

/// 一个已经登记的函数参数静态签名。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionParameterSignature {
    /// 参数显示名称（保留规范化后的字符串）。
    pub name: String,
    /// 参数传递种类。
    pub kind: FunctionParameterKind,
    /// 参数类型；未注解参数在推断期间可以是类型变量。
    pub ty: Type,
    /// 是否声明了默认值。
    pub has_default: bool,
}

/// 一个函数的静态签名旁路记录。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionSignature {
    /// 函数规范化名称。
    pub name: String,
    /// 按源码顺序排列的参数签名。
    pub parameters: Vec<FunctionParameterSignature>,
    /// 推断或显式声明的返回类型。
    pub return_type: Type,
    /// 函数定义源码区间。
    pub span: SourceSpan,
}

impl FunctionSignature {
    /// 创建一份函数签名记录。
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        parameters: impl Into<Vec<FunctionParameterSignature>>,
        return_type: Type,
        span: SourceSpan,
    ) -> Self {
        Self {
            name: name.into(),
            parameters: parameters.into(),
            return_type,
            span,
        }
    }

    /// 返回是否声明了可变位置参数。
    #[must_use]
    pub fn has_varargs(&self) -> bool {
        self.parameters
            .iter()
            .any(|parameter| parameter.kind == FunctionParameterKind::VarArgs)
    }

    /// 返回是否声明了可变关键字参数。
    #[must_use]
    pub fn has_varkwargs(&self) -> bool {
        self.parameters
            .iter()
            .any(|parameter| parameter.kind == FunctionParameterKind::VarKeywords)
    }

    /// 返回指定名称的关键字参数位置。
    #[must_use]
    pub fn named_parameter(&self, name: &str) -> Option<(usize, &FunctionParameterSignature)> {
        self.parameters
            .iter()
            .enumerate()
            .find(|(_, parameter)| parameter.name == name)
    }
}
