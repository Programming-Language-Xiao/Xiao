//! Rust 原生 AST 文档覆盖率适配器。
//!
//! 该 crate 是开发工具的内部组件。TypeScript 编排器通过命令行 JSON
//! 协议调用它，不应把这里的 Rust 数据结构当作 Xiao 语言公共 API。

use proc_macro2::Span;
use serde::{Deserialize, Serialize};
use syn::spanned::Spanned;
use syn::visit::Visit;

/// 当前 Rust AST 适配器 JSON 协议版本。
///
/// 请求和响应都必须携带这个版本号。版本升级时，调用方与适配器
/// 必须在同一变更中更新，并由兼容性测试覆盖。
pub const AST_PROTOCOL_VERSION: u32 = 2;

/// Rust 源文件的 AST 扫描请求。
#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    /// 调用方支持的协议版本。
    pub protocol_version: u32,
    /// 待扫描文件的仓库相对或绝对路径。
    pub files: Vec<String>,
    /// 是否在响应中返回结构大纲。
    ///
    /// 缺省为 `false`：大纲的体积远大于声明，只有需要判断文件结构时
    /// 才值得付出生成和传输它的代价。
    #[serde(default)]
    pub outline: bool,
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
    /// 声明自身所覆盖的最后一行（从 1 开始）。
    ///
    /// 取整个项的跨度终点；与只定位声明名称的 `line` 配对后，
    /// `end_line - line + 1` 就是从声明自身起始行到项尾的行数。
    pub end_line: usize,
}

/// 一个文件的结构大纲。
#[derive(Debug, Serialize)]
pub struct FileOutline {
    /// 大纲所属的源文件。
    pub file: String,
    /// 该文件的顶层项。
    pub nodes: Vec<OutlineNode>,
}

/// 大纲中的一个结构化节点。
///
/// 与 [`Declaration`] 的分工是刻意的：`Declaration` 只服务文档覆盖率，是**扁平**的、
/// 且 `impl` 块自身不入册；大纲则要**递归**到字段、变体、方法与嵌套项，供
/// 「文件过长必须解耦」这类判断使用。两者不能合并——把 `impl` 塞进覆盖率口径
/// 会平白改变一个已验证的数字。
#[derive(Debug, Serialize)]
pub struct OutlineNode {
    /// 节点类别，例如 `function`、`field` 或 `variant`。
    pub kind: String,
    /// 节点名称；`impl` 块用自身类型路径。
    pub name: String,
    /// 起始行（从 1 开始）。
    pub line: usize,
    /// 结束行（从 1 开始）。
    pub end_line: usize,
    /// 从声明自身起始行到项尾的覆盖行数（含首尾，不含上方属性或文档注释）。
    ///
    /// 该字段满足 `line + lines - 1 == end_line`；子节点不会改变这个口径。
    pub lines: usize,
    /// 定义句：**省去名字**的声明头，例如 `fn() -> SourceSpan`。
    ///
    /// 名字已经单独给出，签名里再重复一次只是噪声。
    pub signature: String,
    /// 声明起始行的源码文本（已去除首尾空白），便于人工核对。
    pub source_line: String,
    /// 子节点：字段、变体、方法、嵌套项。
    pub children: Vec<OutlineNode>,
}

/// 扫描输出，供 TypeScript 编排器反序列化。
#[derive(Debug, Serialize)]
pub struct ScanResponse {
    /// 适配器响应所使用的协议版本。
    pub protocol_version: u32,
    /// 成功解析出的声明。
    pub declarations: Vec<Declaration>,
    /// 每个成功解析文件的结构大纲。
    pub outlines: Vec<FileOutline>,
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
        outlines: Vec::new(),
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
                    let lines = source.lines().collect::<Vec<_>>();
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
                        end_line: lines.len().max(1),
                    });
                    visitor.visit_file(&parsed);
                    response.declarations.extend(visitor.declarations);
                    if request.outline {
                        response.outlines.push(FileOutline {
                            file: file.clone(),
                            nodes: outline_items(&parsed.items, &lines),
                        });
                    }
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
            item.sig.ident.span(),
            item.span(),
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
            item.ident.span(),
            item.span(),
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
            item.ident.span(),
            item.span(),
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
            item.ident.span(),
            item.span(),
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
            item.ident.span(),
            item.span(),
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
            item.ident.span(),
            item.span(),
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
            item.ident.span(),
            item.span(),
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
            item.ident.span(),
            item.span(),
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
            item.ident.span(),
            item.span(),
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
                item.use_token.span,
                item.span(),
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
            item.sig.ident.span(),
            item.span(),
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
                identifier.span(),
                field.span(),
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
            item.sig.ident.span(),
            item.span(),
            &item.vis,
            &item.attrs,
        );
        syn::visit::visit_impl_item_fn(self, item);
    }
}

impl DeclarationVisitor {
    /// 将一个 AST 项转换为统一声明记录。
    ///
    /// `header` 只覆盖声明自身的头部（通常是标识符），用于定位 `line`；
    /// `full` 覆盖整个项，用于计算真实的 `end_line`。两者不能合并：
    /// 整项跨度的起点会把上方属性和 `///` 文档注释算进定位行。
    fn push(
        &mut self,
        kind: &str,
        name: &str,
        header: Span,
        full: Span,
        visibility: &syn::Visibility,
        attrs: &[syn::Attribute],
    ) {
        self.declarations.push(Declaration {
            file: self.file.clone(),
            line: header.start().line,
            kind: kind.to_string(),
            name: name.to_string(),
            is_public: matches!(visibility, syn::Visibility::Public(_)),
            has_doc: has_doc(attrs),
            end_line: full.end().line,
        });
    }
}

/// 递归构建一组项的结构大纲。
fn outline_items(items: &[syn::Item], lines: &[&str]) -> Vec<OutlineNode> {
    items
        .iter()
        .filter_map(|item| outline_item(item, lines))
        .collect()
}

/// 把一个 AST 项转换为大纲节点；不产出的项返回 `None`。
///
/// 以 `_` 收尾是**有意的**：大纲是尽力而为的辅助输出，遇到未列举的项跳过即可，
/// 不应该因为它而让整个扫描失败。覆盖率声明那条路径则仍然是穷尽的。
fn outline_item(item: &syn::Item, lines: &[&str]) -> Option<OutlineNode> {
    // `header` 是**声明自身**的起点。整项跨度会把属性算进去——`///` 就是 `#[doc]`
    // 属性，用整项起点会让「所在行」指向文档注释而不是声明，定位就错了。
    let (kind, name, header, children) = match item {
        syn::Item::Fn(inner) => (
            "function",
            inner.sig.ident.to_string(),
            inner.sig.ident.span(),
            Vec::new(),
        ),
        syn::Item::Struct(inner) => (
            "struct",
            inner.ident.to_string(),
            inner.ident.span(),
            outline_fields(inner.fields.iter(), lines),
        ),
        syn::Item::Enum(inner) => (
            "enum",
            inner.ident.to_string(),
            inner.ident.span(),
            outline_variants(&inner.variants, lines),
        ),
        syn::Item::Union(inner) => (
            "union",
            inner.ident.to_string(),
            inner.ident.span(),
            outline_fields(inner.fields.named.iter(), lines),
        ),
        syn::Item::Trait(inner) => (
            "trait",
            inner.ident.to_string(),
            inner.ident.span(),
            outline_trait_items(&inner.items, lines),
        ),
        syn::Item::Impl(inner) => (
            "impl",
            quote_like(&inner.self_ty),
            inner.impl_token.span,
            outline_impl_items(&inner.items, lines),
        ),
        syn::Item::Mod(inner) => (
            "module",
            inner.ident.to_string(),
            inner.ident.span(),
            inner
                .content
                .as_ref()
                .map(|(_, items)| outline_items(items, lines))
                .unwrap_or_default(),
        ),
        syn::Item::Const(inner) => (
            "constant",
            inner.ident.to_string(),
            inner.ident.span(),
            Vec::new(),
        ),
        syn::Item::Static(inner) => (
            "static",
            inner.ident.to_string(),
            inner.ident.span(),
            Vec::new(),
        ),
        syn::Item::Type(inner) => (
            "type",
            inner.ident.to_string(),
            inner.ident.span(),
            Vec::new(),
        ),
        // `use` 不属于大纲要回答的问题（结构/字段/函数/枚举/对象/成员），列出来只是噪声。
        _ => return None,
    };
    Some(node(kind, name, header, item.span(), lines, children))
}

/// 按「声明自身起点 + 整项跨度」组装一个节点。
fn node(
    kind: &str,
    name: String,
    header: Span,
    full: Span,
    lines: &[&str],
    children: Vec<OutlineNode>,
) -> OutlineNode {
    let source_line = line_text(lines, header.start().line);
    OutlineNode {
        signature: declaration_head(&source_line, &name),
        kind: kind.to_string(),
        name,
        line: header.start().line,
        end_line: full.end().line,
        lines: full.end().line.saturating_sub(header.start().line) + 1,
        source_line,
        children,
    }
}

/// 构建结构体、联合体或变体的字段节点。
///
/// 接收迭代器而不是 `&Fields`：`ItemUnion` 的字段是 `FieldsNamed`，
/// 两者没有共同的引用形态。
fn outline_fields<'a>(
    fields: impl Iterator<Item = &'a syn::Field>,
    lines: &[&str],
) -> Vec<OutlineNode> {
    fields
        .filter_map(|field| {
            let identifier = field.ident.as_ref()?;
            Some(node(
                "field",
                identifier.to_string(),
                identifier.span(),
                field.span(),
                lines,
                Vec::new(),
            ))
        })
        .collect()
}

/// 构建枚举变体节点，并下钻到变体自身的字段。
fn outline_variants(
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::Token![,]>,
    lines: &[&str],
) -> Vec<OutlineNode> {
    variants
        .iter()
        .map(|variant| {
            node(
                "variant",
                variant.ident.to_string(),
                variant.ident.span(),
                variant.span(),
                lines,
                outline_fields(variant.fields.iter(), lines),
            )
        })
        .collect()
}

/// 构建 trait 项节点。
fn outline_trait_items(items: &[syn::TraitItem], lines: &[&str]) -> Vec<OutlineNode> {
    items
        .iter()
        .filter_map(|item| {
            let (kind, identifier, full) = match item {
                syn::TraitItem::Fn(inner) => ("method", inner.sig.ident.clone(), inner.span()),
                syn::TraitItem::Const(inner) => ("constant", inner.ident.clone(), inner.span()),
                syn::TraitItem::Type(inner) => ("type", inner.ident.clone(), inner.span()),
                _ => return None,
            };
            Some(node(
                kind,
                identifier.to_string(),
                identifier.span(),
                full,
                lines,
                Vec::new(),
            ))
        })
        .collect()
}

/// 构建实现块内的关联项节点。
fn outline_impl_items(items: &[syn::ImplItem], lines: &[&str]) -> Vec<OutlineNode> {
    items
        .iter()
        .filter_map(|item| {
            let (kind, identifier, full) = match item {
                syn::ImplItem::Fn(inner) => ("method", inner.sig.ident.clone(), inner.span()),
                syn::ImplItem::Const(inner) => ("constant", inner.ident.clone(), inner.span()),
                syn::ImplItem::Type(inner) => ("type", inner.ident.clone(), inner.span()),
                _ => return None,
            };
            Some(node(
                kind,
                identifier.to_string(),
                identifier.span(),
                full,
                lines,
                Vec::new(),
            ))
        })
        .collect()
}

/// 取指定行的源码文本（已去除首尾空白）；越界返回空串。
fn line_text(lines: &[&str], line: usize) -> String {
    if line == 0 {
        return String::new();
    }
    lines
        .get(line - 1)
        .map(|text| text.trim().to_string())
        .unwrap_or_default()
}

/// 由起始源码行派生「定义句」：声明头去掉名字。
///
/// 例：`fn span() -> SourceSpan {` + 名字 `span` → `fn() -> SourceSpan`。
/// 名字既已单列，签名里再重复一次只是噪声。取到第一个 `{` / `;` 为止，
/// 因此多行签名的节点只会给出起始行上的那一段——`line`/`end_line` 才是判断大小的依据。
fn declaration_head(source_line: &str, name: &str) -> String {
    let head = source_line
        .split(['{', ';'])
        .next()
        .unwrap_or(source_line)
        .trim();
    let without_name = match head.find(name) {
        Some(index) => {
            let mut text = String::with_capacity(head.len());
            text.push_str(&head[..index]);
            text.push_str(&head[index + name.len()..]);
            text
        }
        None => head.to_string(),
    };
    without_name
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace("fn (", "fn(")
        .trim_end_matches([',', ':'])
        .trim()
        .to_string()
}

/// 把类型节点渲染成紧凑文本，用于 `impl` 块的名称。
///
/// 只处理 `impl` 自身类型这一种用途，不做完整类型打印——完整打印需要 `quote`，
/// 而这里没有引入新依赖的必要时会退化成源码切片，得不偿失。
fn quote_like(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_else(|| "impl".to_string()),
        _ => "impl".to_string(),
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
    use super::{AST_PROTOCOL_VERSION, ScanRequest, declaration_head, scan};

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
            outline: false,
        });
        assert!(response.errors.is_empty());
        assert_eq!(response.protocol_version, AST_PROTOCOL_VERSION);
        assert_eq!(response.declarations.len(), 4);
        assert!(response.outlines.is_empty());
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
            outline: false,
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
            outline: false,
        });
        assert_eq!(
            response.errors.first().map(|item| item.code.as_str()),
            Some("A0-PARSER-001")
        );
        assert!(response.declarations.is_empty());
        let _ = std::fs::remove_file(path);
    }

    /// 确认省略请求开关时按协议约定默认为关闭，并始终返回空大纲数组。
    #[test]
    fn outline_defaults_to_false() {
        let request: ScanRequest =
            serde_json::from_str(r#"{"protocol_version":2,"files":[]}"#).expect("parse request");
        assert!(!request.outline);
        let response = scan(&request);
        assert!(response.outlines.is_empty());
    }

    /// 确认声明的起始行定位到名称，而结束行取整个项的真实末尾。
    #[test]
    fn declaration_end_line_uses_full_item_span() {
        let path = std::env::temp_dir().join("xiao-doc-coverage-rust-multi-line.rs");
        std::fs::write(
            &path,
            "/// documented\npub fn multi_line(\n    value: usize,\n) -> usize {\n    value\n}\n",
        )
        .expect("write fixture");
        let response = scan(&ScanRequest {
            protocol_version: AST_PROTOCOL_VERSION,
            files: vec![path.to_string_lossy().into_owned()],
            outline: false,
        });
        let declaration = response
            .declarations
            .iter()
            .find(|item| item.name == "multi_line")
            .expect("find declaration");
        assert_eq!(declaration.line, 2);
        assert_eq!(declaration.end_line, 6);
        assert!(declaration.end_line > declaration.line);
        let _ = std::fs::remove_file(path);
    }

    /// 确认请求大纲时节点行数与起止行保持同一口径，不把上方文档注释算入其中。
    #[test]
    fn outline_lines_match_line_and_end_line() {
        let path = std::env::temp_dir().join("xiao-doc-coverage-rust-outline-lines.rs");
        std::fs::write(
            &path,
            "/// documented\npub struct Sample {\n    pub value: usize,\n}\n",
        )
        .expect("write fixture");
        let response = scan(&ScanRequest {
            protocol_version: AST_PROTOCOL_VERSION,
            files: vec![path.to_string_lossy().into_owned()],
            outline: true,
        });
        let node = response
            .outlines
            .first()
            .and_then(|outline| outline.nodes.iter().find(|node| node.name == "Sample"))
            .expect("find outline node");
        assert_eq!(node.line, 2);
        assert_eq!(node.end_line, 4);
        assert_eq!(node.lines, 3);
        assert_eq!(node.line + node.lines - 1, node.end_line);
        let _ = std::fs::remove_file(path);
    }

    /// 锁定函数、静态量、结构体和多行签名的定义句截取规则。
    #[test]
    fn declaration_head_covers_common_shapes() {
        assert_eq!(
            declaration_head("pub fn render(value: usize) -> usize {", "render"),
            "pub fn(value: usize) -> usize"
        );
        assert_eq!(
            declaration_head(
                "static DROP_CALLS: AtomicUsize = AtomicUsize::new(0);",
                "DROP_CALLS"
            ),
            "static : AtomicUsize = AtomicUsize::new(0)"
        );
        assert_eq!(
            declaration_head("pub struct Sample {", "Sample"),
            "pub struct"
        );
        assert_eq!(
            declaration_head("pub fn multi_line(", "multi_line"),
            "pub fn("
        );
    }
}
