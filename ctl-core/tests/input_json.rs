//! Declared input from JSON: `Input::open` reads a `.json` file by its
//! extension, and a problem is placed on its line and column.
#![allow(
    missing_docs,
    reason = "an integration test crate has no public API to document"
)]
#![cfg(all(feature = "input-json", feature = "fixture"))]

use std::fs;

use ctl_core::fixture::Suite;
use ctl_core::input::{Formats, Input};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Server {
    port: u16,
}

#[test]
fn a_json_file_loads_through_its_extension() {
    let suite = Suite::new("input-json");
    let root = suite.scenario("open");
    let path = root.file("app.json", "{\"port\": 8080}\n");
    let server: Server = Input::open(&path).unwrap().parse().unwrap();
    assert_eq!(server, Server { port: 8080 });
    assert_eq!(Formats::builtin().of(&path).unwrap().name(), "json");
}

#[test]
fn a_problem_in_a_json_file_names_the_file_line_and_column() {
    let suite = Suite::new("input-json");
    let root = suite.scenario("trailing-comma");
    let path = root.file("app.json", "{\"port\": 80,}\n");
    let error = Input::open(&path).unwrap().parse::<Server>().unwrap_err();
    assert_eq!(
        error.to_string(),
        format!("{}:1:13: expected a key in double quotes", path.display())
    );
}

#[test]
fn a_file_that_is_not_utf_8_is_refused_when_it_is_opened() {
    let suite = Suite::new("input-json");
    let root = suite.scenario("utf-16");
    let path = root.file("app.json", "");
    fs::write(&path, b"\xff\xfe{\x00}\x00").unwrap();
    assert!(Input::open(&path).is_err());
}
