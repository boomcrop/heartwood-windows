#!/bin/sh
# The release build's way to the game: the files the live website serves, by the list build.sh writes there
# (app-files.txt), so the Windows app carries exactly what players get in the browser. Deploy the website
# first. (On the Mac, with the other folders beside this one, sync_web.sh builds it locally instead.)
set -e
HERE=$(cd "$(dirname "$0")/.." && pwd)
SITE=${HW_SITE:-https://heartwood-game.web.app}
rm -rf "$HERE/web"
mkdir -p "$HERE/web"
LIST="$HERE/web/.files"
curl -fsSL --retry 3 "$SITE/app-files.txt" -o "$LIST"
n=0
while IFS= read -r f; do
  case "$f" in '' | /* | *..*) continue ;; esac
  mkdir -p "$HERE/web/$(dirname "$f")"
  curl -fsSL --retry 3 "$SITE/$f" -o "$HERE/web/$f"
  n=$((n + 1))
done < "$LIST"
rm -f "$LIST"
[ "$n" -gt 20 ] || { echo "stopped: only $n files listed at $SITE/app-files.txt" >&2; exit 1; }
grep -q "signIn === 'browser'" "$HERE/web/js/native.js" || { echo "stopped: the live website's game is older than the Windows app; deploy it first" >&2; exit 1; }
grep -q '"apiKey"' "$HERE/web/js/config.js" || { echo "stopped: the live website has no online services' settings" >&2; exit 1; }
sh "$HERE/tools/check_names.sh" "$HERE/web" "$HERE/src-tauri/src"
echo "fetched $n files from $SITE into $HERE/web"
