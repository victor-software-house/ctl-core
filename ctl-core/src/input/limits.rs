//! How much a text may hold before [`Input::parse`](super::Input::parse)
//! refuses it, and who decides. The order is on [`Limits`]. Three fields hold
//! a text of every format: `max_reader_input_bytes`, `max_depth`, and
//! `max_nodes`. The others name YAML syntax and exist only with `input-yaml`.

use std::fmt;
#[cfg(feature = "input-yaml")]
use std::path::Path;
use std::sync::{PoisonError, RwLock};

use serde::Deserialize;
#[cfg(feature = "input-yaml")]
use serde_saphyr::Budget;
#[cfg(feature = "input-yaml")]
use serde_saphyr::budget::BudgetBreach;

#[cfg(feature = "input-yaml")]
use super::{Input, InputError};

/// Where a repository overrides limits, relative to its root.
#[cfg(feature = "input-yaml")]
pub const REPO_FILE: &str = ".ctl/input.yaml";

macro_rules! limits {
    (
        shared { $( $(#[$sdoc:meta])* $sfield:ident = $sdefault:ident, )* }
        yaml { $( $(#[$ydoc:meta])* $yfield:ident: $yty:ty, )* }
    ) => {
        /// Overrides for the limits a text is held to, one per field. `None`
        /// leaves the field to the next tier.
        ///
        /// Each field is decided on its own, highest first:
        ///
        /// 1. the call: [`Input::limits`](super::Input::limits);
        /// 2. with `input-yaml`, the repository: `limits:` in
        ///    `.ctl/input.yaml`, installed by `use_repo_limits`;
        /// 3. the process: [`set_default_limits`], or `App::input_limits`;
        /// 4. the default: ctl-core's for the three fields every format
        ///    reads, and serde-saphyr's for the YAML-only fields.
        ///
        /// A repository beats the CLI's default because the user owns the
        /// repository, as a user's environment beats the CLI's built-in width.
        ///
        /// The field set follows the enabled format features.
        /// `max_reader_input_bytes`, `max_depth`, and `max_nodes` exist in
        /// every build, and the YAML-only fields exist with `input-yaml`.
        /// Cargo enables a feature for every crate in a build, so another
        /// crate can add fields to a build of yours. Build a value with
        /// `..Limits::NONE`, as in `Limits { max_depth: Some(8),
        /// ..Limits::NONE }`, and it compiles whatever the build enables.
        ///
        /// `enforce_alias_anchor_ratio` is not here: the ratio check is one
        /// comparison after scanning, so turning it off saves nothing. It
        /// stays on; its multiplier and minimum alias count are overridable.
        #[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
        #[serde(deny_unknown_fields, default)]
        pub struct Limits {
            $( $(#[$sdoc])* pub $sfield: Option<usize>, )*
            $( $(#[$ydoc])* #[cfg(feature = "input-yaml")] pub $yfield: Option<$yty>, )*
        }

        impl Limits {
            /// No overrides: every field falls through.
            pub const NONE: Self = Self {
                $( $sfield: None, )*
                $( #[cfg(feature = "input-yaml")] $yfield: None, )*
            };

            /// `self`, with each unset field taken from `lower`.
            #[must_use]
            pub fn or(self, lower: &Self) -> Self {
                Self {
                    $( $sfield: self.$sfield.or(lower.$sfield), )*
                    $( #[cfg(feature = "input-yaml")] $yfield: self.$yfield.or(lower.$yfield), )*
                }
            }

            /// serde-saphyr's settings for these limits. An unset shared
            /// field takes ctl-core's default, and an unset YAML-only field
            /// takes serde-saphyr's.
            #[cfg(feature = "input-yaml")]
            pub(super) fn budget(&self) -> Budget {
                let bounds = self.bounds();
                let mut budget = Budget::default();
                budget.max_reader_input_bytes = Some(bounds.input_bytes);
                budget.max_depth = bounds.depth;
                budget.max_nodes = bounds.nodes;
                $( if let Some(value) = self.$yfield {
                    budget.$yfield = value;
                } )*
                budget
            }

            /// Each set field that is looser than its default.
            #[cfg(feature = "input-yaml")]
            fn loosened(&self) -> Vec<Loosened> {
                let mut loosened = Vec::new();
                $( if let Some(value) = self.$sfield
                    && value > Self::$sdefault
                {
                    loosened.push(Loosened {
                        field: stringify!($sfield),
                        value: value.to_string(),
                        default: Self::$sdefault.to_string(),
                    });
                } )*
                let default = Budget::default();
                $( if let Some(value) = self.$yfield
                    && value > default.$yfield
                {
                    loosened.push(Loosened {
                        field: stringify!($yfield),
                        value: value.to_string(),
                        default: default.$yfield.to_string(),
                    });
                } )*
                loosened
            }
        }
    };
}

limits! {
    shared {
        /// Bytes of input text. Also checked for text already in memory.
        max_reader_input_bytes = DEFAULT_MAX_READER_INPUT_BYTES,
        /// Nesting of YAML sequences and mappings, or of TOML arrays and
        /// tables, with the root table as 1.
        max_depth = DEFAULT_MAX_DEPTH,
        /// YAML sequences, mappings, and scalars in total; in TOML, the root
        /// table and every table, array, and value.
        max_nodes = DEFAULT_MAX_NODES,
    }
    yaml {
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
        /// Nesting of `!include`.
        max_inclusion_depth: u32,
        /// YAML documents in one text.
        max_documents: usize,
        /// Bytes of scalar text in total.
        max_total_scalar_bytes: usize,
        /// Bytes of comment text in total.
        max_total_comment_bytes: usize,
        /// Merge keys (`<<`). ctl-core refuses merge keys whatever this says.
        max_merge_keys: usize,
        /// Aliases allowed before the alias-to-anchor ratio applies.
        alias_anchor_min_aliases: usize,
        /// How many aliases each anchor may have once the ratio applies.
        alias_anchor_ratio_multiplier: usize,
    }
}

impl Limits {
    /// `max_reader_input_bytes` when no tier sets it: 256 MiB.
    pub const DEFAULT_MAX_READER_INPUT_BYTES: usize = 256 * 1024 * 1024;
    /// `max_depth` when no tier sets it.
    pub const DEFAULT_MAX_DEPTH: usize = 64;
    /// `max_nodes` when no tier sets it.
    pub const DEFAULT_MAX_NODES: usize = 250_000;

    /// The three limits every format reads, with ctl-core's default for each
    /// unset field.
    pub(super) fn bounds(&self) -> Bounds {
        Bounds {
            input_bytes: self
                .max_reader_input_bytes
                .unwrap_or(Self::DEFAULT_MAX_READER_INPUT_BYTES),
            depth: self.max_depth.unwrap_or(Self::DEFAULT_MAX_DEPTH),
            nodes: self.max_nodes.unwrap_or(Self::DEFAULT_MAX_NODES),
        }
    }
}

/// The three limits every format reads, as they are in force.
#[derive(Clone, Copy)]
pub(super) struct Bounds {
    pub(super) input_bytes: usize,
    pub(super) depth: usize,
    pub(super) nodes: usize,
}

static PROCESS: RwLock<Limits> = RwLock::new(Limits::NONE);
#[cfg(feature = "input-yaml")]
static REPOSITORY: RwLock<Limits> = RwLock::new(Limits::NONE);

/// Set the process-wide default, which a call's
/// [`Input::limits`](super::Input::limits) beats, and with `input-yaml` a
/// repository's `.ctl/input.yaml` too.
pub fn set_default_limits(limits: Limits) {
    *PROCESS.write().unwrap_or_else(PoisonError::into_inner) = limits;
}

/// Read `limits:` from `root`'s [`REPO_FILE`] and install it for every later
/// parse in this process. A repository without the file overrides nothing.
/// The file itself is read under the process default, not under whatever
/// repository was installed before.
///
/// Returns each limit the repository raises above its default. Print each as
/// `{bin}: warning: {loosened}`. A raised limit is one above ctl-core's or
/// serde-saphyr's default, whatever the CLI set as its process default.
///
/// # Errors
///
/// An [`InputError`] when the file cannot be read, is not YAML, or names a
/// field [`Limits`] does not have.
#[cfg(feature = "input-yaml")]
pub fn use_repo_limits(root: &Path) -> Result<Vec<Loosened>, InputError> {
    let path = root.join(REPO_FILE);
    let limits = match std::fs::read_to_string(&path) {
        Ok(text) => {
            let process = PROCESS
                .read()
                .unwrap_or_else(PoisonError::into_inner)
                .clone();
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
    let loosened = limits.loosened();
    *REPOSITORY.write().unwrap_or_else(PoisonError::into_inner) = limits;
    Ok(loosened)
}

/// A limit a repository's [`REPO_FILE`] raises above its default.
#[cfg(feature = "input-yaml")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loosened {
    /// The [`Limits`] field.
    pub field: &'static str,
    /// The repository's value.
    pub value: String,
    /// The default: ctl-core's for a field every format reads, and
    /// serde-saphyr's for a YAML-only field.
    pub default: String,
}

#[cfg(feature = "input-yaml")]
impl fmt::Display for Loosened {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{REPO_FILE} raises {} to {}, above the default {}",
            self.field, self.value, self.default
        )
    }
}

/// The shape of [`REPO_FILE`].
#[cfg(feature = "input-yaml")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RepoFile {
    #[serde(default)]
    limits: Limits,
}

/// What a parse with `call` uses, field by field.
pub(super) fn effective(call: &Limits) -> Limits {
    let process = PROCESS.read().unwrap_or_else(PoisonError::into_inner);
    #[cfg(feature = "input-yaml")]
    let call = call
        .clone()
        .or(&REPOSITORY.read().unwrap_or_else(PoisonError::into_inner));
    #[cfg(not(feature = "input-yaml"))]
    let call = call.clone();
    call.or(&process)
}

/// Where a user raises a limit a text went past.
#[derive(Clone, Copy)]
pub(super) enum Raise {
    /// In the repository's [`REPO_FILE`].
    #[cfg(feature = "input-yaml")]
    InRepository,
    /// In the tool's process default, for [`REPO_FILE`] itself, which is
    /// read before any repository override applies.
    #[cfg(feature = "input-yaml")]
    InProcess,
    /// Nowhere the user can reach: a build without `input-yaml` reads no
    /// repository file, so only the tool sets limits.
    #[cfg(not(feature = "input-yaml"))]
    InTool,
}

impl Raise {
    /// Where a limit hit by a CLI's own text is raised.
    #[cfg(feature = "input-yaml")]
    pub(super) const FOR_TEXT: Self = Self::InRepository;
    /// Where a limit hit by a CLI's own text is raised.
    #[cfg(not(feature = "input-yaml"))]
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

    /// The [`Limits`] `field` the text went past, with the value in force.
    pub(super) fn new(field: &'static str, limit: usize, raise: Raise) -> Self {
        Self {
            field,
            limit: limit.to_string(),
            raise,
        }
    }

    /// The field serde-saphyr reports as breached, with its value in `budget`.
    /// `None` for a breach this version does not know, which keeps
    /// serde-saphyr's own wording.
    #[cfg(feature = "input-yaml")]
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
            #[cfg(feature = "input-yaml")]
            Raise::InRepository => {
                write!(formatter, "raise limits.{} in {REPO_FILE}", self.field)
            }
            #[cfg(feature = "input-yaml")]
            Raise::InProcess => write!(
                formatter,
                "raise {} in the tool's process default, which reads {REPO_FILE}",
                self.field
            ),
            #[cfg(not(feature = "input-yaml"))]
            Raise::InTool => write!(formatter, "the tool sets this limit"),
        }
    }
}

#[cfg(all(test, feature = "input-yaml"))]
mod tests {
    use serde_saphyr::Budget;

    use super::Limits;

    /// ctl-core states the three shared defaults, and a YAML text meets the
    /// same ones it met under serde-saphyr's. A serde-saphyr release that
    /// moves its defaults fails here before it ships.
    #[test]
    fn the_shared_defaults_match_serde_saphyr() {
        let budget = Budget::default();
        assert_eq!(
            budget.max_reader_input_bytes,
            Some(Limits::DEFAULT_MAX_READER_INPUT_BYTES)
        );
        assert_eq!(budget.max_depth, Limits::DEFAULT_MAX_DEPTH);
        assert_eq!(budget.max_nodes, Limits::DEFAULT_MAX_NODES);
    }
}
