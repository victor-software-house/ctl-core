//! A KDL v2 writer for the Usage spec: nodes, their entries, and the forms a
//! string takes.
//!
//! A string takes the shortest form that reads back as the same string:
//!
//! - a bare identifier, when the identifier grammar reads it as that string and
//!   not as a number or keyword;
//! - a raw multi-line string, for multi-line text whose every character and
//!   line a raw string keeps as written;
//! - a quoted string otherwise.
//!
//! A single-line string may not hold a Newline or a disallowed literal code
//! point ([KDL v2], sections 3.13 and 3.19). A quoted string writes each of
//! them as `\u{…}`.
//!
//! [KDL v2]: https://kdl.dev/spec/

use std::fmt::{self, Display, Formatter, Write as _};

use crate::writedoc;

/// The columns one level of children is indented by.
const INDENT: usize = 4;

/// The quotes that open and close a multi-line string.
const MULTILINE_QUOTES: &[u8] = b"\"\"\"";

/// A KDL document: top-level nodes, each written on its own line.
#[derive(Debug)]
pub(crate) struct Document(pub(crate) Vec<Node>);

impl Display for Document {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for node in &self.0 {
            node.write(f, 0)?;
        }
        Ok(())
    }
}

/// One node: a name, then arguments and properties in the order pushed, then
/// children in a block.
#[derive(Debug)]
pub(crate) struct Node {
    name: String,
    entries: Vec<Entry>,
    children: Vec<Node>,
}

impl Node {
    pub(crate) fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            entries: Vec::new(),
            children: Vec::new(),
        }
    }

    pub(crate) fn arg(&mut self, value: Value) {
        self.entries.push(Entry { key: None, value });
    }

    pub(crate) fn prop(&mut self, key: &'static str, value: Value) {
        self.entries.push(Entry {
            key: Some(key),
            value,
        });
    }

    /// Push `key=#true` for each key whose condition holds, in order.
    pub(crate) fn enable(&mut self, keys: impl IntoIterator<Item = (&'static str, bool)>) {
        for (key, on) in keys {
            if on {
                self.prop(key, Value::Bool(true));
            }
        }
    }

    pub(crate) fn child(&mut self, node: Self) {
        self.children.push(node);
    }

    fn write(&self, f: &mut Formatter<'_>, depth: usize) -> fmt::Result {
        let indent = depth * INDENT;
        write!(f, "{:indent$}", "")?;
        write_word(f, &self.name)?;
        for entry in &self.entries {
            f.write_char(' ')?;
            entry.write(f)?;
        }
        if !self.children.is_empty() {
            f.write_str(" {\n")?;
            for child in &self.children {
                child.write(f, depth + 1)?;
            }
            write!(f, "{:indent$}}}", "")?;
        }
        f.write_char('\n')
    }
}

/// An argument, or a property when it has a key.
#[derive(Debug)]
struct Entry {
    key: Option<&'static str>,
    value: Value,
}

impl Entry {
    fn write(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let place = match self.key {
            Some(key) => {
                write_word(f, key)?;
                f.write_char('=')?;
                Place::Property
            }
            None => Place::Argument,
        };
        match &self.value {
            Value::Bool(true) => f.write_str("#true"),
            Value::Bool(false) => f.write_str("#false"),
            Value::Number(number) => write!(f, "{number}"),
            Value::Word(word) => write_word(f, word),
            Value::Text(text) => write_text(f, text, place),
        }
    }
}

/// What an entry holds, which decides how it is written.
#[derive(Debug)]
pub(crate) enum Value {
    /// `#true` or `#false`.
    Bool(bool),
    /// A decimal integer.
    Number(usize),
    /// A name or a value: bare when it is an identifier, quoted otherwise.
    Word(String),
    /// Prose such as help text. Multi-line text is a raw multi-line string.
    /// Text that a raw string would change, and an argument that starts with
    /// `-`, is quoted with every control character escaped.
    Text(String),
}

/// Where an entry sits, which changes how [`Value::Text`] is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    Argument,
    Property,
}

/// Which control characters a quoted string escapes beyond the ones KDL v2
/// forbids literally.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Controls {
    /// Backspace and form feed as `\b` and `\f`; the rest as written.
    Short,
    /// Every control character as `\u{…}`.
    Escaped,
}

fn write_word(f: &mut Formatter<'_>, word: &str) -> fmt::Result {
    if is_identifier(word) {
        f.write_str(word)
    } else {
        write_quoted(f, word, Controls::Short)
    }
}

// The escaped form keeps what a raw string would change or could not hold: a
// control character, a forbidden character, or a whitespace-only line.
// A multi-line string reads such a line back empty. An argument that starts
// with `-` is quoted so a reader never takes it for a flag.
fn write_text(f: &mut Formatter<'_>, text: &str, place: Place) -> fmt::Result {
    let dashed = place == Place::Argument && text.starts_with('-');
    let multiline = text.contains('\n');
    let raw_would_change = text
        .chars()
        .any(|c| c != '\n' && c != '\t' && (c.is_control() || is_forbidden(c)))
        || (multiline && text.split('\n').any(is_blank));
    if dashed || raw_would_change {
        write_quoted(f, text, Controls::Escaped)
    } else if multiline {
        let hashes = "#".repeat(raw_hashes(text));
        writedoc!(
            f,
            r#"
                {hashes}"""
                {text}
                """{hashes}"#
        )
    } else {
        write_word(f, text)
    }
}

fn write_quoted(f: &mut Formatter<'_>, text: &str, controls: Controls) -> fmt::Result {
    f.write_char('"')?;
    for c in text.chars() {
        match c {
            '"' => f.write_str("\\\"")?,
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\r' => f.write_str("\\r")?,
            '\t' => f.write_str("\\t")?,
            '\u{8}' if controls == Controls::Short => f.write_str("\\b")?,
            '\u{c}' if controls == Controls::Short => f.write_str("\\f")?,
            c if is_forbidden(c) || (controls == Controls::Escaped && c.is_control()) => {
                write!(f, "\\u{{{:x}}}", u32::from(c))?;
            }
            c => f.write_char(c)?,
        }
    }
    f.write_char('"')
}

/// The hashes a raw multi-line string of `text` needs: one more than the
/// longest run of `#` after any `"""` in it. Runs are counted at every
/// offset, so `""""#` needs two.
fn raw_hashes(text: &str) -> usize {
    let bytes = text.as_bytes();
    let longest = bytes
        .windows(MULTILINE_QUOTES.len())
        .enumerate()
        .filter(|(_, window)| *window == MULTILINE_QUOTES)
        .map(|(start, _)| {
            bytes
                .iter()
                .skip(start + MULTILINE_QUOTES.len())
                .take_while(|&&byte| byte == b'#')
                .count()
        })
        .max()
        .unwrap_or(0);
    longest + 1
}

/// Whether `text` written bare reads back as the same string: an identifier
/// string that is not a reserved keyword and does not start like a number.
fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    let after_first = chars.as_str();
    let starts_bare = match first {
        '0'..='9' => false,
        '-' | '+' => match after_first.strip_prefix('.') {
            Some(after_dot) => !starts_with_digit(after_dot),
            None => !starts_with_digit(after_first),
        },
        '.' => !starts_with_digit(after_first),
        _ => true,
    };
    starts_bare
        && !matches!(text, "true" | "false" | "null" | "inf" | "-inf" | "nan")
        && text.chars().all(is_identifier_char)
}

fn starts_with_digit(text: &str) -> bool {
    text.starts_with(|c: char| c.is_ascii_digit())
}

fn is_identifier_char(c: char) -> bool {
    !(matches!(
        c,
        '\\' | '/' | '(' | ')' | '{' | '}' | '[' | ']' | ';' | '"' | '#' | '='
    ) || is_space(c)
        || is_forbidden(c))
}

/// A line holding only whitespace, which a multi-line string reads back empty.
fn is_blank(line: &str) -> bool {
    !line.is_empty() && line.chars().all(is_space)
}

/// A character KDL v2 forbids literally in a single-line string: a Newline or
/// a disallowed literal code point, U+007F included.
fn is_forbidden(c: char) -> bool {
    matches!(
        c,
        '\r' | '\n'
            | '\u{85}'
            | '\u{b}'
            | '\u{c}'
            | '\u{2028}'
            | '\u{2029}'
            | '\0'..='\u{8}'
            | '\u{e}'..='\u{1f}'
            | '\u{7f}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
            | '\u{feff}'
    )
}

/// A KDL v2 `unicode-space`.
fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{202f}' | '\u{205f}' | '\u{3000}'
    )
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::{Document, Node, Value, is_identifier, raw_hashes};

    fn render(node: Node) -> String {
        Document(vec![node]).to_string()
    }

    #[test]
    fn an_identifier_is_not_a_number_or_keyword() {
        for bare in ["a", "-x", "--x", "-", ".", "-.", "-.x", "'", "<x>…", "a,b"] {
            assert!(is_identifier(bare), "{bare:?} should be bare");
        }
        for quoted in [
            "",
            "1",
            "-1",
            "+1",
            ".5",
            "-.5",
            "+.5",
            "true",
            "-inf",
            "nan",
            "#true",
            "a b",
            "a=b",
            "a\u{7f}b",
            "a\u{2028}b",
            "{",
        ] {
            assert!(!is_identifier(quoted), "{quoted:?} should be quoted");
        }
    }

    #[test]
    fn raw_hashes_count_overlapping_quotes() {
        assert_eq!(raw_hashes("plain"), 1);
        assert_eq!(raw_hashes("x\n\"\"\"#\ny"), 2);
        assert_eq!(raw_hashes("x\n\"\"\"\"#\ny"), 2);
        assert_eq!(raw_hashes("x\n\"\"\"##\ny"), 3);
    }

    #[test]
    fn text_takes_the_form_that_reads_back() {
        let mut node = Node::new("n");
        node.arg(Value::Text("-x".to_owned()));
        node.prop("dash", Value::Text("-x".to_owned()));
        node.prop("line", Value::Text("a\u{2028}b".to_owned()));
        node.prop("blank", Value::Text("a\n  \nb".to_owned()));
        let expected = indoc! {r#"
            n "-x" dash=-x line="a\u{2028}b" blank="a\n  \nb"
        "#};
        assert_eq!(render(node), expected);
    }

    #[test]
    fn children_indent_and_raw_text_does_not() {
        let mut child = Node::new("long_help");
        child.arg(Value::Text("a\nb".to_owned()));
        let mut node = Node::new("flag");
        node.arg(Value::Word("--x".to_owned()));
        node.enable([("hide", true), ("global", false)]);
        node.child(child);
        let expected = indoc! {r##"
            flag --x hide=#true {
                long_help #"""
            a
            b
            """#
            }
        "##};
        assert_eq!(render(node), expected);
    }
}
