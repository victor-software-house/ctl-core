# Design

## Decisions

| # | Decision | Alternative | Why the alternative lost |
|--:|:--|:--|:--|
| 1 | The owner's setter beats the environment, which beats the default. | The environment beats the owner. | A CLI that needs compact JSON for a protocol would break under a stray shell variable. The buffer already uses this order. |
| 2 | Values are the kebab-case variant names: `keys-right`, `header-rule`, `pretty-on-terminal`. | Numbers, or the Rust names. | Kebab-case matches the rest of the command line, and a number means nothing to a reader. |
| 3 | An unaccepted value is ignored, and `App` warns once per variable at startup. | Fail the command. | A typo in a shell profile would break every command. The renderer has no error path either. |
| 4 | The three table options resolve inside `RenderOptions`; `View` and `App` reach them through `styles`. | A setter per option on `View` and `App`. | `styles` already carries the options, so a second set of setters would duplicate it. |

## Overrides and disable paths

| Option | Variable | Owner override | Disable lookup |
|:--|:--|:--|:--|
| Record style | `CTL_CORE_RECORD_STYLE` | `record_style` | `record_style_envs(&[])` |
| List style | `CTL_CORE_LIST_STYLE` | `list_style` | `list_style_envs(&[])` |
| Row separation | `CTL_CORE_ROW_SEPARATION` | `row_separation` | `row_separation_envs(&[])` |
| JSON layout | `CTL_CORE_JSON_LAYOUT` | `json_layout` | `json_layout_envs(&[])` |

`App` and `View` pass the record, list, and separation names through
`styles(RenderOptions)`. `View::json_layout_envs` and
`App::json_layout_envs` carry the JSON names.
