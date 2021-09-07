Parsing support for a log target filter, as used by [env_logger] and [tracing::EnvFilter].

We always support the extended tracing format, that is

```text
target[span{field=value}]=level
```

with the following validity rules:

- All fields are optional, and MAY be omitted
- All fields MUST NOT contain the syntax characters `[]{}=,"/`
  - This may be relaxed in the future, to allow e.g. matched brackets in `value` and/or quoting values
- Unlike env_logger/tracing::EnvFilter, we treat a bare level name as a target, not a level directive
  - Adding this behavior back is simple — check if only a target is set and if so try it as a level

Note that these rules do not trim whitespace; you'll likely want to yourself.
If you want further verification, you can add it on after the parse step.
This crate is merely intended to pull the directives out of the format, not
to ensure that the directives are otherwise well-formed or meaningful.

## Features not supported

- With tracing::EnvFilter, parsing is ad-hoc and can often allow odd edge cases through.
  We instead opt to be strict and require exactly matching the syntax, rather than sloppy acceptance.
- Along the same lines, tracing allows (and sometimes expects) using quotes in field filter values.
  Quotes are currently reserved, such that a quoted syntax that allows syntax characters in filter
  fields can be added in the future.
- env_logger supports a global `/regex` directive to filter messages via a regex. This applies
  separately and to all other earlier directives, and as such doesn't quite fit the filter
  iterator design we've taken. However, we have reserved the `/` character for clarity
  and such that a filter field can potentially be added with this syntax in the future.
- tracing::EnvFilter interprets the value of field=value as a regular expression. We explicitly
  leave that level of interpretation up to the consumer, as this library is just for parsing.

[env_logger]: <https://docs.rs/env_logger/>
[tracing::EnvFilter]: <https://docs.rs/tracing-subscriber/0.2/tracing_subscriber/filter/struct.EnvFilter.html>
