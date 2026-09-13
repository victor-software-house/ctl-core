//! MiniJinja rendering for committed operator documents.

use minijinja::{Environment, Error, context};
use serde::Serialize;

use super::Surface;

/// Shared fragment that renders a skill frontmatter version line.
pub const VERSION_FRAGMENT: &str = r"{% macro version_line(version) -%}
version: {{ version }}
{%- endmacro %}";

/// Shared fragment that renders mounted invocations and the no-`--` rule.
pub const INVOCATION_FRAGMENT: &str = r#"{% macro mounted_invocation(surface, examples) -%}
## Invocation

```sh
{% for example in examples -%}
mise run {{ surface.mount }} {{ example }}
{% endfor -%}
```

Never `mise run {{ surface.mount }} --`. The `--` in `#USAGE mount` is mise's
completion bootstrap.
{%- endmacro %}"#;

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

/// Add ctl-core's shared operator fragments to an existing environment.
pub fn add_fragments(environment: &mut Environment<'static>) -> Result<(), Error> {
    for (name, source) in FRAGMENTS {
        environment.add_template(name, source)?;
    }
    Ok(())
}

/// Create a strict [`Environment`] containing the shared fragments.
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
