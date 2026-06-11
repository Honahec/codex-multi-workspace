//! Library support for the `codex-ws` command-line application.

/// Application orchestration for workspace launches.
pub mod app;

/// Command-line interface definitions.
pub mod cli;

/// User-level codex-ws configuration.
pub mod config;

/// Docker sandbox command construction.
pub mod docker;

/// Workspace manifest parsing and validation.
pub mod manifest;

/// Provider configuration loading from the local configuration database.
pub mod provider;

/// Workspace runtime setup validation.
pub mod runtime;

/// Saved workspace manifest management.
pub mod workspace;
