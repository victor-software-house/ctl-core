//! The scanner: the tokens of RFC 8259 with their byte spans, strings decoded
//! and numbers kept as written.

use std::ops::Range;

use crate::tree::{Scalar, ScalarStyle, ScalarSyntax};

/// What a [`Token`] is.
#[derive(Debug)]
pub(super) enum Kind {
    BeginObject,
    EndObject,
    BeginArray,
    EndArray,
    Colon,
    Comma,
    /// A string, a number, `true`, `false` or `null`, with the type its
    /// syntax gave it.
    Scalar(Scalar),
    /// Text that starts no token: a character RFC 8259 has no use for, or a
    /// word that is no literal.
    Other,
    /// The end of the text.
    End,
}

/// A token and the bytes it was written in.
#[derive(Debug)]
pub(super) struct Token {
    pub(super) kind: Kind,
    pub(super) span: Range<usize>,
}

/// A grammar rule a token broke, with the bytes that broke it.
#[derive(Debug)]
pub(super) struct Failure {
    pub(super) span: Range<usize>,
    pub(super) detail: String,
}

/// Reads tokens from a text, one at a time.
#[derive(Debug)]
pub(super) struct Scanner<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Scanner<'a> {
    /// A scanner at the start of `text`, past a leading byte-order mark, which
    /// RFC 8259 section 8.1 lets a reader skip.
    pub(super) fn new(text: &'a str) -> Self {
        let at = if text.starts_with('\u{feff}') {
            '\u{feff}'.len_utf8()
        } else {
            0
        };
        Self { text, at }
    }

    /// The next token. The end of the text is a token, not an error.
    pub(super) fn next(&mut self) -> Result<Token, Failure> {
        self.skip_whitespace();
        let start = self.at;
        let Some(first) = self.byte(start) else {
            return Ok(Token {
                kind: Kind::End,
                span: start..start,
            });
        };
        let kind = match first {
            b'{' => Kind::BeginObject,
            b'}' => Kind::EndObject,
            b'[' => Kind::BeginArray,
            b']' => Kind::EndArray,
            b':' => Kind::Colon,
            b',' => Kind::Comma,
            b'"' => return self.string(),
            b'-' | b'0'..=b'9' => return self.number(),
            b'a'..=b'z' | b'A'..=b'Z' => return Ok(self.word()),
            _ => {
                self.at += self.width_at(start);
                Kind::Other
            }
        };
        if !matches!(kind, Kind::Other) {
            self.at += 1;
        }
        Ok(Token {
            kind,
            span: start..self.at,
        })
    }

    /// The first character after the whitespace that follows the value, if
    /// the text has one.
    pub(super) fn trailing(&mut self) -> Option<Range<usize>> {
        self.skip_whitespace();
        self.byte(self.at)
            .map(|_| self.at..self.at + self.width_at(self.at))
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.byte(self.at), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn byte(&self, at: usize) -> Option<u8> {
        self.text.as_bytes().get(at).copied()
    }

    /// The bytes of the character that starts at `at`.
    fn width_at(&self, at: usize) -> usize {
        self.text
            .get(at..)
            .and_then(|rest| rest.chars().next())
            .map_or(1, char::len_utf8)
    }

    fn source(&self, span: Range<usize>) -> &'a str {
        self.text.get(span).unwrap_or_default()
    }

    /// A run of letters and digits: `true`, `false` or `null`, or else text
    /// that starts no token.
    fn word(&mut self) -> Token {
        let start = self.at;
        while matches!(
            self.byte(self.at),
            Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')
        ) {
            self.at += 1;
        }
        let span = start..self.at;
        let literal = match self.source(span.clone()) {
            "true" => Some(("true", ScalarSyntax::Boolean)),
            "false" => Some(("false", ScalarSyntax::Boolean)),
            "null" => Some(("null", ScalarSyntax::Null)),
            _ => None,
        };
        let kind = literal.map_or(Kind::Other, |(written, syntax)| {
            Kind::Scalar(Scalar::new(written, ScalarStyle::Plain).with_syntax(syntax))
        });
        Token { kind, span }
    }

    /// A number, by `-? (0 | [1-9][0-9]*) (\.[0-9]+)? ([eE][+-]?[0-9]+)?`. The
    /// run of characters a number can hold is read whole, so a number that
    /// breaks the grammar is refused as it was written.
    fn number(&mut self) -> Result<Token, Failure> {
        let start = self.at;
        while matches!(
            self.byte(self.at),
            Some(b'0'..=b'9' | b'+' | b'-' | b'.' | b'e' | b'E')
        ) {
            self.at += 1;
        }
        let span = start..self.at;
        let written = self.source(span.clone());
        let Some(float) = number_shape(written.as_bytes()) else {
            return Err(Failure {
                span,
                detail: format!("number `{written}` is not valid JSON"),
            });
        };
        let syntax = if float {
            ScalarSyntax::Float
        } else {
            ScalarSyntax::Integer
        };
        Ok(Token {
            kind: Kind::Scalar(Scalar::new(written, ScalarStyle::Plain).with_syntax(syntax)),
            span,
        })
    }

    /// A string, from its opening quote to its closing one, with each escape
    /// decoded.
    fn string(&mut self) -> Result<Token, Failure> {
        let start = self.at;
        let mut value = String::new();
        let mut at = start + 1;
        let mut run = at;
        loop {
            match self.byte(at) {
                None => return Err(self.unclosed(start)),
                Some(b'"') => {
                    value.push_str(self.source(run..at));
                    self.at = at + 1;
                    let scalar = Scalar::new(value, ScalarStyle::DoubleQuoted)
                        .with_syntax(ScalarSyntax::Text);
                    return Ok(Token {
                        kind: Kind::Scalar(scalar),
                        span: start..self.at,
                    });
                }
                Some(b'\\') => {
                    value.push_str(self.source(run..at));
                    at = self.escape(start, at, &mut value)?;
                    run = at;
                }
                Some(control @ 0x00..=0x1f) => {
                    return Err(Failure {
                        span: at..at + 1,
                        detail: format!("control character U+{control:04X} must be escaped"),
                    });
                }
                Some(_) => at += 1,
            }
        }
    }

    fn unclosed(&self, start: usize) -> Failure {
        Failure {
            span: start..self.text.len(),
            detail: "a string is not closed".to_owned(),
        }
    }

    /// Decodes the escape that starts at the backslash at `at` into `value`,
    /// and returns where the string goes on. `start` is the opening quote.
    fn escape(&self, start: usize, at: usize, value: &mut String) -> Result<usize, Failure> {
        let Some(letter) = self.text.get(at + 1..).and_then(|rest| rest.chars().next()) else {
            return Err(self.unclosed(start));
        };
        let end = at + 1 + letter.len_utf8();
        let decoded = match letter {
            '"' | '\\' | '/' => letter,
            'b' => '\u{8}',
            'f' => '\u{c}',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'u' => return self.unicode(start, at, value),
            _ => {
                return Err(Failure {
                    span: at..end,
                    detail: format!("unknown escape `\\{letter}`"),
                });
            }
        };
        value.push(decoded);
        Ok(end)
    }

    /// Decodes `\uXXXX` at `at`, with the second escape of a surrogate pair
    /// when the first is a high surrogate. A lone surrogate is refused, because
    /// a Rust string cannot hold one.
    fn unicode(&self, start: usize, at: usize, value: &mut String) -> Result<usize, Failure> {
        let first = self.hex4(start, at)?;
        let after = at + 6;
        let decoded = match first {
            0xd800..=0xdbff => {
                let low = (self.byte(after) == Some(b'\\') && self.byte(after + 1) == Some(b'u'))
                    .then(|| self.hex4(start, after))
                    .transpose()?
                    .filter(|low| (0xdc00..=0xdfff).contains(low));
                low.map(|low| {
                    let high = u32::from(first) - 0xd800;
                    (
                        0x1_0000 + (high << 10) + (u32::from(low) - 0xdc00),
                        after + 6,
                    )
                })
            }
            0xdc00..=0xdfff => None,
            _ => Some((u32::from(first), after)),
        };
        let (character, end) = decoded
            .and_then(|(code, end)| char::from_u32(code).map(|character| (character, end)))
            .ok_or_else(|| Failure {
                span: at..after,
                detail: format!(
                    "escape `{}` is half of a surrogate pair",
                    self.source(at..after)
                ),
            })?;
        value.push(character);
        Ok(end)
    }

    /// The four hex digits after the `\u` at `at`.
    fn hex4(&self, start: usize, at: usize) -> Result<u16, Failure> {
        let mut code = 0_u16;
        for place in at + 2..at + 6 {
            let Some(byte) = self.byte(place) else {
                return Err(self.unclosed(start));
            };
            let Some(digit) = char::from(byte).to_digit(16) else {
                return Err(Failure {
                    span: at..place + self.width_at(place),
                    detail: format!(
                        "unknown escape `{}`",
                        self.source(at..place + self.width_at(place))
                    ),
                });
            };
            code = (code << 4) | u16::try_from(digit).unwrap_or_default();
        }
        Ok(code)
    }
}

/// Whether `written` is a number, and a float when so: it has a fraction or
/// an exponent. `None` when it breaks the grammar.
fn number_shape(written: &[u8]) -> Option<bool> {
    let digits = |from: usize| {
        written.get(from..).map_or(0, |rest| {
            rest.iter().take_while(|byte| byte.is_ascii_digit()).count()
        })
    };
    let mut at = usize::from(written.first() == Some(&b'-'));
    match written.get(at) {
        Some(b'0') => at += 1,
        Some(b'1'..=b'9') => at += digits(at),
        _ => return None,
    }
    let mut float = false;
    if written.get(at) == Some(&b'.') {
        float = true;
        let fraction = digits(at + 1);
        if fraction == 0 {
            return None;
        }
        at += 1 + fraction;
    }
    if matches!(written.get(at), Some(b'e' | b'E')) {
        float = true;
        at += 1;
        if matches!(written.get(at), Some(b'+' | b'-')) {
            at += 1;
        }
        let exponent = digits(at);
        if exponent == 0 {
            return None;
        }
        at += exponent;
    }
    (at == written.len()).then_some(float)
}
