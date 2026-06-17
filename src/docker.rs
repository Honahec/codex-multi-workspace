use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use directories::BaseDirs;
use thiserror::Error;

use crate::config::default_state_root;
use crate::manifest::WorkspaceManifest;

const CONTAINER_CODEX_DIR: &str = "/root/.codex";
const CONTAINER_SESSIONS_DIR: &str = "/root/.codex/sessions";
const CONTAINER_SKILLS_DIR: &str = "/root/.codex/skills";
/// Container directory under which workspace folders are mounted.
pub const CONTAINER_WORKSPACE_ROOT: &str = "/workspace";

const CODEX_SANDBOX_MODE: &str = "danger-full-access";

/// Default Codex CLI Docker image used for sandbox launches.
pub const DEFAULT_CODEX_IMAGE: &str = "ghcr.io/honahec/codex-multi-workspace:latest";

/// Version label expected on the locally built Codex workspace image.
pub const DEFAULT_CODEX_IMAGE_VERSION: &str = "7";

/// Runtime paths and image settings used to construct a Docker sandbox command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerLaunchConfig {
    image: String,
    sessions_root: PathBuf,
    skills_path: PathBuf,
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
            skills_path: default_skills_path_from_home()
                .unwrap_or_else(|| PathBuf::from(".agents/skills")),
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
            skills_path: self.skills_path.clone(),
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

    /// Return the host skills directory.
    ///
    /// # Returns
    ///
    /// Host directory mounted read-only as `/root/.codex/skills`.
    #[must_use]
    pub fn skills_path(&self) -> &Path {
        &self.skills_path
    }

    /// Return a copy of this configuration with a different host skills directory.
    ///
    /// # Arguments
    ///
    /// * `skills_path` - Host directory containing Codex skills.
    ///
    /// # Returns
    ///
    /// A Docker launch configuration with the same image and sessions root.
    #[must_use]
    pub fn with_skills_path(&self, skills_path: PathBuf) -> Self {
        Self {
            image: self.image.clone(),
            sessions_root: self.sessions_root.clone(),
            skills_path,
        }
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
}

impl Default for DockerLaunchConfig {
    fn default() -> Self {
        let sessions_root = default_state_root().unwrap_or_else(|_| PathBuf::from(".codex-ws"));
        Self::new(DEFAULT_CODEX_IMAGE.to_owned(), sessions_root)
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

    /// A workspace folder path cannot be represented as a stable container directory name.
    #[error("workspace folder '{path}' does not have a usable directory name")]
    InvalidWorkspaceFolderName {
        /// Workspace folder path from the manifest.
        path: PathBuf,
    },

    /// Two workspace folders would mount to the same container path.
    #[error("multiple workspace folders are named '{folder_name}'")]
    DuplicateWorkspaceFolderName {
        /// Directory name shared by multiple workspace folders.
        folder_name: String,
    },
}

/// Provider configuration files written on the host before launching Docker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfigFiles {
    auth_path: PathBuf,
    config_path: PathBuf,
}

impl ProviderConfigFiles {
    /// Create provider configuration file paths.
    ///
    /// # Arguments
    ///
    /// * `auth_path` - Host path to the generated Codex auth JSON file.
    /// * `config_path` - Host path to the generated Codex config TOML file.
    ///
    /// # Returns
    ///
    /// Provider configuration file paths used for Docker.
    #[must_use]
    pub fn new(auth_path: PathBuf, config_path: PathBuf) -> Self {
        Self {
            auth_path,
            config_path,
        }
    }

    /// Return the host auth JSON path.
    ///
    /// # Returns
    ///
    /// Host path to the generated Codex auth JSON file.
    #[must_use]
    pub fn auth_path(&self) -> &Path {
        &self.auth_path
    }

    /// Return the host config TOML path.
    ///
    /// # Returns
    ///
    /// Host path to the generated Codex config TOML file.
    #[must_use]
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }
}

/// Build a Docker command for launching a Codex workspace sandbox.
///
/// # Arguments
///
/// * `provider_files` - Generated provider configuration files mounted into the sandbox.
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
    provider_files: &ProviderConfigFiles,
    manifest: &WorkspaceManifest,
    launch_config: &DockerLaunchConfig,
) -> Result<Command, DockerError> {
    let args = docker_run_args(provider_files, manifest, launch_config)?;
    let mut command = Command::new("docker");
    command.args(args);
    Ok(command)
}

/// Return the container mount target for each workspace folder.
///
/// # Arguments
///
/// * `manifest` - Workspace manifest whose folders should be mounted.
///
/// # Returns
///
/// Container paths under [`CONTAINER_WORKSPACE_ROOT`] using each folder's directory name.
///
/// # Errors
///
/// Returns [`DockerError::InvalidWorkspaceFolderName`] when a folder has no usable final path
/// component, or [`DockerError::DuplicateWorkspaceFolderName`] when two folders share a name.
pub fn workspace_mount_targets(manifest: &WorkspaceManifest) -> Result<Vec<String>, DockerError> {
    let mut seen_names = HashSet::with_capacity(manifest.folders().len());
    let mut targets = Vec::with_capacity(manifest.folders().len());

    for folder in manifest.folders() {
        let folder_name = workspace_folder_name(folder)?;
        if !seen_names.insert(folder_name.to_owned()) {
            return Err(DockerError::DuplicateWorkspaceFolderName {
                folder_name: folder_name.to_owned(),
            });
        }
        targets.push(format!("{CONTAINER_WORKSPACE_ROOT}/{folder_name}"));
    }

    Ok(targets)
}

fn docker_run_args(
    provider_files: &ProviderConfigFiles,
    manifest: &WorkspaceManifest,
    launch_config: &DockerLaunchConfig,
) -> Result<Vec<String>, DockerError> {
    if manifest.folders().is_empty() {
        return Err(DockerError::NoWorkspaceFolders {
            workspace_name: manifest.name().to_owned(),
        });
    }
    let mount_targets = workspace_mount_targets(manifest)?;

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

    args.extend(volume_args(
        provider_files.auth_path(),
        &format!("{CONTAINER_CODEX_DIR}/auth.json"),
        true,
    ));
    args.extend(volume_args(
        provider_files.config_path(),
        &format!("{CONTAINER_CODEX_DIR}/config.toml"),
        false,
    ));
    let sessions_path = launch_config.workspace_sessions_path(manifest.name());
    args.extend(volume_args(&sessions_path, CONTAINER_SESSIONS_DIR, false));
    if launch_config.skills_path().is_dir() {
        args.extend(volume_args(
            launch_config.skills_path(),
            CONTAINER_SKILLS_DIR,
            true,
        ));
    }

    for (folder, target) in manifest.folders().iter().zip(&mount_targets) {
        args.extend(volume_args(folder, target, false));
    }

    args.push("--workdir".to_owned());
    args.push(workdir_for_mount_targets(&mount_targets).to_owned());
    args.push(launch_config.image().to_owned());
    args.extend(["--sandbox".to_owned(), CODEX_SANDBOX_MODE.to_owned()]);

    Ok(args)
}

fn workspace_folder_name(folder: &Path) -> Result<&str, DockerError> {
    let Some(name) = folder.file_name().and_then(|name| name.to_str()) else {
        return Err(DockerError::InvalidWorkspaceFolderName {
            path: folder.to_path_buf(),
        });
    };
    if name.is_empty() || name == "." || name == ".." {
        return Err(DockerError::InvalidWorkspaceFolderName {
            path: folder.to_path_buf(),
        });
    }

    Ok(name)
}

fn workdir_for_mount_targets(mount_targets: &[String]) -> &str {
    if let [target] = mount_targets {
        return target;
    }

    CONTAINER_WORKSPACE_ROOT
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

fn default_skills_path_from_home() -> Option<PathBuf> {
    BaseDirs::new().map(|dirs| dirs.home_dir().join(".agents").join("skills"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::manifest::{RuntimeConfig, SandboxConfig};
    use crate::runtime::{RuntimeTool, RuntimeToolVersion};

    static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn test_provider_files() -> ProviderConfigFiles {
        ProviderConfigFiles::new(
            PathBuf::from("/tmp/codex-ws-provider/auth.json"),
            PathBuf::from("/tmp/codex-ws-provider/config.toml"),
        )
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

    fn test_launch_config(skills_path: PathBuf) -> DockerLaunchConfig {
        DockerLaunchConfig::new("codex-ws:test".to_owned(), PathBuf::from("/host/.codex-ws"))
            .with_skills_path(skills_path)
    }

    #[test]
    fn docker_run_args_mounts_provider_workspace_and_sessions() {
        let temp_dir = TestTempDir::create();
        let skills_path = temp_dir.path().join("skills");
        fs::create_dir(&skills_path).expect("skills directory should be created");
        let args = docker_run_args(
            &test_provider_files(),
            &test_manifest(false),
            &test_launch_config(skills_path.clone()),
        )
        .expect("docker args should build");
        let skills_mount = format!("{}:/root/.codex/skills:ro", skills_path.display());

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
                "/tmp/codex-ws-provider/auth.json:/root/.codex/auth.json:ro",
                "-v",
                "/tmp/codex-ws-provider/config.toml:/root/.codex/config.toml",
                "-v",
                "/host/.codex-ws/workspace-name/sessions:/root/.codex/sessions",
                "-v",
                &skills_mount,
                "-v",
                "/projects/backend:/workspace/backend",
                "-v",
                "/projects/frontend:/workspace/frontend",
                "--workdir",
                "/workspace",
                "codex-ws:test",
                "--sandbox",
                "danger-full-access",
            ]
        );
    }

    #[test]
    fn docker_run_args_uses_single_workspace_folder_as_workdir() {
        let manifest = WorkspaceManifest::new(
            "workspace-name".to_owned(),
            vec![PathBuf::from("/projects/backend")],
            SandboxConfig::default(),
        )
        .expect("manifest should be valid");

        let args = docker_run_args(
            &test_provider_files(),
            &manifest,
            &test_launch_config(PathBuf::from("/missing/skills")),
        )
        .expect("docker args should build");

        assert!(
            args.windows(2)
                .any(|window| window == ["--workdir", "/workspace/backend"])
        );
    }

    #[test]
    fn docker_run_args_rejects_duplicate_workspace_folder_names() {
        let manifest = WorkspaceManifest::new(
            "workspace-name".to_owned(),
            vec![
                PathBuf::from("/projects/backend"),
                PathBuf::from("/other/backend"),
            ],
            SandboxConfig::default(),
        )
        .expect("manifest should be valid");

        let error = docker_run_args(
            &test_provider_files(),
            &manifest,
            &test_launch_config(PathBuf::from("/missing/skills")),
        )
        .expect_err("duplicate workspace folder names should fail")
        .to_string();

        assert_eq!(error, "multiple workspace folders are named 'backend'");
    }

    #[test]
    fn docker_run_args_omits_network_none_when_network_is_enabled() {
        let args = docker_run_args(
            &test_provider_files(),
            &test_manifest(true),
            &test_launch_config(PathBuf::from("/missing/skills")),
        )
        .expect("docker args should build");

        assert!(!args.iter().any(|arg| arg == "--network"));
        assert!(!args.iter().any(|arg| arg == "none"));
    }

    #[test]
    fn docker_run_args_passes_runtime_environment_variables() {
        let manifest = WorkspaceManifest::with_runtime(
            "workspace-name".to_owned(),
            vec![PathBuf::from("/projects/backend")],
            SandboxConfig::default(),
            RuntimeConfig::with_setup(
                None,
                vec![RuntimeToolVersion::new(
                    RuntimeTool::Python,
                    "3.13".to_owned(),
                )],
                vec!["python3".to_owned(), "python3-pip".to_owned()],
                vec!["python3 -m pip install maturin".to_owned()],
            ),
        )
        .expect("manifest should be valid");

        let args = docker_run_args(
            &test_provider_files(),
            &manifest,
            &test_launch_config(PathBuf::from("/missing/skills")),
        )
        .expect("docker args should build");

        assert!(
            args.windows(2)
                .any(|window| window == ["-e", "CODEX_WS_PYTHON_VERSION=3.13"])
        );
        assert!(
            args.windows(2)
                .any(|window| window == ["-e", "CODEX_WS_APT_PACKAGES=python3 python3-pip"])
        );
        assert!(args.windows(2).any(|window| {
            window
                == [
                    "-e",
                    "CODEX_WS_SETUP_COMMANDS=python3 -m pip install maturin",
                ]
        }));
    }

    #[test]
    fn docker_run_args_skips_missing_skills_directory() {
        let args = docker_run_args(
            &test_provider_files(),
            &test_manifest(false),
            &test_launch_config(PathBuf::from("/missing/skills")),
        )
        .expect("docker args should build");

        assert!(!args.iter().any(|arg| arg.contains("/root/.codex/skills")));
    }

    #[test]
    fn container_name_replaces_unsupported_characters() {
        assert_eq!(
            container_name("my workspace/main"),
            "codex-ws-my-workspace-main"
        );
    }

    #[derive(Debug)]
    struct TestTempDir {
        path: PathBuf,
    }

    impl TestTempDir {
        fn create() -> Self {
            let counter = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "codex-ws-docker-test-{}-{timestamp}-{counter}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("temporary test directory should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestTempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
