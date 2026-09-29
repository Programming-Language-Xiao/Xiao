//! Xiao 的结构化消息目录、不可变语言上下文和安全回退渲染。

use std::collections::BTreeMap;

/// 内置消息目录的格式版本。
pub const CATALOG_VERSION: u16 = 1;

/// 一条占位参数所允许的类型。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParamKind {
    /// 不翻译的文本值。
    Text,
    /// 十进制整数。
    Integer,
    /// 布尔值。
    Boolean,
}

/// 随消息身份保存的未本地化参数。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MessageParam {
    /// 不翻译的文本值。
    Text(String),
    /// 十进制整数。
    Integer(i128),
    /// 布尔值。
    Boolean(bool),
}

impl MessageParam {
    fn kind(&self) -> ParamKind {
        match self {
            Self::Text(_) => ParamKind::Text,
            Self::Integer(_) => ParamKind::Integer,
            Self::Boolean(_) => ParamKind::Boolean,
        }
    }

    fn safe_text(&self) -> String {
        match self {
            Self::Text(value) => format!("{value:?}"),
            Self::Integer(value) => value.to_string(),
            Self::Boolean(value) => value.to_string(),
        }
    }

    fn display_text(&self) -> String {
        match self {
            Self::Text(value) => value.clone(),
            Self::Integer(value) => value.to_string(),
            Self::Boolean(value) => value.to_string(),
        }
    }
}

/// 单个目录条目及其需要的参数签名。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MessageTemplate {
    /// 不随语言改变的消息身份。
    pub id: String,
    /// 只使用 `{name}` 形式的安全插值模板。
    pub text: String,
    /// 需要的参数及其类型。
    pub params: BTreeMap<String, ParamKind>,
}

/// 一套可校验的语言目录。
#[derive(Clone, Debug)]
pub struct Catalog {
    /// 规范化语言标签。
    pub locale: String,
    /// 目录格式版本。
    pub version: u16,
    entries: BTreeMap<String, MessageTemplate>,
}

impl Catalog {
    /// 拒绝重复消息身份、无效格式和不匹配的模板签名。
    pub fn new(locale: impl Into<String>, entries: Vec<MessageTemplate>) -> Result<Self, String> {
        let locale = normalize_locale_tag(locale.into());
        let mut registered = BTreeMap::new();
        for entry in entries {
            if entry.id.is_empty() || entry.text.is_empty() {
                return Err("消息身份和模板不得为空".to_owned());
            }
            let fields = template_fields(&entry.text)?;
            let declared = entry.params.keys().cloned().collect::<std::collections::BTreeSet<_>>();
            if !fields.is_subset(&declared) {
                return Err(format!("消息 {} 的参数签名与模板不一致", entry.id));
            }
            if registered.insert(entry.id.clone(), entry).is_some() {
                return Err("消息身份重复".to_owned());
            }
        }
        Ok(Self {
            locale,
            version: CATALOG_VERSION,
            entries: registered,
        })
    }

    fn get(&self, id: &str) -> Option<&MessageTemplate> {
        self.entries.get(id)
    }
}

/// 一个入口在运行期间共享的不可变有效语言。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocaleContext {
    locale: String,
}

impl LocaleContext {
    /// 用已规范化的标签建立运行上下文。
    #[must_use]
    pub fn new(locale: impl Into<String>) -> Self {
        Self {
            locale: normalize_locale_tag(locale.into()),
        }
    }

    /// 校验配置，只接受首批提供的两种语言及其别名。
    pub fn from_config(value: &str) -> Result<Self, &'static str> {
        match value.to_ascii_lowercase().as_str() {
            "zh" | "zh-cn" | "en" | "en-us" => Ok(Self::new(value)),
            _ => Err("X11-CONFIG-002"),
        }
    }

    /// 返回稳定的规范语言标签。
    #[must_use]
    pub fn tag(&self) -> &str {
        &self.locale
    }
}

fn normalize_locale_tag(locale: String) -> String {
    match locale.to_ascii_lowercase().as_str() {
        "zh" | "zh-cn" => "zh-CN".to_owned(),
        "en" | "en-us" => "en-US".to_owned(),
        _ => locale,
    }
}

impl Default for LocaleContext {
    fn default() -> Self {
        Self::new("zh-CN")
    }
}

/// 已使用的回退级别。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fallback {
    /// 精确标签。
    Exact,
    /// 同一种基础语言。
    Base,
    /// 内置英语参考目录。
    English,
    /// 保留消息身份与安全转义的原始参数。
    Identity,
}

/// 目录渲染结果；调用方保留自己的错误码和机器字段。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedMessage {
    /// 最终的人类可读文本。
    pub text: String,
    /// 目录查找结果。
    pub fallback: Fallback,
    /// 参数类型或模板格式错误时为真。
    pub format_failed: bool,
}

/// 所有组件共享的无副作用渲染器。
pub struct MessageRenderer {
    catalogs: BTreeMap<String, Catalog>,
}

impl MessageRenderer {
    /// 检查相同消息身份在所有目录中的参数类型一致。
    pub fn new(catalogs: Vec<Catalog>) -> Result<Self, String> {
        let mut registered = BTreeMap::new();
        let mut signatures = BTreeMap::new();
        for catalog in catalogs {
            if catalog.version != CATALOG_VERSION || registered.contains_key(&catalog.locale) {
                return Err("目录版本或语言标签冲突".to_owned());
            }
            for (id, entry) in &catalog.entries {
                if signatures
                    .insert(id.clone(), entry.params.clone())
                    .is_some_and(|previous| previous != entry.params)
                {
                    return Err(format!("消息 {id} 的跨语言参数签名不一致"));
                }
            }
            registered.insert(catalog.locale.clone(), catalog);
        }
        Ok(Self {
            catalogs: registered,
        })
    }

    /// 按精确标签、基础语言、英语目录和消息身份的顺序安全渲染。
    #[must_use]
    pub fn render(
        &self,
        locale: &LocaleContext,
        id: &str,
        params: &BTreeMap<String, MessageParam>,
    ) -> RenderedMessage {
        let base = locale.tag().split('-').next().unwrap_or_default();
        let base_catalog = match base {
            "zh" => "zh-CN",
            "en" => "en-US",
            _ => base,
        };
        for (tag, fallback) in [
            (locale.tag(), Fallback::Exact),
            (base_catalog, Fallback::Base),
            ("en-US", Fallback::English),
        ] {
            if let Some(entry) = self.catalogs.get(tag).and_then(|catalog| catalog.get(id)) {
                return match interpolate(entry, params) {
                    Ok(text) => RenderedMessage {
                        text,
                        fallback,
                        format_failed: false,
                    },
                    Err(_) => RenderedMessage {
                        text: identity_text(id, params),
                        fallback: Fallback::Identity,
                        format_failed: true,
                    },
                };
            }
        }
        RenderedMessage {
            text: identity_text(id, params),
            fallback: Fallback::Identity,
            format_failed: false,
        }
    }
}

fn identity_text(id: &str, params: &BTreeMap<String, MessageParam>) -> String {
    if params.is_empty() {
        return id.to_owned();
    }
    let values = params
        .iter()
        .map(|(key, value)| format!("{key}={}", value.safe_text()))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{id} ({values})")
}

fn template_fields(template: &str) -> Result<std::collections::BTreeSet<String>, String> {
    let mut result = std::collections::BTreeSet::new();
    let mut remaining = template;
    while let Some(start) = remaining.find(['{', '}']) {
        if remaining.as_bytes()[start] == b'}' {
            return Err("模板含孤立的闭合括号".to_owned());
        }
        let suffix = &remaining[start + 1..];
        let Some(end) = suffix.find('}') else {
            return Err("模板含未闭合的参数".to_owned());
        };
        let field = &suffix[..end];
        if field.is_empty()
            || !field
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            return Err("模板参数名无效".to_owned());
        }
        result.insert(field.to_owned());
        remaining = &suffix[end + 1..];
    }
    Ok(result)
}

fn interpolate(
    entry: &MessageTemplate,
    params: &BTreeMap<String, MessageParam>,
) -> Result<String, ()> {
    if params.len() != entry.params.len()
        || entry
            .params
            .iter()
            .any(|(name, kind)| params.get(name).is_none_or(|value| value.kind() != *kind))
    {
        return Err(());
    }
    let mut text = String::new();
    let mut remaining = entry.text.as_str();
    while let Some(start) = remaining.find('{') {
        text.push_str(&remaining[..start]);
        let suffix = &remaining[start + 1..];
        let end = suffix.find('}').ok_or(())?;
        text.push_str(&params.get(&suffix[..end]).ok_or(())?.display_text());
        remaining = &suffix[end + 1..];
    }
    text.push_str(remaining);
    Ok(text)
}

fn template(
    id: &str,
    text: &str,
    params: &[(&str, ParamKind)],
) -> MessageTemplate {
    MessageTemplate {
        id: id.to_owned(),
        text: text.to_owned(),
        params: params
            .iter()
            .map(|(name, kind)| ((*name).to_owned(), *kind))
            .collect(),
    }
}

fn add_entry(
    chinese: &mut Vec<MessageTemplate>,
    english: &mut Vec<MessageTemplate>,
    id: &str,
    chinese_text: &str,
    english_text: &str,
    params: &[(&str, ParamKind)],
) {
    chinese.push(template(id, chinese_text, params));
    english.push(template(id, english_text, params));
}

/// 返回内置的中英双语消息目录。
#[must_use]
pub fn builtin_renderer() -> MessageRenderer {
    let mut chinese = Vec::new();
    let mut english = Vec::new();
    let no_params = &[];
    add_entry(
        &mut chinese,
        &mut english,
        "xiao.status.ready",
        "就绪",
        "ready",
        no_params,
    );
    add_entry(
        &mut chinese,
        &mut english,
        "xiao.status.cancelled",
        "请求已取消",
        "request cancelled",
        no_params,
    );
    add_entry(
        &mut chinese,
        &mut english,
        "xiao.debug.title",
        "Xiao 诊断",
        "Xiao diagnostics",
        no_params,
    );

    let text = ParamKind::Text;
    let integer = ParamKind::Integer;
    add_entry(
        &mut chinese,
        &mut english,
        "x05.package.config_read",
        "无法读取包配置 {path}：{reason}",
        "unable to read package configuration {path}: {reason}",
        &[("path", text), ("reason", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x05.package.missing_dependency",
        "无法读取依赖包配置 {path}：{reason}",
        "unable to read dependency package configuration {path}: {reason}",
        &[("path", text), ("reason", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x05.package.git_dependency_unresolved",
        "Git 依赖 \"{package}\" 尚未进入本地路径同步；远程版本解析留待 E3D",
        "Git dependency \"{package}\" has not been synchronized locally; remote version resolution is deferred to E3D",
        &[("package", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x05.package.identity_mismatch",
        "依赖名称 \"{requested}\" 指向了包 \"{actual}\"，包名必须一致",
        "dependency name \"{requested}\" points to package \"{actual}\"; package names must match",
        &[("requested", text), ("actual", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x05.package.identity_conflict",
        "包名 \"{package}\" 同时绑定到 {first} 和 {second}",
        "package \"{package}\" is bound to both {first} and {second}",
        &[("package", text), ("first", text), ("second", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x05.package.dependency_cycle",
        "包依赖环：{cycle}",
        "package dependency cycle: {cycle}",
        &[("cycle", text)],
    );

    add_entry(
        &mut chinese,
        &mut english,
        "runtime.type_mismatch",
        "期望类型 {expected}，实际为 {actual}",
        "expected type {expected}, got {actual}",
        &[("expected", text), ("actual", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.use_after_release",
        "对象已经释放，不能继续访问",
        "object has already been released and cannot be accessed",
        no_params,
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.weak_upgrade",
        "弱引用指向的对象已经释放",
        "the object referenced by the weak reference has been released",
        no_params,
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.table_state",
        "表状态应为 {expected}，实际为 {actual}",
        "expected table state {expected}, got {actual}",
        &[("expected", text), ("actual", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.division_by_zero",
        "除数不能为零",
        "division by zero while evaluating {operator}",
        &[("operator", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.index_out_of_bounds",
        "容器索引超出长度",
        "container index {index} is outside length {length}",
        &[("container", text), ("length", integer), ("index", integer)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.key_not_found",
        "字典中不存在该键",
        "key {key} was not found in {container}",
        &[("container", text), ("key", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.unhashable_element",
        "该类型的值不能作为集合元素或字典键",
        "a value of this type cannot be used as a set element or dictionary key",
        &[("type_name", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.set_operation_requires_sets",
        "集合运算要求两侧都是集合",
        "set operations require sets on both sides",
        &[("type_name", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.set_comparison_requires_sets",
        "集合比较要求两侧都是集合",
        "set comparisons require sets on both sides",
        &[("type_name", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.set_membership_requires_hashable",
        "成员判定的左操作数必须是可哈希值",
        "the left operand of membership testing must be hashable",
        &[("type_name", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.iterable_required",
        "for 的右侧必须是可迭代容器",
        "the right side of for must be an iterable container",
        &[("type_name", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.cross_thread",
        "首版 Runtime 对象不能跨线程传递",
        "Runtime objects cannot be transferred across threads in this release",
        no_params,
    );

    add_entry(
        &mut chinese,
        &mut english,
        "x11.protocol.core_crash",
        "Rust 核心处理请求时发生内部崩溃",
        "the Rust core crashed while handling the request",
        no_params,
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x11.protocol.cancelled",
        "请求已取消",
        "request cancelled",
        no_params,
    );

    MessageRenderer::new(vec![
        Catalog::new("zh-CN", chinese).expect("内置中文目录经过测试验证"),
        Catalog::new("en-US", english).expect("内置英文目录经过测试验证"),
    ])
    .expect("内置目录参数签名一致")
}
