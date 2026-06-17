use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

use crate::runtime::{
    CODEX_WS_APT_PACKAGES_ENV, CODEX_WS_SETUP_COMMANDS_ENV, RuntimeEnvironmentVariable,
    RuntimeSpecError, RuntimeTool, RuntimeToolVersion, validate_apt_packages,
    validate_runtime_tool_versions, validate_setup_commands, validate_tool_version,
};

/// Workspace manifest describing folders and sandbox options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceManifest {
    name: String,
    folders: Vec<PathBuf>,
    sandbox: SandboxConfig,
    runtime: RuntimeConfig,
}

impl WorkspaceManifest {
    /// Create a workspace manifest.
    ///
    /// # Arguments
    ///
    /// * `name` - Stable workspace name used for session routing.
    /// * `folders` - One or more project folders included in the workspace.
    /// * `sandbox` - Runtime options applied when launching the sandbox.
    ///
    /// # Returns
    ///
    /// A validated workspace manifest.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::EmptyName`] when `name` is blank.
    /// Returns [`ManifestError::NoFolders`] when no folders are provided.
    pub fn new(
        name: String,
        folders: Vec<PathBuf>,
        sandbox: SandboxConfig,
    ) -> Result<Self, ManifestError> {
        Self::with_runtime(name, folders, sandbox, RuntimeConfig::default())
    }

    /// Create a workspace manifest with runtime settings.
    ///
    /// # Arguments
    ///
    /// * `name` - Stable workspace name used for session routing.
    /// * `folders` - One or more project folders included in the workspace.
    /// * `sandbox` - Runtime sandbox options applied when launching the sandbox.
    /// * `runtime` - Container runtime image settings for this workspace.
    ///
    /// # Returns
    ///
    /// A validated workspace manifest.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::EmptyName`] when `name` is blank.
    /// Returns [`ManifestError::NoFolders`] when no folders are provided.
    /// Returns [`ManifestError::EmptyRuntimeImage`] when `runtime.image` is blank.
    pub fn with_runtime(
        name: String,
        folders: Vec<PathBuf>,
        sandbox: SandboxConfig,
        runtime: RuntimeConfig,
    ) -> Result<Self, ManifestError> {
        if name.trim().is_empty() {
            return Err(ManifestError::EmptyName);
        }

        if folders.is_empty() {
            return Err(ManifestError::NoFolders);
        }

        if runtime.image().is_some_and(|image| image.trim().is_empty()) {
            return Err(ManifestError::EmptyRuntimeImage);
        }

        Ok(Self {
            name,
            folders,
            sandbox,
            runtime,
        })
    }

    /// Return the workspace name.
    ///
    /// # Returns
    ///
    /// The workspace name as a borrowed string slice.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return workspace folders.
    ///
    /// # Returns
    ///
    /// A slice of folder paths included in this workspace.
    #[must_use]
    pub fn folders(&self) -> &[PathBuf] {
        &self.folders
    }

    /// Return sandbox runtime options.
    ///
    /// # Returns
    ///
    /// The sandbox configuration for this workspace.
    #[must_use]
    pub fn sandbox(&self) -> &SandboxConfig {
        &self.sandbox
    }

    /// Return container runtime options.
    ///
    /// # Returns
    ///
    /// The runtime configuration for this workspace.
    #[must_use]
    pub fn runtime(&self) -> &RuntimeConfig {
        &self.runtime
    }
}

/// Sandbox options loaded from a workspace manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SandboxConfig {
    network: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self { network: true }
    }
}

impl SandboxConfig {
    /// Create a sandbox configuration.
    ///
    /// # Arguments
    ///
    /// * `network` - Whether the sandbox should allow network access.
    ///
    /// # Returns
    ///
    /// A sandbox configuration value.
    #[must_use]
    pub const fn new(network: bool) -> Self {
        Self { network }
    }

    /// Return whether sandbox network access is enabled.
    ///
    /// # Returns
    ///
    /// `true` when network access is enabled.
    #[must_use]
    pub const fn network(&self) -> bool {
        self.network
    }
}

/// Container runtime options loaded from a workspace manifest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeConfig {
    image: Option<String>,
    tool_versions: Vec<RuntimeToolVersion>,
    apt_packages: Vec<String>,
    setup_commands: Vec<String>,
}

impl RuntimeConfig {
    /// Create a runtime configuration.
    ///
    /// # Arguments
    ///
    /// * `image` - Optional Docker image used for this workspace.
    ///
    /// # Returns
    ///
    /// A runtime configuration value.
    #[must_use]
    pub fn new(image: Option<String>) -> Self {
        Self {
            image,
            tool_versions: Vec::new(),
            apt_packages: Vec::new(),
            setup_commands: Vec::new(),
        }
    }

    /// Create a runtime configuration with startup setup.
    ///
    /// # Arguments
    ///
    /// * `image` - Optional Docker image used for this workspace.
    /// * `tool_versions` - Declarative language runtime versions installed before Codex starts.
    /// * `apt_packages` - Apt packages installed before language tools and Codex start.
    /// * `setup_commands` - Shell commands run by the entrypoint before Codex starts.
    ///
    /// # Returns
    ///
    /// A runtime configuration value.
    #[must_use]
    pub fn with_setup(
        image: Option<String>,
        tool_versions: Vec<RuntimeToolVersion>,
        apt_packages: Vec<String>,
        setup_commands: Vec<String>,
    ) -> Self {
        Self {
            image,
            tool_versions,
            apt_packages,
            setup_commands,
        }
    }

    /// Return the workspace-specific Docker image.
    ///
    /// # Returns
    ///
    /// `Some(image)` when the manifest selects a runtime image, otherwise `None`.
    #[must_use]
    pub fn image(&self) -> Option<&str> {
        self.image.as_deref()
    }

    /// Return declarative runtime tool versions.
    ///
    /// # Returns
    ///
    /// Runtime tool versions requested by this workspace.
    #[must_use]
    pub fn tool_versions(&self) -> &[RuntimeToolVersion] {
        &self.tool_versions
    }

    /// Return apt packages installed before Codex starts.
    ///
    /// # Returns
    ///
    /// Apt package names requested by this workspace.
    #[must_use]
    pub fn apt_packages(&self) -> &[String] {
        &self.apt_packages
    }

    /// Return setup commands run before Codex starts.
    ///
    /// # Returns
    ///
    /// Shell commands requested by this workspace.
    #[must_use]
    pub fn setup_commands(&self) -> &[String] {
        &self.setup_commands
    }

    /// Return Docker environment variables for runtime setup.
    ///
    /// # Returns
    ///
    /// Entrypoint variables generated from configured apt packages and setup commands.
    #[must_use]
    pub fn environment_variables(&self) -> Vec<RuntimeEnvironmentVariable> {
        let mut variables = Vec::with_capacity(self.tool_versions.len() + 2);
        variables.extend(
            self.tool_versions
                .iter()
                .map(RuntimeToolVersion::environment_variable),
        );

        if !self.apt_packages.is_empty() {
            variables.push(RuntimeEnvironmentVariable::new(
                CODEX_WS_APT_PACKAGES_ENV,
                self.apt_packages.join(" "),
            ));
        }

        if !self.setup_commands.is_empty() {
            variables.push(RuntimeEnvironmentVariable::new(
                CODEX_WS_SETUP_COMMANDS_ENV,
                self.setup_commands.join("\n"),
            ));
        }

        variables
    }
}

/// Errors returned while loading or validating workspace manifests.
#[derive(Debug, Error)]
pub enum ManifestError {
    /// The manifest file could not be read.
    #[error("failed to read workspace manifest '{path}': {source}")]
    Read {
        /// Manifest path that failed to read.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// The manifest YAML could not be parsed.
    #[error("invalid workspace manifest YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// The workspace name was empty or only whitespace.
    #[error("workspace manifest name cannot be empty")]
    EmptyName,

    /// The workspace did not include any folders.
    #[error("workspace manifest must include at least one folder")]
    NoFolders,

    /// The workspace runtime image was empty or only whitespace.
    #[error("workspace manifest runtime image cannot be empty")]
    EmptyRuntimeImage,

    /// The workspace runtime language selection was invalid.
    #[error("invalid workspace runtime: {0}")]
    RuntimeSpec(#[from] RuntimeSpecError),

    /// A workspace folder path does not exist.
    #[error("workspace folder '{path}' does not exist")]
    FolderMissing {
        /// Missing workspace folder path.
        path: PathBuf,
    },

    /// A workspace folder path exists but is not a directory.
    #[error("workspace folder '{path}' is not a directory")]
    FolderNotDirectory {
        /// Non-directory workspace folder path.
        path: PathBuf,
    },
}

#[derive(Debug, Deserialize)]
struct RawWorkspaceManifest {
    name: String,
    folders: Vec<PathBuf>,
    #[serde(default)]
    sandbox: RawSandboxConfig,
    #[serde(default)]
    runtime: Option<RawRuntimeConfig>,
}

#[derive(Debug, Deserialize)]
struct RawSandboxConfig {
    #[serde(default = "default_sandbox_network")]
    network: bool,
}

impl Default for RawSandboxConfig {
    fn default() -> Self {
        Self {
            network: default_sandbox_network(),
        }
    }
}

const fn default_sandbox_network() -> bool {
    true
}

#[derive(Debug, Default, Deserialize)]
struct RawRuntimeConfig {
    image: Option<String>,
    python: Option<String>,
    node: Option<String>,
    go: Option<String>,
    rust: Option<String>,
    java: Option<String>,
    clang: Option<String>,
    c: Option<String>,
    cpp: Option<String>,
    ruby: Option<String>,
    php: Option<String>,
    deno: Option<String>,
    bun: Option<String>,
    zig: Option<String>,
    dotnet: Option<String>,
    #[serde(default)]
    apt: Vec<String>,
    #[serde(default)]
    setup: Vec<String>,
}

impl TryFrom<RawWorkspaceManifest> for WorkspaceManifest {
    type Error = ManifestError;

    fn try_from(raw: RawWorkspaceManifest) -> Result<Self, Self::Error> {
        let runtime = raw.runtime.unwrap_or_default().try_into()?;
        Self::with_runtime(
            raw.name,
            raw.folders,
            SandboxConfig::new(raw.sandbox.network),
            runtime,
        )
    }
}

impl TryFrom<RawRuntimeConfig> for RuntimeConfig {
    type Error = ManifestError;

    fn try_from(raw: RawRuntimeConfig) -> Result<Self, Self::Error> {
        runtime_from_raw(raw)
    }
}

fn runtime_from_raw(raw: RawRuntimeConfig) -> Result<RuntimeConfig, ManifestError> {
    let image = raw
        .image
        .as_ref()
        .map(|runtime_image| runtime_image.trim().to_owned());
    let tool_versions = validate_runtime_tool_versions(runtime_tool_versions(&raw)?)?;
    let apt_packages = validate_apt_packages(raw.apt)?;
    let setup_commands = validate_setup_commands(raw.setup)?;

    Ok(RuntimeConfig::with_setup(
        image,
        tool_versions,
        apt_packages,
        setup_commands,
    ))
}

fn runtime_tool_versions(
    raw: &RawRuntimeConfig,
) -> Result<Vec<RuntimeToolVersion>, RuntimeSpecError> {
    let raw_versions = [
        (RuntimeTool::Python, raw.python.clone()),
        (RuntimeTool::Node, raw.node.clone()),
        (RuntimeTool::Go, raw.go.clone()),
        (RuntimeTool::Rust, raw.rust.clone()),
        (RuntimeTool::Java, raw.java.clone()),
        (RuntimeTool::Clang, raw.clang.clone()),
        (RuntimeTool::C, raw.c.clone()),
        (RuntimeTool::Cpp, raw.cpp.clone()),
        (RuntimeTool::Ruby, raw.ruby.clone()),
        (RuntimeTool::Php, raw.php.clone()),
        (RuntimeTool::Deno, raw.deno.clone()),
        (RuntimeTool::Bun, raw.bun.clone()),
        (RuntimeTool::Zig, raw.zig.clone()),
        (RuntimeTool::Dotnet, raw.dotnet.clone()),
    ];
    let mut versions = Vec::with_capacity(raw_versions.len());

    for (tool, version) in raw_versions {
        if let Some(version) = validate_tool_version(tool, version)? {
            versions.push(version);
        }
    }

    Ok(versions)
}

/// Load a workspace manifest from a YAML file.
///
/// # Arguments
///
/// * `manifest_path` - Path to the YAML workspace manifest.
///
/// # Returns
///
/// A validated workspace manifest.
///
/// # Errors
///
/// Returns [`ManifestError::Read`] when the file cannot be read.
/// Returns [`ManifestError::Yaml`] when YAML parsing fails.
/// Returns validation errors when required fields are missing or invalid.
pub fn load_workspace_manifest(manifest_path: &Path) -> Result<WorkspaceManifest, ManifestError> {
    let manifest_yaml =
        fs::read_to_string(manifest_path).map_err(|source| ManifestError::Read {
            path: manifest_path.to_path_buf(),
            source,
        })?;
    parse_workspace_manifest(&manifest_yaml)
}

/// Parse a workspace manifest from YAML.
///
/// # Arguments
///
/// * `manifest_yaml` - YAML text containing workspace manifest fields.
///
/// # Returns
///
/// A validated workspace manifest.
///
/// # Errors
///
/// Returns [`ManifestError::Yaml`] when YAML parsing fails.
/// Returns validation errors when required fields are missing or invalid.
pub fn parse_workspace_manifest(manifest_yaml: &str) -> Result<WorkspaceManifest, ManifestError> {
    let raw_manifest = serde_yaml::from_str::<RawWorkspaceManifest>(manifest_yaml)?;
    raw_manifest.try_into()
}

/// Validate that every workspace folder exists and is a directory.
///
/// # Arguments
///
/// * `manifest` - Workspace manifest whose folders should be checked.
///
/// # Returns
///
/// `Ok(())` when all workspace folders exist and are directories.
///
/// # Errors
///
/// Returns [`ManifestError::FolderMissing`] when a folder path does not exist.
/// Returns [`ManifestError::FolderNotDirectory`] when a folder path is not a directory.
pub fn validate_workspace_folders(manifest: &WorkspaceManifest) -> Result<(), ManifestError> {
    for folder in manifest.folders() {
        if !folder.exists() {
            return Err(ManifestError::FolderMissing {
                path: folder.clone(),
            });
        }

        if !folder.is_dir() {
            return Err(ManifestError::FolderNotDirectory {
                path: folder.clone(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

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
                "codex-ws-test-{}-{timestamp}-{counter}",
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

    #[test]
    fn parse_workspace_manifest_supports_multiple_folders_and_network() {
        let manifest = parse_workspace_manifest(
            r#"
name: workspace-name
folders:
  - /projects/backend
  - /projects/frontend
sandbox:
  network: true
"#,
        )
        .expect("manifest should parse");

        assert_eq!(manifest.name(), "workspace-name");
        assert_eq!(
            manifest.folders(),
            &[
                PathBuf::from("/projects/backend"),
                PathBuf::from("/projects/frontend")
            ]
        );
        assert!(manifest.sandbox().network());
        assert_eq!(manifest.runtime().image(), None);
    }

    #[test]
    fn parse_workspace_manifest_supports_single_folder() {
        let manifest = parse_workspace_manifest(
            r#"
name: single-project
folders:
  - /projects/backend
"#,
        )
        .expect("manifest should parse");

        assert_eq!(manifest.name(), "single-project");
        assert_eq!(manifest.folders(), &[PathBuf::from("/projects/backend")]);
        assert!(manifest.sandbox().network());
        assert_eq!(manifest.runtime().image(), None);
    }

    #[test]
    fn parse_workspace_manifest_supports_runtime_image() {
        let manifest = parse_workspace_manifest(
            r#"
name: rust-project
folders:
  - /projects/rust-project
runtime:
  image: rust-codex-ws:latest
"#,
        )
        .expect("manifest should parse");

        assert_eq!(manifest.runtime().image(), Some("rust-codex-ws:latest"));
    }

    #[test]
    fn parse_workspace_manifest_supports_declarative_runtime_tools() {
        let manifest = parse_workspace_manifest(
            r#"
name: toolchain-project
folders:
  - /projects/toolchain-project
runtime:
  python: "3.13"
  node: "22"
  go: "1.24"
  rust: "1.86"
  java: "21"
  clang: "20"
  c: "20"
  cpp: "20"
  ruby: "3.4"
  php: "8.4"
  deno: "2"
  bun: "1"
  zig: "0.14"
  dotnet: "9"
"#,
        )
        .expect("manifest should parse");

        let variables = manifest.runtime().environment_variables();
        assert_eq!(
            variables
                .iter()
                .map(crate::runtime::RuntimeEnvironmentVariable::docker_assignment)
                .collect::<Vec<_>>(),
            vec![
                "CODEX_WS_PYTHON_VERSION=3.13".to_owned(),
                "CODEX_WS_NODE_VERSION=22".to_owned(),
                "CODEX_WS_GO_VERSION=1.24".to_owned(),
                "CODEX_WS_RUST_VERSION=1.86".to_owned(),
                "CODEX_WS_JAVA_VERSION=21".to_owned(),
                "CODEX_WS_CLANG_VERSION=20".to_owned(),
                "CODEX_WS_C_VERSION=20".to_owned(),
                "CODEX_WS_CPP_VERSION=20".to_owned(),
                "CODEX_WS_RUBY_VERSION=3.4".to_owned(),
                "CODEX_WS_PHP_VERSION=8.4".to_owned(),
                "CODEX_WS_DENO_VERSION=2".to_owned(),
                "CODEX_WS_BUN_VERSION=1".to_owned(),
                "CODEX_WS_ZIG_VERSION=0.14".to_owned(),
                "CODEX_WS_DOTNET_VERSION=9".to_owned()
            ]
        );
    }

    #[test]
    fn parse_workspace_manifest_supports_runtime_apt_packages() {
        let manifest = parse_workspace_manifest(
            r#"
name: python-project
folders:
  - /projects/python-project
runtime:
  apt:
    - python3
    - python3-pip
"#,
        )
        .expect("manifest should parse");

        assert_eq!(
            manifest.runtime().environment_variables()[0].docker_assignment(),
            "CODEX_WS_APT_PACKAGES=python3 python3-pip"
        );
    }

    #[test]
    fn parse_workspace_manifest_supports_runtime_setup_commands() {
        let manifest = parse_workspace_manifest(
            r#"
name: rust-project
folders:
  - /projects/rust-project
runtime:
  setup:
    - curl -fsSL https://sh.rustup.rs | sh -s -- -y
    - . "$HOME/.cargo/env"
"#,
        )
        .expect("manifest should parse");

        let variables = manifest.runtime().environment_variables();
        assert_eq!(
            variables
                .iter()
                .map(crate::runtime::RuntimeEnvironmentVariable::docker_assignment)
                .collect::<Vec<_>>(),
            vec!["CODEX_WS_SETUP_COMMANDS=curl -fsSL https://sh.rustup.rs | sh -s -- -y\n. \"$HOME/.cargo/env\"".to_owned()]
        );
    }

    #[test]
    fn parse_workspace_manifest_supports_runtime_apt_and_setup() {
        let manifest = parse_workspace_manifest(
            r#"
name: mixed-project
folders:
  - /projects/mixed-project
runtime:
  apt:
    - build-essential
  setup:
    - echo ready
"#,
        )
        .expect("manifest should parse");

        let variables = manifest.runtime().environment_variables();
        assert_eq!(
            variables
                .iter()
                .map(crate::runtime::RuntimeEnvironmentVariable::docker_assignment)
                .collect::<Vec<_>>(),
            vec![
                "CODEX_WS_APT_PACKAGES=build-essential".to_owned(),
                "CODEX_WS_SETUP_COMMANDS=echo ready".to_owned()
            ]
        );
    }

    #[test]
    fn parse_workspace_manifest_rejects_empty_name() {
        let error = parse_workspace_manifest(
            r#"
name: " "
folders:
  - /projects/backend
"#,
        )
        .expect_err("blank name should fail");

        assert!(matches!(error, ManifestError::EmptyName));
    }

    #[test]
    fn parse_workspace_manifest_rejects_empty_folders() {
        let error = parse_workspace_manifest(
            r#"
name: empty-workspace
folders: []
"#,
        )
        .expect_err("empty folders should fail");

        assert!(matches!(error, ManifestError::NoFolders));
    }

    #[test]
    fn parse_workspace_manifest_rejects_empty_runtime_image() {
        let error = parse_workspace_manifest(
            r#"
name: workspace
folders:
  - /projects/backend
runtime:
  image: " "
"#,
        )
        .expect_err("blank runtime image should fail");

        assert!(matches!(error, ManifestError::EmptyRuntimeImage));
    }

    #[test]
    fn parse_workspace_manifest_rejects_invalid_apt_package() {
        let error = parse_workspace_manifest(
            r#"
name: workspace
folders:
  - /projects/backend
runtime:
  apt:
    - python3;curl
"#,
        )
        .expect_err("invalid apt package should fail");

        assert!(matches!(
            error,
            ManifestError::RuntimeSpec(crate::runtime::RuntimeSpecError::InvalidAptPackage {
                package
            }) if package == "python3;curl"
        ));
    }

    #[test]
    fn parse_workspace_manifest_rejects_invalid_tool_version() {
        let error = parse_workspace_manifest(
            r#"
name: workspace
folders:
  - /projects/backend
runtime:
  go: "1.24;curl"
"#,
        )
        .expect_err("invalid tool version should fail");

        assert!(matches!(
            error,
            ManifestError::RuntimeSpec(crate::runtime::RuntimeSpecError::InvalidToolVersion {
                tool: crate::runtime::RuntimeTool::Go,
                version
            }) if version == "1.24;curl"
        ));
    }

    #[test]
    fn parse_workspace_manifest_rejects_conflicting_c_and_cpp_versions() {
        let error = parse_workspace_manifest(
            r#"
name: workspace
folders:
  - /projects/backend
runtime:
  c: "20"
  cpp: "21"
"#,
        )
        .expect_err("conflicting compiler versions should fail");

        assert!(matches!(
            error,
            ManifestError::RuntimeSpec(
                crate::runtime::RuntimeSpecError::ConflictingCompilerVersions {
                    first,
                    second
                }
            ) if first == "20" && second == "21"
        ));
    }

    #[test]
    fn validate_workspace_folders_accepts_existing_directories() {
        let temp_dir = TestTempDir::create();
        let folder = temp_dir.path().join("project");
        fs::create_dir(&folder).expect("workspace folder should be created");
        let manifest = WorkspaceManifest::new(
            "workspace".to_owned(),
            vec![folder],
            SandboxConfig::default(),
        )
        .expect("manifest should be valid");

        validate_workspace_folders(&manifest).expect("folder validation should pass");
    }

    #[test]
    fn validate_workspace_folders_rejects_missing_paths() {
        let temp_dir = TestTempDir::create();
        let missing_folder = temp_dir.path().join("missing");
        let manifest = WorkspaceManifest::new(
            "workspace".to_owned(),
            vec![missing_folder.clone()],
            SandboxConfig::default(),
        )
        .expect("manifest should be valid");

        let error = validate_workspace_folders(&manifest).expect_err("missing folder should fail");

        assert!(matches!(
            error,
            ManifestError::FolderMissing { path } if path == missing_folder
        ));
    }

    #[test]
    fn validate_workspace_folders_rejects_files() {
        let temp_dir = TestTempDir::create();
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, "not a directory").expect("file should be written");
        let manifest = WorkspaceManifest::new(
            "workspace".to_owned(),
            vec![file_path.clone()],
            SandboxConfig::default(),
        )
        .expect("manifest should be valid");

        let error = validate_workspace_folders(&manifest).expect_err("file path should fail");

        assert!(matches!(
            error,
            ManifestError::FolderNotDirectory { path } if path == file_path
        ));
    }
}
