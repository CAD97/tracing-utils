use crate::{Event, Field, Span, EVENT_QUEUE};
use chrono::prelude::*;
use std::{marker::PhantomData, sync::Arc};
use tracing::{span, Subscriber};
use tracing_subscriber::{
    field::RecordFields,
    layer,
    registry::{LookupSpan, SpanRef},
};

/// A tracing [layer](mod@layer) that records events and spans.
#[derive(Debug, Clone, Copy)]
pub struct Layer<S> {
    _inner: PhantomData<S>,
}

impl<S> Layer<S> {
    pub fn new() -> Self {
        Default::default()
    }
}

impl<S> Default for Layer<S> {
    fn default() -> Self {
        smartstring::validate();
        Layer {
            _inner: PhantomData,
        }
    }
}

impl<S> tracing_subscriber::Layer<S> for Layer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: layer::Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        on_span(span, attrs);
    }

    fn on_record(&self, id: &span::Id, values: &span::Record<'_>, ctx: layer::Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found; this is a bug");
        on_span(span, values);
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: layer::Context<'_, S>) {
        let span = ctx.event_span(event);
        on_event(event, span);
    }
}

fn on_span<'a, R, S>(span: SpanRef<'a, S>, fields: &R)
where
    R: RecordFields,
    S: LookupSpan<'a>,
{
    let mut ext = span.extensions_mut();
    match ext.get_mut::<Arc<Span>>() {
        Some(archived) => {
            fields.record(&mut Visitor(&mut *Arc::make_mut(archived)));
        }
        None => {
            let mut archived = Span {
                meta: span.metadata(),
                fields: Default::default(),
                parent: span
                    .parent()
                    .and_then(|span| span.extensions().get().map(Arc::clone)),
            };
            fields.record(&mut Visitor(&mut archived));
            ext.insert(Arc::new(archived));
        }
    }
}

fn on_event<'a, S>(event: &tracing::Event<'_>, span: Option<SpanRef<'a, S>>)
where
    S: LookupSpan<'a>,
{
    let mut archived = Event {
        meta: event.metadata(),
        timestamp: Local::now().naive_local(),
        fields: Default::default(),
        span: span.and_then(|span| span.extensions().get().map(Arc::clone)),
    };
    event.record(&mut Visitor(&mut archived));
    EVENT_QUEUE.push(Arc::new(archived));
}

struct Visitor<'a, R>(&'a mut R);

impl tracing::field::Visit for Visitor<'_, Span> {
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.0.record_field(field, || Field::I64(value))
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0.record_field(field, || Field::U64(value))
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.0.record_field(field, || Field::Bool(value))
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.record_field(field, || Field::Str(value.into()))
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        self.0
            .record_field(field, || Field::Error(format!("{}", value).into()))
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0
            .record_field(field, || Field::Debug(format!("{:?}", value).into()))
    }
}

impl tracing::field::Visit for Visitor<'_, Event> {
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.0.record_field(field, || Field::I64(value))
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0.record_field(field, || Field::U64(value))
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.0.record_field(field, || Field::Bool(value))
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.record_field(field, || Field::Str(value.into()))
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        self.0
            .record_field(field, || Field::Error(format!("{}", value).into()))
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0
            .record_field(field, || Field::Debug(format!("{:?}", value).into()))
    }
}
