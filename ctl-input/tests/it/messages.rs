//! The wording of each problem type, and a process default that replaces it.

use ctl_input::format::{Fault, Raise, Refusal};
use ctl_input::{EnglishMessages, Messages, set_messages};

fn say(fault: &Fault) -> String {
    EnglishMessages.message(fault)
}

#[test]
fn english_words_each_problem_type_without_a_parser_option() {
    let faults = [
        (
            Fault::Syntax {
                detail: "unclosed bracket".to_owned(),
            },
            "unclosed bracket",
        ),
        (
            Fault::DuplicateKey {
                key: "prefix".to_owned(),
            },
            "duplicate key `prefix`",
        ),
        (Fault::MergeKey, "merge key `<<` is not allowed"),
        (
            Fault::ExpectedKind {
                expected: "a mapping".to_owned(),
                found: "a sequence".to_owned(),
            },
            "expected a mapping, found a sequence",
        ),
        (
            Fault::InvalidValue {
                detail: "port must not be 0".to_owned(),
            },
            "port must not be 0",
        ),
        (
            Fault::UnknownField {
                field: "extra".to_owned(),
                expected: vec!["a", "b"],
            },
            "unknown field `extra`, expected one of `a`, `b`",
        ),
        (
            Fault::UnknownField {
                field: "extra".to_owned(),
                expected: Vec::new(),
            },
            "unknown field `extra`, there are no fields",
        ),
        (
            Fault::UnknownVariant {
                variant: "Round".to_owned(),
                expected: vec!["Empty"],
            },
            "unknown variant `Round`, expected one of `Empty`",
        ),
        (
            Fault::UnknownVariant {
                variant: "Round".to_owned(),
                expected: Vec::new(),
            },
            "unknown variant `Round`, there are no variants",
        ),
        (
            Fault::MissingField { field: "port" },
            "missing field `port`",
        ),
    ];
    for (fault, words) in faults {
        assert_eq!(say(&fault), words);
        assert_eq!(fault.to_string(), words);
    }
}

#[test]
fn a_limit_names_where_to_raise_it() {
    let limit = |raise| Fault::Limit {
        name: "max_depth",
        value: "2".to_owned(),
        raise,
    };
    assert_eq!(
        say(&limit(Raise::InRepository)),
        "exceeds max_depth (2); raise limits.max_depth in .ctl/input.yaml"
    );
    assert_eq!(
        say(&limit(Raise::InProcess)),
        "exceeds max_depth (2); raise max_depth in the tool's process default, which reads .ctl/input.yaml"
    );
}

#[test]
fn a_limit_a_cli_format_found_is_worded_as_one_of_a_text_in_memory() {
    let fault = Fault::limit("max_depth", 2);
    assert!(matches!(
        &fault,
        Fault::Limit { name: "max_depth", value, raise: Raise::InRepository } if value == "2"
    ));
    assert_eq!(
        say(&fault),
        "exceeds max_depth (2); raise limits.max_depth in .ctl/input.yaml"
    );
}

struct Shout;

impl Messages for Shout {
    fn message(&self, fault: &Fault) -> String {
        EnglishMessages.message(fault).to_uppercase()
    }
}

static SHOUT: Shout = Shout;

/// The process wording is global, so this is the one test that sets it.
#[test]
fn a_custom_wording_changes_the_text_and_never_the_fault_or_the_span() {
    let refusal = Refusal::new(Fault::MissingField { field: "port" }, Some(3..7), None);
    assert_eq!(refusal.to_string(), "missing field `port`");
    set_messages(&SHOUT);
    let shouted = refusal.to_string();
    set_messages(&EnglishMessages);
    assert_eq!(shouted, "MISSING FIELD `PORT`");
    assert_eq!(refusal.fault(), &Fault::MissingField { field: "port" });
    assert_eq!(refusal.span(), Some(3..7));
    assert_eq!(refusal.to_string(), "missing field `port`");
}
