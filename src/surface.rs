//! Clap-derived operator metadata and validation.
//!
//! Clap remains the command grammar. [`Surface`] extracts the stable operator
//! view used by committed skills, installed instructions, and contract tests.
//! MiniJinja rendering is separately gated behind `surface-templates`.

use std::collections::{BTreeMap, BTreeSet};

use clap::{Arg, Command};

use crate::usage;

#[cfg(feature = "surface-templates")]
mod templates;
#[cfg(feature = "surface-templates")]
pub use templates::{
    COMMANDS_FRAGMENT, INVOCATION_FRAGMENT, VERSION_FRAGMENT, add_fragments, environment, render,
};

/// Operator-facing projection of one Clap command graph.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "surface-templates", derive(serde::Serialize))]
pub struct Surface {
    /// Executable name declared by the root Clap command.
    pub binary: String,
    /// Mise task name used for mounted invocations.
    pub mount: String,
    /// Root command description.
    pub about: String,
    /// Root package version, when Clap declares one.
    pub version: Option<String>,
    /// Arguments and flags declared directly on the root, in Clap order.
    pub arguments: Vec<SurfaceArgument>,
    /// Ancestor global arguments, always empty for the root.
    pub inherited_arguments: Vec<SurfaceArgument>,
    /// Root subcommands in Clap declaration order, including hidden commands.
    pub commands: Vec<SurfaceCommand>,
    /// Usage KDL for the mounted task name.
    pub usage_kdl: String,
    /// Exact `#USAGE mount` line for a served mise task.
    pub mount_line: String,
    /// Consumer-owned operator notes keyed for skill or instruction templates.
    pub notes: BTreeMap<String, String>,
}

/// One command in a [`Surface`].
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "surface-templates", derive(serde::Serialize))]
pub struct SurfaceCommand {
    /// Command name.
    pub name: String,
    /// Full command path below the root.
    pub path: String,
    /// All declared aliases, including hidden aliases.
    pub aliases: Vec<String>,
    /// Aliases Clap exposes in help and completion.
    pub visible_aliases: Vec<String>,
    /// Whether Clap hides this command.
    pub hidden: bool,
    /// Command description.
    pub about: String,
    /// Arguments and flags declared directly on this command, in Clap order.
    ///
    /// Ancestor globals remain on their declaring command instead of being
    /// duplicated into every descendant.
    pub arguments: Vec<SurfaceArgument>,
    /// Ancestor global arguments this command also accepts, root-first.
    pub inherited_arguments: Vec<SurfaceArgument>,
    /// Nested subcommands in Clap declaration order.
    pub commands: Vec<Self>,
}

/// One positional argument or flag in a [`Surface`].
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "surface-templates", derive(serde::Serialize))]
pub struct SurfaceArgument {
    /// Clap argument identifier.
    pub id: String,
    /// Positional index, absent for options and flags.
    pub index: Option<usize>,
    /// Short flag name.
    pub short: Option<char>,
    /// Long flag name without leading dashes.
    pub long: Option<String>,
    /// Visible short aliases.
    pub visible_short_aliases: Vec<char>,
    /// All short aliases, including hidden aliases.
    pub short_aliases: Vec<char>,
    /// Visible long aliases.
    pub visible_aliases: Vec<String>,
    /// All long aliases, including hidden aliases.
    pub aliases: Vec<String>,
    /// Value names shown by Clap.
    pub value_names: Vec<String>,
    /// Argument description.
    pub help: String,
    /// Whether Clap requires the argument.
    pub requirement: SurfaceRequirement,
    /// Whether Clap propagates the argument to subcommands.
    pub scope: SurfaceScope,
    /// Whether Clap hides the argument.
    pub hidden: bool,
    /// Whether the argument action accepts values.
    pub takes_values: bool,
}

/// Whether Clap requires an argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "surface-templates", derive(serde::Serialize))]
#[cfg_attr(feature = "surface-templates", serde(rename_all = "snake_case"))]
pub enum SurfaceRequirement {
    /// The invocation can omit this argument.
    Optional,
    /// The invocation must provide this argument.
    Required,
}

/// How far Clap propagates an argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "surface-templates", derive(serde::Serialize))]
#[cfg_attr(feature = "surface-templates", serde(rename_all = "snake_case"))]
pub enum SurfaceScope {
    /// The argument belongs only to its declaring command.
    Local,
    /// The argument remains available to nested commands.
    Global,
}

impl Surface {
    /// Extract an operator surface from a Clap
    /// [`CommandFactory`](clap::CommandFactory).
    #[must_use]
    pub fn new<C: clap::CommandFactory>(mount: impl Into<String>) -> Self {
        Self::from_command(C::command(), mount)
    }

    /// Extract an operator surface from a Clap command graph.
    #[must_use]
    pub fn from_command(mut command: Command, mount: impl Into<String>) -> Self {
        let mut declared_arguments = BTreeMap::new();
        collect_declarations(&command, "", &mut declared_arguments);
        command.build();
        let mount = mount.into();
        let binary = command.get_name().to_owned();
        let about = command
            .get_about()
            .map(ToString::to_string)
            .unwrap_or_default();
        let version = command.get_version().map(ToOwned::to_owned);
        let arguments = declared_arguments_for(&command, "", &declared_arguments);
        let inherited_arguments = arguments
            .iter()
            .filter(|argument| argument.scope == SurfaceScope::Global)
            .cloned()
            .collect::<Vec<_>>();
        let commands = command
            .get_subcommands()
            .filter(|child| declared_arguments.contains_key(child.get_name()))
            .map(|child| surface_command(child, "", &declared_arguments, &inherited_arguments))
            .collect();
        let usage_kdl = usage::spec(command, &mount);
        let mount_line = usage::mount_line(&mount);
        Self {
            binary,
            mount,
            about,
            version,
            arguments,
            inherited_arguments: Vec::new(),
            commands,
            usage_kdl,
            mount_line,
            notes: BTreeMap::new(),
        }
    }

    /// Add consumer-owned prose for a template audience or section.
    #[must_use]
    pub fn note(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.notes.insert(name.into(), value.into());
        self
    }

    /// Long flags that have no short, as `(command path, long name)`.
    ///
    /// The root path is empty. Inherited globals are not repeated on children.
    /// Hidden flags are included so a long-only option cannot hide from the
    /// check. [`FormatLong`](crate::flags::FormatLong) and
    /// [`ColorLong`](crate::flags::ColorLong) leave `--format` / `--color`
    /// without shorts on purpose; pass those names to [`Self::require_shorts`].
    #[must_use]
    pub fn long_options_without_short(&self) -> Vec<(String, String)> {
        let mut missing = Vec::new();
        collect_missing_shorts("", &self.arguments, &mut missing);
        for command in &self.commands {
            collect_command_missing_shorts(command, &mut missing);
        }
        missing
    }

    /// Fail when a long option has no operator-visible short, except tokens in
    /// `allow`.
    ///
    /// A root allowance is the displayed long option (`--format`). A nested
    /// allowance includes its command path (`status --archived`). Chassis
    /// mixins that deliberately leave a letter for the consumer (`--format`
    /// for `-f`/`--file`, `--color` for `-c`) belong there.
    pub fn require_shorts<'a>(
        &self,
        allow: impl IntoIterator<Item = &'a str>,
    ) -> Result<(), String> {
        let allowed: BTreeSet<&str> = allow.into_iter().collect();
        let missing: Vec<(String, String)> = self
            .long_options_without_short()
            .into_iter()
            .filter(|(path, long)| !allowed.contains(scoped_long(path, long).as_str()))
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        let listing = missing
            .iter()
            .map(|(path, long)| {
                if path.is_empty() {
                    format!("--{long}")
                } else {
                    format!("{path} --{long}")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        Err(format!("long option has no short: {listing}"))
    }
}

fn scoped_long(path: &str, long: &str) -> String {
    if path.is_empty() {
        format!("--{long}")
    } else {
        format!("{path} --{long}")
    }
}

fn collect_missing_shorts(
    path: &str,
    arguments: &[SurfaceArgument],
    missing: &mut Vec<(String, String)>,
) {
    for argument in arguments {
        if let Some(long) = &argument.long
            && argument.short.is_none()
            && argument.visible_short_aliases.is_empty()
        {
            missing.push((path.to_owned(), long.clone()));
        }
    }
}

fn collect_command_missing_shorts(command: &SurfaceCommand, missing: &mut Vec<(String, String)>) {
    collect_missing_shorts(&command.path, &command.arguments, missing);
    for child in &command.commands {
        collect_command_missing_shorts(child, missing);
    }
}

fn surface_command(
    command: &Command,
    parent: &str,
    declared_arguments: &BTreeMap<String, BTreeSet<String>>,
    inherited_arguments: &[SurfaceArgument],
) -> SurfaceCommand {
    let name = command.get_name().to_owned();
    let path = if parent.is_empty() {
        name.clone()
    } else {
        format!("{parent} {name}")
    };
    let arguments = declared_arguments_for(command, &path, declared_arguments);
    let mut child_inherited_arguments = inherited_arguments.to_vec();
    child_inherited_arguments.extend(
        arguments
            .iter()
            .filter(|argument| argument.scope == SurfaceScope::Global)
            .cloned(),
    );
    SurfaceCommand {
        name,
        path: path.clone(),
        aliases: command.get_all_aliases().map(ToOwned::to_owned).collect(),
        visible_aliases: command
            .get_visible_aliases()
            .map(ToOwned::to_owned)
            .collect(),
        hidden: command.is_hide_set(),
        about: command
            .get_about()
            .map(ToString::to_string)
            .unwrap_or_default(),
        arguments,
        inherited_arguments: inherited_arguments.to_vec(),
        commands: command
            .get_subcommands()
            .filter(|child| {
                declared_arguments.contains_key(&format!("{path} {}", child.get_name()))
            })
            .map(|child| {
                surface_command(child, &path, declared_arguments, &child_inherited_arguments)
            })
            .collect(),
    }
}

fn declared_arguments_for(
    command: &Command,
    path: &str,
    declared_arguments: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<SurfaceArgument> {
    command
        .get_arguments()
        .filter(|argument| declared_argument(declared_arguments, path, argument))
        .map(argument)
        .collect()
}

fn collect_declarations(
    command: &Command,
    path: &str,
    declared_arguments: &mut BTreeMap<String, BTreeSet<String>>,
) {
    declared_arguments.insert(
        path.to_owned(),
        command
            .get_arguments()
            .map(|argument| argument.get_id().to_string())
            .collect(),
    );
    for child in command.get_subcommands() {
        let child_path = if path.is_empty() {
            child.get_name().to_owned()
        } else {
            format!("{path} {}", child.get_name())
        };
        collect_declarations(child, &child_path, declared_arguments);
    }
}

fn declared_argument(
    declared_arguments: &BTreeMap<String, BTreeSet<String>>,
    path: &str,
    argument: &Arg,
) -> bool {
    declared_arguments
        .get(path)
        .is_some_and(|arguments| arguments.contains(argument.get_id().as_str()))
}

fn argument(argument: &Arg) -> SurfaceArgument {
    SurfaceArgument {
        id: argument.get_id().to_string(),
        index: argument.get_index(),
        short: argument.get_short(),
        long: argument.get_long().map(ToOwned::to_owned),
        visible_short_aliases: argument.get_visible_short_aliases().unwrap_or_default(),
        short_aliases: argument.get_all_short_aliases().unwrap_or_default(),
        visible_aliases: argument
            .get_visible_aliases()
            .unwrap_or_default()
            .iter()
            .map(|alias| (*alias).to_owned())
            .collect(),
        aliases: argument
            .get_all_aliases()
            .unwrap_or_default()
            .iter()
            .map(|alias| (*alias).to_owned())
            .collect(),
        value_names: argument
            .get_value_names()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect(),
        help: argument
            .get_help()
            .map(ToString::to_string)
            .unwrap_or_default(),
        requirement: if argument.is_required_set() {
            SurfaceRequirement::Required
        } else {
            SurfaceRequirement::Optional
        },
        scope: if argument.is_global_set() {
            SurfaceScope::Global
        } else {
            SurfaceScope::Local
        },
        hidden: argument.is_hide_set(),
        takes_values: argument.get_action().takes_values(),
    }
}

#[cfg(test)]
mod tests {
    use clap::{ArgAction, Parser, Subcommand};
    #[cfg(feature = "surface-templates")]
    use indoc::indoc;
    #[cfg(feature = "surface-templates")]
    use serde::Serialize;

    #[cfg(feature = "surface-templates")]
    use super::render;
    use super::{Surface, SurfaceArgument, SurfaceScope};

    #[derive(Parser)]
    #[command(name = "toy", version = "1.2.3", about = "Control toys")]
    struct Cli {
        #[arg(short, long, global = true, help = "Select a profile")]
        profile: Option<String>,
        #[command(subcommand)]
        command: Commands,
    }

    #[derive(Subcommand)]
    enum Commands {
        /// Show current state.
        #[command(alias = "state", visible_alias = "ls")]
        Status {
            /// Include archived records.
            #[arg(long, action = ArgAction::SetTrue)]
            archived: bool,
        },
        /// Mutate one item.
        Item {
            #[command(subcommand)]
            command: ItemCommand,
        },
        #[command(hide = true)]
        Internal,
    }

    #[derive(Subcommand)]
    enum ItemCommand {
        /// Create one item.
        Add {
            /// Item name.
            name: String,
        },
    }

    #[test]
    fn extracts_the_complete_clap_surface() {
        let surface = Surface::new::<Cli>("t");
        assert_eq!(surface.binary, "toy");
        assert_eq!(surface.mount, "t");
        assert_eq!(surface.version.as_deref(), Some("1.2.3"));
        assert_eq!(surface.about, "Control toys");
        assert_eq!(
            surface.inherited_arguments.as_slice(),
            &[] as &[SurfaceArgument]
        );
        assert!(surface.usage_kdl.contains("status"));
        assert_eq!(
            surface.mount_line,
            r#"#USAGE mount "mise run --quiet t -- --usage-spec=t""#
        );
        let status = &surface.commands[0];
        assert_eq!(status.visible_aliases, ["ls"]);
        assert_eq!(status.aliases, ["state", "ls"]);
        assert_eq!(status.about, "Show current state");
        assert_eq!(status.arguments[0].long.as_deref(), Some("archived"));
        assert!(!status.arguments[0].takes_values);
        assert_eq!(status.arguments.len(), 1);
        assert_eq!(status.inherited_arguments.len(), 1);
        assert_eq!(
            status.inherited_arguments[0].long.as_deref(),
            Some("profile")
        );
        assert_eq!(status.inherited_arguments[0].scope, SurfaceScope::Global);
        assert!(
            status
                .arguments
                .iter()
                .all(|argument| !matches!(argument.id.as_str(), "help" | "version" | "profile"))
        );
        let item = &surface.commands[1];
        assert_eq!(item.commands[0].path, "item add");
        assert_eq!(item.commands[0].arguments[0].index, Some(1));
        assert_eq!(item.commands[0].inherited_arguments.len(), 1);
        assert_eq!(
            item.commands[0].inherited_arguments[0].long.as_deref(),
            Some("profile")
        );
        assert!(surface.commands[2].hidden);
        assert!(
            surface
                .arguments
                .iter()
                .any(|arg| arg.long.as_deref() == Some("profile"))
        );
        assert!(
            surface
                .arguments
                .iter()
                .all(|argument| !matches!(argument.id.as_str(), "help" | "version"))
        );
        let noted = surface.note("skill", "Prefer the mounted task.");
        assert_eq!(noted.notes["skill"], "Prefer the mounted task.");
    }

    #[cfg(feature = "surface-templates")]
    #[derive(Serialize)]
    struct Content<'a> {
        version: &'a str,
        invocations: [&'a str; 2],
    }

    #[cfg(feature = "surface-templates")]
    #[test]
    fn shared_fragments_render_committed_operator_blocks() {
        let surface = Surface::new::<Cli>("t");
        let template = indoc! {r#"
            {%- from "ctl/version.md.jinja" import version_line -%}
            {%- from "ctl/invocation.md.jinja" import mounted_invocation -%}
            {%- from "ctl/commands.md.jinja" import command_inventory -%}
            ---
            {{ version_line(content.version) }}
            ---

            {{ mounted_invocation(surface, content.invocations) }}

            {{ command_inventory(surface) -}}
        "#};
        let rendered = render(
            "operator.md.jinja",
            template,
            &surface,
            &Content {
                version: "1.2.3",
                invocations: ["status", "item add demo"],
            },
        )
        .unwrap_or_else(|error| panic!("render operator template: {error}"));
        let expected = indoc! {r"
            ---
            version: 1.2.3
            ---

            ## Invocation

            ```sh
            mise run t status
            mise run t item add demo
            ```

            Never `mise run t --`. The `--` in `#USAGE mount` is mise's
            completion bootstrap.

            ## Commands

            | Command | Aliases | Purpose |
            |:--|:--|:--|
            | `status` | `ls` | Show current state |
            | `item` | — | Mutate one item |
        "};
        assert_eq!(rendered, expected);
        assert!(!rendered.contains("internal"));
    }

    #[test]
    fn long_only_status_flag_fails_require_shorts() {
        let error = Surface::new::<Cli>("t")
            .require_shorts([])
            .expect_err("archived is long-only");
        assert!(error.contains("--archived"), "{error}");
    }

    #[derive(Parser)]
    struct ShortsCli {
        #[arg(long, global = true)]
        verbose: bool,
        #[command(subcommand)]
        command: ShortsCommand,
    }

    #[derive(Subcommand)]
    enum ShortsCommand {
        Status {
            #[arg(long)]
            archived: bool,
        },
        #[command(hide = true)]
        Internal {
            #[arg(long)]
            secret: bool,
        },
    }

    #[test]
    fn reports_hidden_flags_and_inherited_globals_once() {
        let surface = Surface::new::<ShortsCli>("t");
        assert_eq!(
            surface.long_options_without_short(),
            [
                (String::new(), "verbose".to_owned()),
                ("status".to_owned(), "archived".to_owned()),
                ("internal".to_owned(), "secret".to_owned()),
            ]
        );
        let error = surface
            .require_shorts([])
            .expect_err("three flags are long-only");
        assert_eq!(
            error,
            "long option has no short: --verbose, status --archived, internal --secret"
        );
    }

    #[test]
    fn require_shorts_accepts_an_allow_list() {
        Surface::new::<ShortsCli>("t")
            .require_shorts(["--verbose", "status --archived", "internal --secret"])
            .unwrap_or_else(|error| panic!("{error}"));
    }

    #[test]
    fn a_root_allowance_does_not_exempt_a_nested_flag() {
        let error = Surface::new::<ShortsCli>("t")
            .require_shorts(["--verbose", "--archived", "internal --secret"])
            .expect_err("archived needs its command path");
        assert_eq!(error, "long option has no short: status --archived");
    }

    #[derive(Parser)]
    struct ShortAliasCli {
        #[arg(long, visible_short_alias = 'x')]
        expanded: bool,
    }

    #[test]
    fn a_short_alias_satisfies_the_contract() {
        Surface::new::<ShortAliasCli>("t")
            .require_shorts([])
            .unwrap_or_else(|error| panic!("{error}"));
    }

    #[derive(Parser)]
    struct HiddenShortAliasCli {
        #[arg(long, short_alias = 'x')]
        expanded: bool,
    }

    #[test]
    fn a_hidden_short_alias_is_not_an_operator_short() {
        let error = Surface::new::<HiddenShortAliasCli>("t")
            .require_shorts([])
            .expect_err("the only short is hidden");
        assert_eq!(error, "long option has no short: --expanded");
    }

    #[derive(Parser)]
    struct OwnedFile {
        #[command(flatten)]
        format: crate::flags::FormatLong,
        #[command(flatten)]
        color: crate::flags::ColorLong,
        #[arg(short = 'f', long)]
        file: Option<String>,
        #[arg(short = 'c', long)]
        config: Option<String>,
    }

    #[test]
    fn format_long_and_color_long_are_the_allow_list() {
        let surface = Surface::new::<OwnedFile>("x");
        let error = surface
            .require_shorts([])
            .expect_err("format/color/no-color are long-only");
        assert!(error.contains("--format"), "{error}");
        assert!(error.contains("--color"), "{error}");
        assert!(error.contains("--no-color"), "{error}");
        surface
            .require_shorts(["--format", "--color", "--no-color"])
            .unwrap_or_else(|error| panic!("{error}"));
    }

    #[derive(Parser)]
    struct DefaultOutput {
        #[command(flatten)]
        output: crate::flags::OutputArgs,
    }

    #[test]
    fn output_args_only_exempt_no_color() {
        Surface::new::<DefaultOutput>("x")
            .require_shorts(["--no-color"])
            .unwrap_or_else(|error| panic!("{error}"));
    }
}
