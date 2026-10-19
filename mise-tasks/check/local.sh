#!/usr/bin/env bash
#MISE description="This repository's own checks"
#MISE depends=["check:rust-tasks"]
# The repository's own checks. check:repo runs check:local after the shared
# checks, so a repository replaces this body, or adds #MISE depends, with its
# own checks. A starter: the template writes this file once and never rewrites it.
true
