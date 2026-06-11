#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${CODEX_WS_APT_PACKAGES:-}" ]]; then
    apt-get update
    apt-get install -y --no-install-recommends ${CODEX_WS_APT_PACKAGES}
    rm -rf /var/lib/apt/lists/*
fi

if [[ -n "${CODEX_WS_SETUP_COMMANDS:-}" ]]; then
    CODEX_WS_SETUP_SCRIPT="$(mktemp)"
    export CODEX_WS_SETUP_SCRIPT
    {
        printf 'set -euo pipefail\n'
        printf '%s\n' "${CODEX_WS_SETUP_COMMANDS}"
    } > "${CODEX_WS_SETUP_SCRIPT}"
fi

exec bash --login -c 'if [[ -n "${CODEX_WS_SETUP_SCRIPT:-}" ]]; then source "${CODEX_WS_SETUP_SCRIPT}"; fi; exec /usr/local/bin/codex "$@"' bash "$@"
