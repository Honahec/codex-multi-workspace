# Codex Workspace Project Plan

Last Updated: 2026-06-11

## Project Goal

Build `codex-ws`, a Rust CLI that launches a pinned Codex CLI sandbox for a selected
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
  - `settings.config.auth`
  - `settings.config.config`
- [x] Added unit tests for provider filtering and mapping.

## Pending

- [ ] Define the workspace manifest schema.
- [ ] Parse workspace manifests from YAML.
- [ ] Support single-folder and multi-folder workspaces.
- [ ] Validate workspace folder paths before launch.
- [ ] Implement sandbox configuration options, including network access.
- [ ] Start Docker with a pinned Codex CLI version.
- [ ] Pass selected auth and config files into the sandbox.
- [ ] Mount workspace folders into the sandbox.
- [ ] Mount workspace sessions from the host-managed session directory.
- [ ] Route sessions using `~/.codex-ws/<workspace-name>/sessions`.
- [ ] Add unit tests for manifest parsing.
- [ ] Add integration tests for launch command construction.
- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo clippy -- -D warnings`.
- [ ] Run `cargo test`.
- [ ] Run `cargo build`.

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
