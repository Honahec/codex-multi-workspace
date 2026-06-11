# Codex Workspace Project Plan

Last Updated: 2026-06-11

## Project Goal

Build `codex-ws`, a Rust CLI that launches Codex CLI inside a Docker runtime for
one or more project folders, while reusing local provider configuration and keeping
workspace conversation sessions under a predictable host path.

## Completed

- [x] Created the `codex-ws` Rust CLI with a `run` command for selecting a provider,
  workspace manifest, sessions root, and optional runtime image.
- [x] Loaded Codex provider configuration from the local cc-switch SQLite database using
  the real `settings_config` schema.
- [x] Added workspace manifest parsing for workspace name, folders, sandbox networking,
  optional runtime image, apt packages, and setup commands.
- [x] Validated workspace folders before launching Docker.
- [x] Built Docker launch construction for mounted workspace folders, provider config,
  sessions, optional skills, sandbox networking, runtime setup environment variables, and image
  selection.
- [x] Replaced the Codex Universal default runtime with a lightweight Ubuntu 22.04 image
  containing Codex CLI, Node.js 22, Git, `curl`, and `bubblewrap`.
- [x] Added per-workspace runtime setup through `runtime.apt` and `runtime.setup`, avoiding
  the 40GB Universal image for the default path.
- [x] Switched persistent workspace state to only
  `.codex-ws/<workspace>/sessions`, avoiding persistence of the full container
  `/root/.codex` directory.
- [x] Mounted generated provider auth/config files as run-scoped inputs under the workspace
  sessions root so Docker can access them on hosts with restricted shared paths.
- [x] Mounted host `~/.agents/skills` into the container read-only when the directory
  exists, while allowing startup without skills.
- [x] Added a workspace configuration registry under `~/.codex-ws/config/workspace`.
- [x] Added `workspace ls` for listing saved workspace manifests.
- [x] Added `workspace add <workspace-name>` for creating and editing a templated manifest.
- [x] Allowed `run --workspace <workspace-name>` to resolve saved workspace manifests.
- [x] Defaulted workspace sandbox networking to enabled so Codex CLI can reach the configured
  model provider.
- [x] Prepared crates.io package metadata for `codex-multi-workspace`.
- [x] Added a concise user-facing README covering install, workspace setup, runtime image,
  cc-switch, and lightweight runtime setup.
- [x] Added tag-based GitHub Actions release publishing for GHCR and crates.io.
- [x] Published the initial `codex-multi-workspace` crate release to crates.io.
- [x] Pushed the `v0.1.0` release tag to trigger the GHCR runtime image release.
- [x] Fixed the release workflow crates.io version check to send an explicit User-Agent.
- [x] Added `config set <config-name> <config-value>` and `config get [config-name]`
  for user-level codex-ws configuration under the real-home `~/.codex-ws/config`
  directory, matching cc-switch's home-directory strategy.
- [x] Supported `cc-switch-db` as the first config key and used it as the default
  provider database path when `run --config-db` is not passed.
- [x] Made home-directory expansion work on Windows hosts where `HOME` may be absent
  or injected by Git/MSYS/Cygwin shells.

## Pending

- [ ] Publish the lightweight GHCR runtime image with image version `6`.
