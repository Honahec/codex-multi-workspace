# codex-multi-workspace

[![CI](https://github.com/Honahec/codex-multi-workspace/actions/workflows/release.yml/badge.svg)](https://github.com/Honahec/codex-multi-workspace/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/codex-multi-workspace.svg)](https://crates.io/crates/codex-multi-workspace)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)

Run Codex CLI in Docker with saved single-folder or multi-folder workspaces.

`codex-multi-workspace` provides the `codex-ws` command. It reads Codex provider
profiles from `cc-switch`, mounts one or more project folders into a container,
and keeps each workspace's Codex sessions under `~/.codex-ws`.

## Requirements

- Docker
- Rust/Cargo for installation
- A cc-switch database with at least one Codex provider

The default runtime image is published to GHCR and is based on Ubuntu 22.04. It
includes Codex CLI, Node.js 22, Git, `curl`, and `bubblewrap`. Other language
runtimes can be installed per workspace with `runtime.apt` and `runtime.setup`,
or by selecting a custom Docker image.

## Install

```sh
cargo install codex-multi-workspace
```

## Create a Workspace

```sh
codex-ws workspace add my-workspace
```

This creates and opens:

```text
~/.codex-ws/config/workspace/my-workspace.yaml
```

Example:

```yaml
name: my-workspace
folders:
  - /absolute/path/to/project

# Optional runtime setup for the lightweight Ubuntu image.
# runtime:
#   apt:
#     - python3
#     - python3-pip
#   setup:
#     - python3 -m pip install --user maturin
```

List saved workspaces:

```sh
codex-ws workspace ls
```

## Configure cc-switch

If your cc-switch database is not in the legacy Unix-style location, persist its
path once:

```sh
codex-ws config set cc-switch-db /path/to/cc-switch.db
```

Read it back:

```sh
codex-ws config get cc-switch-db
```

## Run

```sh
codex-ws run \
  --provider OpenAI \
  --workspace my-workspace
```

You can also pass a manifest path directly:

```sh
codex-ws run --provider OpenAI --workspace /path/to/workspace.yaml
```

`--config-db` still overrides the saved `cc-switch-db` value for one run.
Like cc-switch, `codex-ws` resolves `~` from the OS user home directory instead
of trusting the `HOME` environment variable, which avoids common Windows shell
path mismatches.

Workspace folders are mounted under `/workspace` using their original directory
names. A single-folder workspace starts Codex in that project directory. A
multi-folder workspace starts Codex in `/workspace`, with each project available
as `/workspace/<folder-name>`. Folder names must be unique within one workspace.

Codex runs with its internal command sandbox disabled inside the container. The
Docker container is the workspace boundary, and `sandbox.network: false` still
maps to Docker's `--network none`.

## Runtime Image

By default, `codex-ws` uses:

```text
ghcr.io/honahec/codex-multi-workspace:latest
```

Override it when needed:

```sh
codex-ws run --provider OpenAI --workspace my-workspace --image my-codex-runtime:latest
```

Workspace manifests can also request extra packages and setup commands before
Codex starts:

```yaml
runtime:
  apt:
    - python3
    - python3-pip
    - build-essential
  setup:
    - python3 -m pip install --user maturin
```

`runtime.apt` is installed with `apt-get install --no-install-recommends` inside
the container. `runtime.setup` commands run in a login shell immediately before
Codex, so PATH changes or sourced environment files can affect the Codex session.
For heavier stacks, use `runtime.image` in the manifest or `--image`.

**Welcome Stars and PRs.**
