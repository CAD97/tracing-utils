use parse_env_filter::{
    eager::{filters, Filter, SpanFilter},
    FieldFilter, ParseError,
};

#[test]
fn tracing_examples() {
    assert_eq!(
        filters("target[span{field=value}]=level").unwrap(),
        vec![Filter {
            target: "target",
            span: Some(vec![SpanFilter {
                name: "span",
                fields: Some(vec![FieldFilter {
                    name: "field",
                    value: Some("value")
                }])
            }]),
            level: Some("level")
        }]
    );

    assert_eq!(
        filters("tokio::net=info").unwrap(),
        vec![Filter {
            target: "tokio::net",
            span: None,
            level: Some("info"),
        }]
    );

    assert_eq!(
        filters("my_crate[span_a]=trace").unwrap(),
        vec![Filter {
            target: "my_crate",
            span: Some(vec![SpanFilter {
                name: "span_a",
                fields: None
            }]),
            level: Some("trace")
        }]
    );

    assert_eq!(
        filters("[span_b{name=bob}]").unwrap(),
        vec![Filter {
            target: "",
            span: Some(vec![SpanFilter {
                name: "span_b",
                fields: Some(vec![FieldFilter {
                    name: "name",
                    value: Some("bob")
                }])
            }]),
            level: None
        }]
    );

    assert_eq!(
        filters(r#"[span_b{name="bob"}]"#),
        Err(ParseError::ReservedSyntax)
    );
}

#[test]
fn envlogger_examples() {
    assert_eq!(
        filters("hello").unwrap(),
        vec![Filter {
            target: "hello",
            span: None,
            level: None
        }]
    );

    assert_eq!(
        filters("trace").unwrap(),
        vec![Filter {
            target: "trace",
            span: None,
            level: None
        }]
    );

    assert_eq!(
        filters("TRACE").unwrap(),
        vec![Filter {
            target: "TRACE",
            span: None,
            level: None
        }]
    );

    assert_eq!(
        filters("info").unwrap(),
        vec![Filter {
            target: "info",
            span: None,
            level: None
        }]
    );

    assert_eq!(
        filters("INFO").unwrap(),
        vec![Filter {
            target: "INFO",
            span: None,
            level: None
        }]
    );

    assert_eq!(
        filters("hello=debug").unwrap(),
        vec![Filter {
            target: "hello",
            span: None,
            level: Some("debug")
        }]
    );

    assert_eq!(
        filters("hello=DEBUG").unwrap(),
        vec![Filter {
            target: "hello",
            span: None,
            level: Some("DEBUG")
        }]
    );

    assert_eq!(
        filters("hello,std::option").unwrap(),
        vec![
            Filter {
                target: "hello",
                span: None,
                level: None
            },
            Filter {
                target: "std::option",
                span: None,
                level: None
            }
        ]
    );

    assert_eq!(
        filters("error,hello=warn").unwrap(),
        vec![
            Filter {
                target: "error",
                span: None,
                level: None
            },
            Filter {
                target: "hello",
                span: None,
                level: Some("warn")
            }
        ]
    );

    assert_eq!(
        filters("off").unwrap(),
        vec![Filter {
            target: "off",
            span: None,
            level: None
        }]
    );

    assert_eq!(
        filters("OFF").unwrap(),
        vec![Filter {
            target: "OFF",
            span: None,
            level: None
        }]
    );
}

#[test]
fn envlogger_regex() {
    assert_eq!(filters("hello/foo"), Err(ParseError::ReservedSyntax));
    assert_eq!(filters("info/f.o"), Err(ParseError::ReservedSyntax));

    assert_eq!(
        filters("hello=debug/foo*foo"),
        Err(ParseError::ReservedSyntax)
    );

    assert_eq!(
        filters("error,hello=warn/[0-9]scopes"),
        Err(ParseError::ReservedSyntax)
    );
}

#[test]
fn negative_examples() {
    assert_eq!(filters("[a[a]"), Err(ParseError::BadSyntax));
    /* tracing::EnvFilter gives:
    Filter {
        target: "",
        span: Some([SpanFilter {
            name: "a[a",
            fields: None,
        }]),
        level: None,
    }
    */

    assert_eq!(filters("[[]"), Err(ParseError::BadSyntax));
    /* tracing::EnvFilter gives:
    Filter {
        target: "",
        span: Some([SpanFilter {
            name: "",
            fields: None,
        }]),
        level: None,
    }
    */

    assert_eq!(filters("[=]"), Err(ParseError::BadSyntax));
    /* tracing::EnvFilter gives:
    Filter {
        target: "",
        span: Some([SpanFilter {
            name: "=",
            fields: None,
        }]),
        level: None,
    }
    */

    assert_eq!(filters("[}]"), Err(ParseError::BadSyntax));
    /* tracing::EnvFilter gives:
    Filter {
        target: "",
        span: Some([SpanFilter {
            name: "}",
            fields: None,
        }]),
        level: None,
    }
    */
}

#[test]
fn unique_examples() {
    assert_eq!(
        filters("=warn").unwrap(),
        vec![Filter {
            target: "",
            span: None,
            level: Some("warn")
        }]
    );
}
