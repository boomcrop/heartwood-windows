#!/bin/sh
# Fails when any file under the given paths names the tools the game was made with: the website's rule, whose
# list is kept in one place, ../heartwood-deploy/build.sh (NAMES=...). The Firebase libraries in web/lib are
# left out, as there: they're Google's code.
# check_names.sh PATH...   (an installer or other binary: check_names.sh --binary FILE...)
HERE=$(cd "$(dirname "$0")/.." && pwd)
NAMES=$(sed -n "s/^NAMES='\(.*\)'\$/\1/p" "$HERE/../heartwood-deploy/build.sh")
if [ -z "$NAMES" ]; then echo "stopped: no NAMES= line in ../heartwood-deploy/build.sh" >&2; exit 1; fi
if [ "$1" = "--binary" ]; then
  shift
  # (only whole names in a binary: short words turn up in compressed data by chance)
  STRONG=$(echo "$NAMES" | tr '|' '\n' | grep -v -e '\\b' -e '^artifact$' | paste -sd'|' -)
  if grep -aliE "$STRONG" "$@"; then echo "stopped: the files above name tools they shouldn't" >&2; exit 1; fi
  exit 0
fi
if grep -rliE --exclude-dir=lib "$NAMES" "$@"; then echo "stopped: the files above name tools they shouldn't" >&2; exit 1; fi
