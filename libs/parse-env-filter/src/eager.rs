//! Eagerly fully-parsed event filters.

extern crate alloc;

use crate::{FieldFilter, ParseError};
use alloc::vec::Vec;
use core::convert::TryFrom;

/// Parse a series of filters out of a directive string.
///
/// This is an eager, allocating version of [the root `filters`](crate::filters).
#[allow(clippy::result_unit_err)]
pub fn filters(directives: &str) -> Result<Vec<Filter<'_>>, ParseError> {
    crate::filters(directives)
        .map(|filter| Filter::try_from(filter?))
        .collect()
}

/// A single event filter, `target[span{field=value}]=level`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter<'a> {
    pub target: &'a str,
    pub span: Option<Vec<SpanFilter<'a>>>,
    pub level: Option<&'a str>,
}

impl<'a> TryFrom<crate::Filter<'a>> for Filter<'a> {
    type Error = ParseError;

    fn try_from(filter: crate::Filter<'a>) -> Result<Self, Self::Error> {
        Ok(Filter {
            target: filter.target,
            span: filter
                .span
                .map(|filters| {
                    filters
                        .map(|filter| SpanFilter::try_from(filter?))
                        .collect()
                })
                .transpose()?,
            level: filter.level,
        })
    }
}

/// A single span filter, `[span{field=value}]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanFilter<'a> {
    pub name: &'a str,
    pub fields: Option<Vec<FieldFilter<'a>>>,
}

impl<'a> TryFrom<crate::SpanFilter<'a>> for SpanFilter<'a> {
    type Error = ParseError;

    fn try_from(filter: crate::SpanFilter<'a>) -> Result<Self, Self::Error> {
        Ok(SpanFilter {
            name: filter.name,
            fields: filter.fields.map(|filters| filters.collect()).transpose()?,
        })
    }
}
