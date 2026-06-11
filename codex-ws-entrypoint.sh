#!/usr/bin/env bash
set -euo pipefail

/opt/codex/setup_universal.sh
exec bash --login -c 'exec /usr/local/bin/codex "$@"' bash "$@"
