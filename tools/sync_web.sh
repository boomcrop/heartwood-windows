#!/bin/sh
# Builds the website with ../heartwood-deploy/build.sh (the one place the game's files are put together) and
# copies the result in as the app's game, without the website's own pages. Then checks that nothing the app
# ships names the tools it was made with, the same rule build.sh keeps for the website.
# Run it before every build that should carry game changes (the release workflow runs it too).
set -e
HERE=$(cd "$(dirname "$0")/.." && pwd)
DEPLOY="$HERE/../heartwood-deploy"
sh "$DEPLOY/build.sh"
rm -rf "$HERE/web"
cp -R "$DEPLOY/public" "$HERE/web"
# the website's app-link file is for Apple's servers; the moderation, download and sign-in pages are the website's
rm -rf "$HERE/web/.well-known"
rm -f "$HERE/web/admin.html" "$HERE/web/download.html" "$HERE/web/desktop-signin.html"
sh "$HERE/tools/check_names.sh" "$HERE/web" "$HERE/src-tauri/src"
echo "copied the game into $HERE/web ($(find "$HERE/web" -type f | wc -l | tr -d ' ') files)"
