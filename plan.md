# Codex Workspace Project Plan

Last Updated: 2026-06-11

## Project Goal

Build `codex-ws`, a Rust CLI that launches a Codex CLI sandbox for a selected
configuration and workspace. A workspace may contain one project folder or multiple
folders, while conversation sessions remain host-managed and are mounted into the
sandbox at launch time.

## Completed

- [x] Captured the initial product direction.
- [x] Identified the Rust CLI target: `codex-ws`.
- [x] Identified the need to reuse existing Codex provider configuration.
- [x] Identified the workspace manifest requirements.
- [x] Identified Docker sandbox launch as the runtime boundary.
- [x] Identified host-managed session routing for workspace history.
- [x] Created this live, trackable project plan.
- [x] Initialized the Rust CLI crate structure for `codex-ws`.
- [x] Defined the initial CLI command surface for selecting a provider configuration and
  workspace.
- [x] Added structured errors with `thiserror` for provider loading failures.
- [x] Added application-level error handling with `anyhow`.
- [x] Implemented reading Codex provider rows from `~/.cc-switch/cc-switch.db`.
- [x] Implemented provider filtering for `app_type == "codex"`.
- [x] Mapped provider fields into internal Rust types:
  - `name`
  - `settings_config.auth`
  - `settings_config.config`
- [x] Added unit tests for provider filtering and mapping.
- [x] Defined the workspace manifest schema.
- [x] Implemented parsing workspace manifests from YAML.
- [x] Supported single-folder and multi-folder workspaces in manifests.
- [x] Implemented sandbox network configuration parsing.
- [x] Added unit tests for manifest parsing.
- [x] Implemented workspace folder path validation before launch.
- [x] Added unit tests for workspace folder validation.
- [x] Implemented Docker sandbox launch command construction with a default Codex CLI image.
- [x] Added provider auth and config mounts for the sandbox.
- [x] Added workspace folder mounts for the sandbox.
- [x] Added host-managed workspace session mount routing.
- [x] Added integration-style tests for Docker launch command construction.
- [x] Wired the CLI to load providers, parse manifests, validate folders, create session
  directories, and execute Docker.
- [x] Fixed provider loading for the real `settings_config` column shape.
- [x] Added generated host config files for provider auth JSON and config TOML before Docker launch.
- [x] Ran `cargo fmt --check`.
- [x] Ran `cargo clippy -- -D warnings`.
- [x] Ran `cargo test`.
- [x] Ran `cargo build`.

## Pending

No pending items.

## Update Rules

- Update this file immediately after each meaningful implementation step.
- Move finished work from `Pending` to `Completed`.
- Add newly discovered work to `Pending`.
- Keep every item actionable and verifiable.
- Keep this file in English.
- Keep the `Last Updated` field current with an explicit date.

## Acceptance Criteria

- `codex-ws` can select a Codex provider configuration.
- `codex-ws` can load and validate a workspace manifest.
- `codex-ws` can launch a Docker sandbox with the selected configuration.
- The sandbox receives the intended workspace folders.
- The sandbox reads and writes sessions through the host-managed workspace session path.
- Formatting, linting, tests, and build checks pass without warnings.
