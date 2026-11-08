#!/bin/sh
set -eu
coach_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$coach_directory/study-coach-app" "$@"
