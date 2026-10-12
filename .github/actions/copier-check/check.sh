#!/usr/bin/env bash
# Fails when the newest template tag would change this repository. A newer
# tag that renders the same files passes with a notice: the repository takes
# the new _commit with its next real template change.
set -euo pipefail
answers=.copier-answers.yml
recorded() { sed -n 's/^_commit: //p' "$answers"; }

current=$(recorded)
# copier update refuses a dirty tree, so whatever it changes is what the
# newest tag brings.
uvx copier@9.18.2 update --quiet --defaults
latest=$(recorded)
changes=$(git status --porcelain --untracked-files=all -- . ":(exclude)$answers")
if [ -n "$changes" ]; then
  echo "::error::Template $latest changes this repository, which records $current. Run copier update and commit the result."
  git status --short -- . ":(exclude)$answers"
  git diff -- . ":(exclude)$answers"
  exit 1
fi
if [ "$latest" = "$current" ]; then
  echo "On the newest template tag, $current."
else
  echo "::notice::Template $latest renders no change here; the repository records $current."
fi
