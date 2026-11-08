#!/bin/sh
set -eu
coach_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
coach_test=$(mktemp -d "${TMPDIR:-/tmp}/study-coach-unix-installer-XXXXXXXX")
trap 'rm -rf -- "$coach_test"' EXIT HUP INT TERM
mkdir "$coach_test/package" "$coach_test/data"
for coach_file in start.sh install.sh uninstall.sh README.txt; do cp "$coach_root/packaging/$coach_file" "$coach_test/package/$coach_file"; done
# This inert fixture checks installer behavior only; it is not an application build.
printf '%s\n' '#!/bin/sh' 'exit 0' > "$coach_test/package/study-coach-app"
(
    cd "$coach_test/package"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum README.txt install.sh start.sh study-coach-app uninstall.sh > SHA256SUMS
    else
        shasum -a 256 README.txt install.sh start.sh study-coach-app uninstall.sh > SHA256SUMS
    fi
)
sh "$coach_test/package/install.sh" --prefix "$coach_test/installed" --no-link
[ -f "$coach_test/installed/.study-coach-install" ] || { echo 'Installation marker missing.' >&2; exit 1; }
printf '%s\n' 'preserve this unrelated file' > "$coach_test/installed/keep-user-file.txt"
printf '%s\n' 'preserve this user data' > "$coach_test/data/notes.txt"
sh "$coach_test/installed/start.sh"
printf '%s\n' 'checksum test' >> "$coach_test/package/README.txt"
if sh "$coach_test/package/install.sh" --prefix "$coach_test/must-not-install" --no-link > "$coach_test/rejection.txt" 2>&1; then
    echo 'A damaged package was accepted.' >&2; exit 1
fi
[ ! -e "$coach_test/must-not-install" ] || { echo 'Integrity failure created an installation.' >&2; exit 1; }
sh "$coach_test/installed/uninstall.sh"
[ ! -f "$coach_test/installed/study-coach-app" ] || { echo 'Removal left the application fixture.' >&2; exit 1; }
[ -f "$coach_test/installed/keep-user-file.txt" ] && [ -f "$coach_test/data/notes.txt" ] || { echo 'Removal deleted unrelated data.' >&2; exit 1; }
printf '%s\n' 'Unix installer logic PASS: installation, launcher, checksum rejection, and data preservation.'
printf '%s\n' 'This fixture does not verify a native Linux or macOS application binary.'
