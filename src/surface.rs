//! Clap-derived operator metadata and shared MiniJinja fragments.
//!
//! Clap remains the command grammar. [`Surface`] extracts the stable operator
//! view used by committed skills and installed instructions, while the shared
//! fragments own repeated invocation, version, and command-inventory prose.

use std::collections::{BTreeMap, BTreeSet};

use clap::{Arg, Command};
use minijinja::{Environment, Error, context};
use serde::Serialize;

use crate::usage;

/// Shared fragment that renders a skill frontmatter version line.
pub const VERSION_FRAGMENT: &str = r"{% macro version_line(version) -%}
version: {{ version }}
{%- endmacro %}";

/// Shared fragment that renders mounted invocations and the no-`--` rule.
pub const INVOCATION_FRAGMENT: &str = r"{% macro mounted_invocation(surface, examples) -%}
## Invocation

```sh
{% for example in examples -%}
mise run {{ surface.mount }} {{ example }}
{% endfor -%}
```

Never `mise run {{ surface.mount }} --`. The `--` in `#USAGE mount` is mise's
completion bootstrap.
{%- endmacro %}";

/// Shared fragment that renders the visible top-level Clap commands.
pub const COMMANDS_FRAGMENT: &str = r#"{% macro command_inventory(surface) -%}
## Commands

| Command | Aliases | Purpose |
|:--|:--|:--|
{% for command in surface.commands if not command.hidden -%}
| `{{ command.name }}` | {% if command.visible_aliases %}`{{ command.visible_aliases | join("`, `") }}`{% else %}—{% endif %} | {{ command.about | replace("|", "\\|") | replace("\n", " ") }} |
{% endfor -%}
{{- "" -}}
{%- endmacro %}"#;

const FRAGMENTS: [(&str, &str); 3] = [
    ("ctl/version.md.jinja", VERSION_FRAGMENT),
    ("ctl/invocation.md.jinja", INVOCATION_FRAGMENT),
    ("ctl/commands.md.jinja", COMMANDS_FRAGMENT),
];

/// Serializable operator-facing projection of one Clap command graph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
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
    /// Nested subcommands in Clap declaration order.
    pub commands: Vec<Self>,
}

/// One positional argument or flag in a [`Surface`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
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
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceRequirement {
    /// The invocation can omit this argument.
    Optional,
    /// The invocation must provide this argument.
    Required,
}

/// How far Clap propagates an argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
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
        let arguments = command
            .get_arguments()
            .filter(|argument| declared_argument(&declared_arguments, "", argument))
            .map(argument)
            .collect();
        let commands = command
            .get_subcommands()
            .filter(|child| declared_arguments.contains_key(child.get_name()))
            .map(|child| surface_command(child, "", &declared_arguments))
            .collect();
        let usage_kdl = usage::spec(command, &mount);
        let mount_line = usage::mount_line(&mount);
        Self {
            binary,
            mount,
            about,
            version,
            arguments,
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
}

/// Add ctl-core's shared operator fragments to an existing environment.
pub fn add_fragments(environment: &mut Environment<'static>) -> Result<(), Error> {
    for (name, source) in FRAGMENTS {
        environment.add_template(name, source)?;
    }
    Ok(())
}

/// Create a strict `MiniJinja` environment containing the shared fragments.
pub fn environment() -> Result<Environment<'static>, Error> {
    let mut environment = Environment::new();
    environment.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
    environment.set_keep_trailing_newline(true);
    add_fragments(&mut environment)?;
    Ok(environment)
}

/// Render one operator template with a [`Surface`] and consumer-owned context.
pub fn render<T: Serialize>(
    name: &'static str,
    source: &'static str,
    surface: &Surface,
    content: &T,
) -> Result<String, Error> {
    let mut environment = environment()?;
    environment.add_template(name, source)?;
    environment
        .get_template(name)?
        .render(context! { surface, content })
}

fn surface_command(
    command: &Command,
    parent: &str,
    declared_arguments: &BTreeMap<String, BTreeSet<String>>,
) -> SurfaceCommand {
    let name = command.get_name().to_owned();
    let path = if parent.is_empty() {
        name.clone()
    } else {
        format!("{parent} {name}")
    };
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
        arguments: command
            .get_arguments()
            .filter(|argument| declared_argument(declared_arguments, &path, argument))
            .map(argument)
            .collect(),
        commands: command
            .get_subcommands()
            .filter(|child| {
                declared_arguments.contains_key(&format!("{path} {}", child.get_name()))
            })
            .map(|child| surface_command(child, &path, declared_arguments))
            .collect(),
    }
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
    use indoc::indoc;
    use serde::Serialize;

    use super::{Surface, render};

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
        assert!(
            status
                .arguments
                .iter()
                .all(|argument| !matches!(argument.id.as_str(), "help" | "version" | "profile"))
        );
        let item = &surface.commands[1];
        assert_eq!(item.commands[0].path, "item add");
        assert_eq!(item.commands[0].arguments[0].index, Some(1));
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

    #[derive(Serialize)]
    struct Content<'a> {
        version: &'a str,
        invocations: [&'a str; 2],
    }

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
}
