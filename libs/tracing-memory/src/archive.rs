type SmartString = smartstring::SmartString<smartstring::LazyCompact>;

use chrono::prelude::*;
use indexmap::IndexMap;
use replace_with::replace_with_or_abort;
use std::sync::Arc;

/// A record of a tracing [event](https://docs.rs/tracing/0.1/tracing/index.html#events).
#[derive(Debug, Clone)]
pub struct Event {
    pub(crate) meta: &'static tracing::Metadata<'static>,
    pub(crate) timestamp: NaiveDateTime,
    pub(crate) fields: FieldMap,
    pub(crate) span: Option<Arc<Span>>,
}

/// A record of a tracing [span](https://docs.rs/tracing/0.1/tracing/index.html#spans).
#[derive(Debug, Clone)]
pub struct Span {
    pub(crate) meta: &'static tracing::Metadata<'static>,
    pub(crate) fields: FieldMap,
    pub(crate) parent: Option<Arc<Span>>,
}

type FieldMap = IndexMap<&'static str, Field, ahash::RandomState>;

/// A field recorded on some tracing event/span.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Field {
    I64(i64),
    U64(u64),
    Bool(bool),
    Str(SmartString),
    Error(SmartString),
    Debug(SmartString),
    Multiple(Vec<Field>),
}

impl Event {
    /// The [`tracing::Metadata`] describing this event.
    pub fn meta(&self) -> &'static tracing::Metadata<'static> {
        self.meta
    }

    /// The time at which this event was fired.
    pub fn timestamp(&self) -> NaiveDateTime {
        self.timestamp
    }

    /// A recorded field on this event.
    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.get(name)
    }

    /// All recorded fields on this event.
    pub fn fields(&self) -> impl Iterator<Item = (&'static str, &Field)> + '_ {
        self.fields.iter().map(|(&name, field)| (name, field))
    }

    /// The containing span, if any.
    pub fn span(&self) -> Option<&Span> {
        self.span.as_deref()
    }

    pub(crate) fn record_field(
        &mut self,
        field: &tracing::field::Field,
        value: impl Fn() -> Field,
    ) {
        self.fields
            .entry(field.name())
            .and_modify(|entry| {
                replace_with_or_abort(entry, |field| match field {
                    Field::Multiple(mut fields) => {
                        fields.push(value());
                        Field::Multiple(fields)
                    }
                    field => Field::Multiple(vec![field, value()]),
                })
            })
            .or_insert_with(value);
    }
}

impl Span {
    /// The [`tracing::Metadata`] describing this span.
    pub fn meta(&self) -> &'static tracing::Metadata<'static> {
        self.meta
    }

    /// A recorded field on this span.
    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.get(name)
    }

    /// All recorded fields on this span.
    pub fn fields(&self) -> impl Iterator<Item = (&'static str, &Field)> + '_ {
        self.fields.iter().map(|(&name, field)| (name, field))
    }

    /// The containing span, if any.
    pub fn parent(&self) -> Option<&Span> {
        self.parent.as_deref()
    }

    pub(crate) fn record_field(
        &mut self,
        field: &tracing::field::Field,
        value: impl Fn() -> Field,
    ) {
        self.fields
            .entry(field.name())
            .and_modify(|entry| {
                replace_with_or_abort(entry, |field| match field {
                    Field::Multiple(mut fields) => {
                        fields.push(value());
                        Field::Multiple(fields)
                    }
                    field => Field::Multiple(vec![field, value()]),
                })
            })
            .or_insert_with(value);
    }
}

impl Field {
    /// The field, as would be presented to [`tracing::field::Visit::record_debug`].
    ///
    /// If the field was recorded multiple times, `record_debug` is called multiple times.
    pub fn with_debug<'a, R>(
        &'a self,
        record_debug: impl 'a + FnMut(&dyn std::fmt::Debug) -> R,
    ) -> impl Iterator<Item = R> + 'a {
        struct WithDebug<'a, F>(&'a [Field], Vec<&'a [Field]>, F);
        impl<F, R> Iterator for WithDebug<'_, F>
        where
            F: FnMut(&dyn std::fmt::Debug) -> R,
        {
            type Item = R;
            fn next(&mut self) -> Option<Self::Item> {
                if self.0.is_empty() {
                    self.0 = self.1.pop().unwrap_or(&[]);
                }

                match self.0 {
                    [] => None,
                    [head, tail @ ..] => {
                        let res = match head {
                            Field::I64(value) => self.2(value),
                            Field::U64(value) => self.2(value),
                            Field::Bool(value) => self.2(value),
                            Field::Str(value) => self.2(&&**value as &&str),
                            Field::Error(value) => self.2(&format_args!("{}", value)),
                            Field::Debug(value) => self.2(&format_args!("{}", value)),
                            Field::Multiple(values) => {
                                if tail.is_empty() {
                                    self.0 = &**values;
                                } else if !values.is_empty() {
                                    self.1.push(&**values);
                                }
                                return self.next();
                            }
                        };
                        self.0 = tail;
                        Some(res)
                    }
                }
            }
        }

        WithDebug(std::slice::from_ref(self), vec![], record_debug)
    }
}
