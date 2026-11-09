//! `MiniJinja` rendering for a CLI's generated documentation files.

use minijinja::{Environment, Error, context};
use serde::Serialize;

use super::Surface;

/// Template source for `ctl/version.md.jinja`: the macro
/// `version_line(version)` renders the `version:` line of a generated file's
/// frontmatter.
pub const VERSION_FRAGMENT: &str = r"{% macro version_line(version) -%}
version: {{ version }}
{%- endmacro %}";

/// Template source for `ctl/invocation.md.jinja`: the macro
/// `mounted_invocation(surface, examples)` renders example `mise run` lines
/// and the rule that they take no `--`.
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

/// Template source for `ctl/commands.md.jinja`: the macro
/// `command_inventory(surface)` renders a table of the visible top-level
/// commands, with their visible aliases and descriptions.
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

/// Add ctl-core's shared fragments to `environment`, under the names
/// `ctl/version.md.jinja`, `ctl/invocation.md.jinja`, and
/// `ctl/commands.md.jinja`. A template imports them from there.
///
/// # Errors
///
/// Returns minijinja's [`Error`] when a fragment does not compile under
/// `environment`'s syntax.
pub fn add_fragments(environment: &mut Environment<'static>) -> Result<(), Error> {
    for (name, source) in FRAGMENTS {
        environment.add_template(name, source)?;
    }
    Ok(())
}

/// Create an [`Environment`] that holds the shared fragments.
///
/// It is strict: an undefined variable is an error, not an empty string. It
/// keeps the trailing newline of a template.
///
/// # Errors
///
/// Returns the [`Error`] from [`add_fragments`].
pub fn environment() -> Result<Environment<'static>, Error> {
    let mut environment = Environment::new();
    environment.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
    environment.set_keep_trailing_newline(true);
    add_fragments(&mut environment)?;
    Ok(environment)
}

/// Render the template `source`, named `name`, in a fresh
/// [`environment`].
///
/// The template sees two variables: `surface`, the [`Surface`], and `content`,
/// the CLI's own serializable context.
///
/// # Errors
///
/// Returns minijinja's [`Error`] when `source` does not compile, or when
/// rendering fails, such as on a variable the strict environment does not
/// define.
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
