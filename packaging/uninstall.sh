#!/bin/sh
set -eu
coach_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
[ -f "$coach_directory/.study-coach-install" ] && [ "$(cat "$coach_directory/.study-coach-install")" = 'study-coach-rust:1' ] || {
    echo 'No recognized installation marker. Remove a portable package manually.' >&2; exit 1
}
coach_command=${HOME:?HOME is required}/.local/bin/study-coach
if [ -L "$coach_command" ] && [ "$(readlink "$coach_command")" = "$coach_directory/study-coach-app" ]; then
    rm -- "$coach_command"
fi
for coach_name in study-coach-app start.sh install.sh uninstall.sh README.txt SHA256SUMS .study-coach-install; do
    rm -f -- "$coach_directory/$coach_name"
done
rmdir -- "$coach_directory" 2>/dev/null || true
printf '%s\n' 'Study Coach program files removed. Your notes, history, settings, and API keys were preserved in the application data folder.'
