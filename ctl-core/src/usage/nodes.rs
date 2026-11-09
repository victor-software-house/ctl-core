//! The Usage spec of a clap [`Command`], as KDL nodes.
//!
//! The root command's attributes are top-level nodes, and each subcommand is a
//! `cmd` node. Flags, positional arguments, groups, completions, and
//! subcommands follow in that order. clap's own help and version flags are
//! not in the spec. clap adds them when it builds a command, and the spec is
//! read from the command as declared.

use clap::builder::PossibleValue;
use clap::{Arg, ArgAction, Command, ValueHint};

use super::kdl::{Document, Node, Value};
use super::plain::plain;

/// The order clap gives a subcommand that sets none.
const DEFAULT_ORDER: usize = 999;

/// The spec of `command`, named `bin`.
///
/// The usage line is rendered last: clap adds its help and version flags
/// while it renders, and every other part reads the command as declared.
///
/// # Panics
///
/// Panics when an argument conflicts with an id that is neither an argument
/// nor a group of its command, as [`Command::get_arg_conflicts_with`] does.
pub(crate) fn document(mut command: Command, bin: &str) -> Document {
    let body = Body::new(&command);
    let mut nodes = Vec::new();
    let mut line = |name: &str, value: Value| {
        let mut node = Node::new(name);
        node.arg(value);
        nodes.push(node);
    };
    line("name", Value::Text(bin.to_owned()));
    line("bin", Value::Text(bin.to_owned()));
    let metadata = [
        ("version", command.get_version().map(str::to_owned)),
        (
            "long_version",
            command.get_long_version().map(str::to_owned),
        ),
        ("about", command.get_about().map(plain)),
        ("long_about", command.get_long_about().map(plain)),
    ];
    for (name, value) in metadata {
        if let Some(value) = value {
            line(name, Value::Text(value));
        }
    }
    line("unknown_flags", Value::Text(body.unknown_flags.to_owned()));
    if command.is_multicall_set() {
        line("multicall", Value::Bool(true));
    }
    let settings = parsing_settings(&command)
        .into_iter()
        .chain(layout_settings(&command, Scope::Root));
    for setting in settings {
        match setting {
            Setting::True(name) => line(name, Value::Bool(true)),
            Setting::False(name) => line(name, Value::Bool(false)),
            Setting::Text(name, text) => line(name, Value::Text(text)),
        }
    }
    let usage = plain(&command.render_usage());
    if !usage.is_empty() {
        line("usage", Value::Text(usage));
    }
    nodes.extend(body.into_nodes());
    Document(nodes)
}

/// Whether settings are written for the root command or a subcommand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope {
    Root,
    Subcommand,
}

/// One command setting the spec records.
#[derive(Debug)]
enum Setting {
    True(&'static str),
    False(&'static str),
    Text(&'static str, String),
}

/// The settings that change how a command parses, in the spec's order.
/// `args_override_self` is written only when off, because the spec's default
/// is on.
fn parsing_settings(command: &Command) -> Vec<Setting> {
    let has_subcommands = command.has_subcommands();
    let has_version = command.get_version().is_some() || command.get_long_version().is_some();
    let mut settings = Vec::new();
    let leading = [
        (
            "external_subcommand",
            command.is_allow_external_subcommands_set(),
        ),
        (
            "arg_required_else_help",
            command.is_arg_required_else_help_set(),
        ),
        ("disable_help_flag", command.is_disable_help_flag_set()),
        (
            "disable_help_subcommand",
            has_subcommands && command.is_disable_help_subcommand_set(),
        ),
        (
            "disable_version_flag",
            has_version && command.is_disable_version_flag_set(),
        ),
        (
            "dont_delimit_trailing_values",
            command.is_dont_delimit_trailing_values_set(),
        ),
    ];
    settings.extend(
        leading
            .into_iter()
            .filter(|(_, on)| *on)
            .map(|(name, _)| Setting::True(name)),
    );
    if !command.is_args_override_self() {
        settings.push(Setting::False("args_override_self"));
    }
    let trailing = [
        (
            "subcommand_negates_reqs",
            command.is_subcommand_negates_reqs_set(),
        ),
        (
            "args_conflicts_with_subcommands",
            command.is_args_conflicts_with_subcommands_set(),
        ),
        (
            "subcommand_precedence_over_arg",
            command.is_subcommand_precedence_over_arg_set(),
        ),
        (
            "allow_missing_positional",
            command.is_allow_missing_positional_set(),
        ),
    ];
    settings.extend(
        trailing
            .into_iter()
            .filter(|(_, on)| *on)
            .map(|(name, _)| Setting::True(name)),
    );
    settings
}

/// The settings that change how a command's help is laid out, in the spec's
/// order. The root records `subcommand_required` only when it has
/// subcommands.
fn layout_settings(command: &Command, scope: Scope) -> Vec<Setting> {
    let required = command.is_subcommand_required_set()
        && match scope {
            Scope::Root => command.has_subcommands(),
            Scope::Subcommand => true,
        };
    let mut settings = Vec::new();
    if required {
        settings.push(Setting::True("subcommand_required"));
    }
    if let Some(heading) = command.get_subcommand_help_heading() {
        settings.push(Setting::Text("subcommand_help_heading", heading.to_owned()));
    }
    if let Some(name) = command.get_subcommand_value_name() {
        settings.push(Setting::Text("subcommand_value_name", name.to_owned()));
    }
    let switches = [
        ("next_line_help", command.is_next_line_help_set()),
        ("flatten_help", command.is_flatten_help_set()),
    ];
    settings.extend(
        switches
            .into_iter()
            .filter(|(_, on)| *on)
            .map(|(name, _)| Setting::True(name)),
    );
    settings
}

/// What a command holds besides its own attributes, as the nodes the spec
/// writes after them.
#[derive(Debug)]
struct Body {
    flags: Vec<Node>,
    positionals: Vec<Node>,
    groups: Vec<Node>,
    completions: Vec<Node>,
    subcommands: Vec<Node>,
    /// `value` when the command passes an unknown flag on to an argument,
    /// `error` when clap rejects it.
    unknown_flags: &'static str,
}

impl Body {
    /// # Panics
    ///
    /// As [`write_conflicts`].
    fn new(command: &Command) -> Self {
        let (positionals, flags): (Vec<&Arg>, Vec<&Arg>) =
            command.get_arguments().partition(|arg| arg.is_positional());
        let flag_order = Order::of(&flags);
        let positional_order = Order::of(&positionals);
        let forwards = command
            .get_arguments()
            .any(|arg| arg.is_allow_hyphen_values_set() || arg.is_trailing_var_arg_set());
        let subcommand_order = if command
            .get_subcommands()
            .all(|subcommand| subcommand.get_display_order() == DEFAULT_ORDER)
        {
            Order::Hidden
        } else {
            Order::Shown
        };
        Self {
            flags: flags
                .into_iter()
                .map(|arg| flag_node(command, arg, flag_order))
                .collect(),
            positionals: positionals
                .into_iter()
                .map(|arg| positional_node(command, arg, positional_order))
                .collect(),
            groups: group_nodes(command),
            completions: completion_nodes(command),
            subcommands: command
                .get_subcommands()
                .map(|subcommand| command_node(subcommand, subcommand_order))
                .collect(),
            unknown_flags: if forwards { "value" } else { "error" },
        }
    }

    fn into_nodes(self) -> impl Iterator<Item = Node> {
        self.flags
            .into_iter()
            .chain(self.positionals)
            .chain(self.groups)
            .chain(self.completions)
            .chain(self.subcommands)
    }
}

/// Whether nodes write their display order. clap numbers every argument as
/// it is declared, so the spec writes the numbers only when they reorder a
/// list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Order {
    Shown,
    Hidden,
}

impl Order {
    fn of(args: &[&Arg]) -> Self {
        if args.is_sorted_by_key(|arg| arg.get_display_order()) {
            Self::Hidden
        } else {
            Self::Shown
        }
    }

    fn write(self, node: &mut Node, order: usize) {
        match self {
            Self::Shown => node.prop("display_order", Value::Number(order)),
            Self::Hidden => {}
        }
    }
}

/// # Panics
///
/// As [`write_conflicts`].
fn command_node(command: &Command, order: Order) -> Node {
    let body = Body::new(command);
    let mut node = Node::new("cmd");
    node.arg(Value::Word(command.get_name().to_owned()));
    node.enable([("hide", command.is_hide_set())]);
    order.write(&mut node, command.get_display_order());
    let settings = layout_settings(command, Scope::Subcommand)
        .into_iter()
        .chain(parsing_settings(command));
    for setting in settings {
        match setting {
            Setting::True(name) => node.prop(name, Value::Bool(true)),
            Setting::False(name) => node.prop(name, Value::Bool(false)),
            Setting::Text(name, text) => node.prop(name, Value::Word(text)),
        }
    }
    let visible: Vec<&str> = command.get_visible_aliases().collect();
    let hidden: Vec<&str> = command
        .get_all_aliases()
        .filter(|alias| !visible.contains(alias))
        .collect();
    if !visible.is_empty() {
        node.child(word_list("alias", &visible));
    }
    if !hidden.is_empty() {
        let mut aliases = word_list("alias", &hidden);
        aliases.enable([("hide", true)]);
        node.child(aliases);
    }
    let texts = [
        (
            "help",
            "long_help",
            command.get_about(),
            command.get_long_about(),
        ),
        (
            "before_help",
            "before_long_help",
            command.get_before_help(),
            command.get_before_long_help(),
        ),
        (
            "after_help",
            "after_long_help",
            command.get_after_help(),
            command.get_after_long_help(),
        ),
    ];
    for (short_key, long_name, short, long) in texts {
        if let Some(short) = short {
            node.prop(short_key, Value::Text(plain(short)));
        }
        if let Some(long) = long {
            node.child(text_line(long_name, plain(long)));
        }
    }
    node.prop("unknown_flags", Value::Text(body.unknown_flags.to_owned()));
    for child in body.into_nodes() {
        node.child(child);
    }
    node
}

/// How an argument's action reads in the spec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    /// Takes a value each time; `Append` also collects them.
    Value {
        collects: bool,
    },
    /// A switch with no value. `negates` marks clap's `SetFalse`.
    Switch {
        negates: bool,
    },
    Count,
    /// One of clap's own actions, by its spec name.
    Builtin(&'static str),
}

impl Action {
    /// clap marks `ArgAction` non-exhaustive. An action added later takes one
    /// value when clap says it takes values, and is a switch otherwise.
    #[expect(
        clippy::match_same_arms,
        reason = "SetTrue and an action clap adds later both read as a plain switch"
    )]
    fn of(arg: &Arg) -> Self {
        match arg.get_action() {
            ArgAction::Set => Self::Value { collects: false },
            ArgAction::Append => Self::Value { collects: true },
            ArgAction::SetTrue => Self::Switch { negates: false },
            ArgAction::SetFalse => Self::Switch { negates: true },
            ArgAction::Count => Self::Count,
            ArgAction::Help => Self::Builtin("help"),
            ArgAction::HelpShort => Self::Builtin("help_short"),
            ArgAction::HelpLong => Self::Builtin("help_long"),
            ArgAction::Version => Self::Builtin("version"),
            action if action.takes_values() => Self::Value { collects: false },
            _ => Self::Switch { negates: false },
        }
    }

    fn takes_value(self) -> bool {
        match self {
            Self::Value { .. } => true,
            Self::Switch { .. } | Self::Count | Self::Builtin(_) => false,
        }
    }

    fn repeats(self) -> bool {
        match self {
            Self::Value { collects } => collects,
            Self::Count => true,
            Self::Switch { .. } | Self::Builtin(_) => false,
        }
    }
}

/// A flag's spellings: the visible ones in its declaration, the hidden ones
/// in an `alias` child.
#[derive(Debug)]
struct Spellings {
    shorts: Vec<char>,
    hidden_shorts: Vec<char>,
    longs: Vec<String>,
    hidden_longs: Vec<String>,
}

impl Spellings {
    fn of(arg: &Arg) -> Self {
        let shorts = arg.get_short_and_visible_aliases().unwrap_or_default();
        let hidden_shorts = arg
            .get_all_short_aliases()
            .unwrap_or_default()
            .into_iter()
            .filter(|short| !shorts.contains(short))
            .collect();
        let longs: Vec<String> = arg
            .get_long_and_visible_aliases()
            .unwrap_or_default()
            .into_iter()
            .map(str::to_owned)
            .collect();
        let hidden_longs = arg
            .get_all_aliases()
            .unwrap_or_default()
            .into_iter()
            .filter(|long| !longs.iter().any(|visible| visible == long))
            .map(str::to_owned)
            .collect();
        Self {
            shorts,
            hidden_shorts,
            longs,
            hidden_longs,
        }
    }

    /// The flag's name: its first long spelling, else its first short one.
    fn name(&self) -> String {
        let long = self.longs.iter().chain(&self.hidden_longs).next().cloned();
        let short = || {
            self.shorts
                .iter()
                .chain(&self.hidden_shorts)
                .next()
                .map(char::to_string)
        };
        long.or_else(short).unwrap_or_default()
    }

    /// For a `SetFalse` flag with one long spelling and no short one, take
    /// that spelling out as the flag's negation, `--<long>`.
    fn take_negation(&mut self, action: Action) -> Option<String> {
        let single = self.shorts.is_empty()
            && self.hidden_shorts.is_empty()
            && self.longs.len() + self.hidden_longs.len() == 1;
        if action != (Action::Switch { negates: true }) || !single {
            return None;
        }
        let long = self
            .longs
            .pop()
            .or_else(|| self.hidden_longs.first().cloned())?;
        Some(format!("--{long}"))
    }

    /// The declaration: the visible spellings, after `name:` when the first
    /// of them does not already name the flag.
    fn declaration(&self, name: &str) -> String {
        let forms: Vec<String> = self
            .shorts
            .iter()
            .map(|short| format!("-{short}"))
            .chain(self.longs.iter().map(|long| format!("--{long}")))
            .collect();
        let forms = forms.join(" ");
        let named = match (self.longs.first(), self.shorts.first()) {
            (Some(long), _) => long == name,
            (None, Some(short)) => short.to_string() == name,
            (None, None) => false,
        };
        if named {
            forms
        } else if forms.is_empty() {
            format!("{name}:")
        } else {
            format!("{name}: {forms}")
        }
    }

    fn hidden_node(&self) -> Option<Node> {
        if self.hidden_shorts.is_empty() && self.hidden_longs.is_empty() {
            return None;
        }
        let mut node = Node::new("alias");
        for short in &self.hidden_shorts {
            node.arg(Value::Text(format!("-{short}")));
        }
        for long in &self.hidden_longs {
            node.arg(Value::Text(format!("--{long}")));
        }
        node.enable([("hide", true)]);
        Some(node)
    }
}

/// # Panics
///
/// As [`write_conflicts`].
fn flag_node(command: &Command, arg: &Arg, order: Order) -> Node {
    let action = Action::of(arg);
    let mut spellings = Spellings::of(arg);
    let name = spellings.name();
    let negation = spellings.take_negation(action);
    let value = action
        .takes_value()
        .then(|| ArgValue::of_flag(command, arg, action, &name));
    let mut node = Node::new("flag");
    node.arg(Value::Word(spellings.declaration(&name)));
    if let Some(help) = arg.get_help() {
        node.prop("help", Value::Text(plain(help)));
    }
    if let Some(aliases) = spellings.hidden_node() {
        node.child(aliases);
    }
    if let Some(long_help) = arg.get_long_help() {
        node.child(text_line("long_help", plain(long_help)));
    }
    node.enable([
        ("required", arg.is_required_set()),
        ("var", action.repeats()),
        ("hide", arg.is_hide_set()),
    ]);
    node.enable(hidden_parts(arg));
    node.enable([
        ("global", arg.is_global_set()),
        ("count", action == Action::Count),
    ]);
    if let Action::Builtin(name) = action {
        node.prop("action", Value::Text(name.to_owned()));
    }
    node.enable([
        (
            "allow_hyphen_values",
            value.is_some() && arg.is_allow_hyphen_values_set(),
        ),
        (
            "allow_negative_numbers",
            value.as_ref().is_some_and(|value| value.negative_numbers),
        ),
    ]);
    if let Some(terminator) = value.as_ref().and_then(|value| value.terminator.clone()) {
        node.prop("value_terminator", Value::Text(terminator));
    }
    if let Some(negation) = negation {
        node.prop("negate", Value::Text(negation));
    }
    write_conflicts(&mut node, command, arg);
    node.enable([
        ("exclusive", arg.is_exclusive_set()),
        ("require_equals", arg.is_require_equals_set()),
        (
            "value_optional",
            value.is_some()
                && arg
                    .get_num_args()
                    .is_some_and(|range| range.min_values() == 0 && range.max_values() > 0),
        ),
    ]);
    if let Some(heading) = arg.get_help_heading() {
        node.prop("help_heading", Value::Text(heading.to_owned()));
    }
    order.write(&mut node, arg.get_display_order());
    write_defaults(&mut node, arg, Value::Word);
    if let Some(value) = value {
        node.child(value.node(arg));
    }
    node
}

/// # Panics
///
/// As [`write_conflicts`].
fn positional_node(command: &Command, arg: &Arg, order: Order) -> Node {
    let value = ArgValue::of_positional(command, arg);
    let label = value.label();
    let misreading = value.misreading(&label);
    let mut node = Node::new("arg");
    node.arg(Value::Word(label));
    if let Some(help) = arg.get_help() {
        node.prop("help", Value::Text(plain(help)));
    }
    if let Some(long_help) = arg.get_long_help() {
        node.prop("help_long", Value::Text(plain(long_help)));
    }
    if !value.required {
        node.prop("required", Value::Bool(false));
    }
    if value.dash == Dash::Automatic {
        node.prop("double_dash", Value::Word("automatic".to_owned()));
    }
    if misreading.double_dash {
        node.prop("double_dash", Value::Word("optional".to_owned()));
    }
    value.write_shape(&mut node);
    misreading.write_single(&mut node);
    node.enable([("hide", arg.is_hide_set())]);
    node.enable(hidden_parts(arg));
    write_conflicts(&mut node, command, arg);
    write_defaults(&mut node, arg, Value::Text);
    if let Some(heading) = arg.get_help_heading() {
        node.prop("help_heading", Value::Text(heading.to_owned()));
    }
    order.write(&mut node, arg.get_display_order());
    if let Some(names) = misreading.names_node(&value.name) {
        node.child(names);
    }
    if let Some(choices) = choices_node(arg) {
        node.child(choices);
    }
    node
}

/// The parts of an argument's help it hides, as spec property names.
fn hidden_parts(arg: &Arg) -> [(&'static str, bool); 6] {
    [
        ("hide_default_value", arg.is_hide_default_value_set()),
        ("hide_env", arg.is_hide_env_set()),
        ("hide_env_values", arg.is_hide_env_values_set()),
        ("hide_possible_values", arg.is_hide_possible_values_set()),
        ("hide_short_help", arg.is_hide_short_help_set()),
        ("hide_long_help", arg.is_hide_long_help_set()),
    ]
}

/// Where a value sits relative to `--`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dash {
    /// Before or after `--`.
    Optional,
    /// Only after `--`: clap's `last`.
    Required,
    /// Takes the rest of the line, dashes included: clap's
    /// `trailing_var_arg`.
    Automatic,
}

/// The value an argument takes: a positional itself, or a flag's `arg`
/// child.
#[derive(Debug)]
struct ArgValue {
    name: String,
    names: Vec<String>,
    required: bool,
    repeats: bool,
    bounds: Option<Bounds>,
    delimiter: Option<char>,
    negative_numbers: bool,
    terminator: Option<String>,
    dash: Dash,
}

/// How many values one occurrence takes, when that can be more than one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Bounds {
    min: usize,
    /// `None` when clap sets no upper bound.
    max: Option<usize>,
}

impl ArgValue {
    fn of_flag(command: &Command, arg: &Arg, action: Action, flag: &str) -> Self {
        let names = value_names(arg);
        let delimiter = arg.get_value_delimiter();
        let bounds = bounds(arg, names.len());
        let repeats = delimiter.is_some()
            || action.repeats()
            || arg
                .get_num_args()
                .is_some_and(|range| range.max_values() > 1)
            || bounds.is_some();
        Self {
            name: names.first().cloned().unwrap_or_else(|| flag.to_owned()),
            names,
            required: true,
            repeats,
            bounds,
            delimiter: delimiter.filter(char::is_ascii),
            negative_numbers: negative_numbers(command, arg),
            terminator: terminator(arg, repeats),
            dash: Dash::Optional,
        }
    }

    fn of_positional(command: &Command, arg: &Arg) -> Self {
        let names = value_names(arg);
        let delimiter = arg.get_value_delimiter();
        let bounds = bounds(arg, names.len());
        let repeats = delimiter.is_some() || Action::of(arg).repeats() || bounds.is_some();
        let dash = if arg.is_last_set() {
            Dash::Required
        } else if arg.is_trailing_var_arg_set() {
            Dash::Automatic
        } else {
            Dash::Optional
        };
        Self {
            name: positional_name(arg),
            names,
            required: arg.is_required_set(),
            repeats,
            bounds,
            delimiter: delimiter.filter(char::is_ascii),
            negative_numbers: negative_numbers(command, arg),
            terminator: terminator(arg, repeats),
            dash,
        }
    }

    /// The value count when every occurrence takes the same number of values,
    /// more than one.
    fn fixed_count(&self) -> Option<usize> {
        self.bounds
            .filter(|bounds| self.repeats && bounds.max == Some(bounds.min) && bounds.min > 1)
            .map(|bounds| bounds.min)
    }

    /// The usage placeholder: `<name>` when required and `[name]` when not.
    /// A repeating value adds `…`, a fixed count writes one placeholder per
    /// slot, and a value that only follows `--` starts with `--` and a space.
    fn label(&self) -> String {
        let fixed = self.fixed_count();
        let after_dash = |text: String| match self.dash {
            Dash::Required => format!("-- {text}"),
            Dash::Optional | Dash::Automatic => text,
        };
        if self.names.len() > 1 || fixed.is_some() {
            let slots = if self.names.len() > 1 {
                self.names.clone()
            } else {
                vec![self.name.clone(); fixed.unwrap_or(1)]
            };
            let placeholders: Vec<String> = slots.iter().map(|slot| self.wrap(slot)).collect();
            return after_dash(placeholders.join(" "));
        }
        let label = self.wrap(&after_dash(self.name.clone()));
        if self.repeats {
            format!("{label}…")
        } else {
            label
        }
    }

    fn wrap(&self, text: &str) -> String {
        if self.required {
            format!("<{text}>")
        } else {
            format!("[{text}]")
        }
    }

    /// Write how many values the argument takes and how they are read.
    fn write_shape(&self, node: &mut Node) {
        node.enable([("var", self.repeats)]);
        if let Some(bounds) = self.bounds {
            node.prop("var_min", Value::Number(bounds.min));
            if let Some(max) = bounds.max {
                node.prop("var_max", Value::Number(max));
            }
        }
        if let Some(delimiter) = self.delimiter {
            node.prop("delimiter", Value::Text(delimiter.to_string()));
        }
        node.enable([("allow_negative_numbers", self.negative_numbers)]);
        if let Some(terminator) = &self.terminator {
            node.prop("value_terminator", Value::Text(terminator.clone()));
        }
    }

    /// What mise would read into `label`, this value's [`label`](Self::label),
    /// beyond the name clap means.
    fn misreading(&self, label: &str) -> Misreading {
        let one_name = self.names.len() <= 1 && self.fixed_count().is_none();
        let placeholders = one_name && splits_into_placeholders(label);
        Misreading {
            placeholders,
            ellipsis: !placeholders
                && !self.repeats
                && (self.name.ends_with("...") || self.name.ends_with('…')),
            double_dash: match self.dash {
                Dash::Optional => self.name.starts_with("-- "),
                Dash::Required | Dash::Automatic => false,
            },
        }
    }

    /// The `arg` child of a flag that takes a value.
    ///
    /// A flag's value keeps a name that starts with `--` and a space. mise
    /// reads that prefix as a value that only follows `--`, which changes
    /// nothing for a flag's value.
    fn node(&self, arg: &Arg) -> Node {
        let label = self.label();
        let misreading = self.misreading(&label);
        let mut node = Node::new("arg");
        node.arg(Value::Word(label));
        self.write_shape(&mut node);
        misreading.write_single(&mut node);
        if let Some(names) = misreading.names_node(&self.name) {
            node.child(names);
        }
        if let Some(choices) = choices_node(arg) {
            node.child(choices);
        }
        node
    }
}

/// What mise would read into a value's label beyond the name clap means.
///
/// mise parses an `arg` name as usage syntax, and clap gives a value name no
/// syntax at all. mise reads three forms as more than a name:
///
/// - a label that splits into several `<…>` or `[…]` words, as that many
///   values;
/// - a name that ends in `...` or `…`, as variadic;
/// - a name that starts with `--` and a space, as a value that only follows
///   `--`.
///
/// mise applies an argument's properties after it parses the name, so a
/// property states clap's meaning again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Misreading {
    placeholders: bool,
    ellipsis: bool,
    double_dash: bool,
}

impl Misreading {
    /// Write `var=#false` when the label would read as more than one value.
    fn write_single(self, node: &mut Node) {
        if self.placeholders || self.ellipsis {
            node.prop("var", Value::Bool(false));
        }
    }

    /// A `value_names` child holding `name` whole, when the label would split
    /// into placeholders. mise takes the argument's name from it.
    fn names_node(self, name: &str) -> Option<Node> {
        self.placeholders.then(|| {
            let mut node = Node::new("value_names");
            node.arg(Value::Word(name.to_owned()));
            node
        })
    }
}

/// Whether mise reads `label` as several placeholders: a label holding ASCII
/// whitespace that splits into two words or more, each enclosed in `<…>` or
/// in `[…]`.
fn splits_into_placeholders(label: &str) -> bool {
    let enclosed = |word: &str| {
        (word.starts_with('<') && word.ends_with('>'))
            || (word.starts_with('[') && word.ends_with(']'))
    };
    label.bytes().any(|byte| byte.is_ascii_whitespace())
        && label.split_whitespace().count() > 1
        && label.split_whitespace().all(enclosed)
}

/// The value count clap reads per occurrence, when it can be more than one.
/// A delimiter gives none: clap counts raw words before it splits them, so
/// a range would count something else than the spec.
fn bounds(arg: &Arg, names: usize) -> Option<Bounds> {
    if arg.get_value_delimiter().is_some() {
        return None;
    }
    let Some(range) = arg.get_num_args() else {
        return (names > 1).then_some(Bounds {
            min: names,
            max: Some(names),
        });
    };
    let max = range.max_values();
    (max > 1).then(|| Bounds {
        min: range.min_values(),
        max: (max != usize::MAX).then_some(max),
    })
}

fn negative_numbers(command: &Command, arg: &Arg) -> bool {
    arg.is_allow_negative_numbers_set() || command.is_allow_negative_numbers_set()
}

/// The value terminator, which only a repeating value has.
fn terminator(arg: &Arg, repeats: bool) -> Option<String> {
    arg.get_value_terminator()
        .filter(|_| repeats)
        .map(ToString::to_string)
}

/// The value names the spec can write: all of them for a fixed count with
/// one name per slot, else the first.
fn value_names(arg: &Arg) -> Vec<String> {
    let names: Vec<String> = arg
        .get_value_names()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect();
    let one_per_slot = arg.get_value_delimiter().is_none()
        && arg.get_num_args().is_none_or(|range| {
            range.min_values() == names.len() && range.max_values() == names.len()
        });
    if names.len() <= 1 || one_per_slot {
        names
    } else {
        names.into_iter().take(1).collect()
    }
}

/// A positional's name: its first value name, else its id.
fn positional_name(arg: &Arg) -> String {
    value_names(arg)
        .into_iter()
        .next()
        .unwrap_or_else(|| arg.get_id().to_string())
}

/// How the spec refers to an argument from another one: `--long`, `-s`, or
/// a positional's name.
fn selector(arg: &Arg) -> Option<String> {
    match (arg.get_long(), arg.get_short()) {
        (Some(long), _) => Some(format!("--{long}")),
        (None, Some(short)) => Some(format!("-{short}")),
        (None, None) => arg.is_positional().then(|| positional_name(arg)),
    }
}

/// Write what `arg` conflicts with: one selector as a property when it reads
/// back as one, else a `conflicts` child.
///
/// # Panics
///
/// Panics when `arg` conflicts with an id that is neither an argument nor a
/// group of `command`, as [`Command::get_arg_conflicts_with`] does.
fn write_conflicts(node: &mut Node, command: &Command, arg: &Arg) {
    let mut conflicts: Vec<String> = command
        .get_arg_conflicts_with(arg)
        .into_iter()
        .filter_map(selector)
        .collect();
    if conflicts.len() > 1 || conflicts.first().is_some_and(|only| !fits_property(only)) {
        node.child(text_list("conflicts", conflicts));
    } else if let Some(only) = conflicts.pop() {
        node.prop("conflicts", Value::Text(only));
    }
}

/// Whether a selector written as a property reads back as one selector. A
/// value of several words, one of them dashed, reads as a list of flags.
fn fits_property(selector: &str) -> bool {
    let mut words = selector.split_whitespace();
    words.nth(1).is_none()
        || !selector
            .split_whitespace()
            .any(|word| word.starts_with('-'))
}

/// Write `arg`'s defaults: one as a property built by `value`, several as a
/// `default` child with a node per value. clap splits a default on the
/// value delimiter, so each part is one value.
fn write_defaults(node: &mut Node, arg: &Arg, value: fn(String) -> Value) {
    let delimiter = arg.get_value_delimiter();
    let mut defaults: Vec<String> = Vec::new();
    for default in arg.get_default_values() {
        let default = default.to_string_lossy();
        match delimiter {
            Some(delimiter) => defaults.extend(default.split(delimiter).map(str::to_owned)),
            None => defaults.push(default.into_owned()),
        }
    }
    if defaults.len() > 1 {
        let mut list = Node::new("default");
        for default in &defaults {
            list.child(Node::new(default));
        }
        node.child(list);
    } else if let Some(only) = defaults.pop() {
        node.prop("default", value(only));
    }
}

/// The possible values: bare words when none has help, a hidden flag, or an
/// alias, else a `choice` child each.
fn choices_node(arg: &Arg) -> Option<Node> {
    let values = arg.get_possible_values();
    if values.is_empty() {
        return None;
    }
    let detailed = values.iter().any(|value| {
        value.get_help().is_some()
            || value.is_hide_set()
            || value.get_name_and_aliases().nth(1).is_some()
    });
    let mut node = Node::new("choices");
    for value in &values {
        if detailed {
            node.child(choice_node(value));
        } else {
            node.arg(Value::Word(value.get_name().to_owned()));
        }
    }
    node.enable([("ignore_case", arg.is_ignore_case_set())]);
    Some(node)
}

/// One possible value. clap never shows a possible value's alias in help, so
/// each alias is hidden.
fn choice_node(value: &PossibleValue) -> Node {
    let mut node = Node::new("choice");
    node.arg(Value::Text(value.get_name().to_owned()));
    if let Some(help) = value.get_help() {
        node.prop("help", Value::Text(plain(help)));
    }
    node.enable([("hide", value.is_hide_set())]);
    for alias in value.get_name_and_aliases().skip(1) {
        let mut alias_node = text_line("alias", alias.to_owned());
        alias_node.enable([("hide", true)]);
        node.child(alias_node);
    }
    node
}

/// The groups that state a rule: at least two members, and not both
/// optional and open to several. clap derive adds such an open group for
/// every `Args` struct, which states nothing.
fn group_nodes(command: &Command) -> Vec<Node> {
    let mut nodes = Vec::new();
    for group in command.get_groups() {
        let members: Vec<String> = group
            .get_args()
            .filter_map(|id| command.get_arguments().find(|arg| arg.get_id() == id))
            .filter_map(selector)
            .collect();
        let required = group.is_required_set();
        let multiple = group.clone().is_multiple();
        if members.len() < 2 || (multiple && !required) {
            continue;
        }
        let mut node = Node::new("group");
        node.arg(Value::Text(group.get_id().to_string()));
        for member in members {
            node.arg(Value::Text(member));
        }
        node.enable([("required", required), ("multiple", multiple)]);
        nodes.push(node);
    }
    nodes
}

/// One `complete` node per value name with a value hint the spec names,
/// keyed by the lowercased name. A later argument with the same name
/// replaces the type in place.
fn completion_nodes(command: &Command) -> Vec<Node> {
    let mut completions: Vec<(String, &'static str)> = Vec::new();
    for arg in command.get_arguments() {
        let Some(hint) = hint_type(arg.get_value_hint()) else {
            continue;
        };
        let name = if arg.is_positional() {
            positional_name(arg)
        } else if Action::of(arg).takes_value() {
            value_names(arg)
                .into_iter()
                .next()
                .unwrap_or_else(|| Spellings::of(arg).name())
        } else {
            continue;
        };
        let key = name.to_lowercase();
        match completions.iter_mut().find(|(seen, _)| *seen == key) {
            Some(completion) => completion.1 = hint,
            None => completions.push((key, hint)),
        }
    }
    completions
        .into_iter()
        .map(|(name, hint)| {
            let mut node = text_line("complete", name);
            node.prop("type", Value::Text(hint.to_owned()));
            node
        })
        .collect()
}

/// The spec's completion type for a clap value hint. clap marks
/// `ValueHint` non-exhaustive; a hint added later gets no completion.
#[expect(
    clippy::match_same_arms,
    reason = "Unknown and a hint clap adds later both get no completion"
)]
fn hint_type(hint: ValueHint) -> Option<&'static str> {
    match hint {
        ValueHint::Unknown => None,
        ValueHint::Other => Some("none"),
        ValueHint::AnyPath | ValueHint::FilePath => Some("path"),
        ValueHint::DirPath => Some("dir"),
        ValueHint::ExecutablePath => Some("executable"),
        ValueHint::CommandName | ValueHint::CommandString => Some("command"),
        ValueHint::CommandWithArguments => Some("command_args"),
        ValueHint::Username => Some("username"),
        ValueHint::Hostname => Some("hostname"),
        ValueHint::Url => Some("url"),
        ValueHint::EmailAddress => Some("email"),
        _ => None,
    }
}

fn word_list(name: &str, words: &[&str]) -> Node {
    let mut node = Node::new(name);
    for word in words {
        node.arg(Value::Word((*word).to_owned()));
    }
    node
}

fn text_list(name: &str, texts: Vec<String>) -> Node {
    let mut node = Node::new(name);
    for text in texts {
        node.arg(Value::Text(text));
    }
    node
}

fn text_line(name: &str, text: String) -> Node {
    let mut node = Node::new(name);
    node.arg(Value::Text(text));
    node
}

#[cfg(test)]
mod tests {
    use super::splits_into_placeholders;

    #[test]
    fn only_a_label_of_enclosed_words_splits() {
        for split in ["<<A> <B>>", "[[A] [B]]", "<A> <B>", "<A> [B]"] {
            assert!(splits_into_placeholders(split), "{split:?} should split");
        }
        for whole in [
            "<X>",
            "[<A> <B>]",
            "<[A] [B]>",
            "<A B>",
            "<-- X>",
            "<x...>",
            "<A>\u{a0}<B>",
        ] {
            assert!(
                !splits_into_placeholders(whole),
                "{whole:?} should not split"
            );
        }
    }
}
