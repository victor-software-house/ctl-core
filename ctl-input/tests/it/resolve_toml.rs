//! TOML scalars resolve by how they are written, as the `toml` crate reads a
//! field: a string fills only a string field, and a number only a number.

use ctl_input::format::{Format, Mismatch, Resolved, TOML, Wanted};
use ctl_input::tree::{NodeKind, Scalar, Tag};
use indoc::indoc;

use crate::common::{get, read};

/// The scalar the TOML text `line` writes as `value`.
fn scalar(line: &str) -> Scalar {
    let root = read(&TOML, line);
    let NodeKind::Scalar(scalar) = get(&root, "value").kind() else {
        panic!("not a scalar");
    };
    scalar.clone()
}

fn resolve(scalar: &Scalar, wanted: Wanted) -> Result<Resolved<'_>, Mismatch> {
    TOML.resolve(scalar, None, wanted)
}

#[test]
fn a_quoted_string_fills_only_a_string_field() {
    for line in [
        "value = \"8080\"",
        "value = '8080'",
        "value = \"\"\"8080\"\"\"",
        "value = '''8080'''",
    ] {
        let found = scalar(line);
        assert_eq!(
            resolve(&found, Wanted::Str),
            Ok(Resolved::Str("8080")),
            "{line}"
        );
        assert_eq!(
            resolve(&found, Wanted::Any),
            Ok(Resolved::Str("8080")),
            "{line}"
        );
        for wanted in [Wanted::Int, Wanted::Float, Wanted::Bool, Wanted::Null] {
            assert_eq!(
                resolve(&found, wanted),
                Err(Mismatch::new(wanted)),
                "{line}"
            );
        }
    }
}

#[test]
fn a_quoted_boolean_is_a_string() {
    let found = scalar("value = \"true\"");
    assert_eq!(
        resolve(&found, Wanted::Bool),
        Err(Mismatch::new(Wanted::Bool))
    );
}

#[test]
fn a_boolean_fills_a_bool_field_only() {
    let found = scalar("value = false");
    assert_eq!(resolve(&found, Wanted::Bool), Ok(Resolved::Bool(false)));
    assert_eq!(resolve(&found, Wanted::Any), Ok(Resolved::Bool(false)));
    for wanted in [Wanted::Int, Wanted::Float, Wanted::Str, Wanted::Null] {
        assert_eq!(resolve(&found, wanted), Err(Mismatch::new(wanted)));
    }
}

#[test]
fn an_integer_fills_an_integer_or_a_float_field() {
    for (line, number) in [
        ("value = 8080", 8080),
        ("value = 0xFF", 255),
        ("value = 0o17", 15),
        ("value = 0b101", 5),
        ("value = 1_000", 1000),
        ("value = -5", -5),
    ] {
        let found = scalar(line);
        assert_eq!(
            resolve(&found, Wanted::Int),
            Ok(Resolved::Int(number)),
            "{line}"
        );
        assert_eq!(
            resolve(&found, Wanted::Any),
            Ok(Resolved::Int(number)),
            "{line}"
        );
        assert!(
            matches!(resolve(&found, Wanted::Float), Ok(Resolved::Float(_))),
            "{line}"
        );
        for wanted in [Wanted::Bool, Wanted::Str, Wanted::Null] {
            assert_eq!(
                resolve(&found, wanted),
                Err(Mismatch::new(wanted)),
                "{line}"
            );
        }
    }
}

#[test]
fn a_float_fills_a_float_field_and_not_an_integer_field() {
    for (line, number) in [
        ("value = 2.5", 2.5),
        ("value = 1e3", 1000.0),
        ("value = inf", f64::INFINITY),
        ("value = -inf", f64::NEG_INFINITY),
    ] {
        let found = scalar(line);
        assert_eq!(
            resolve(&found, Wanted::Float),
            Ok(Resolved::Float(number)),
            "{line}"
        );
        assert_eq!(
            resolve(&found, Wanted::Any),
            Ok(Resolved::Float(number)),
            "{line}"
        );
        assert_eq!(
            resolve(&found, Wanted::Int),
            Err(Mismatch::new(Wanted::Int)),
            "{line}"
        );
    }
    let not_a_number = scalar("value = nan");
    assert!(matches!(
        resolve(&not_a_number, Wanted::Float),
        Ok(Resolved::Float(number)) if number.is_nan()
    ));
}

#[test]
fn a_datetime_fills_a_string_field_as_its_text() {
    for text in ["1979-05-27T07:32:00Z", "1979-05-27", "07:32:00"] {
        let found = scalar(&format!("value = {text}"));
        assert_eq!(
            resolve(&found, Wanted::Str),
            Ok(Resolved::Str(text)),
            "{text}"
        );
        assert_eq!(
            resolve(&found, Wanted::Any),
            Ok(Resolved::Str(text)),
            "{text}"
        );
        assert_eq!(
            resolve(&found, Wanted::Int),
            Err(Mismatch::new(Wanted::Int)),
            "{text}"
        );
    }
}

#[test]
fn no_scalar_fills_a_null_field() {
    let text = indoc! {r#"
        a = "null"
        b = 0
        c = false
    "#};
    let root = read(&TOML, text);
    for key in ["a", "b", "c"] {
        let NodeKind::Scalar(found) = get(&root, key).kind() else {
            panic!("not a scalar");
        };
        assert_eq!(
            resolve(found, Wanted::Null),
            Err(Mismatch::new(Wanted::Null)),
            "{key}"
        );
    }
}

#[test]
fn the_tag_is_ignored() {
    let found = scalar("value = 1");
    let tag = Tag::new("!override");
    assert_eq!(
        TOML.resolve(&found, Some(&tag), Wanted::Int),
        Ok(Resolved::Int(1))
    );
}
