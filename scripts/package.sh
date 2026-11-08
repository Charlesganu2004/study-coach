#!/bin/sh
set -eu
coach_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
coach_output=${1:-$coach_root/dist}
case "$coach_output" in /*) ;; *) coach_output=$PWD/$coach_output ;; esac
cd "$coach_root"
command -v cargo >/dev/null 2>&1 || { echo 'Cargo is required to build a package. Users of the package do not need Rust.' >&2; exit 1; }
coach_host=$(rustc -vV | sed -n 's/^host: //p')
case "$coach_host" in *-linux-*|*-apple-darwin) ;; *) echo 'Use scripts/package.ps1 for a native Windows package.' >&2; exit 1 ;; esac
coach_version=$(sed -n 's/^version = "\([0-9][0-9.A-Za-z-]*\)"$/\1/p' Cargo.toml | head -n 1)
[ -n "$coach_version" ] || { echo 'Cannot read package version.' >&2; exit 1; }
cargo check --offline --locked --bin study-coach-app
cargo build --offline --locked --release --bin study-coach-app
mkdir -p "$coach_output"
coach_stage=$(mktemp -d "$coach_output/.stage-XXXXXXXX")
coach_name=StudyCoach-$coach_version-$coach_host
mkdir "$coach_stage/$coach_name"
coach_target=${CARGO_TARGET_DIR:-$coach_root/target}
cp "$coach_target/release/study-coach-app" "$coach_stage/$coach_name/study-coach-app"
for coach_file in start.sh install.sh uninstall.sh README.txt; do cp "$coach_root/packaging/$coach_file" "$coach_stage/$coach_name/$coach_file"; done
chmod 755 "$coach_stage/$coach_name/study-coach-app" "$coach_stage/$coach_name/"*.sh
(
    cd "$coach_stage/$coach_name"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum README.txt install.sh start.sh study-coach-app uninstall.sh > SHA256SUMS
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 README.txt install.sh start.sh study-coach-app uninstall.sh > SHA256SUMS
    else
        echo 'sha256sum or shasum is required.' >&2; exit 1
    fi
)
coach_archive=$coach_output/$coach_name.tar.gz
tar -czf "$coach_archive" -C "$coach_stage" "$coach_name"
if command -v sha256sum >/dev/null 2>&1; then
    (cd "$coach_output" && sha256sum "$coach_name.tar.gz") > "$coach_archive.sha256"
else
    (cd "$coach_output" && shasum -a 256 "$coach_name.tar.gz") > "$coach_archive.sha256"
fi
case "$coach_stage" in "$coach_output"/.stage-????????) rm -rf -- "$coach_stage" ;; *) echo 'Staging cleanup boundary check failed.' >&2; exit 1 ;; esac
printf 'Package: %s\n' "$coach_archive"
cat "$coach_archive.sha256"
