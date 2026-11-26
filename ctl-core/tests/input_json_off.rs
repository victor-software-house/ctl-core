//! A build with `input` and not `input-json` reads no JSON: a `.json` file
//! is refused, naming the feature it needs. `test:input-json-off` runs this
//! file with `input` alone; a build with `input-json` skips it, because it
//! reads JSON.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(all(feature = "input", not(feature = "input-json")))]

use std::path::Path;

use ctl_core::input::{Formats, Input};

#[test]
fn a_json_file_names_the_feature_it_needs() {
    let error = Input::open(Path::new("app.json")).unwrap_err();
    assert_eq!(error.problems.len(), 1, "{error}");
    assert_eq!(error.problems[0].line, None);
    assert_eq!(
        error.problems[0].message,
        "`.json` needs ctl-core's input-json feature; name the file *.yaml or *.yml"
    );
}

#[test]
fn the_builtin_registry_holds_no_json() {
    let formats = Formats::builtin();
    assert_eq!(formats.names().collect::<Vec<_>>(), ["yaml"]);
    assert!(formats.get("json").is_none());
}
