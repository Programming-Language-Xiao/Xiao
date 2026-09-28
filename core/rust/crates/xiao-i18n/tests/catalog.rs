//! 内置消息目录和回退链的隔离回归。

use std::collections::BTreeMap;

use xiao_i18n::{
    Catalog, Fallback, LocaleContext, MessageParam, MessageRenderer, MessageTemplate, ParamKind,
    builtin_renderer,
};

fn entry(text: &str) -> MessageTemplate {
    MessageTemplate {
        id: "xiao.error.example".to_owned(),
        text: text.to_owned(),
        params: BTreeMap::from([("count".to_owned(), ParamKind::Integer)]),
    }
}

#[test]
fn builtin_catalog_preserves_default_text_and_normalizes_aliases() {
    let renderer = builtin_renderer();
    let params = BTreeMap::new();
    assert_eq!(
        renderer
            .render(&LocaleContext::default(), "xiao.status.cancelled", &params)
            .text,
        "请求已取消"
    );
    assert_eq!(
        renderer
            .render(&LocaleContext::new("en"), "xiao.status.cancelled", &params)
            .text,
        "request cancelled"
    );
    assert_eq!(LocaleContext::from_config("zh").expect("zh").tag(), "zh-CN");
    assert_eq!(LocaleContext::from_config("en").expect("en").tag(), "en-US");
    assert_eq!(
        LocaleContext::from_config("xx").expect_err("invalid locale"),
        "X11-CONFIG-002"
    );
}

#[test]
fn exact_base_english_and_identity_fallbacks_are_deterministic() {
    let renderer = MessageRenderer::new(vec![
        Catalog::new("zh-CN", vec![entry("有 {count} 项")]).expect("zh catalog"),
        Catalog::new("en-US", vec![entry("{count} entries")]).expect("en catalog"),
    ])
    .expect("renderer");
    let params = BTreeMap::from([("count".to_owned(), MessageParam::Integer(3))]);
    for (locale, fallback, text) in [
        ("zh-CN", Fallback::Exact, "有 3 项"),
        ("zh-HK", Fallback::Base, "有 3 项"),
        ("fr-FR", Fallback::English, "3 entries"),
    ] {
        let result = renderer.render(&LocaleContext::new(locale), "xiao.error.example", &params);
        assert_eq!((result.fallback, result.text.as_str()), (fallback, text));
    }
    let unknown = renderer.render(&LocaleContext::new("fr-FR"), "xiao.error.missing", &params);
    assert_eq!(unknown.fallback, Fallback::Identity);
    assert_eq!(unknown.text, "xiao.error.missing (count=3)");
}

#[test]
fn invalid_types_and_templates_never_hide_original_identity() {
    let catalog = Catalog::new("zh-CN", vec![entry("有 {count} 项")]).expect("catalog");
    let renderer = MessageRenderer::new(vec![catalog]).expect("renderer");
    let params = BTreeMap::from([("count".to_owned(), MessageParam::Text("\nBAD".to_owned()))]);
    let result = renderer.render(&LocaleContext::default(), "xiao.error.example", &params);
    assert!(result.format_failed);
    assert_eq!(result.text, "xiao.error.example (count=\"\\nBAD\")");
    assert!(Catalog::new("zh-CN", vec![entry("{unknown}")]).is_err());
    assert!(Catalog::new("zh-CN", vec![entry("{count")]).is_err());
    assert!(Catalog::new("zh-CN", vec![entry("{count}"), entry("{count}")]).is_err());
    let wrong_signature = MessageTemplate {
        params: BTreeMap::new(),
        ..entry("static")
    };
    assert!(
        MessageRenderer::new(vec![
            Catalog::new("zh-CN", vec![entry("有 {count} 项")]).expect("zh"),
            Catalog::new("en-US", vec![wrong_signature]).expect("en"),
        ])
        .is_err()
    );
}

#[test]
fn interpolation_does_not_reprocess_inserted_user_text() {
    let entry = MessageTemplate {
        id: "xiao.error.example".to_owned(),
        text: "{name} ({count})".to_owned(),
        params: BTreeMap::from([
            ("name".to_owned(), ParamKind::Text),
            ("count".to_owned(), ParamKind::Integer),
        ]),
    };
    let renderer = MessageRenderer::new(vec![Catalog::new("zh-CN", vec![entry]).expect("catalog")])
        .expect("renderer");
    let params = BTreeMap::from([
        ("name".to_owned(), MessageParam::Text("{count}".to_owned())),
        ("count".to_owned(), MessageParam::Integer(7)),
    ]);
    assert_eq!(
        renderer
            .render(&LocaleContext::default(), "xiao.error.example", &params)
            .text,
        "{count} (7)"
    );
}
