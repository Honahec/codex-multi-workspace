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

#[cfg(test)]
mod tests {
    use super::*;

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
}
