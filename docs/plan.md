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
  optional runtime image, and Codex Universal language versions.
- [x] Validated workspace folders before launching Docker.
- [x] Built Docker launch construction for mounted workspace folders, provider config,
  sessions, optional skills, sandbox networking, runtime environment variables, and image
  selection.
- [x] Added the GHCR runtime image based on Codex Universal with Codex CLI,
  `bubblewrap`, and a wrapper entrypoint that runs Universal setup before Codex.
- [x] Supported runtime specs such as `golang:1.25.1` by mapping them to Codex Universal
  `CODEX_ENV_*` variables and validating against the supported version matrix.
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
  cc-switch, and Codex Universal usage.
- [x] Added tag-based GitHub Actions release publishing for GHCR and crates.io.
- [x] Published the initial `codex-multi-workspace` crate release to crates.io.

## Pending

- [ ] Push a release tag to build and publish the default GHCR runtime image.
