use std::fs;
use std::path::{Path, PathBuf};
use std::process::{ExitCode, ExitStatus};

use anyhow::{Context, Result, anyhow};

use crate::cli::RunArgs;
use crate::docker::{DockerLaunchConfig, ProviderConfigFiles, build_docker_run_command};
use crate::manifest::{load_workspace_manifest, validate_workspace_folders};
use crate::provider::{CodexProvider, load_codex_providers};

/// Run configuration derived from CLI arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfig {
    provider_name: String,
    workspace_path: PathBuf,
    provider_database_path: PathBuf,
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
        docker_launch_config: DockerLaunchConfig,
    ) -> Self {
        Self {
            provider_name,
            workspace_path,
            provider_database_path,
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
    #[must_use]
    pub fn from_args(args: RunArgs) -> Self {
        Self::new(
            args.provider,
            expand_home_path(args.workspace),
            expand_home_path(args.config_db),
            DockerLaunchConfig::new(args.image, expand_home_path(args.sessions_root)),
        )
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

    let sessions_path = config
        .docker_launch_config()
        .workspace_sessions_path(manifest.name());
    fs::create_dir_all(&sessions_path).with_context(|| {
        format!(
            "failed to create workspace sessions directory '{}'",
            sessions_path.display()
        )
    })?;

    let provider_files = write_provider_config_files(
        &provider,
        &config
            .docker_launch_config()
            .sessions_root()
            .join(manifest.name())
            .join("config"),
    )?;
    let mut command =
        build_docker_run_command(&provider_files, &manifest, config.docker_launch_config())
            .context("failed to build Docker launch command")?;
    let status = command.status().context("failed to execute Docker")?;

    Ok(exit_code_from_status(status))
}

fn write_provider_config_files(
    provider: &CodexProvider,
    config_dir: &Path,
) -> Result<ProviderConfigFiles> {
    fs::create_dir_all(config_dir).with_context(|| {
        format!(
            "failed to create provider config directory '{}'",
            config_dir.display()
        )
    })?;

    let auth_path = config_dir.join("auth.json");
    let config_path = config_dir.join("config.toml");
    fs::write(&auth_path, provider.auth_json()).with_context(|| {
        format!(
            "failed to write provider auth file '{}'",
            auth_path.display()
        )
    })?;
    fs::write(&config_path, provider.config_toml()).with_context(|| {
        format!(
            "failed to write provider config file '{}'",
            config_path.display()
        )
    })?;

    Ok(ProviderConfigFiles::new(auth_path, config_path))
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

fn expand_home_path(path: PathBuf) -> PathBuf {
    let Some(path_text) = path.to_str() else {
        return path;
    };

    if path_text == "~" {
        return home_dir().unwrap_or(path);
    }

    if let Some(rest) = path_text.strip_prefix("~/")
        && let Some(home) = home_dir()
    {
        return home.join(rest);
    }

    path
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

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
    fn expand_home_path_leaves_absolute_paths_unchanged() {
        assert_eq!(
            expand_home_path(PathBuf::from("/tmp/workspace.yaml")),
            PathBuf::from("/tmp/workspace.yaml")
        );
    }

    #[test]
    fn write_provider_config_files_writes_auth_json_and_config_toml() {
        let temp_dir = TestTempDir::create();
        let provider = CodexProvider::new(
            "primary".to_owned(),
            "{\n  \"OPENAI_API_KEY\": \"test-key\"\n}".to_owned(),
            "model = \"gpt-5.5\"\n".to_owned(),
        );

        let files = write_provider_config_files(&provider, temp_dir.path())
            .expect("provider config files should be written");

        assert_eq!(
            fs::read_to_string(files.auth_path()).expect("auth file should be readable"),
            "{\n  \"OPENAI_API_KEY\": \"test-key\"\n}"
        );
        assert_eq!(
            fs::read_to_string(files.config_path()).expect("config file should be readable"),
            "model = \"gpt-5.5\"\n"
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
                "codex-ws-app-test-{}-{timestamp}-{counter}",
                std::process::id(),
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
