//! How much a YAML text may hold before [`Input::parse`] refuses it, and who
//! decides. The order is on [`Limits`].

use std::fmt;
use std::path::Path;
use std::sync::{PoisonError, RwLock};

use serde::Deserialize;
use serde_saphyr::Budget;
use serde_saphyr::budget::BudgetBreach;

use super::{Input, InputError};

/// Where a repository overrides limits, relative to its root.
pub const REPO_FILE: &str = ".ctl/input.yaml";

macro_rules! limits {
    ($( $(#[$doc:meta])* $field:ident: $ty:ty $(=> $wrap:ident)?, )*) => {
        /// Overrides for serde-saphyr's [`Budget`], one per field. `None`
        /// leaves the field to the next tier.
        ///
        /// Each field is decided on its own, highest first:
        ///
        /// 1. the call: [`Input::limits`];
        /// 2. the repository: `limits:` in [`REPO_FILE`], installed by
        ///    [`use_repo_limits`];
        /// 3. the process: [`set_default_limits`], or `App::input_limits`;
        /// 4. serde-saphyr's own default.
        ///
        /// A repository beats a tool's default because the operator owns
        /// it, as an operator's environment beats a tool's built-in width.
        #[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
        #[serde(deny_unknown_fields, default)]
        pub struct Limits {
            $( $(#[$doc])* pub $field: Option<$ty>, )*
        }

        impl Limits {
            /// No overrides: every field falls through.
            pub const NONE: Self = Self { $( $field: None, )* };

            /// `self`, with each unset field taken from `lower`.
            #[must_use]
            pub fn or(self, lower: &Self) -> Self {
                Self { $( $field: self.$field.or(lower.$field), )* }
            }

            /// serde-saphyr's defaults with these overrides applied.
            #[must_use]
            pub fn budget(&self) -> Budget {
                let mut budget = Budget::default();
                $( if let Some(value) = self.$field {
                    $( let value = $wrap(value); )?
                    budget.$field = value;
                } )*
                budget
            }
        }
    };
}

limits! {
    /// Bytes of input text. Also checked for text already in memory.
    max_reader_input_bytes: usize => Some,
    /// Comment events buffered while resolving an ambiguous entry.
    max_buffered_comment_events: usize,
    /// Characters scanned to resolve a simple key.
    simple_key_max_lookahead: usize,
    /// Simultaneously nested flow collections.
    flow_nesting_limit: usize,
    /// Parser events in total.
    max_events: usize,
    /// Alias (`*ref`) events.
    max_aliases: usize,
    /// Distinct anchor (`&anchor`) definitions.
    max_anchors: usize,
    /// Event copies retained for anchors.
    max_recorded_anchor_events: usize,
    /// Bytes of string payload retained for anchors.
    max_recorded_anchor_bytes: usize,
    /// Nested expansions in one property interpolation.
    max_property_expansion_depth: usize,
    /// Bytes inspected across all property interpolation.
    max_total_property_interpolation_work: usize,
    /// Nesting of sequences and mappings.
    max_depth: usize,
    /// Nesting of `!include`.
    max_inclusion_depth: u32,
    /// YAML documents in one text.
    max_documents: usize,
    /// Sequences, mappings, and scalars in total.
    max_nodes: usize,
    /// Bytes of scalar text in total.
    max_total_scalar_bytes: usize,
    /// Bytes of comment text in total.
    max_total_comment_bytes: usize,
    /// Merge keys (`<<`). ctl-core refuses merge keys whatever this says.
    max_merge_keys: usize,
    /// Whether aliases may outnumber anchors only by the ratio below.
    enforce_alias_anchor_ratio: bool,
    /// Aliases allowed before the ratio applies.
    alias_anchor_min_aliases: usize,
    /// How many aliases each anchor may have once the ratio applies.
    alias_anchor_ratio_multiplier: usize,
}

static PROCESS: RwLock<Limits> = RwLock::new(Limits::NONE);
static REPOSITORY: RwLock<Limits> = RwLock::new(Limits::NONE);

/// Set the process-wide default, which a repository's [`REPO_FILE`] and a
/// call's [`Input::limits`] both beat.
pub fn set_default_limits(limits: Limits) {
    *PROCESS.write().unwrap_or_else(PoisonError::into_inner) = limits;
}

/// Read `limits:` from `root`'s [`REPO_FILE`] and install it for every later
/// parse in this process. A repository without the file overrides nothing.
/// The file itself is read under the process default, not under whatever
/// repository was installed before.
///
/// # Errors
///
/// An [`InputError`] when the file cannot be read, is not YAML, or names a
/// field [`Limits`] does not have.
pub fn use_repo_limits(root: &Path) -> Result<(), InputError> {
    let path = root.join(REPO_FILE);
    let limits = match std::fs::read_to_string(&path) {
        Ok(text) => {
            let process = PROCESS
                .read()
                .unwrap_or_else(PoisonError::into_inner)
                .budget();
            Input::new(path.display().to_string(), text)
                .parse_within::<RepoFile>(&process, Raise::InProcess)?
                .limits
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Limits::NONE,
        Err(error) => {
            return Err(InputError::whole(
                path.display().to_string(),
                error.to_string(),
            ));
        }
    };
    *REPOSITORY.write().unwrap_or_else(PoisonError::into_inner) = limits;
    Ok(())
}

/// The shape of [`REPO_FILE`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RepoFile {
    #[serde(default)]
    limits: Limits,
}

/// What a parse with `call` uses, field by field.
pub(super) fn effective(call: &Limits) -> Limits {
    let repository = REPOSITORY.read().unwrap_or_else(PoisonError::into_inner);
    let process = PROCESS.read().unwrap_or_else(PoisonError::into_inner);
    call.clone().or(&repository).or(&process)
}

/// A limit a text went past: the field and the value in force.
/// Where an operator raises a limit a text went past.
#[derive(Clone, Copy)]
pub(super) enum Raise {
    /// In the repository's [`REPO_FILE`].
    InRepository,
    /// In the tool's process default, for [`REPO_FILE`] itself, which is
    /// read before any repository override applies.
    InProcess,
}

pub(super) struct Exceeded {
    field: &'static str,
    limit: String,
    raise: Raise,
}

impl Exceeded {
    pub(super) fn input_bytes(limit: usize, raise: Raise) -> Self {
        Self {
            field: "max_reader_input_bytes",
            limit: limit.to_string(),
            raise,
        }
    }

    /// The field serde-saphyr reports as breached, with its value in `budget`.
    /// `None` for a breach this version does not know, which keeps
    /// serde-saphyr's own wording.
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "BudgetBreach is non_exhaustive: a variant added later keeps serde-saphyr's wording"
    )]
    pub(super) fn breach(breach: &BudgetBreach, budget: &Budget, raise: Raise) -> Option<Self> {
        let (field, limit) = match breach {
            BudgetBreach::Events { .. } => ("max_events", budget.max_events.to_string()),
            BudgetBreach::Aliases { .. } => ("max_aliases", budget.max_aliases.to_string()),
            BudgetBreach::Anchors { .. } => ("max_anchors", budget.max_anchors.to_string()),
            BudgetBreach::Depth { .. } => ("max_depth", budget.max_depth.to_string()),
            BudgetBreach::InclusionDepth { .. } => (
                "max_inclusion_depth",
                budget.max_inclusion_depth.to_string(),
            ),
            BudgetBreach::Documents { .. } => ("max_documents", budget.max_documents.to_string()),
            BudgetBreach::Nodes { .. } => ("max_nodes", budget.max_nodes.to_string()),
            BudgetBreach::ScalarBytes { .. } => (
                "max_total_scalar_bytes",
                budget.max_total_scalar_bytes.to_string(),
            ),
            BudgetBreach::CommentBytes { .. } => (
                "max_total_comment_bytes",
                budget.max_total_comment_bytes.to_string(),
            ),
            BudgetBreach::MergeKeys { .. } => ("max_merge_keys", budget.max_merge_keys.to_string()),
            BudgetBreach::AliasAnchorRatio { .. } => (
                "alias_anchor_ratio_multiplier",
                budget.alias_anchor_ratio_multiplier.to_string(),
            ),
            BudgetBreach::InputBytes { .. } => (
                "max_reader_input_bytes",
                budget
                    .max_reader_input_bytes
                    .map_or_else(|| "none".to_owned(), |limit| limit.to_string()),
            ),
            BudgetBreach::RecordedAnchorEvents { .. } => (
                "max_recorded_anchor_events",
                budget.max_recorded_anchor_events.to_string(),
            ),
            BudgetBreach::RecordedAnchorBytes { .. } => (
                "max_recorded_anchor_bytes",
                budget.max_recorded_anchor_bytes.to_string(),
            ),
            BudgetBreach::PropertyExpansionDepth { .. } => (
                "max_property_expansion_depth",
                budget.max_property_expansion_depth.to_string(),
            ),
            BudgetBreach::PropertyInterpolationWork { .. } => (
                "max_total_property_interpolation_work",
                budget.max_total_property_interpolation_work.to_string(),
            ),
            _ => return None,
        };
        Some(Self {
            field,
            limit,
            raise,
        })
    }
}

impl fmt::Display for Exceeded {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "exceeds {} ({}); ", self.field, self.limit)?;
        match self.raise {
            Raise::InRepository => {
                write!(formatter, "raise limits.{} in {REPO_FILE}", self.field)
            }
            Raise::InProcess => write!(
                formatter,
                "raise {} in the tool's process default, which reads {REPO_FILE}",
                self.field
            ),
        }
    }
}
