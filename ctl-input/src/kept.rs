//! The tree an [`Input`](super::Input) keeps, so every operation on its text
//! shares one read.
//!
//! A read depends on the text, its format, the limits in force and the
//! reading, and an input fixes the first two. So the tree is kept with the
//! limits and the reading it was read under, and read again when either the
//! limits or the kind of reading it needs does not match. A refusal is kept
//! the same way, so a text the limits keep out is refused once and not read
//! again, until the limits change.

use std::sync::{Arc, Mutex, PoisonError};

use super::format::{Format, Reading, Refusal};
use super::limits::Limits;
use super::tree::Node;

/// What reading a text gave: its tree, or why the format refused it.
pub(super) type Read = Result<Node, Refusal>;

/// The tree of one input and what it was read under.
///
/// The tree is derived from the text, which the input fixes, so a clone starts
/// with the same contents and two inputs with the same fields are equal
/// whether or not either has been read.
#[derive(Debug, Default)]
pub(super) struct Kept(Mutex<Option<Held>>);

/// A read, with the limits and the reading it was made under.
#[derive(Clone, Debug)]
struct Held {
    limits: Limits,
    reading: Reading,
    read: Arc<Read>,
}

impl Held {
    /// Whether this read answers a call for `reading` under `limits`.
    ///
    /// A strict tree also serves a listing, which asks less of the text. A
    /// listing never serves a strict read, nor does a refusal serve a listing:
    /// the strict reading refused what the listing may accept.
    fn serves(&self, limits: &Limits, reading: Reading) -> bool {
        self.limits == *limits
            && (self.reading == reading || (reading == Reading::Listing && self.read.is_ok()))
    }
}

impl Clone for Kept {
    fn clone(&self) -> Self {
        Self(Mutex::new(self.held()))
    }
}

impl PartialEq for Kept {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for Kept {}

impl Kept {
    fn held(&self) -> Option<Held> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The read of `text` by `format` for `reading` under `limits`: the kept
    /// one when it serves, else a new one, which replaces it.
    pub(super) fn tree(
        &self,
        format: &dyn Format,
        text: &str,
        limits: &Limits,
        reading: Reading,
    ) -> Arc<Read> {
        let mut held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(kept) = held.as_ref()
            && kept.serves(limits, reading)
        {
            return Arc::clone(&kept.read);
        }
        let read = Arc::new(format.read(text, limits, reading));
        *held = Some(Held {
            limits: limits.clone(),
            reading,
            read: Arc::clone(&read),
        });
        read
    }
}
