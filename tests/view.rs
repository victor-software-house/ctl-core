//! One typed model across pretty, colorless, and JSON.
#![allow(missing_docs)]
#![cfg(feature = "view")]

use ctl_core::{
    ColorMode, Document, Envelope, ErrorBody, Fields, MessageKind, OutputFormat, Present, Stream,
    View,
};
use serde::Serialize;

#[derive(Serialize)]
struct Status {
    pending: usize,
    failed: bool,
}

impl Present for Status {
    fn present(&self) -> Document {
        Document::new().fields(Fields::new().row("pending", self.pending.to_string()))
    }

    fn message_kind(&self) -> MessageKind {
        if self.failed {
            MessageKind::Error
        } else {
            MessageKind::Success
        }
    }

    fn exit_code(&self) -> u8 {
        if self.failed { 2 } else { 0 }
    }
}

#[test]
fn same_model_feeds_pretty_and_json() {
    let status = Status {
        pending: 2,
        failed: false,
    };
    let pretty = View::new(OutputFormat::Pretty, ColorMode::Never)
        .width(60)
        .capture(&status)
        .unwrap();
    let json = View::new(OutputFormat::Json, ColorMode::Always)
        .capture(&status)
        .unwrap();

    assert_eq!(pretty.stream(), Stream::Stdout);
    assert!(pretty.text().contains("pending"));
    assert!(pretty.text().contains('2'));
    assert!(!pretty.text().contains('\u{1b}'));
    assert_eq!(json.stream(), Stream::Stdout);
    assert_eq!(json.text(), "{\"pending\":2,\"failed\":false}\n");
    assert!(!json.text().contains('\u{1b}'));
}

#[test]
fn error_pretty_uses_stderr_and_failure_exit() {
    let status = Status {
        pending: 1,
        failed: true,
    };
    let captured = View::new(OutputFormat::Pretty, ColorMode::Never)
        .width(60)
        .capture(&status)
        .unwrap();
    assert_eq!(captured.stream(), Stream::Stderr);
    assert_eq!(captured.exit_code(), std::process::ExitCode::from(2));
}

#[test]
fn quiet_suppresses_only_successful_pretty_output() {
    let success = Status {
        pending: 0,
        failed: false,
    };
    let failure = Status {
        pending: 1,
        failed: true,
    };
    let quiet_pretty = View::new(OutputFormat::Pretty, ColorMode::Never)
        .quiet(true)
        .capture(&success)
        .unwrap();
    let quiet_error = View::new(OutputFormat::Pretty, ColorMode::Never)
        .quiet(true)
        .width(60)
        .capture(&failure)
        .unwrap();
    let quiet_json = View::new(OutputFormat::Json, ColorMode::Never)
        .quiet(true)
        .capture(&success)
        .unwrap();

    assert_eq!(quiet_pretty.stream(), Stream::None);
    assert_eq!(quiet_pretty.bytes(), &[] as &[u8]);
    assert_eq!(quiet_error.stream(), Stream::Stderr);
    assert_ne!(quiet_error.bytes(), &[] as &[u8]);
    assert_eq!(quiet_json.stream(), Stream::Stdout);
    assert_ne!(quiet_json.bytes(), &[] as &[u8]);
}

#[test]
fn envelope_ok_tag() {
    let json = serde_json::to_value(Envelope::ok("demo")).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["schema_version"], 1);
}

#[test]
fn envelope_err_tag() {
    let json = serde_json::to_value(Envelope::<()>::err(ErrorBody::new("toy", "nope"))).unwrap();
    assert_eq!(json["status"], "err");
    assert_eq!(json["error"]["bin"], "toy");
}
