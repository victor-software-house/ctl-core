//! YAML scalars resolve by the rules vsh-serde-saphyr applies: one row per
//! line of the scalar table in the open-input research notes.

use std::borrow::Cow;

use ctl_input::format::{Format, Mismatch, Resolved, Wanted, YAML};
use ctl_input::tree::{NodeKind, Scalar, ScalarStyle, Tag};

use crate::common::{borrowed, get, read};

const STR_TAG: &str = "tag:yaml.org,2002:str";

fn resolve<'a>(
    scalar: &'a Scalar,
    tag: Option<&str>,
    wanted: Wanted,
) -> Result<Resolved<'a>, Mismatch> {
    YAML.resolve(scalar, tag.map(Tag::new).as_ref(), wanted)
}

fn plain(text: &str) -> Scalar {
    Scalar::new(text, ScalarStyle::Plain)
}

/// What each `Wanted` gives for `scalar`, in the order Any, Bool, Int, Float,
/// Str, Null: `None` for a mismatch.
fn row<'a>(scalar: &'a Scalar, tag: Option<&str>) -> [Option<Resolved<'a>>; 6] {
    [
        Wanted::Any,
        Wanted::Bool,
        Wanted::Int,
        Wanted::Float,
        Wanted::Str,
        Wanted::Null,
    ]
    .map(|wanted| resolve(scalar, tag, wanted).ok())
}

#[test]
fn a_float_with_a_trailing_zero_is_not_an_integer_or_a_bool() {
    let scalar = plain("1.10");
    assert_eq!(
        row(&scalar, None),
        [
            Some(Resolved::Float(1.1)),
            None,
            None,
            Some(Resolved::Float(1.1)),
            Some(borrowed("1.10")),
            None,
        ]
    );
}

#[test]
fn a_leading_zero_is_a_float_and_never_an_integer() {
    let scalar = plain("010");
    assert_eq!(
        row(&scalar, None),
        [
            Some(Resolved::Float(10.0)),
            None,
            None,
            Some(Resolved::Float(10.0)),
            Some(borrowed("010")),
            None,
        ]
    );
}

#[test]
fn the_three_radixes_are_integers() {
    for (text, number) in [
        ("0x10", 16),
        ("0o17", 15),
        ("0b101", 5),
        ("-7", -7),
        ("42", 42),
    ] {
        let scalar = plain(text);
        let found = row(&scalar, None);
        assert_eq!(found[0], Some(Resolved::Int(number)), "{text}");
        assert_eq!(found[2], Some(Resolved::Int(number)), "{text}");
        assert_eq!(found[4], Some(borrowed(text)), "{text}");
        assert_eq!(found[1], None, "{text}");
        assert_eq!(found[5], None, "{text}");
    }
}

#[test]
fn an_integer_above_the_signed_range_is_unsigned() {
    let text = "340282366920938463463374607431768211455";
    let scalar = plain(text);
    assert_eq!(
        resolve(&scalar, None, Wanted::Int),
        Ok(Resolved::Uint(u128::MAX))
    );
}

#[test]
fn infinity_and_not_a_number_fill_a_float_but_not_a_typeless_read() {
    for text in [".inf", "-.inf", ".nan"] {
        let scalar = plain(text);
        assert_eq!(
            resolve(&scalar, None, Wanted::Any),
            Err(Mismatch::new(Wanted::Any)),
            "{text}"
        );
        assert_eq!(
            resolve(&scalar, None, Wanted::Str),
            Ok(borrowed(text)),
            "{text}"
        );
        assert_eq!(
            resolve(&scalar, None, Wanted::Int),
            Err(Mismatch::new(Wanted::Int)),
            "{text}"
        );
    }
    let infinite = plain(".inf");
    assert_eq!(
        resolve(&infinite, None, Wanted::Float),
        Ok(Resolved::Float(f64::INFINITY))
    );
    let nan = plain(".nan");
    assert!(matches!(
        resolve(&nan, None, Wanted::Float),
        Ok(Resolved::Float(number)) if number.is_nan()
    ));
}

#[test]
fn only_the_six_yaml_1_2_spellings_fill_a_bool() {
    for (text, truth) in [
        ("true", true),
        ("True", true),
        ("TRUE", true),
        ("false", false),
        ("False", false),
        ("FALSE", false),
    ] {
        let scalar = plain(text);
        let found = row(&scalar, None);
        assert_eq!(found[0], Some(Resolved::Bool(truth)), "{text}");
        assert_eq!(found[1], Some(Resolved::Bool(truth)), "{text}");
        assert_eq!(found[4], Some(borrowed(text)), "{text}");
    }
    for text in ["no", "yes", "on", "off", "y", "n"] {
        let scalar = plain(text);
        let found = row(&scalar, None);
        assert_eq!(found[0], Some(borrowed(text)), "{text}");
        assert_eq!(found[1], None, "{text}");
        assert_eq!(found[4], Some(borrowed(text)), "{text}");
    }
}

#[test]
fn a_plain_null_fills_only_a_null() {
    for text in ["~", "null", "Null", "NULL", ""] {
        let scalar = plain(text);
        assert_eq!(
            row(&scalar, None),
            [
                Some(Resolved::Null),
                None,
                None,
                None,
                None,
                Some(Resolved::Null)
            ],
            "{text:?}"
        );
    }
}

#[test]
fn a_quoted_scalar_is_a_string_for_any_and_still_fills_a_typed_field() {
    let number = Scalar::new("8", ScalarStyle::DoubleQuoted);
    let found = row(&number, None);
    assert_eq!(found[0], Some(borrowed("8")));
    assert_eq!(found[2], Some(Resolved::Int(8)));
    assert_eq!(found[4], Some(borrowed("8")));
    let truth = Scalar::new("true", ScalarStyle::SingleQuoted);
    let found = row(&truth, None);
    assert_eq!(found[0], Some(borrowed("true")));
    assert_eq!(found[1], Some(Resolved::Bool(true)));
    let empty = Scalar::new("", ScalarStyle::DoubleQuoted);
    let found = row(&empty, None);
    assert_eq!(
        found[4],
        Some(borrowed("")),
        "a quoted empty string is a string"
    );
    assert_eq!(found[5], None);
}

#[test]
fn a_string_tag_makes_the_scalar_text() {
    let scalar = plain("1");
    let found = row(&scalar, Some(STR_TAG));
    assert_eq!(found[0], Some(borrowed("1")));
    assert_eq!(found[4], Some(borrowed("1")));
    assert!(
        found[1..4].iter().all(Option::is_none),
        "a string tag refuses a typed field, as the fork's validate_tag does: {found:?}"
    );
    let null = plain("~");
    assert_eq!(
        resolve(&null, Some(STR_TAG), Wanted::Str),
        Ok(borrowed("~")),
        "a string tag keeps a null spelling a string"
    );
}

#[test]
fn a_local_tag_does_not_change_how_the_scalar_reads() {
    let scalar = plain("0");
    assert_eq!(
        resolve(&scalar, Some("!override"), Wanted::Int),
        Ok(Resolved::Int(0))
    );
}

#[test]
fn a_binary_scalar_resolves_to_its_decoded_text_as_owned() {
    let text = "x: !!binary aGVsbG8=\n";
    let root = read(&YAML, text);
    let node = get(&root, "x");
    let NodeKind::Scalar(scalar) = node.kind() else {
        panic!("not a scalar");
    };
    let resolved = YAML.resolve(scalar, node.tag(), Wanted::Any);
    assert_eq!(resolved, Ok(borrowed("hello")));
    assert!(
        matches!(resolved, Ok(Resolved::Str(Cow::Owned(_)))),
        "{resolved:?}"
    );
}

#[test]
fn a_tagged_non_string_is_refused_for_a_string_field() {
    for (text, tag) in [
        ("8080", "tag:yaml.org,2002:int"),
        ("~", "tag:yaml.org,2002:int"),
        ("1.5", "tag:yaml.org,2002:float"),
    ] {
        let scalar = plain(text);
        assert_eq!(
            resolve(&scalar, Some(tag), Wanted::Str),
            Err(Mismatch::new(Wanted::Str)),
            "{tag} {text}"
        );
    }
}

#[test]
fn a_binary_scalar_is_decoded_as_owned_text_for_a_string_field() {
    let scalar = plain("aGk=");
    let resolved = resolve(&scalar, Some("tag:yaml.org,2002:binary"), Wanted::Str);
    assert_eq!(resolved, Ok(borrowed("hi")));
    assert!(matches!(resolved, Ok(Resolved::Str(Cow::Owned(_)))));
}

#[test]
fn a_null_spelling_is_text_when_it_is_quoted_or_tagged_a_string() {
    let tilde = Scalar::new("~", ScalarStyle::SingleQuoted);
    assert_eq!(resolve(&tilde, None, Wanted::Str), Ok(borrowed("~")));
    let tagged = plain("~");
    assert_eq!(
        resolve(&tagged, Some(STR_TAG), Wanted::Str),
        Ok(borrowed("~"))
    );
    assert!(matches!(
        resolve(&tagged, Some(STR_TAG), Wanted::Str),
        Ok(Resolved::Str(Cow::Borrowed(_)))
    ));
}

#[test]
fn a_plain_string_resolves_borrowed_from_the_scalar() {
    let scalar = plain("demo");
    assert!(matches!(
        resolve(&scalar, None, Wanted::Any),
        Ok(Resolved::Str(Cow::Borrowed("demo")))
    ));
}
