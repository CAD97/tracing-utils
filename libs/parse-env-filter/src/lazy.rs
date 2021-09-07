use crate::ParseError;

/// Parse a series of filters out of a directive string.
///
/// Note that this is a lazy operation, including validation; parsing/validation
/// are done simultaneously and on demand in zero-alloc streaming fashion.
pub fn filters(directives: &str) -> Filters<'_> {
    Filters { directives }
}

/// Parser-iterator of [Filter]s.
#[derive(Debug, Clone)]
pub struct Filters<'a> {
    directives: &'a str,
}

/// A single event filter, `target[span{field=value}]=level`.
///
/// Span directives are not parsed/validated until pulled.
#[derive(Debug, Clone)]
pub struct Filter<'a> {
    pub target: &'a str,
    pub span: Option<SpanFilters<'a>>,
    pub level: Option<&'a str>,
}

/// Parser-iterator of [SpanFilter]s.
#[derive(Debug, Clone)]
pub struct SpanFilters<'a> {
    directives: &'a str,
}

/// A single span filter, `[span{field=value}]`.
///
/// Field directives are not parsed/validated until pulled.
#[derive(Debug, Clone)]
pub struct SpanFilter<'a> {
    pub name: &'a str,
    pub fields: Option<FieldFilters<'a>>,
}

/// Parser-iterator of [FieldFilter]s.
#[derive(Debug, Clone)]
pub struct FieldFilters<'a> {
    directives: &'a str,
}

/// A single field filter, `{field=value}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldFilter<'a> {
    pub name: &'a str,
    pub value: Option<&'a str>,
}

#[repr(u8)]
#[derive(Clone, Copy)]
enum Syntax {
    LBrack = b'[',
    RBrack = b']',
    LBrace = b'{',
    RBrace = b'}',
    Equal = b'=',
    Comma = b',',
}

fn find_any_syntax(haystack: &str) -> (usize, Option<Syntax>) {
    use Syntax::*;
    haystack
        .bytes()
        .enumerate()
        .find_map(|(i, b)| match b {
            b'[' => Some((i, LBrack)),
            b']' => Some((i, RBrack)),
            b'{' => Some((i, LBrace)),
            b'}' => Some((i, RBrace)),
            b'=' => Some((i, Equal)),
            b',' => Some((i, Comma)),
            _ => None,
        })
        .map_or_else(|| (haystack.len(), None), |(i, c)| (i, Some(c)))
}

// % represents end-of-text
macro_rules! switch_syntax {
    ($haystack:expr => |$i:ident| {
        $($($syntax:tt)|+ => $expr:expr),* $(,)?
    }) => {
        #[allow(unused_variables)]
        match find_any_syntax($haystack) {
            $(($i, $(switch_syntax!(@syntax $syntax))|+) => $expr,)*
        }
    };

    (@syntax '[') => (Some(Syntax::LBrack));
    (@syntax ']') => (Some(Syntax::RBrack));
    (@syntax '{') => (Some(Syntax::LBrace));
    (@syntax '}') => (Some(Syntax::RBrace));
    (@syntax '=') => (Some(Syntax::Equal));
    (@syntax ',') => (Some(Syntax::Comma));
    (@syntax  % ) => (None);
}

fn find_syntax(haystack: &str, syntax: Syntax) -> Option<usize> {
    haystack.bytes().position(move |b| b == syntax as u8)
}

impl<'a> Filters<'a> {
    fn err<T>(&mut self) -> Result<T, ParseError> {
        self.directives = "";
        Err(ParseError::BadSyntax)
    }

    fn target(&mut self) -> Result<&'a str, ParseError> {
        switch_syntax!(self.directives => |i| {
            // target]
            // target{
            // target}
            //       👆
            ']' | '{' | '}' => self.err(),

            // target[
            // target=
            // target,
            // target%
            //       👆
            '[' | '=' | ',' | % => {
                let target = &self.directives[..i];
                self.directives = &self.directives[i..];
                Ok(target)
            },
        })
    }

    fn span(&mut self) -> Result<Option<SpanFilters<'a>>, ParseError> {
        // at this point, we know directive starts with one of `[=,%`
        if let Some(stripped) = self.directives.strip_prefix('[') {
            self.directives = stripped;
            match find_syntax(self.directives, Syntax::RBrack) {
                None => self.err(),
                // span]
                //     👆
                Some(i) => {
                    let directives = &self.directives[..i];
                    self.directives = &self.directives[i + 1..];
                    Ok(Some(SpanFilters { directives }))
                }
            }
        } else {
            Ok(None)
        }
    }

    fn level(&mut self) -> Result<Option<&'a str>, ParseError> {
        // validate we have no junk after span directive
        if self.directives.is_empty() || self.directives.starts_with(',') {
            return Ok(None);
        }
        if let Some(stripped) = self.directives.strip_prefix('=') {
            self.directives = stripped;
        } else {
            return self.err();
        }
        switch_syntax!(self.directives => |i| {
            // level[
            // level]
            // level{
            // level}
            // level=
            //      👆
            '[' | ']' | '{' | '}' | '=' => self.err(),

            // level,
            // level%
            //      👆
            ',' | % => {
                let level = &self.directives[..i];
                self.directives = &self.directives[i..];
                Ok(Some(level))
            },
        })
    }

    fn comma(&mut self) -> Result<(), ParseError> {
        if let Some(stripped) = self.directives.strip_prefix(',') {
            self.directives = stripped;
            Ok(())
        } else if self.directives.is_empty() {
            Ok(())
        } else {
            self.err()
        }
    }
}

impl<'a> Iterator for Filters<'a> {
    type Item = Result<Filter<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.directives.is_empty() {
            return None;
        }

        // Reserved syntax
        if self.directives.contains('"') || self.directives.contains('/') {
            let _ = self.err::<()>();
            return Some(Err(ParseError::ReservedSyntax));
        }

        Some((|| {
            let target = self.target()?;
            let span = self.span()?;
            let level = self.level()?;
            self.comma()?;
            Ok(Filter {
                target,
                span,
                level,
            })
        })())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            0,
            Some(
                self.directives
                    .as_bytes()
                    .iter()
                    .filter(|&&b| b == b',')
                    .count()
                    + !self.directives.is_empty() as usize,
            ),
        )
    }
}

impl<'a> SpanFilters<'a> {
    fn err<T>(&mut self) -> Result<T, ParseError> {
        self.directives = "";
        Err(ParseError::BadSyntax)
    }

    fn name(&mut self) -> Result<&'a str, ParseError> {
        switch_syntax!(self.directives => |i| {
            // span[
            // span]
            // span}
            // span=
            //     👆
            '[' | ']' | '}' | '=' => self.err(),

            // span{
            // span,
            // span%
            //     👆
            '{' | ',' | % => {
                let name = &self.directives[..i];
                self.directives = &self.directives[i..];
                Ok(name)
            },
        })
    }

    fn fields(&mut self) -> Result<Option<FieldFilters<'a>>, ParseError> {
        // at this point, we know directive starts with one of `{,%`
        if let Some(stripped) = self.directives.strip_prefix('{') {
            self.directives = stripped;
            match find_syntax(self.directives, Syntax::RBrace) {
                None => self.err(),
                // field}
                //      👆
                Some(i) => {
                    let directives = &self.directives[..i];
                    self.directives = &self.directives[i + 1..];
                    Ok(Some(FieldFilters { directives }))
                }
            }
        } else {
            Ok(None)
        }
    }

    fn comma(&mut self) -> Result<(), ParseError> {
        if let Some(stripped) = self.directives.strip_prefix(',') {
            self.directives = stripped;
            Ok(())
        } else if self.directives.is_empty() {
            Ok(())
        } else {
            self.err()
        }
    }
}

impl<'a> Iterator for SpanFilters<'a> {
    type Item = Result<SpanFilter<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.directives.is_empty() {
            return None;
        }

        // Reserved syntax
        if self.directives.contains('"') || self.directives.contains('/') {
            let _ = self.err::<()>();
            return Some(Err(ParseError::ReservedSyntax));
        }

        Some((|| {
            let name = self.name()?;
            let fields = self.fields()?;
            self.comma()?;
            Ok(SpanFilter { name, fields })
        })())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            0,
            Some(
                self.directives
                    .as_bytes()
                    .iter()
                    .filter(|&&b| b == b',')
                    .count()
                    + !self.directives.is_empty() as usize,
            ),
        )
    }
}

impl<'a> FieldFilters<'a> {
    fn err<T>(&mut self) -> Result<T, ParseError> {
        self.directives = "";
        Err(ParseError::BadSyntax)
    }

    fn name(&mut self) -> Result<&'a str, ParseError> {
        switch_syntax!(self.directives => |i| {
            // field[
            // field]
            // field{
            // field}
            //      👆
            '[' | ']' | '{' | '}' => self.err(),

            // field=
            // field,
            // field%
            //      👆
            '=' | ',' | % => {
                let name = &self.directives[..i];
                self.directives = &self.directives[i..];
                Ok(name)
            },
        })
    }

    fn value(&mut self) -> Result<Option<&'a str>, ParseError> {
        // at this point, we know directive starts with one of `=,%`
        if let Some(stripped) = self.directives.strip_prefix('=') {
            self.directives = stripped;
        } else {
            return Ok(None);
        }
        switch_syntax!(self.directives => |i| {
            // value[
            // value]
            // value{
            // value}
            // value=
            //      👆
            '[' | ']' | '{' | '}' | '=' => self.err(),

            // value,
            // value%
            //      👆
            ',' | % => {
                let value = &self.directives[..i];
                self.directives = &self.directives[i..];
                Ok(Some(value))
            },
        })
    }

    fn comma(&mut self) -> Result<(), ParseError> {
        if let Some(stripped) = self.directives.strip_prefix(',') {
            self.directives = stripped;
            Ok(())
        } else if self.directives.is_empty() {
            Ok(())
        } else {
            self.err()
        }
    }
}

impl<'a> Iterator for FieldFilters<'a> {
    type Item = Result<FieldFilter<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.directives.is_empty() {
            return None;
        }

        // Reserved syntax
        if self.directives.contains('"') || self.directives.contains('/') {
            let _ = self.err::<()>();
            return Some(Err(ParseError::ReservedSyntax));
        }

        Some((|| {
            let name = self.name()?;
            let value = self.value()?;
            self.comma()?;
            Ok(FieldFilter { name, value })
        })())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            0,
            Some(
                self.directives
                    .as_bytes()
                    .iter()
                    .filter(|&&b| b == b',')
                    .count()
                    + !self.directives.is_empty() as usize,
            ),
        )
    }
}
