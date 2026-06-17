#!/usr/bin/env bash
set -euo pipefail

export MISE_DATA_DIR="${MISE_DATA_DIR:-/opt/mise}"
export MISE_CACHE_DIR="${MISE_CACHE_DIR:-/var/cache/mise}"
export MISE_CONFIG_DIR="${MISE_CONFIG_DIR:-/root/.config/mise}"
export PATH="/root/.local/bin:${MISE_DATA_DIR}/shims:${PATH}"

if [[ -n "${CODEX_WS_APT_PACKAGES:-}" ]]; then
    apt-get update
    apt-get install -y --no-install-recommends ${CODEX_WS_APT_PACKAGES}
    rm -rf /var/lib/apt/lists/*
fi

install_apt_packages() {
    apt-get update
    apt-get install -y --no-install-recommends "$@"
    rm -rf /var/lib/apt/lists/*
}

install_clang() {
    local version="$1"
    local codename
    codename="$(lsb_release -cs)"
    curl -fsSL https://apt.llvm.org/llvm-snapshot.gpg.key | \
        gpg --dearmor -o /etc/apt/keyrings/llvm-snapshot.gpg
    printf 'deb [signed-by=/etc/apt/keyrings/llvm-snapshot.gpg] https://apt.llvm.org/%s/ llvm-toolchain-%s-%s main\n' \
        "${codename}" "${codename}" "${version}" > /etc/apt/sources.list.d/llvm.list
    install_apt_packages \
        "clang-${version}" \
        "clang-tools-${version}" \
        "lld-${version}" \
        "llvm-${version}"
    ln -sf "/usr/bin/clang-${version}" /usr/local/bin/clang
    ln -sf "/usr/bin/clang++-${version}" /usr/local/bin/clang++
}

if [[ -n "${CODEX_WS_RUBY_VERSION:-}" ]]; then
    mise settings ruby.compile false
fi

if [[ -n "${CODEX_WS_PHP_VERSION:-}" ]]; then
    install_apt_packages \
        autoconf \
        bison \
        build-essential \
        libcurl4-openssl-dev \
        libfreetype6-dev \
        libgd-dev \
        libjpeg-dev \
        libonig-dev \
        libpq-dev \
        libpng-dev \
        libreadline-dev \
        libsqlite3-dev \
        libssl-dev \
        libwebp-dev \
        libxml2-dev \
        libxslt1-dev \
        libzip-dev \
        pkg-config \
        re2c
fi

if [[ -n "${CODEX_WS_DOTNET_VERSION:-}" ]]; then
    install_apt_packages \
        libicu70
fi

CLANG_VERSION="${CODEX_WS_CLANG_VERSION:-${CODEX_WS_C_VERSION:-${CODEX_WS_CPP_VERSION:-}}}"
if [[ -n "${CODEX_WS_C_VERSION:-}" && -n "${CLANG_VERSION}" && "${CODEX_WS_C_VERSION}" != "${CLANG_VERSION}" ]]; then
    printf 'conflicting C/C++ runtime versions: %s and %s\n' "${CLANG_VERSION}" "${CODEX_WS_C_VERSION}" >&2
    exit 2
fi
if [[ -n "${CODEX_WS_CPP_VERSION:-}" && -n "${CLANG_VERSION}" && "${CODEX_WS_CPP_VERSION}" != "${CLANG_VERSION}" ]]; then
    printf 'conflicting C/C++ runtime versions: %s and %s\n' "${CLANG_VERSION}" "${CODEX_WS_CPP_VERSION}" >&2
    exit 2
fi
if [[ -n "${CLANG_VERSION}" ]]; then
    install_clang "${CLANG_VERSION}"
fi

if [[ -n "${CODEX_WS_PYTHON_VERSION:-}" ]]; then
    uv python install "${CODEX_WS_PYTHON_VERSION}"
    UV_PYTHON_BIN="$(uv python find "${CODEX_WS_PYTHON_VERSION}")"
    export PATH="$(dirname "${UV_PYTHON_BIN}"):${PATH}"
fi

MISE_TOOL_ARGS=()
if [[ -n "${CODEX_WS_NODE_VERSION:-}" ]]; then
    MISE_TOOL_ARGS+=("node@${CODEX_WS_NODE_VERSION}")
fi
if [[ -n "${CODEX_WS_GO_VERSION:-}" ]]; then
    MISE_TOOL_ARGS+=("go@${CODEX_WS_GO_VERSION}")
fi
if [[ -n "${CODEX_WS_RUST_VERSION:-}" ]]; then
    MISE_TOOL_ARGS+=("rust@${CODEX_WS_RUST_VERSION}")
fi
if [[ -n "${CODEX_WS_JAVA_VERSION:-}" ]]; then
    MISE_TOOL_ARGS+=("java@${CODEX_WS_JAVA_VERSION}")
fi
if [[ -n "${CODEX_WS_RUBY_VERSION:-}" ]]; then
    MISE_TOOL_ARGS+=("ruby@${CODEX_WS_RUBY_VERSION}")
fi
if [[ -n "${CODEX_WS_PHP_VERSION:-}" ]]; then
    MISE_TOOL_ARGS+=("php@${CODEX_WS_PHP_VERSION}")
fi
if [[ -n "${CODEX_WS_DENO_VERSION:-}" ]]; then
    MISE_TOOL_ARGS+=("deno@${CODEX_WS_DENO_VERSION}")
fi
if [[ -n "${CODEX_WS_BUN_VERSION:-}" ]]; then
    MISE_TOOL_ARGS+=("bun@${CODEX_WS_BUN_VERSION}")
fi
if [[ -n "${CODEX_WS_ZIG_VERSION:-}" ]]; then
    MISE_TOOL_ARGS+=("zig@${CODEX_WS_ZIG_VERSION}")
fi
if [[ -n "${CODEX_WS_DOTNET_VERSION:-}" ]]; then
    MISE_TOOL_ARGS+=("dotnet@${CODEX_WS_DOTNET_VERSION}")
fi
if (( ${#MISE_TOOL_ARGS[@]} > 0 )); then
    mise use --global --yes "${MISE_TOOL_ARGS[@]}"
    mise install --yes
    eval "$(mise activate bash)"
fi

if [[ -n "${CODEX_WS_SETUP_COMMANDS:-}" ]]; then
    CODEX_WS_SETUP_SCRIPT="$(mktemp)"
    export CODEX_WS_SETUP_SCRIPT
    {
        printf 'set -euo pipefail\n'
        printf '%s\n' "${CODEX_WS_SETUP_COMMANDS}"
    } > "${CODEX_WS_SETUP_SCRIPT}"
fi

exec bash --login -c 'export PATH="/root/.local/bin:${MISE_DATA_DIR:-/opt/mise}/shims:${PATH}"; if command -v mise >/dev/null 2>&1; then eval "$(mise activate bash)"; fi; if [[ -n "${CODEX_WS_SETUP_SCRIPT:-}" ]]; then source "${CODEX_WS_SETUP_SCRIPT}"; fi; exec /usr/local/bin/codex "$@"' bash "$@"
