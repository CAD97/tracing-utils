use std::str::FromStr;

use once_cell::sync::Lazy;
use regex::Regex;
use tracing::{metadata::LevelFilter, Level};
use tracing_memory::Event;

type SStr = smartstring::SmartString<smartstring::LazyCompact>;
type SVec<T, const N: usize> = smallvec::SmallVec<[T; N]>;

#[derive(Debug, Default)]
pub(crate) struct EventFilter {
    directives: SVec<Directive, 2>,
}

#[derive(Debug)]
struct Directive {
    target: Option<SStr>,
    span: Option<SStr>,
    field: Option<FieldDirective>,
    level: LevelFilter,
}

#[derive(Debug)]
struct FieldDirective {
    name: SStr,
    value: Option<SStr>,
}

impl EventFilter {
    pub fn includes(&self, event: &Event) -> bool {
        if self.directives.is_empty() {
            return true;
        }

        let mut included = false;

        for directive in &self.directives {
            let mut this_directive_applies = true;

            if let Some(target_directive) = &directive.target {
                this_directive_applies &= event
                    .meta()
                    .target()
                    .matches(target_directive.as_str())
                    .any(|_| true);
            }

            // FIXME: should require being in `target` (if provided)
            if let Some(span_directive) = &directive.span {
                this_directive_applies &= std::iter::successors(event.span(), |span| span.parent())
                    .filter(|span| {
                        span.meta()
                            .name()
                            .matches(span_directive.as_str())
                            .any(|_| true)
                    })
                    .any(|_| true);
            }

            for field_directive in &directive.field {
                // FIXME: should require being in `span` (if provided)
                // FIXME: `value` should be treated as a regex
                this_directive_applies &= event
                    .fields()
                    .chain(
                        std::iter::successors(event.span(), |span| span.parent())
                            .flat_map(|span| span.fields()),
                    )
                    .filter(|(name, _value)| {
                        name.matches(field_directive.name.as_str()).any(|_| true)
                    })
                    .filter(|(_name, value)| {
                        if let Some(value_directive) = &field_directive.value {
                            // FIXME: avoid format! call where possible (i.e. primitive, str fields)
                            value
                                .with_debug(|field| {
                                    let field = format!("{:?}", field);
                                    field.matches(value_directive.as_str()).any(|_| true)
                                })
                                .any(std::convert::identity)
                        } else {
                            true
                        }
                    })
                    .any(|_| true);
            }

            if this_directive_applies {
                included = *event.meta().level() <= directive.level;
            }
        }

        included
    }

    pub fn excludes(&self, event: &Event) -> bool {
        !self.includes(event)
    }
}

impl FromStr for EventFilter {
    type Err = (); // TODO: nicer error message
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Adapted from <tracing_subscriber@54780fb::EnvFilter>::try_new
        // https://github.com/tokio-rs/tracing/blob/54780fb/tracing-subscriber/src/filter/env/mod.rs#L166-L171
        if s.is_empty() {
            return Ok(EventFilter::default());
        }
        let directives = s.split(',').map(|s| s.parse()).collect::<Result<_, _>>()?;
        Ok(EventFilter { directives })
    }
}

impl FromStr for Directive {
    type Err = (); // TODO: actual error messages
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Adapted from <tracing_subscriber@54780fb::filter::Directive as FromStr>::from_str
        // https://github.com/tokio-rs/tracing/blob/54780fb/tracing-subscriber/src/filter/env/directive.rs#L177-L266
        // target[span{field=value}]=level
        #[rustfmt::skip]
        static DIRECTIVE_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r#"(?x)
                ^
                (?P<target>[\w:-]+)? # target
                (?:                  # [span{field=value}]
                    \[(?P<span>[^\]]*)\]
                )?
                (?:                  # =level
                    =(?P<level>(?i:trace|debug|info|warn|error|off|[0-5]))?
                    #          ^^^.
                    #              `note: we match log level names case-insensitively
                )?
                $
            "#).unwrap()
        });
        #[rustfmt::skip]
        static SPAN_PART_RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r#"(?x)
                (?P<name>[^\]\{]+)? # span
                (?:                 # {field=value}
                    \{(?P<fields>[^\}]*)\}
                )?
            "#).unwrap());
        #[rustfmt::skip]
        static FIELD_PART_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r#"(?x)
                (?P<name>[^=]+) # field
                (?:             # =value
                    =(?P<value>[^,]+)
                )?
            "#).unwrap()
        });

        let caps = DIRECTIVE_RE.captures(s).ok_or(())?;

        if let Some(level) = caps
            .name("global_level")
            .and_then(|s| s.as_str().parse().ok())
        {
            return Ok(Directive {
                target: None,
                span: None,
                field: None,
                level,
            });
        }

        let target = caps.name("target").and_then(|c| {
            let s = c.as_str();
            if s.parse::<Level>().is_ok() {
                None
            } else {
                Some(s.into())
            }
        });

        let (span, field) = caps
            .name("span")
            .map(|cap| {
                let caps = SPAN_PART_RE.captures(cap.as_str()).ok_or(())?;
                let span = caps.name("name").map(|c| c.as_str().into());
                let field = caps
                    .name("fields")
                    .map(|cap| {
                        let caps = FIELD_PART_RE.captures(cap.as_str()).ok_or(())?;
                        let name = caps.name("name").unwrap().as_str().into();
                        let value = caps.name("value").map(|c| c.as_str().into());
                        Ok(FieldDirective { name, value })
                    })
                    .transpose()?;
                Ok((span, field))
            })
            .transpose()?
            .unwrap_or((None, None));

        let level = caps
            .name("level")
            .map(|l| l.as_str().parse().map_err(drop))
            .transpose()?
            // Setting the target without the level enables every level for that target
            .unwrap_or(LevelFilter::TRACE);

        Ok(Directive {
            target,
            span,
            field,
            level,
        })
    }
}
