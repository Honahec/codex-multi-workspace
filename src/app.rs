use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, ExitStatus};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};

use crate::cli::RunArgs;
use crate::docker::{
    DEFAULT_CODEX_IMAGE, DEFAULT_CODEX_IMAGE_VERSION, DockerLaunchConfig, ProviderConfigFiles,
    build_docker_run_command,
};
use crate::manifest::{WorkspaceManifest, load_workspace_manifest, validate_workspace_folders};
use crate::provider::{CodexProvider, load_codex_providers};
use crate::workspace::{expand_home_path, resolve_workspace_path};

/// Run configuration derived from CLI arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfig {
    provider_name: String,
    workspace_path: PathBuf,
    provider_database_path: PathBuf,
    image_override: Option<String>,
    docker_launch_config: DockerLaunchConfig,
}

impl RunConfig {
    /// Create run configuration.
    ///
    /// # Arguments
    ///
    /// * `provider_name` - Provider name selected by the user.
    /// * `workspace_path` - Path to the workspace manifest YAML file.
    /// * `provider_database_path` - Path to the local provider configuration database.
    /// * `image_override` - Optional CLI-selected image for this launch.
    /// * `docker_launch_config` - Docker image and sessions-root settings.
    ///
    /// # Returns
    ///
    /// A run configuration value.
    #[must_use]
    pub fn new(
        provider_name: String,
        workspace_path: PathBuf,
        provider_database_path: PathBuf,
        image_override: Option<String>,
        docker_launch_config: DockerLaunchConfig,
    ) -> Self {
        Self {
            provider_name,
            workspace_path,
            provider_database_path,
            image_override,
            docker_launch_config,
        }
    }

    /// Build run configuration from parsed CLI arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - Parsed `run` command arguments.
    ///
    /// # Returns
    ///
    /// A run configuration with shell-style home-directory paths expanded.
    ///
    /// # Errors
    ///
    /// Returns an error when a workspace name cannot be resolved.
    pub fn from_args(args: RunArgs) -> Result<Self> {
        let sessions_root = expand_home_path(args.sessions_root);
        let workspace_path = resolve_workspace_path(args.workspace, &sessions_root)?;

        Ok(Self::new(
            args.provider,
            workspace_path,
            expand_home_path(args.config_db),
            args.image,
            DockerLaunchConfig::new(DEFAULT_CODEX_IMAGE.to_owned(), sessions_root),
        ))
    }

    /// Return the selected provider name.
    ///
    /// # Returns
    ///
    /// Provider name requested by the user.
    #[must_use]
    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }

    /// Return the workspace manifest path.
    ///
    /// # Returns
    ///
    /// Path to the workspace manifest YAML file.
    #[must_use]
    pub fn workspace_path(&self) -> &Path {
        &self.workspace_path
    }

    /// Return the provider database path.
    ///
    /// # Returns
    ///
    /// Path to the local provider configuration database.
    #[must_use]
    pub fn provider_database_path(&self) -> &Path {
        &self.provider_database_path
    }

    /// Return Docker launch settings.
    ///
    /// # Returns
    ///
    /// Docker image and session-root settings.
    #[must_use]
    pub fn docker_launch_config(&self) -> &DockerLaunchConfig {
        &self.docker_launch_config
    }

    fn effective_docker_launch_config(&self, manifest: &WorkspaceManifest) -> DockerLaunchConfig {
        if let Some(image) = &self.image_override {
            return self.docker_launch_config.with_image(image.clone());
        }

        if let Some(image) = manifest.runtime().image() {
            return self.docker_launch_config.with_image(image.to_owned());
        }

        self.docker_launch_config.clone()
    }
}

/// Execute the configured workspace launch.
///
/// # Arguments
///
/// * `config` - Run configuration derived from CLI arguments.
///
/// # Returns
///
/// The process exit code that should be returned by the CLI.
///
/// # Errors
///
/// Returns an error when provider loading, manifest loading, folder validation, session directory
/// creation, Docker command construction, or Docker execution fails.
pub fn run_workspace(config: &RunConfig) -> Result<ExitCode> {
    let providers = load_codex_providers(config.provider_database_path()).with_context(|| {
        format!(
            "failed to load providers from '{}'",
            config.provider_database_path().display()
        )
    })?;
    let provider = select_provider(providers, config.provider_name())?;
    let manifest = load_workspace_manifest(config.workspace_path()).with_context(|| {
        format!(
            "failed to load workspace manifest '{}'",
            config.workspace_path().display()
        )
    })?;

    validate_workspace_folders(&manifest).context("workspace folder validation failed")?;
    let docker_launch_config = config.effective_docker_launch_config(&manifest);

    let sessions_path = docker_launch_config.workspace_sessions_path(manifest.name());
    create_host_directory(&sessions_path, "workspace sessions")?;
    ensure_default_image(docker_launch_config.image())?;

    let provider_config = write_provider_config_files(
        &provider,
        &manifest,
        &docker_launch_config
            .sessions_root()
            .join(manifest.name())
            .join("provider-config"),
    )?;
    let mut command =
        build_docker_run_command(provider_config.files(), &manifest, &docker_launch_config)
            .context("failed to build Docker launch command")?;
    let status = command.status().context("failed to execute Docker")?;

    Ok(exit_code_from_status(status))
}

fn write_provider_config_files(
    provider: &CodexProvider,
    manifest: &WorkspaceManifest,
    provider_config_root: &Path,
) -> Result<RunScopedProviderConfig> {
    let config_dir = create_run_scoped_directory(provider_config_root, "codex-ws-provider")?;

    let auth_path = config_dir.path().join("auth.json");
    let config_path = config_dir.path().join("config.toml");
    fs::write(&auth_path, provider.auth_json()).with_context(|| {
        format!(
            "failed to write provider auth file '{}'",
            auth_path.display()
        )
    })?;
    let config_toml = trusted_workspace_config(provider.config_toml(), manifest);
    fs::write(&config_path, config_toml).with_context(|| {
        format!(
            "failed to write provider config file '{}'",
            config_path.display()
        )
    })?;

    Ok(RunScopedProviderConfig::new(
        config_dir,
        ProviderConfigFiles::new(auth_path, config_path),
    ))
}

fn trusted_workspace_config(provider_config_toml: &str, manifest: &WorkspaceManifest) -> String {
    let mut config =
        String::with_capacity(provider_config_toml.len() + manifest.folders().len() * 64);
    config.push_str(provider_config_toml.trim_end());
    config.push_str("\n\n");

    for index in 0..manifest.folders().len() {
        config.push_str(&format!(
            "[projects.\"/workspace/{}\"]\ntrust_level = \"trusted\"\n\n",
            index + 1
        ));
    }

    config
}

fn create_host_directory(path: &Path, label: &str) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create {label} directory '{}'", path.display()))
}

fn create_run_scoped_directory(root: &Path, prefix: &str) -> Result<RunScopedDirectory> {
    fs::create_dir_all(root).with_context(|| {
        format!(
            "failed to create run-scoped root directory '{}'",
            root.display()
        )
    })?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_nanos();
    let path = root.join(format!("{prefix}-{}-{timestamp}", std::process::id()));
    fs::create_dir(&path)
        .with_context(|| format!("failed to create run-scoped directory '{}'", path.display()))?;
    Ok(RunScopedDirectory::new(path))
}

fn ensure_default_image(image: &str) -> Result<()> {
    if image != DEFAULT_CODEX_IMAGE {
        return Ok(());
    }

    let inspect_output = Command::new("docker")
        .args([
            "image",
            "inspect",
            image,
            "--format",
            "{{ index .Config.Labels \"org.openai.codex-ws.image-version\" }}",
        ])
        .output()
        .context("failed to inspect Docker image")?;
    let image_version = String::from_utf8_lossy(&inspect_output.stdout);
    if inspect_output.status.success() && image_version.trim() == DEFAULT_CODEX_IMAGE_VERSION {
        return Ok(());
    }

    let pull_status = Command::new("docker")
        .args(["pull", image])
        .status()
        .context("failed to pull Codex workspace Docker image")?;
    if pull_status.success() {
        return Ok(());
    }

    Err(anyhow!("failed to pull Docker image '{image}'"))
}

fn select_provider(providers: Vec<CodexProvider>, provider_name: &str) -> Result<CodexProvider> {
    providers
        .into_iter()
        .find(|provider| provider.name() == provider_name)
        .ok_or_else(|| anyhow!("Codex provider '{provider_name}' was not found"))
}

fn exit_code_from_status(status: ExitStatus) -> ExitCode {
    match status.code() {
        Some(0) => ExitCode::SUCCESS,
        Some(_) | None => ExitCode::FAILURE,
    }
}

#[derive(Debug)]
struct RunScopedProviderConfig {
    _directory: RunScopedDirectory,
    files: ProviderConfigFiles,
}

impl RunScopedProviderConfig {
    fn new(directory: RunScopedDirectory, files: ProviderConfigFiles) -> Self {
        Self {
            _directory: directory,
            files,
        }
    }

    fn files(&self) -> &ProviderConfigFiles {
        &self.files
    }
}

#[derive(Debug)]
struct RunScopedDirectory {
    path: PathBuf,
}

impl RunScopedDirectory {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for RunScopedDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn select_provider_returns_matching_provider() {
        let provider = CodexProvider::new(
            "primary".to_owned(),
            "auth.json".to_owned(),
            "config.toml".to_owned(),
        );

        let selected = select_provider(vec![provider.clone()], "primary")
            .expect("provider should be selected");

        assert_eq!(selected, provider);
    }

    #[test]
    fn select_provider_rejects_missing_provider() {
        let error = select_provider(Vec::new(), "missing")
            .expect_err("missing provider should fail")
            .to_string();

        assert_eq!(error, "Codex provider 'missing' was not found");
    }

    #[test]
    fn write_provider_config_files_writes_auth_json_and_config_toml() {
        let temp_dir = TestTempDir::create();
        let provider = CodexProvider::new(
            "primary".to_owned(),
            "{\n  \"OPENAI_API_KEY\": \"test-key\"\n}".to_owned(),
            "model = \"gpt-5.5\"\n".to_owned(),
        );
        let manifest = WorkspaceManifest::new(
            "workspace".to_owned(),
            vec![PathBuf::from("/host/project")],
            crate::manifest::SandboxConfig::default(),
        )
        .expect("manifest should be valid");

        let provider_config =
            write_provider_config_files(&provider, &manifest, &temp_dir.path().join("config"))
                .expect("provider config files should be written");

        assert_eq!(
            fs::read_to_string(provider_config.files().auth_path())
                .expect("auth file should be readable"),
            "{\n  \"OPENAI_API_KEY\": \"test-key\"\n}"
        );
        assert_eq!(
            fs::read_to_string(provider_config.files().config_path())
                .expect("config file should be readable"),
            "model = \"gpt-5.5\"\n\n[projects.\"/workspace/1\"]\ntrust_level = \"trusted\"\n\n"
        );
    }

    #[test]
    fn effective_docker_launch_config_uses_manifest_runtime_image() {
        let config = RunConfig::new(
            "primary".to_owned(),
            PathBuf::from("/tmp/workspace.yaml"),
            PathBuf::from("/tmp/cc-switch.db"),
            None,
            DockerLaunchConfig::new(
                DEFAULT_CODEX_IMAGE.to_owned(),
                PathBuf::from("/host/.codex-ws"),
            ),
        );
        let manifest = WorkspaceManifest::with_runtime(
            "workspace".to_owned(),
            vec![PathBuf::from("/host/project")],
            crate::manifest::SandboxConfig::default(),
            crate::manifest::RuntimeConfig::new(Some("rust-codex-ws:latest".to_owned())),
        )
        .expect("manifest should be valid");

        let launch_config = config.effective_docker_launch_config(&manifest);

        assert_eq!(launch_config.image(), "rust-codex-ws:latest");
    }

    #[test]
    fn effective_docker_launch_config_prefers_cli_image_override() {
        let config = RunConfig::new(
            "primary".to_owned(),
            PathBuf::from("/tmp/workspace.yaml"),
            PathBuf::from("/tmp/cc-switch.db"),
            Some("cli-codex-ws:latest".to_owned()),
            DockerLaunchConfig::new(
                DEFAULT_CODEX_IMAGE.to_owned(),
                PathBuf::from("/host/.codex-ws"),
            ),
        );
        let manifest = WorkspaceManifest::with_runtime(
            "workspace".to_owned(),
            vec![PathBuf::from("/host/project")],
            crate::manifest::SandboxConfig::default(),
            crate::manifest::RuntimeConfig::new(Some("manifest-codex-ws:latest".to_owned())),
        )
        .expect("manifest should be valid");

        let launch_config = config.effective_docker_launch_config(&manifest);

        assert_eq!(launch_config.image(), "cli-codex-ws:latest");
    }

    #[test]
    fn trusted_workspace_config_trusts_every_container_workspace_path() {
        let manifest = WorkspaceManifest::new(
            "workspace".to_owned(),
            vec![
                PathBuf::from("/host/backend"),
                PathBuf::from("/host/frontend"),
            ],
            crate::manifest::SandboxConfig::default(),
        )
        .expect("manifest should be valid");

        let config = trusted_workspace_config("model = \"gpt-5.5\"\n", &manifest);

        assert!(config.contains("[projects.\"/workspace/1\"]\ntrust_level = \"trusted\""));
        assert!(config.contains("[projects.\"/workspace/2\"]\ntrust_level = \"trusted\""));
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
                "codex-ws-app-test-{}-{timestamp}-{counter}",
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
