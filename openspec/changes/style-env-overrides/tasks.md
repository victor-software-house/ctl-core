# Tasks

Compile, format, lint, and test tasks run on the build host with `mise run verify`.

## 1. Overrides

- [x] 1.1 Read the record style, list style, and row separation from their variables in `RenderOptions`, after the owner's setter; proof: `the_environment_picks_each_style_the_owner_left_open`
- [x] 1.2 Read the JSON layout from its variable in `View`; proof: `the_environment_picks_the_json_layout_the_owner_left_open`
- [x] 1.3 Replace or disable each name list on `RenderOptions`, `View`, and `App`; proof: `style_env_names_can_be_replaced_or_disabled` and `app_json_layout_envs_reach_every_view`
- [x] 1.4 Ignore an unaccepted value, and have `App` warn with the accepted values; proof: `an_unknown_style_value_is_ignored_and_the_warning_names_the_choices` and `view_warnings_cover_styles_and_the_json_layout`
- [x] 1.5 Run qctl built against this branch with each variable set; proof: a boxed record, compact JSON, and the warning line for `CTL_CORE_LIST_STYLE=boxes`

## 2. Documents

- [x] 2.1 Name each variable in `docs/presentation.md` and `AGENTS.md`, check off task 1.5 of `choose-visual-identity`, and add a patch changeset; proof: `mise run verify` passes
- [ ] 2.2 Close CTC-013 when the pull request merges; proof: `mise run q check` passes
