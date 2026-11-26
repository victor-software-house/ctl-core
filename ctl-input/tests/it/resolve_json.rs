//! JSON scalars resolve by how they are written: a string fills only a string
//! field, a number only a number field, and `null` only an optional or unit
//! field.

use ctl_input::format::{Format, JSON, Mismatch, Resolved, Wanted};
use ctl_input::tree::{NodeKind, Scalar, ScalarStyle};

use crate::common::{borrowed, get, read};

/// The scalar the JSON text `json` writes as `value`.
fn scalar(json: &str) -> Scalar {
    let root = read(&JSON, &format!("{{\"value\": {json}}}"));
    let NodeKind::Scalar(scalar) = get(&root, "value").kind() else {
        panic!("not a scalar");
    };
    scalar.clone()
}

fn resolve(scalar: &Scalar, wanted: Wanted) -> Result<Resolved<'_>, Mismatch> {
    JSON.resolve(scalar, None, wanted)
}

const OTHERS: [Wanted; 6] = [
    Wanted::Bool,
    Wanted::Int,
    Wanted::Float,
    Wanted::Str,
    Wanted::Null,
    Wanted::Any,
];

fn refuses_all_but(found: &Scalar, allowed: &[Wanted]) {
    for wanted in OTHERS {
        if !allowed.contains(&wanted) {
            assert_eq!(
                resolve(found, wanted),
                Err(Mismatch::new(wanted)),
                "{found:?} as {wanted:?}"
            );
        }
    }
}

#[test]
fn a_string_fills_only_a_string_field() {
    for json in ["\"8080\"", "\"true\"", "\"null\"", "\"\""] {
        let found = scalar(json);
        let text = found.value().to_owned();
        assert_eq!(resolve(&found, Wanted::Str), Ok(borrowed(&text)), "{json}");
        assert_eq!(resolve(&found, Wanted::Any), Ok(borrowed(&text)), "{json}");
        refuses_all_but(&found, &[Wanted::Str, Wanted::Any]);
    }
}

#[test]
fn a_boolean_fills_only_a_bool_field() {
    for (json, expected) in [("true", true), ("false", false)] {
        let found = scalar(json);
        assert_eq!(resolve(&found, Wanted::Bool), Ok(Resolved::Bool(expected)));
        assert_eq!(resolve(&found, Wanted::Any), Ok(Resolved::Bool(expected)));
        refuses_all_but(&found, &[Wanted::Bool, Wanted::Any]);
    }
}

#[test]
fn an_integer_fills_an_integer_or_a_float_field() {
    for (json, number) in [("8080", 8080_i32), ("-1", -1), ("0", 0), ("-0", 0)] {
        let found = scalar(json);
        let int = Resolved::Int(i128::from(number));
        assert_eq!(resolve(&found, Wanted::Int), Ok(int.clone()));
        assert_eq!(resolve(&found, Wanted::Any), Ok(int));
        let float = Resolved::Float(f64::from(number));
        assert_eq!(resolve(&found, Wanted::Float), Ok(float));
        refuses_all_but(&found, &[Wanted::Int, Wanted::Float, Wanted::Any]);
    }
}

#[test]
fn minus_zero_keeps_its_sign_as_a_float() {
    let resolved = resolve(&scalar("-0"), Wanted::Float);
    assert!(
        matches!(resolved, Ok(Resolved::Float(number)) if number == 0.0 && number.is_sign_negative()),
        "{resolved:?}"
    );
}

#[test]
fn an_integer_between_i128_and_u128_reads_as_a_u128_and_then_as_a_float() {
    let between_i64_and_i128 = scalar("18446744073709551616");
    assert_eq!(
        resolve(&between_i64_and_i128, Wanted::Int),
        Ok(Resolved::Int(18_446_744_073_709_551_616))
    );
    let between_i128_and_u128 = scalar("170141183460469231731687303715884105728");
    assert_eq!(
        resolve(&between_i128_and_u128, Wanted::Int),
        Ok(Resolved::Uint(
            170_141_183_460_469_231_731_687_303_715_884_105_728
        ))
    );
    let beyond_every_integer = scalar("340282366920938463463374607431768211456");
    assert_eq!(
        resolve(&beyond_every_integer, Wanted::Int),
        Err(Mismatch::new(Wanted::Int))
    );
    assert!(matches!(
        resolve(&beyond_every_integer, Wanted::Any),
        Ok(Resolved::Float(_))
    ));
    assert!(matches!(
        resolve(&beyond_every_integer, Wanted::Float),
        Ok(Resolved::Float(_))
    ));
}

#[test]
fn a_float_fills_only_a_float_field() {
    for (json, number) in [
        ("1.5", 1.5),
        ("-0.25", -0.25),
        ("1e3", 1000.0),
        ("2E-1", 0.2),
    ] {
        let found = scalar(json);
        assert_eq!(resolve(&found, Wanted::Float), Ok(Resolved::Float(number)));
        assert_eq!(resolve(&found, Wanted::Any), Ok(Resolved::Float(number)));
        refuses_all_but(&found, &[Wanted::Float, Wanted::Any]);
    }
}

#[test]
fn a_float_that_overflows_to_infinity_is_refused() {
    for json in ["1e400", "-1e400", "1E999"] {
        let found = scalar(json);
        assert_eq!(
            resolve(&found, Wanted::Float),
            Err(Mismatch::new(Wanted::Float)),
            "{json}"
        );
        assert_eq!(
            resolve(&found, Wanted::Any),
            Err(Mismatch::new(Wanted::Any).with_detail(format!("number `{json}` is out of range"))),
            "{json}"
        );
    }
}

#[test]
fn an_integer_no_float_holds_is_refused_for_a_typeless_read_with_its_text() {
    let json = "9".repeat(400);
    let found = scalar(&json);
    assert_eq!(
        resolve(&found, Wanted::Any),
        Err(Mismatch::new(Wanted::Any).with_detail(format!("number `{json}` is out of range")))
    );
}

#[test]
fn a_float_that_underflows_reads_as_zero() {
    assert_eq!(
        resolve(&scalar("1e-400"), Wanted::Float),
        Ok(Resolved::Float(0.0))
    );
}

#[test]
fn null_fills_a_unit_or_optional_field_only() {
    let found = scalar("null");
    assert_eq!(resolve(&found, Wanted::Null), Ok(Resolved::Null));
    assert_eq!(resolve(&found, Wanted::Any), Ok(Resolved::Null));
    refuses_all_but(&found, &[Wanted::Null, Wanted::Any]);
}

#[test]
fn a_scalar_with_no_syntax_is_text() {
    let found = Scalar::new("8080", ScalarStyle::Plain);
    assert_eq!(resolve(&found, Wanted::Str), Ok(borrowed("8080")));
    assert_eq!(
        resolve(&found, Wanted::Int),
        Err(Mismatch::new(Wanted::Int))
    );
}
