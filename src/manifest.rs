use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// Workspace manifest describing folders and sandbox options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceManifest {
    name: String,
    folders: Vec<PathBuf>,
    sandbox: SandboxConfig,
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
        if name.trim().is_empty() {
            return Err(ManifestError::EmptyName);
        }

        if folders.is_empty() {
            return Err(ManifestError::NoFolders);
        }

        Ok(Self {
            name,
            folders,
            sandbox,
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
}

/// Sandbox options loaded from a workspace manifest.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SandboxConfig {
    network: bool,
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
}

#[derive(Debug, Default, Deserialize)]
struct RawSandboxConfig {
    #[serde(default)]
    network: bool,
}

impl TryFrom<RawWorkspaceManifest> for WorkspaceManifest {
    type Error = ManifestError;

    fn try_from(raw: RawWorkspaceManifest) -> Result<Self, Self::Error> {
        Self::new(
            raw.name,
            raw.folders,
            SandboxConfig::new(raw.sandbox.network),
        )
    }
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
        assert!(!manifest.sandbox().network());
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
