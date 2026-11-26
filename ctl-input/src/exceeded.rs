//! The limit a text went past, worded for the person who must raise it, and
//! where that person raises it.

use std::fmt;

#[cfg(feature = "yaml")]
use serde_saphyr::budget::BudgetBreach;
#[cfg(feature = "yaml")]
use serde_saphyr::options::AliasLimits;
#[cfg(feature = "yaml")]
use serde_saphyr::{Budget, Error};

use super::format::Fault;
#[cfg(feature = "yaml")]
use super::limits::REPO_FILE;

/// Where a user raises a limit a text went past.
#[derive(Clone, Copy)]
pub(super) enum Raise {
    /// In the repository's [`REPO_FILE`].
    #[cfg(feature = "yaml")]
    InRepository,
    /// In the tool's process default, for [`REPO_FILE`] itself, which is
    /// read before any repository override applies.
    #[cfg(feature = "yaml")]
    InProcess,
    /// Nowhere the user can reach: a build without `yaml` reads no
    /// repository file, so only the tool sets limits.
    #[cfg(not(feature = "yaml"))]
    InTool,
}

impl Raise {
    /// Where a limit hit by a CLI's own text is raised.
    #[cfg(feature = "yaml")]
    pub(super) const FOR_TEXT: Self = Self::InRepository;
    /// Where a limit hit by a CLI's own text is raised.
    #[cfg(not(feature = "yaml"))]
    pub(super) const FOR_TEXT: Self = Self::InTool;
}

/// A limit a text went past: the field and the value in force.
pub(super) struct Exceeded {
    field: &'static str,
    limit: String,
    raise: Raise,
}

impl Exceeded {
    pub(super) fn input_bytes(limit: usize, raise: Raise) -> Self {
        Self::new("max_reader_input_bytes", limit, raise)
    }

    /// The [`Limits`](super::Limits) `field` the text went past, with the value
    /// in force.
    pub(super) fn new(field: &'static str, limit: usize, raise: Raise) -> Self {
        Self {
            field,
            limit: limit.to_string(),
            raise,
        }
    }

    /// The limit serde-saphyr reports as breached by `error`: a budget breach
    /// or one of the three alias replay errors, with its value in `budget` or
    /// `alias`. `None` for any other error.
    #[cfg(feature = "yaml")]
    pub(super) fn of(
        error: &Error,
        budget: &Budget,
        alias: &AliasLimits,
        raise: Raise,
    ) -> Option<Self> {
        if let Error::Budget { breach, .. } = error {
            Self::breach(breach, budget, raise)
        } else {
            Self::alias_limit(error, alias, raise)
        }
    }

    /// The limit one of the three alias replay errors reports, with its value
    /// in `alias`. `None` for any other error, a budget breach included.
    #[cfg(feature = "yaml")]
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "Error is non_exhaustive and only these three variants name an alias replay limit"
    )]
    pub(super) fn alias_limit(error: &Error, alias: &AliasLimits, raise: Raise) -> Option<Self> {
        let (field, limit) = match error {
            Error::AliasReplayLimitExceeded { .. } => {
                ("max_total_replayed_events", alias.max_total_replayed_events)
            }
            Error::AliasReplayStackDepthExceeded { .. } => {
                ("max_replay_stack_depth", alias.max_replay_stack_depth)
            }
            Error::AliasExpansionLimitExceeded { .. } => (
                "max_alias_expansions_per_anchor",
                alias.max_alias_expansions_per_anchor,
            ),
            _ => return None,
        };
        Some(Self::new(field, limit, raise))
    }

    /// The limit as a refusal of a format's read: the [`Limits`](super::Limits)
    /// field the text went past, and the value in force.
    pub(super) fn fault(&self) -> Fault {
        Fault::Limit {
            name: self.field,
            value: self.limit.clone(),
        }
    }

    /// The field serde-saphyr reports as breached, with its value in `budget`.
    /// `None` for a breach this version does not know, which keeps
    /// serde-saphyr's own wording.
    #[cfg(feature = "yaml")]
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
            #[cfg(feature = "yaml")]
            Raise::InRepository => {
                write!(formatter, "raise limits.{} in {REPO_FILE}", self.field)
            }
            #[cfg(feature = "yaml")]
            Raise::InProcess => write!(
                formatter,
                "raise {} in the tool's process default, which reads {REPO_FILE}",
                self.field
            ),
            #[cfg(not(feature = "yaml"))]
            Raise::InTool => write!(formatter, "the tool sets this limit"),
        }
    }
}
