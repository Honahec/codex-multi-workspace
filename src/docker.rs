use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

use crate::manifest::WorkspaceManifest;

const CONTAINER_CODEX_DIR: &str = "/root/.codex";
const CONTAINER_SESSIONS_DIR: &str = "/root/.codex/sessions";
const CONTAINER_WORKSPACE_ROOT: &str = "/workspace";

/// Default Codex CLI Docker image used for sandbox launches.
pub const DEFAULT_CODEX_IMAGE: &str = "codex-ws:latest";

/// Version label expected on the locally built Codex workspace image.
pub const DEFAULT_CODEX_IMAGE_VERSION: &str = "4";

/// Runtime paths and image settings used to construct a Docker sandbox command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerLaunchConfig {
    image: String,
    sessions_root: PathBuf,
}

impl DockerLaunchConfig {
    /// Create Docker launch configuration.
    ///
    /// # Arguments
    ///
    /// * `image` - Docker image containing the Codex CLI.
    /// * `sessions_root` - Host directory where per-workspace sessions are stored.
    ///
    /// # Returns
    ///
    /// A Docker launch configuration.
    #[must_use]
    pub fn new(image: String, sessions_root: PathBuf) -> Self {
        Self {
            image,
            sessions_root,
        }
    }

    /// Return the Docker image.
    ///
    /// # Returns
    ///
    /// Docker image name used for the sandbox.
    #[must_use]
    pub fn image(&self) -> &str {
        &self.image
    }

    /// Return a copy of this configuration with a different Docker image.
    ///
    /// # Arguments
    ///
    /// * `image` - Docker image that should replace the current image.
    ///
    /// # Returns
    ///
    /// A Docker launch configuration with the same host paths and a new image.
    #[must_use]
    pub fn with_image(&self, image: String) -> Self {
        Self {
            image,
            sessions_root: self.sessions_root.clone(),
        }
    }

    /// Return the sessions root directory.
    ///
    /// # Returns
    ///
    /// Host directory containing per-workspace session directories.
    #[must_use]
    pub fn sessions_root(&self) -> &Path {
        &self.sessions_root
    }

    /// Return the host sessions path for one workspace.
    ///
    /// # Arguments
    ///
    /// * `workspace_name` - Workspace name used as the host session directory key.
    ///
    /// # Returns
    ///
    /// Host path mounted as `/root/.codex/sessions` inside the sandbox.
    #[must_use]
    pub fn workspace_sessions_path(&self, workspace_name: &str) -> PathBuf {
        self.sessions_root().join(workspace_name).join("sessions")
    }

    /// Return the host Codex home path for one workspace.
    ///
    /// # Arguments
    ///
    /// * `workspace_name` - Workspace name used as the host Codex home directory key.
    ///
    /// # Returns
    ///
    /// Host path mounted as `/root/.codex` inside the sandbox.
    #[must_use]
    pub fn workspace_codex_home_path(&self, workspace_name: &str) -> PathBuf {
        self.sessions_root().join(workspace_name).join("codex-home")
    }
}

impl Default for DockerLaunchConfig {
    fn default() -> Self {
        Self::new(
            DEFAULT_CODEX_IMAGE.to_owned(),
            default_sessions_root_from_home().unwrap_or_else(|| PathBuf::from(".codex-ws")),
        )
    }
}

/// Errors returned while constructing Docker launch commands.
#[derive(Debug, Error)]
pub enum DockerError {
    /// The workspace manifest did not contain any folders.
    #[error("workspace '{workspace_name}' does not contain any folders")]
    NoWorkspaceFolders {
        /// Workspace name from the manifest.
        workspace_name: String,
    },
}

/// Codex home directory written on the host before launching Docker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexHome {
    path: PathBuf,
}

impl CodexHome {
    /// Create a Codex home directory mount.
    ///
    /// # Arguments
    ///
    /// * `path` - Host path to the generated writable Codex home directory.
    ///
    /// # Returns
    ///
    /// Codex home directory mount used for Docker.
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Return the host Codex home path.
    ///
    /// # Returns
    ///
    /// Host path to the generated writable Codex home directory.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Build a Docker command for launching a Codex workspace sandbox.
///
/// # Arguments
///
/// * `codex_home` - Generated writable Codex home directory mounted into the sandbox.
/// * `manifest` - Validated workspace manifest.
/// * `launch_config` - Docker image and host path settings.
///
/// # Returns
///
/// A `docker run` command with provider, workspace, and session mounts.
///
/// # Errors
///
/// Returns [`DockerError::NoWorkspaceFolders`] when the manifest has no folders.
pub fn build_docker_run_command(
    codex_home: &CodexHome,
    manifest: &WorkspaceManifest,
    launch_config: &DockerLaunchConfig,
) -> Result<Command, DockerError> {
    let args = docker_run_args(codex_home, manifest, launch_config)?;
    let mut command = Command::new("docker");
    command.args(args);
    Ok(command)
}

fn docker_run_args(
    codex_home: &CodexHome,
    manifest: &WorkspaceManifest,
    launch_config: &DockerLaunchConfig,
) -> Result<Vec<String>, DockerError> {
    if manifest.folders().is_empty() {
        return Err(DockerError::NoWorkspaceFolders {
            workspace_name: manifest.name().to_owned(),
        });
    }

    let mut args = vec![
        "run".to_owned(),
        "--rm".to_owned(),
        "-it".to_owned(),
        "--name".to_owned(),
        container_name(manifest.name()),
    ];

    if !manifest.sandbox().network() {
        args.extend(["--network".to_owned(), "none".to_owned()]);
    }

    for variable in manifest.runtime().environment_variables() {
        args.extend(["-e".to_owned(), variable.docker_assignment()]);
    }

    args.extend(volume_args(codex_home.path(), CONTAINER_CODEX_DIR, false));
    let sessions_path = launch_config.workspace_sessions_path(manifest.name());
    args.extend(volume_args(&sessions_path, CONTAINER_SESSIONS_DIR, false));

    for (index, folder) in manifest.folders().iter().enumerate() {
        let target = format!("{CONTAINER_WORKSPACE_ROOT}/{}", index + 1);
        args.extend(volume_args(folder, &target, false));
    }

    args.push("--workdir".to_owned());
    args.push(format!("{CONTAINER_WORKSPACE_ROOT}/1"));
    args.push(launch_config.image().to_owned());

    Ok(args)
}

fn volume_args(source: &Path, target: &str, read_only: bool) -> [String; 2] {
    let mode = if read_only { ":ro" } else { "" };
    [
        "-v".to_owned(),
        format!("{}:{target}{mode}", source.display()),
    ]
}

fn container_name(workspace_name: &str) -> String {
    let mut name = String::with_capacity("codex-ws-".len() + workspace_name.len());
    name.push_str("codex-ws-");
    for character in workspace_name.chars() {
        if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
            name.push(character);
        } else {
            name.push('-');
        }
    }
    name
}

fn default_sessions_root_from_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex-ws"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{RuntimeConfig, SandboxConfig};
    use crate::runtime::RuntimeLanguageVersion;

    fn test_codex_home() -> CodexHome {
        CodexHome::new(PathBuf::from("/host/.codex-ws/workspace-name/codex-home"))
    }

    fn test_manifest(network: bool) -> WorkspaceManifest {
        WorkspaceManifest::new(
            "workspace-name".to_owned(),
            vec![
                PathBuf::from("/projects/backend"),
                PathBuf::from("/projects/frontend"),
            ],
            SandboxConfig::new(network),
        )
        .expect("manifest should be valid")
    }

    fn test_launch_config() -> DockerLaunchConfig {
        DockerLaunchConfig::new("codex-ws:test".to_owned(), PathBuf::from("/host/.codex-ws"))
    }

    #[test]
    fn docker_run_args_mounts_provider_workspace_and_sessions() {
        let args = docker_run_args(
            &test_codex_home(),
            &test_manifest(false),
            &test_launch_config(),
        )
        .expect("docker args should build");

        assert_eq!(
            args,
            vec![
                "run",
                "--rm",
                "-it",
                "--name",
                "codex-ws-workspace-name",
                "--network",
                "none",
                "-v",
                "/host/.codex-ws/workspace-name/codex-home:/root/.codex",
                "-v",
                "/host/.codex-ws/workspace-name/sessions:/root/.codex/sessions",
                "-v",
                "/projects/backend:/workspace/1",
                "-v",
                "/projects/frontend:/workspace/2",
                "--workdir",
                "/workspace/1",
                "codex-ws:test",
            ]
        );
    }

    #[test]
    fn docker_run_args_omits_network_none_when_network_is_enabled() {
        let args = docker_run_args(
            &test_codex_home(),
            &test_manifest(true),
            &test_launch_config(),
        )
        .expect("docker args should build");

        assert!(!args.iter().any(|arg| arg == "--network"));
        assert!(!args.iter().any(|arg| arg == "none"));
    }

    #[test]
    fn docker_run_args_passes_runtime_environment_variables() {
        let runtime =
            RuntimeLanguageVersion::parse("golang:1.25.1").expect("runtime spec should parse");
        let manifest = WorkspaceManifest::with_runtime(
            "workspace-name".to_owned(),
            vec![PathBuf::from("/projects/backend")],
            SandboxConfig::default(),
            RuntimeConfig::with_language_versions(None, vec![runtime]),
        )
        .expect("manifest should be valid");

        let args = docker_run_args(&test_codex_home(), &manifest, &test_launch_config())
            .expect("docker args should build");

        assert!(
            args.windows(2)
                .any(|window| window == ["-e", "CODEX_ENV_GO_VERSION=1.25.1"])
        );
    }

    #[test]
    fn container_name_replaces_unsupported_characters() {
        assert_eq!(
            container_name("my workspace/main"),
            "codex-ws-my-workspace-main"
        );
    }
}
