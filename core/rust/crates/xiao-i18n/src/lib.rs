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
            let declared = entry
                .params
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
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

/// 先按结构化参数渲染；目录只要求原始整句时，安全地以 `message` 参数重试。
///
/// 该入口供协议、报告、日志和独立诊断窗口共享，避免每个输出边界复制不同的
/// 缺参回退逻辑。原始文本只作为展示参数，不会写回机器字段。
#[must_use]
pub fn render_with_original_message(
    renderer: &MessageRenderer,
    locale: &LocaleContext,
    id: &str,
    params: &BTreeMap<String, MessageParam>,
    original: &str,
) -> RenderedMessage {
    let rendered = renderer.render(locale, id, params);
    if !rendered.format_failed || params.contains_key("message") {
        return rendered;
    }
    let fallback_params = BTreeMap::from([(
        "message".to_owned(),
        MessageParam::Text(original.to_owned()),
    )]);
    let fallback = renderer.render(locale, id, &fallback_params);
    if fallback.format_failed {
        rendered
    } else {
        fallback
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

fn template(id: &str, text: &str, params: &[(&str, ParamKind)]) -> MessageTemplate {
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
    add_entry(
        &mut chinese,
        &mut english,
        "xiao.debug.standalone_ready",
        "原生调试产物已启动；诊断事件通道已就绪。",
        "native debug artifact started; diagnostic event channel is ready.",
        no_params,
    );
    add_entry(
        &mut chinese,
        &mut english,
        "xiao.debug.session_end",
        "诊断会话结束：{reason}",
        "diagnostic session ended: {reason}",
        &[("reason", ParamKind::Text)],
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
        "x02.type.undefined_name",
        "未定义名称 {name}",
        "undefined name {name}",
        &[("name", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x02.type.assignment_mismatch",
        "不能把 {actual} 赋给已锁定的 {expected}",
        "cannot assign {actual} to locked {expected}",
        &[("actual", text), ("expected", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x02.type.uninitialized_read",
        "名称 {name} 在复合赋值前不能读取",
        "name {name} cannot be read before compound assignment",
        &[("name", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x02.type.invalid_compound_operands",
        "复合赋值不能作用于 {left} 和 {right}",
        "compound assignment cannot operate on {left} and {right}",
        &[("left", text), ("right", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x02.type.compound_result_mismatch",
        "复合赋值结果 {result} 不符合 {expected}",
        "compound assignment result {result} does not match {expected}",
        &[("result", text), ("expected", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x02.type.implicit_conversion",
        "不能隐式把 {source} 转换为 {target}",
        "cannot implicitly convert {source} to {target}",
        &[("source", text), ("target", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x02.type.call_arity",
        "函数需要 {expected} 个参数，实际得到 {actual}",
        "function expects {expected} arguments but received {actual}",
        &[("expected", integer), ("actual", integer)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x04.type.for_requires_iterable",
        "for 的右侧必须是可迭代容器，实际为 {actual_type}",
        "the right side of for must be iterable, got {actual_type}",
        &[("actual_type", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x04.type.condition_requires_bool",
        "条件必须是 bool，实际为 {actual_type}",
        "condition must be bool, got {actual_type}",
        &[("actual_type", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x02.type.non_name_assignment_target",
        "P2 只支持标量名称的复合赋值",
        "P2 compound assignment supports scalar names only",
        no_params,
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x02.type.non_constant_initializer",
        "const 的初始化表达式必须能在编译期求值",
        "a const initializer must be evaluable at compile time",
        no_params,
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x02.type.conversion_arity",
        "标量转换函数必须接收一个参数",
        "a scalar conversion function must receive one argument",
        no_params,
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x07.type.fatal_constructor",
        "FatalError 不能构造为可恢复错误对象",
        "FatalError cannot be constructed as a recoverable error",
        no_params,
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
        "runtime.user_error",
        "{message}",
        "{message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.invalid_handle",
        "{message}",
        "invalid handle: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.refcount_invariant",
        "{message}",
        "reference-count invariant violated: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.table_init",
        "{message}",
        "table initialization failed: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.table_drop",
        "{message}",
        "table drop hook failed: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.numeric_overflow",
        "{message}",
        "numeric overflow: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.selector_bounds",
        "{message}",
        "selector bounds error: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.selector_step",
        "{message}",
        "selector step error: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.random_count",
        "{message}",
        "random selection count error: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.random_seed",
        "{message}",
        "random seed error: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.invalid_value",
        "{message}",
        "invalid value: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "runtime.allocation",
        "无法分配 Runtime 对象",
        "unable to allocate a Runtime object",
        no_params,
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

    let syntax_entries = [
        ("x01.lex.inconsistent_indent", "syntax error: {message}"),
        ("x01.lex.invalid_backtick_escape", "syntax error: {message}"),
        ("x01.lex.invalid_character", "syntax error: {message}"),
        ("x01.lex.invalid_escape", "syntax error: {message}"),
        ("x01.lex.invalid_number", "syntax error: {message}"),
        ("x01.lex.unmatched_delimiter", "syntax error: {message}"),
        ("x01.lex.unterminated_backtick", "syntax error: {message}"),
        ("x01.lex.unterminated_delimiter", "syntax error: {message}"),
        (
            "x01.lex.unterminated_doc_comment",
            "syntax error: {message}",
        ),
        ("x01.lex.unterminated_string", "syntax error: {message}"),
        ("x01.parse.empty_selector", "syntax error: {message}"),
        ("x01.parse.invalid_assignment", "syntax error: {message}"),
        (
            "x01.parse.invalid_assignment_target",
            "syntax error: {message}",
        ),
        ("x01.parse.invalid_cast_target", "syntax error: {message}"),
        ("x01.parse.invalid_expression", "syntax error: {message}"),
        ("x01.parse.invalid_path_segment", "syntax error: {message}"),
        ("x01.parse.invalid_random_count", "syntax error: {message}"),
        (
            "x01.parse.missing_assignment_value",
            "syntax error: {message}",
        ),
        ("x01.parse.missing_call_argument", "syntax error: {message}"),
        ("x01.parse.missing_delimiter", "syntax error: {message}"),
        ("x01.parse.missing_expression", "syntax error: {message}"),
        (
            "x01.parse.missing_group_expression",
            "syntax error: {message}",
        ),
        ("x01.parse.missing_member_name", "syntax error: {message}"),
        ("x01.parse.missing_new_callee", "syntax error: {message}"),
        ("x01.parse.missing_path_segment", "syntax error: {message}"),
        ("x01.parse.missing_random_count", "syntax error: {message}"),
        (
            "x01.parse.missing_range_endpoint",
            "syntax error: {message}",
        ),
        ("x01.parse.missing_step", "syntax error: {message}"),
        ("x01.parse.missing_tuple_element", "syntax error: {message}"),
        ("x01.parse.missing_unary_operand", "syntax error: {message}"),
        ("x01.parse.new_call_parentheses", "syntax error: {message}"),
        ("x01.parse.step_without_selector", "syntax error: {message}"),
        (
            "x01.parse.trailing_selector_comma",
            "syntax error: {message}",
        ),
        ("x01.parse.unsupported_block", "syntax error: {message}"),
        (
            "x01.parse.unsupported_expression",
            "syntax error: {message}",
        ),
        ("x02.parse.invalid_declaration", "syntax error: {message}"),
        (
            "x02.parse.invalid_declaration_tail",
            "syntax error: {message}",
        ),
        (
            "x02.parse.invalid_declaration_target",
            "syntax error: {message}",
        ),
        (
            "x02.parse.invalid_declaration_value",
            "syntax error: {message}",
        ),
        ("x02.parse.missing_const_value", "syntax error: {message}"),
        (
            "x03.parse.const_path_not_supported",
            "syntax error: {message}",
        ),
        (
            "x03.parse.const_set_type_not_supported",
            "syntax error: {message}",
        ),
        (
            "x03.parse.empty_set_type_annotation",
            "syntax error: {message}",
        ),
        ("x03.parse.invalid_dict_key", "syntax error: {message}"),
        ("x03.parse.invalid_set_separator", "syntax error: {message}"),
        (
            "x03.parse.invalid_set_type_annotation",
            "syntax error: {message}",
        ),
        ("x03.parse.invalid_set_type_term", "syntax error: {message}"),
        ("x03.parse.missing_array_element", "syntax error: {message}"),
        ("x03.parse.missing_dict_equal", "syntax error: {message}"),
        ("x03.parse.missing_dict_value", "syntax error: {message}"),
        ("x03.parse.missing_set_comma", "syntax error: {message}"),
        ("x03.parse.missing_set_element", "syntax error: {message}"),
        ("x03.parse.mixed_brace_entries", "syntax error: {message}"),
        (
            "x03.parse.set_type_path_not_supported",
            "syntax error: {message}",
        ),
        (
            "x03.parse.trailing_set_type_pipe",
            "syntax error: {message}",
        ),
        ("x04.parse.control_header_tail", "syntax error: {message}"),
        ("x04.parse.duplicate_main", "syntax error: {message}"),
        ("x04.parse.empty_block", "syntax error: {message}"),
        ("x04.parse.for_header_tail", "syntax error: {message}"),
        ("x04.parse.for_missing_in", "syntax error: {message}"),
        ("x04.parse.for_missing_iterable", "syntax error: {message}"),
        ("x04.parse.function_header_tail", "syntax error: {message}"),
        ("x04.parse.function_parentheses", "syntax error: {message}"),
        ("x04.parse.invalid_parameter", "syntax error: {message}"),
        ("x04.parse.invalid_return", "syntax error: {message}"),
        ("x04.parse.invalid_return_type", "syntax error: {message}"),
        ("x04.parse.loop_control_tail", "syntax error: {message}"),
        ("x04.parse.main_tail", "syntax error: {message}"),
        ("x04.parse.missing_block_newline", "syntax error: {message}"),
        ("x04.parse.missing_condition", "syntax error: {message}"),
        ("x04.parse.missing_function_name", "syntax error: {message}"),
        ("x04.parse.missing_indent", "syntax error: {message}"),
        ("x04.parse.return_tail", "syntax error: {message}"),
        ("x04.parse.unexpected_indent", "syntax error: {message}"),
        ("x05.parse.empty_table_body", "syntax error: {message}"),
        ("x05.parse.import_tail", "syntax error: {message}"),
        ("x05.parse.invalid_import_alias", "syntax error: {message}"),
        ("x05.parse.invalid_table_member", "syntax error: {message}"),
        ("x05.parse.invalid_table_name", "syntax error: {message}"),
        ("x05.parse.missing_from_import", "syntax error: {message}"),
        ("x05.parse.missing_import_alias", "syntax error: {message}"),
        ("x05.parse.missing_import_name", "syntax error: {message}"),
        ("x05.parse.missing_import_path", "syntax error: {message}"),
        (
            "x05.parse.missing_import_segment",
            "syntax error: {message}",
        ),
        ("x05.parse.missing_table_indent", "syntax error: {message}"),
        ("x05.parse.nested_table", "syntax error: {message}"),
        ("x05.parse.relative_import", "syntax error: {message}"),
        ("x05.parse.table_body_newline", "syntax error: {message}"),
        ("x05.parse.table_header_tail", "syntax error: {message}"),
        ("x05.parse.trailing_import_comma", "syntax error: {message}"),
        (
            "x05.parse.unexpected_table_indent",
            "syntax error: {message}",
        ),
        ("x05.parse.wildcard_import", "syntax error: {message}"),
        ("x07.parse.catch_header_tail", "syntax error: {message}"),
        ("x07.parse.catch_missing_as", "syntax error: {message}"),
        ("x07.parse.missing_handler", "syntax error: {message}"),
        ("x07.parse.missing_raise_value", "syntax error: {message}"),
        ("x07.parse.raise_tail", "syntax error: {message}"),
    ];
    for (id, english_text) in syntax_entries {
        add_entry(
            &mut chinese,
            &mut english,
            id,
            "{message}",
            english_text,
            &[("message", text)],
        );
    }

    let remaining_entries = [
        ("x01.lex.invalid", "lexer error: {message}"),
        ("x02.type.arithmetic_error", "type error: {message}"),
        ("x02.type.assign_immutable", "type error: {message}"),
        ("x02.type.duplicate_declaration", "type error: {message}"),
        ("x02.type.invalid_binary_operands", "type error: {message}"),
        ("x02.type.invalid_conversion", "type error: {message}"),
        ("x02.type.invalid_string_boolean", "type error: {message}"),
        ("x02.type.invalid_unary_operand", "type error: {message}"),
        ("x02.type.member_not_scalar", "type error: {message}"),
        ("x02.type.unification_error", "type error: {message}"),
        ("x03.type.array_element_mismatch", "type error: {message}"),
        (
            "x03.type.container_binding_expected",
            "type error: {message}",
        ),
        (
            "x03.type.container_redeclaration_mismatch",
            "type error: {message}",
        ),
        (
            "x03.type.duplicate_container_declaration",
            "type error: {message}",
        ),
        ("x03.type.duplicate_dictionary_key", "type error: {message}"),
        ("x03.type.index_out_of_bounds", "type error: {message}"),
        ("x03.type.invalid_path", "type error: {message}"),
        ("x03.type.invalid_path_segment", "type error: {message}"),
        ("x03.type.key_not_found", "type error: {message}"),
        ("x03.type.path_requires_container", "type error: {message}"),
        ("x03.type.random_count_integer", "type error: {message}"),
        ("x03.type.random_count_negative", "type error: {message}"),
        ("x03.type.random_count_range", "type error: {message}"),
        ("x03.type.random_empty_source", "type error: {message}"),
        ("x03.type.random_seed_arity", "type error: {message}"),
        ("x03.type.random_seed_integer", "type error: {message}"),
        ("x03.type.random_seed_non_negative", "type error: {message}"),
        ("x03.type.random_seed_range", "type error: {message}"),
        (
            "x03.type.random_without_replacement_exhausted",
            "type error: {message}",
        ),
        (
            "x03.type.selector_assignment_immutable",
            "type error: {message}",
        ),
        (
            "x03.type.selector_assignment_operator",
            "type error: {message}",
        ),
        ("x03.type.selector_assignment_plan", "type error: {message}"),
        (
            "x03.type.selector_assignment_random",
            "type error: {message}",
        ),
        ("x03.type.selector_assignment_root", "type error: {message}"),
        (
            "x03.type.selector_assignment_scalar_required",
            "type error: {message}",
        ),
        (
            "x03.type.selector_assignment_target",
            "type error: {message}",
        ),
        ("x03.type.selector_assignment_type", "type error: {message}"),
        ("x03.type.selector_invalid_index", "type error: {message}"),
        (
            "x03.type.selector_range_unordered_path",
            "type error: {message}",
        ),
        (
            "x03.type.selector_source_not_ordered",
            "type error: {message}",
        ),
        ("x03.type.selector_step_integer", "type error: {message}"),
        ("x03.type.selector_step_zero", "type error: {message}"),
        (
            "x03.type.selector_unordered_advanced",
            "type error: {message}",
        ),
        (
            "x03.type.set_comparison_requires_sets",
            "type error: {message}",
        ),
        ("x03.type.set_constructor_arity", "type error: {message}"),
        ("x03.type.set_duplicate_element", "type error: {message}"),
        (
            "x03.type.set_element_type_mismatch",
            "type error: {message}",
        ),
        ("x03.type.set_index_unsupported", "type error: {message}"),
        (
            "x03.type.set_initializer_requires_set",
            "type error: {message}",
        ),
        (
            "x03.type.set_membership_element_mismatch",
            "type error: {message}",
        ),
        (
            "x03.type.set_membership_requires_set",
            "type error: {message}",
        ),
        (
            "x03.type.set_operation_requires_sets",
            "type error: {message}",
        ),
        (
            "x03.type.set_type_path_not_supported",
            "type error: {message}",
        ),
        ("x03.type.set_unhashable_element", "type error: {message}"),
        (
            "x03.type.set_unhashable_membership",
            "type error: {message}",
        ),
        (
            "x03.type.typed_prefix_requires_container",
            "type error: {message}",
        ),
        ("x04.type.break_outside_loop", "type error: {message}"),
        ("x04.type.call_argument", "type error: {message}"),
        ("x04.type.call_argument_type", "type error: {message}"),
        ("x04.type.continue_outside_loop", "type error: {message}"),
        ("x04.type.default_parameter_type", "type error: {message}"),
        ("x04.type.duplicate_function", "type error: {message}"),
        ("x04.type.implicit_none_return", "type error: {message}"),
        ("x04.type.return_mismatch", "type error: {message}"),
        ("x04.type.return_outside_function", "type error: {message}"),
        ("x04.type.star_argument_type", "type error: {message}"),
        ("x04.type.unresolved_parameter", "type error: {message}"),
        ("x04.type.unresolved_return", "type error: {message}"),
        (
            "x05.config.dictionary_requires_equals",
            "configuration error: {message}",
        ),
        (
            "x05.config.dotted_table_forbidden",
            "configuration error: {message}",
        ),
        (
            "x05.config.duplicate_dictionary_key",
            "configuration error: {message}",
        ),
        ("x05.config.duplicate_key", "configuration error: {message}"),
        (
            "x05.config.duplicate_table",
            "configuration error: {message}",
        ),
        (
            "x05.config.executable_member_forbidden",
            "configuration error: {message}",
        ),
        (
            "x05.config.indented_table_header",
            "configuration error: {message}",
        ),
        (
            "x05.config.instance_table_forbidden",
            "configuration error: {message}",
        ),
        (
            "x05.config.integer_out_of_range",
            "configuration error: {message}",
        ),
        (
            "x05.config.invalid_backtick_name",
            "configuration error: {message}",
        ),
        (
            "x05.config.invalid_dependency_constraint",
            "configuration error: {message}",
        ),
        (
            "x05.config.invalid_dependency_git",
            "configuration error: {message}",
        ),
        (
            "x05.config.invalid_dependency_path",
            "configuration error: {message}",
        ),
        (
            "x05.config.invalid_dependency_source",
            "configuration error: {message}",
        ),
        (
            "x05.config.invalid_dictionary_key",
            "configuration error: {message}",
        ),
        (
            "x05.config.invalid_export_path",
            "configuration error: {message}",
        ),
        ("x05.config.invalid_float", "configuration error: {message}"),
        (
            "x05.config.invalid_source_git_ref",
            "configuration error: {message}",
        ),
        (
            "x05.config.invalid_string",
            "configuration error: {message}",
        ),
        (
            "x05.config.invalid_table_name",
            "configuration error: {message}",
        ),
        (
            "x05.config.missing_array_comma",
            "configuration error: {message}",
        ),
        (
            "x05.config.missing_dependency_path",
            "configuration error: {message}",
        ),
        (
            "x05.config.missing_dictionary_comma",
            "configuration error: {message}",
        ),
        (
            "x05.config.missing_equals",
            "configuration error: {message}",
        ),
        (
            "x05.config.missing_project_field",
            "configuration error: {message}",
        ),
        (
            "x05.config.missing_project_table",
            "configuration error: {message}",
        ),
        (
            "x05.config.missing_source_field",
            "configuration error: {message}",
        ),
        (
            "x05.config.missing_table_close",
            "configuration error: {message}",
        ),
        (
            "x05.config.nested_table_forbidden",
            "configuration error: {message}",
        ),
        (
            "x05.config.non_literal_value",
            "configuration error: {message}",
        ),
        (
            "x05.config.sign_requires_number",
            "configuration error: {message}",
        ),
        (
            "x05.config.top_level_requires_table",
            "configuration error: {message}",
        ),
        (
            "x05.config.trailing_expression_forbidden",
            "configuration error: {message}",
        ),
        ("x05.config.type_mismatch", "configuration error: {message}"),
        (
            "x05.config.unknown_dependency_field",
            "configuration error: {message}",
        ),
        ("x05.config.unknown_field", "configuration error: {message}"),
        (
            "x05.config.unknown_source_field",
            "configuration error: {message}",
        ),
        ("x05.config.unknown_table", "configuration error: {message}"),
        ("x05.module.case_fold_conflict", "module error: {message}"),
        (
            "x05.module.file_namespace_conflict",
            "module error: {message}",
        ),
        (
            "x05.module.import_binding_conflict",
            "module error: {message}",
        ),
        ("x05.module.import_cycle", "module error: {message}"),
        ("x05.module.invalid_file_name", "module error: {message}"),
        (
            "x05.module.invalid_qualifier_use",
            "module error: {message}",
        ),
        ("x05.module.invalid_utf8", "module error: {message}"),
        (
            "x05.module.missing_import_symbol",
            "module error: {message}",
        ),
        (
            "x05.module.missing_import_target",
            "module error: {message}",
        ),
        (
            "x05.module.namespace_case_fold_conflict",
            "module error: {message}",
        ),
        ("x05.module.non_utf8_path", "module error: {message}"),
        (
            "x05.module.project_root_not_directory",
            "module error: {message}",
        ),
        ("x05.module.read_directory", "module error: {message}"),
        ("x05.module.read_file", "module error: {message}"),
        ("x05.module.read_file_type", "module error: {message}"),
        (
            "x05.package.invalid_metadata",
            "package metadata error: {message}",
        ),
        ("x05.type.assign_method", "type error: {message}"),
        ("x05.type.constructor_argument", "type error: {message}"),
        (
            "x05.type.constructor_argument_type",
            "type error: {message}",
        ),
        ("x05.type.constructor_arity", "type error: {message}"),
        ("x05.type.drop_arity", "type error: {message}"),
        ("x05.type.duplicate_table", "type error: {message}"),
        ("x05.type.duplicate_table_member", "type error: {message}"),
        (
            "x05.type.dynamic_table_initializer",
            "type error: {message}",
        ),
        ("x05.type.invalid_table_member", "type error: {message}"),
        ("x05.type.lifecycle_return", "type error: {message}"),
        ("x05.type.method_default_parameter", "type error: {message}"),
        (
            "x05.type.method_implicit_none_return",
            "type error: {message}",
        ),
        ("x05.type.method_requires_self", "type error: {message}"),
        ("x05.type.new_requires_table", "type error: {message}"),
        ("x05.type.private_table_member", "type error: {message}"),
        (
            "x05.type.singleton_not_constructible",
            "type error: {message}",
        ),
        (
            "x05.type.table_field_type_mismatch",
            "type error: {message}",
        ),
        ("x05.type.unknown_table", "type error: {message}"),
        ("x05.type.unknown_table_member", "type error: {message}"),
        ("x06.lifetime.dynamic_check", "lifetime error: {message}"),
        ("x06.lifetime.invalid_edge", "lifetime error: {message}"),
        ("x06.lifetime.strong_cycle", "lifetime error: {message}"),
        ("x06.lifetime.unknown_value", "lifetime error: {message}"),
        ("x07.type.catch_order", "type error: {message}"),
        ("x07.type.fatal_not_catchable", "type error: {message}"),
        ("x07.type.invalid_catch_type", "type error: {message}"),
        ("x07.type.raise_requires_error", "type error: {message}"),
    ];
    for (id, english_text) in remaining_entries {
        add_entry(
            &mut chinese,
            &mut english,
            id,
            "{message}",
            english_text,
            &[("message", text)],
        );
    }

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
    add_entry(
        &mut chinese,
        &mut english,
        "x11.protocol.frame",
        "{message}",
        "protocol framing failed: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x11.protocol.request",
        "{message}",
        "invalid protocol request: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x11.protocol.optimization_unavailable",
        "{message}",
        "optimization is unavailable: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x11.protocol.test_response",
        "{message}",
        "invalid test response: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x11.driver.native_build",
        "{message}",
        "native build failed: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x11.driver.rejected",
        "{message}",
        "driver rejected the request: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x11.diagnostics.start_failed",
        "{message}",
        "diagnostics startup failed: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x11.diagnostics.activation_cleanup_failed",
        "{message}",
        "unable to clean up diagnostics activation: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x11.diagnostics.activation_write_failed",
        "{message}",
        "unable to write diagnostics activation: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x11.diagnostics.component_copy_failed",
        "{message}",
        "unable to stage the diagnostics component: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x11.repl.package.root_conflict",
        "{message}",
        "package root conflict: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x11.repl.package.view_failed",
        "{message}",
        "unable to query the package view: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "x05.package.operation",
        "{message}",
        "package operation failed: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "fatal.runtime_invariant",
        "{message}",
        "runtime invariant failure: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "fatal.corrupt_artifact",
        "{message}",
        "corrupt artifact: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "fatal.out_of_memory",
        "{message}",
        "out of memory: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "fatal.stack_overflow",
        "{message}",
        "stack overflow: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "fatal.hardware",
        "{message}",
        "hardware failure: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "fatal.internal",
        "{message}",
        "internal failure: {message}",
        &[("message", text)],
    );
    add_entry(
        &mut chinese,
        &mut english,
        "xiao.debug.status",
        "运行 {elapsed} ms | 内存 {memory} / 峰值 {peak} B | 错误 {errors} | 断点 {breakpoints} | 钩子 {hooks}",
        "run {elapsed} ms | memory {memory} / peak {peak} B | errors {errors} | breakpoints {breakpoints} | hooks {hooks}",
        &[
            ("elapsed", text),
            ("memory", text),
            ("peak", text),
            ("errors", text),
            ("breakpoints", text),
            ("hooks", text),
        ],
    );

    MessageRenderer::new(vec![
        Catalog::new("zh-CN", chinese).expect("内置中文目录经过测试验证"),
        Catalog::new("en-US", english).expect("内置英文目录经过测试验证"),
    ])
    .expect("内置目录参数签名一致")
}
