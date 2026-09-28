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
            if fields != entry.params.keys().cloned().collect() {
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

/// 返回首批可用的中英双语内置消息目录。
#[must_use]
pub fn builtin_renderer() -> MessageRenderer {
    let entries = [
        ("xiao.status.ready", "就绪", "ready"),
        ("xiao.status.cancelled", "请求已取消", "request cancelled"),
        ("xiao.debug.title", "Xiao 诊断", "Xiao diagnostics"),
    ];
    let catalog = |tag: &str, index: usize| {
        Catalog::new(
            tag,
            entries
                .iter()
                .map(|(id, zh, en)| MessageTemplate {
                    id: (*id).to_owned(),
                    text: if index == 0 { *zh } else { *en }.to_owned(),
                    params: BTreeMap::new(),
                })
                .collect(),
        )
        .expect("内置目录经过测试验证")
    };
    MessageRenderer::new(vec![catalog("zh-CN", 0), catalog("en-US", 1)])
        .expect("内置目录参数签名一致")
}
