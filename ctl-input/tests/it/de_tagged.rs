//! `Tagged` reads a tag beside its value.

use std::collections::BTreeMap;

use ctl_input::de::Tagged;
use ctl_input::format::{TOML, YAML};
use serde::Deserialize;

use crate::common::typed;

#[derive(Debug, Deserialize, PartialEq)]
struct Sizes {
    code_span_size: Tagged<u64>,
}

#[test]
fn a_local_tag_is_read_as_written_beside_the_value() {
    assert_eq!(
        typed::<Sizes>(&YAML, "code_span_size: !override 0\n").unwrap(),
        Sizes {
            code_span_size: Tagged(0, Some("!override".to_owned()))
        }
    );
}

#[test]
fn a_node_with_no_tag_reads_as_none() {
    assert_eq!(
        typed::<Sizes>(&YAML, "code_span_size: 3\n")
            .unwrap()
            .code_span_size,
        Tagged(3, None)
    );
    assert_eq!(
        typed::<Sizes>(&TOML, "code_span_size = 3\n")
            .unwrap()
            .code_span_size,
        Tagged(3, None)
    );
}

#[test]
fn a_standard_tag_is_spelled_by_its_handle_and_suffix() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Named {
        a: Tagged<String>,
    }
    assert_eq!(
        typed::<Named>(&YAML, "a: !!str 5\n").unwrap().a,
        Tagged("5".to_owned(), Some("tag:yaml.org,2002:str".to_owned()))
    );
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
enum Setting {
    List(Vec<String>),
    Word(String),
}

#[test]
fn the_value_is_read_without_its_tag_so_an_untagged_enum_reads_it() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Tags {
        styles: Tagged<Setting>,
        level: Tagged<Setting>,
    }
    let tags = typed::<Tags>(&YAML, "styles: !override [Acme]\nlevel: warning\n").unwrap();
    assert_eq!(
        tags.styles,
        Tagged(
            Setting::List(vec!["Acme".to_owned()]),
            Some("!override".to_owned())
        )
    );
    assert_eq!(
        tags.level,
        Tagged(Setting::Word("warning".to_owned()), None)
    );
}

#[test]
fn a_tag_inside_a_value_serde_buffers_is_not_passed_on() {
    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(untagged)]
    enum Wrapped {
        Item(Tagged<u64>),
    }
    #[derive(Debug, Deserialize, PartialEq)]
    struct Flattened {
        name: String,
        #[serde(flatten)]
        rest: BTreeMap<String, Tagged<u64>>,
    }
    let wrapped = typed::<BTreeMap<String, Wrapped>>(&YAML, "a: !override 0\n").unwrap();
    assert_eq!(wrapped["a"], Wrapped::Item(Tagged(0, None)));
    let flattened = typed::<Flattened>(&YAML, "name: n\nsize: !override 4\n").unwrap();
    assert_eq!(flattened.rest["size"], Tagged(4, None));
}

#[test]
fn another_deserializer_reads_the_value_and_no_tag() {
    let tagged: Tagged<u32> = serde_json::from_str("5").unwrap();
    assert_eq!(tagged, Tagged(5, None));
}
