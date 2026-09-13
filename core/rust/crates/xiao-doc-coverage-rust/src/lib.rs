//! Rust 原生 AST 文档覆盖率适配器。
//!
//! 该 crate 是开发工具的内部组件。TypeScript 编排器通过命令行 JSON
//! 协议调用它，不应把这里的 Rust 数据结构当作 Xiao 语言公共 API。

use serde::{Deserialize, Serialize};
use syn::visit::Visit;

/// 当前 Rust AST 适配器 JSON 协议版本。
///
/// 请求和响应都必须携带这个版本号。版本升级时，调用方与适配器
/// 必须在同一变更中更新，并由兼容性测试覆盖。
pub const AST_PROTOCOL_VERSION: u32 = 1;

/// Rust 源文件的 AST 扫描请求。
#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    /// 调用方支持的协议版本。
    pub protocol_version: u32,
    /// 待扫描文件的仓库相对或绝对路径。
    pub files: Vec<String>,
}

/// 一个可计入文档覆盖率的 Rust 声明记录。
#[derive(Debug, Serialize)]
pub struct Declaration {
    /// 声明所在的源文件。
    pub file: String,
    /// 声明的第一行（从 1 开始）。
    pub line: usize,
    /// 声明类别，例如 `function` 或 `struct`。
    pub kind: String,
    /// 声明名称；匿名模块使用固定占位名。
    pub name: String,
    /// 是否为 crate 外可见的公共项。
    pub is_public: bool,
    /// 是否检测到直接关联的 Rustdoc。
    pub has_doc: bool,
}

/// 扫描输出，供 TypeScript 编排器反序列化。
#[derive(Debug, Serialize)]
pub struct ScanResponse {
    /// 适配器响应所使用的协议版本。
    pub protocol_version: u32,
    /// 成功解析出的声明。
    pub declarations: Vec<Declaration>,
    /// 无法解析的文件及稳定错误文本。
    pub errors: Vec<ScanError>,
}

/// 单个文件的解析错误。
#[derive(Debug, Serialize)]
pub struct ScanError {
    /// 出错文件。
    pub file: String,
    /// 稳定错误类别。
    pub code: String,
    /// 面向工具维护者的原始错误。
    pub message: String,
}

/// 扫描一批 Rust 文件并返回声明记录。
pub fn scan(request: &ScanRequest) -> ScanResponse {
    let mut response = ScanResponse {
        protocol_version: AST_PROTOCOL_VERSION,
        declarations: Vec::new(),
        errors: Vec::new(),
    };
    if request.protocol_version != AST_PROTOCOL_VERSION {
        response.errors.push(ScanError {
            file: String::new(),
            code: "A0-PROTOCOL-001".to_string(),
            message: format!(
                "不支持的 Rust AST 适配器协议版本：{}（当前支持 {}）",
                request.protocol_version, AST_PROTOCOL_VERSION
            ),
        });
        return response;
    }
    for file in &request.files {
        match std::fs::read_to_string(file) {
            Ok(source) => match syn::parse_file(&source) {
                Ok(parsed) => {
                    let mut visitor = DeclarationVisitor {
                        file: file.clone(),
                        declarations: Vec::new(),
                    };
                    visitor.declarations.push(Declaration {
                        file: file.clone(),
                        line: 1,
                        kind: "module".to_string(),
                        name: "crate".to_string(),
                        is_public: true,
                        has_doc: has_doc(&parsed.attrs),
                    });
                    visitor.visit_file(&parsed);
                    response.declarations.extend(visitor.declarations);
                }
                Err(error) => response.errors.push(ScanError {
                    file: file.clone(),
                    code: "A0-PARSER-001".to_string(),
                    message: error.to_string(),
                }),
            },
            Err(error) => response.errors.push(ScanError {
                file: file.clone(),
                code: "A0-PARSER-002".to_string(),
                message: error.to_string(),
            }),
        }
    }
    response
}

/// 遍历 Rust AST 并收集声明记录的内部访问器。
struct DeclarationVisitor {
    file: String,
    declarations: Vec<Declaration>,
}

impl<'ast> Visit<'ast> for DeclarationVisitor {
    /// 收集函数声明并继续遍历函数体。
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.push(
            "function",
            &item.sig.ident.to_string(),
            item.sig.ident.span().start().line,
            &item.vis,
            &item.attrs,
        );
        syn::visit::visit_item_fn(self, item);
    }

    /// 收集结构体声明并继续遍历字段。
    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.push(
            "struct",
            &item.ident.to_string(),
            item.ident.span().start().line,
            &item.vis,
            &item.attrs,
        );
        syn::visit::visit_item_struct(self, item);
    }

    /// 收集枚举声明并继续遍历成员。
    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.push(
            "enum",
            &item.ident.to_string(),
            item.ident.span().start().line,
            &item.vis,
            &item.attrs,
        );
        syn::visit::visit_item_enum(self, item);
    }

    /// 收集 trait 声明并继续遍历方法。
    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.push(
            "trait",
            &item.ident.to_string(),
            item.ident.span().start().line,
            &item.vis,
            &item.attrs,
        );
        syn::visit::visit_item_trait(self, item);
    }

    /// 收集嵌套模块声明。
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        self.push(
            "module",
            &item.ident.to_string(),
            item.ident.span().start().line,
            &item.vis,
            &item.attrs,
        );
        syn::visit::visit_item_mod(self, item);
    }

    /// 收集常量声明。
    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        self.push(
            "constant",
            &item.ident.to_string(),
            item.ident.span().start().line,
            &item.vis,
            &item.attrs,
        );
        syn::visit::visit_item_const(self, item);
    }

    /// 收集静态变量声明。
    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        self.push(
            "static",
            &item.ident.to_string(),
            item.ident.span().start().line,
            &item.vis,
            &item.attrs,
        );
        syn::visit::visit_item_static(self, item);
    }

    /// 收集类型别名声明。
    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.push(
            "type",
            &item.ident.to_string(),
            item.ident.span().start().line,
            &item.vis,
            &item.attrs,
        );
        syn::visit::visit_item_type(self, item);
    }

    /// 收集联合体声明。
    fn visit_item_union(&mut self, item: &'ast syn::ItemUnion) {
        self.push(
            "union",
            &item.ident.to_string(),
            item.ident.span().start().line,
            &item.vis,
            &item.attrs,
        );
        syn::visit::visit_item_union(self, item);
    }

    /// 收集公共 use 重导出。
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if matches!(&item.vis, syn::Visibility::Public(_)) {
            self.push(
                "reexport",
                "use",
                item.use_token.span.start().line,
                &item.vis,
                &item.attrs,
            );
        }
        syn::visit::visit_item_use(self, item);
    }

    /// 收集 trait 方法声明。
    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        self.push(
            "method",
            &item.sig.ident.to_string(),
            item.sig.ident.span().start().line,
            &syn::Visibility::Inherited,
            &item.attrs,
        );
        syn::visit::visit_trait_item_fn(self, item);
    }

    /// 收集带名称的公共字段。
    fn visit_field(&mut self, field: &'ast syn::Field) {
        if let Some(identifier) = field.ident.as_ref()
            && matches!(&field.vis, syn::Visibility::Public(_))
        {
            let name = identifier.to_string();
            self.push(
                "field",
                &name,
                identifier.span().start().line,
                &field.vis,
                &field.attrs,
            );
        }
        syn::visit::visit_field(self, field);
    }

    /// 收集实现块中的方法。
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.push(
            "method",
            &item.sig.ident.to_string(),
            item.sig.ident.span().start().line,
            &item.vis,
            &item.attrs,
        );
        syn::visit::visit_impl_item_fn(self, item);
    }
}

impl DeclarationVisitor {
    /// 将一个 AST 项转换为统一声明记录。
    fn push(
        &mut self,
        kind: &str,
        name: &str,
        line: usize,
        visibility: &syn::Visibility,
        attrs: &[syn::Attribute],
    ) {
        self.declarations.push(Declaration {
            file: self.file.clone(),
            line,
            kind: kind.to_string(),
            name: name.to_string(),
            is_public: matches!(visibility, syn::Visibility::Public(_)),
            has_doc: has_doc(attrs),
        });
    }
}

/// 判断属性列表是否包含非空 `doc` 属性。
fn has_doc(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attribute| match &attribute.meta {
        syn::Meta::NameValue(value) if value.path.is_ident("doc") => {
            matches!(&value.value, syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(text), .. }) if !text.value().trim().is_empty())
        }
        _ => false,
    })
}

#[cfg(test)]
/// 覆盖 Rust AST 适配器核心行为的测试模块。
mod tests {
    use super::{AST_PROTOCOL_VERSION, ScanRequest, scan};

    /// 确认公共和私有函数都能被发现且公共文档状态正确。
    #[test]
    fn parses_public_and_private_items() {
        let path = std::env::temp_dir().join("xiao-doc-coverage-rust-test.rs");
        std::fs::write(
            &path,
            "/// documented\npub fn visible() {}\n#[doc = \"same line\"] pub fn inline_documented() {}\nfn hidden() {}\n",
        )
        .expect("write fixture");
        let response = scan(&ScanRequest {
            protocol_version: AST_PROTOCOL_VERSION,
            files: vec![path.to_string_lossy().into_owned()],
        });
        assert!(response.errors.is_empty());
        assert_eq!(response.protocol_version, AST_PROTOCOL_VERSION);
        assert_eq!(response.declarations.len(), 4);
        assert!(
            response
                .declarations
                .iter()
                .any(|item| item.is_public && item.has_doc)
        );
        assert!(
            response
                .declarations
                .iter()
                .any(|item| item.name == "inline_documented" && item.is_public && item.has_doc)
        );
        let _ = std::fs::remove_file(path);
    }

    /// 确认协议版本不匹配时不会扫描源文件，并返回稳定错误码。
    #[test]
    fn rejects_unsupported_protocol_version() {
        let response = scan(&ScanRequest {
            protocol_version: AST_PROTOCOL_VERSION + 1,
            files: Vec::new(),
        });
        assert_eq!(response.protocol_version, AST_PROTOCOL_VERSION);
        assert!(response.declarations.is_empty());
        assert_eq!(
            response.errors.first().map(|item| item.code.as_str()),
            Some("A0-PROTOCOL-001")
        );
    }

    /// 确认 Rust 语法错误会被报告而不是被当作空文件通过。
    #[test]
    fn reports_parse_errors() {
        let path = std::env::temp_dir().join("xiao-doc-coverage-rust-invalid.rs");
        std::fs::write(&path, "pub fn broken( {}").expect("write fixture");
        let response = scan(&ScanRequest {
            protocol_version: AST_PROTOCOL_VERSION,
            files: vec![path.to_string_lossy().into_owned()],
        });
        assert_eq!(
            response.errors.first().map(|item| item.code.as_str()),
            Some("A0-PARSER-001")
        );
        assert!(response.declarations.is_empty());
        let _ = std::fs::remove_file(path);
    }
}
