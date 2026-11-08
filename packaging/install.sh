#!/bin/sh
set -eu
coach_source=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
coach_destination=${HOME:?HOME is required}/.local/lib/study-coach
coach_link=1
while [ "$#" -gt 0 ]; do
    case "$1" in
        --prefix) [ "$#" -ge 2 ] || { echo '--prefix requires a directory.' >&2; exit 1; }; coach_destination=$2; shift 2 ;;
        --no-link) coach_link=0; shift ;;
        *) echo 'Usage: sh install.sh [--prefix DIRECTORY] [--no-link]' >&2; exit 1 ;;
    esac
done
case "$coach_destination" in /*) ;; *) echo 'The installation directory must be absolute.' >&2; exit 1 ;; esac
case "$coach_destination" in /|"$HOME"|"$coach_source"|*/../*|*/..|*/./*|*/.) echo 'Choose a separate application folder.' >&2; exit 1 ;; esac
if [ -L "$coach_destination" ]; then echo 'The installation folder cannot be a symbolic link.' >&2; exit 1; fi
if [ -d "$coach_destination" ] && [ -n "$(ls -A "$coach_destination")" ]; then
    if [ ! -f "$coach_destination/.study-coach-install" ] || [ "$(cat "$coach_destination/.study-coach-install")" != 'study-coach-rust:1' ]; then
        echo 'The destination is not empty and is not an existing Study Coach installation.' >&2
        exit 1
    fi
fi
command -v curl >/dev/null 2>&1 || { echo 'curl is required. Install it with your system package manager.' >&2; exit 1; }
coach_seen=' '
coach_count=0
while IFS=' ' read -r coach_hash coach_name; do
    # GNU checksum tools may mark binary files with an asterisk.
    coach_name=${coach_name#\*}
    case "$coach_name" in study-coach-app|start.sh|install.sh|uninstall.sh|README.txt) ;; *) echo 'Unexpected checksum entry.' >&2; exit 1 ;; esac
    case "$coach_seen" in *" $coach_name "*) echo 'Duplicate checksum entry.' >&2; exit 1 ;; esac
    case "$coach_hash" in *[!0-9a-f]*|'') echo 'Invalid checksum.' >&2; exit 1 ;; esac
    [ "${#coach_hash}" -eq 64 ] || { echo 'Invalid checksum length.' >&2; exit 1; }
    [ -f "$coach_source/$coach_name" ] && [ ! -L "$coach_source/$coach_name" ] || { echo 'A package file is missing or is a symbolic link.' >&2; exit 1; }
    if command -v sha256sum >/dev/null 2>&1; then
        coach_actual=$(sha256sum "$coach_source/$coach_name"); coach_actual=${coach_actual%% *}
    elif command -v shasum >/dev/null 2>&1; then
        coach_actual=$(shasum -a 256 "$coach_source/$coach_name"); coach_actual=${coach_actual%% *}
    else
        echo 'sha256sum or shasum is required to verify the package.' >&2; exit 1
    fi
    [ "$coach_actual" = "$coach_hash" ] || { echo "Package checksum mismatch: $coach_name" >&2; exit 1; }
    coach_seen="$coach_seen$coach_name "
    coach_count=$((coach_count + 1))
done < "$coach_source/SHA256SUMS"
[ "$coach_count" -eq 5 ] || { echo 'The package checksum file is incomplete.' >&2; exit 1; }
umask 077
mkdir -p "$coach_destination"
for coach_name in study-coach-app start.sh install.sh uninstall.sh README.txt SHA256SUMS; do
    [ ! -L "$coach_destination/$coach_name" ] || { echo 'An installed file cannot be a symbolic link.' >&2; exit 1; }
    cp "$coach_source/$coach_name" "$coach_destination/$coach_name"
done
chmod 700 "$coach_destination/study-coach-app" "$coach_destination/start.sh" "$coach_destination/install.sh" "$coach_destination/uninstall.sh"
printf '%s\n' 'study-coach-rust:1' > "$coach_destination/.study-coach-install"
if [ "$coach_link" -eq 1 ]; then
    mkdir -p "$HOME/.local/bin"
    coach_command=$HOME/.local/bin/study-coach
    if [ -e "$coach_command" ] || [ -L "$coach_command" ]; then
        if [ ! -L "$coach_command" ] || [ "$(readlink "$coach_command")" != "$coach_destination/study-coach-app" ]; then
            echo 'A different study-coach command exists. Installation is complete; rerun with --no-link to omit the command link.' >&2
            exit 1
        fi
    else
        ln -s "$coach_destination/study-coach-app" "$coach_command"
    fi
fi
printf 'Installed Study Coach in %s\n' "$coach_destination"
printf '%s\n' 'Run the installed start.sh. Your first launch creates local application data and a blank key file.'
