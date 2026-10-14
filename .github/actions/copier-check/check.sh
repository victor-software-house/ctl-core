#!/usr/bin/env bash
# Fails when the newest template tag would change this repository. A newer
# tag that renders the same files passes with a notice: the repository takes
# the new _commit with its next real template change.
set -euo pipefail
answers=.copier-answers.yml
recorded() { sed -n 's/^_commit: //p' "$answers"; }

current=$(recorded)
src=$(sed -n 's/^_src_path: //p' "$answers")
# The template pins the copier that renders it, in its own mise.toml, so this
# check runs the version each tag was tested with and never pins one itself.
tag=$(git ls-remote --tags --refs "$src" 'v*' | sed 's#.*refs/tags/##' | sort -V | tail -n 1)
template=$(mktemp -d)
trap 'rm -rf "$template"' EXIT
git clone --quiet --depth 1 --branch "$tag" "$src" "$template"
copier=$(sed -n 's/^"pypi:copier" = "\(.*\)"$/\1/p' "$template/mise.toml")
if [ -z "$copier" ]; then
  echo "::error::Template $tag pins no copier in mise.toml."
  exit 1
fi
# copier update refuses a dirty tree, so whatever it changes is what the
# newest tag brings.
uvx "copier@$copier" update --quiet --defaults --vcs-ref "$tag"
latest=$(recorded)
changes=$(git status --porcelain --untracked-files=all -- . ":(exclude)$answers")
if [ -n "$changes" ]; then
  echo "::error::Template $latest changes this repository, which records $current. Run copier update and commit the result."
  git status --short -- . ":(exclude)$answers"
  git diff -- . ":(exclude)$answers"
  exit 1
fi
if [ "$latest" = "$current" ]; then
  echo "On the newest template tag, $current (copier $copier)."
else
  echo "::notice::Template $latest renders no change here; the repository records $current."
fi
