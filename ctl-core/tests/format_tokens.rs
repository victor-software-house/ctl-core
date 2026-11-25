//! `FormatName`, the names `--format` accepts, and the message for one it does
//! not.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]

use ctl_core::format::FormatName;

#[test]
fn the_built_in_names_read_as_their_text() {
    for (name, text) in [
        (FormatName::PRETTY, "pretty"),
        (FormatName::JSON, "json"),
        (FormatName::TSV, "tsv"),
        (FormatName::CSV, "csv"),
        (FormatName::MARKDOWN, "markdown"),
    ] {
        assert_eq!(name.as_str(), text);
        assert_eq!(name.to_string(), text);
        assert_eq!(FormatName::new(text), name);
    }
}

#[test]
fn a_name_is_equal_to_the_name_of_the_same_text_only() {
    assert_eq!(FormatName::new("yaml"), FormatName::new("yaml"));
    assert_ne!(FormatName::new("yaml"), FormatName::JSON);
    assert_ne!(FormatName::new("JSON"), FormatName::JSON);
}

#[cfg(feature = "json")]
#[test]
fn a_name_serializes_as_its_text() {
    let text = serde_json::to_string(&FormatName::CSV).unwrap();
    assert_eq!(text, "\"csv\"");
}

#[cfg(feature = "cli")]
mod parser {
    use clap::error::ErrorKind;
    use clap::{CommandFactory, Parser};
    use ctl_core::OutputArgs;
    use ctl_core::format::FormatName;

    #[derive(Parser, Debug)]
    struct Toy {
        #[command(flatten)]
        output: OutputArgs,
    }

    #[test]
    fn the_built_in_names_parse() {
        for (text, name) in [
            ("pretty", FormatName::PRETTY),
            ("json", FormatName::JSON),
            ("tsv", FormatName::TSV),
            ("csv", FormatName::CSV),
            ("markdown", FormatName::MARKDOWN),
        ] {
            let toy = Toy::try_parse_from(["toy", "--format", text]).unwrap();
            assert_eq!(toy.output.format, Some(name), "{text}");
        }
    }

    #[test]
    fn a_name_the_list_lacks_is_refused_with_the_names_it_has() {
        for text in ["", "JSON", "yaml", "xml", "human"] {
            let error = Toy::try_parse_from(["toy", "--format", text]).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::InvalidValue, "{text:?}");
            let message = error.to_string();
            assert!(
                message.contains(&format!(
                    "unknown format `{text}`; use one of pretty, json, tsv, csv, markdown"
                )),
                "{message}"
            );
        }
    }

    #[test]
    fn the_possible_values_carry_their_help_lines() {
        let command = Toy::command();
        let format = command
            .get_arguments()
            .find(|arg| arg.get_id() == "format")
            .unwrap();
        let values = format.get_possible_values();
        let listed = values
            .iter()
            .map(|value| (value.get_name(), value.get_help().map(ToString::to_string)))
            .collect::<Vec<_>>();
        assert_eq!(
            listed,
            [
                ("pretty", Some("Human text, optionally coloured".to_owned())),
                ("json", Some("Machine JSON. Never contains ANSI".to_owned())),
                (
                    "tsv",
                    Some("Tab-separated lines, one record per line".to_owned())
                ),
                ("csv", Some("Comma-separated values (RFC 4180)".to_owned())),
                ("markdown", Some("Markdown tables and text".to_owned())),
            ]
        );
    }
}
