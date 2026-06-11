use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow};

/// Saved workspace manifest entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceEntry {
    name: String,
    path: PathBuf,
}

impl WorkspaceEntry {
    /// Create a saved workspace manifest entry.
    ///
    /// # Arguments
    ///
    /// * `name` - Workspace name derived from the manifest file stem.
    /// * `path` - Manifest file path.
    ///
    /// # Returns
    ///
    /// A workspace entry.
    #[must_use]
    pub fn new(name: String, path: PathBuf) -> Self {
        Self { name, path }
    }

    /// Return the workspace name.
    ///
    /// # Returns
    ///
    /// Workspace name shown by `workspace ls`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the manifest path.
    ///
    /// # Returns
    ///
    /// Path to the saved workspace manifest.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Return the saved workspace manifest directory.
///
/// # Arguments
///
/// * `sessions_root` - codex-ws state root.
///
/// # Returns
///
/// Directory containing saved workspace manifests.
#[must_use]
pub fn workspace_config_dir(sessions_root: &Path) -> PathBuf {
    sessions_root.join("config").join("workspace")
}

/// Return the manifest path for a saved workspace name.
///
/// # Arguments
///
/// * `sessions_root` - codex-ws state root.
/// * `workspace_name` - Saved workspace name.
///
/// # Returns
///
/// Path to the saved workspace manifest.
///
/// # Errors
///
/// Returns an error when the workspace name is empty or contains path separators.
pub fn workspace_manifest_path(sessions_root: &Path, workspace_name: &str) -> Result<PathBuf> {
    validate_workspace_name(workspace_name)?;
    Ok(workspace_config_dir(sessions_root).join(format!("{workspace_name}.yaml")))
}

/// Resolve a `run --workspace` value to a manifest path.
///
/// Path-like values are expanded and returned as paths. Bare workspace names resolve under
/// `~/.codex-ws/config/workspace` relative to the configured sessions root.
///
/// # Arguments
///
/// * `workspace` - User-provided workspace name or path.
/// * `sessions_root` - codex-ws state root.
///
/// # Returns
///
/// Manifest path to load.
///
/// # Errors
///
/// Returns an error when a bare workspace name is invalid.
pub fn resolve_workspace_path(workspace: PathBuf, sessions_root: &Path) -> Result<PathBuf> {
    if is_path_like(&workspace) {
        return Ok(expand_home_path(workspace));
    }

    let Some(workspace_name) = workspace.to_str() else {
        return Ok(workspace);
    };
    workspace_manifest_path(sessions_root, workspace_name)
}

/// List saved workspace manifests.
///
/// # Arguments
///
/// * `sessions_root` - codex-ws state root.
///
/// # Returns
///
/// Sorted workspace entries for `.yaml` files under the workspace config directory.
///
/// # Errors
///
/// Returns an error when the workspace config directory cannot be read.
pub fn list_workspaces(sessions_root: &Path) -> Result<Vec<WorkspaceEntry>> {
    let config_dir = workspace_config_dir(sessions_root);
    if !config_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&config_dir).with_context(|| {
        format!(
            "failed to read workspace config directory '{}'",
            config_dir.display()
        )
    })? {
        let entry = entry.with_context(|| {
            format!(
                "failed to read entry in workspace config directory '{}'",
                config_dir.display()
            )
        })?;
        let path = entry.path();
        if path.extension() != Some(OsStr::new("yaml")) {
            continue;
        }
        let Some(name) = path.file_stem().and_then(OsStr::to_str) else {
            continue;
        };
        entries.push(WorkspaceEntry::new(name.to_owned(), path));
    }

    entries.sort_by(|left, right| left.name().cmp(right.name()));
    Ok(entries)
}

/// Create a saved workspace manifest if needed and open it in an editor.
///
/// # Arguments
///
/// * `sessions_root` - codex-ws state root.
/// * `workspace_name` - Workspace name used for the manifest file.
///
/// # Returns
///
/// Path to the saved workspace manifest.
///
/// # Errors
///
/// Returns an error when the file cannot be created or the editor exits unsuccessfully.
pub fn add_workspace(sessions_root: &Path, workspace_name: &str) -> Result<PathBuf> {
    add_workspace_with_editor(sessions_root, workspace_name, selected_editor())
}

fn add_workspace_with_editor(
    sessions_root: &Path,
    workspace_name: &str,
    editor: String,
) -> Result<PathBuf> {
    let manifest_path = workspace_manifest_path(sessions_root, workspace_name)?;
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create workspace config directory '{}'",
                parent.display()
            )
        })?;
    }

    if !manifest_path.exists() {
        fs::write(&manifest_path, workspace_template(workspace_name)).with_context(|| {
            format!(
                "failed to write workspace manifest template '{}'",
                manifest_path.display()
            )
        })?;
    }

    open_editor(&editor, &manifest_path)?;
    Ok(manifest_path)
}

fn workspace_template(workspace_name: &str) -> String {
    format!(
        r#"# Workspace manifest for codex-ws.
# Replace the folder examples with absolute host paths.
name: {workspace_name}
folders:
  - /absolute/path/to/project

# The container has network access by default so Codex can reach the model provider.
# Advanced offline-only configuration:
# sandbox:
#   network: false

# Optional runtime setup for the lightweight Ubuntu image.
# runtime:
#   apt:
#     - python3
#     - python3-pip
#   setup:
#     - python3 -m pip install --user maturin
"#
    )
}

fn open_editor(editor: &str, path: &Path) -> Result<()> {
    let status = Command::new(editor)
        .arg(path)
        .status()
        .with_context(|| format!("failed to launch editor '{editor}'"))?;
    if status.success() {
        return Ok(());
    }

    Err(anyhow!(
        "editor '{editor}' exited unsuccessfully while editing '{}'",
        path.display()
    ))
}

fn selected_editor() -> String {
    std::env::var("VISUAL")
        .ok()
        .filter(|editor| !editor.trim().is_empty())
        .or_else(|| {
            std::env::var("EDITOR")
                .ok()
                .filter(|editor| !editor.trim().is_empty())
        })
        .unwrap_or_else(|| "vim".to_owned())
}

fn validate_workspace_name(workspace_name: &str) -> Result<()> {
    if workspace_name.trim().is_empty() {
        return Err(anyhow!("workspace name cannot be empty"));
    }
    if Path::new(workspace_name)
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
        || workspace_name.contains('/')
        || workspace_name.contains('\\')
    {
        return Err(anyhow!(
            "workspace name '{workspace_name}' cannot contain path separators"
        ));
    }
    Ok(())
}

fn is_path_like(path: &Path) -> bool {
    if path.is_absolute() {
        return true;
    }
    let Some(path_text) = path.to_str() else {
        return true;
    };
    path_text == "~"
        || path_text.starts_with("~/")
        || path_text.starts_with("./")
        || path_text.starts_with("../")
        || path_text.contains('/')
        || path_text.contains('\\')
        || path.extension().is_some()
}

/// Expand a leading `~` in a path.
///
/// # Arguments
///
/// * `path` - Path that may start with `~` or `~/`.
///
/// # Returns
///
/// The path with a leading home-directory marker expanded when possible.
#[must_use]
pub fn expand_home_path(path: PathBuf) -> PathBuf {
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
    directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn workspace_manifest_path_uses_config_workspace_directory() {
        let path = workspace_manifest_path(Path::new("/host/.codex-ws"), "backend")
            .expect("path should build");

        assert_eq!(
            path,
            PathBuf::from("/host/.codex-ws/config/workspace/backend.yaml")
        );
    }

    #[test]
    fn resolve_workspace_path_maps_names_to_saved_manifest_paths() {
        let path = resolve_workspace_path(PathBuf::from("backend"), Path::new("/host/.codex-ws"))
            .expect("path should resolve");

        assert_eq!(
            path,
            PathBuf::from("/host/.codex-ws/config/workspace/backend.yaml")
        );
    }

    #[test]
    fn resolve_workspace_path_keeps_path_like_values() {
        let path = resolve_workspace_path(
            PathBuf::from("/tmp/workspace.yaml"),
            Path::new("/host/.codex-ws"),
        )
        .expect("path should resolve");

        assert_eq!(path, PathBuf::from("/tmp/workspace.yaml"));
    }

    #[test]
    fn list_workspaces_returns_sorted_yaml_files() {
        let temp_dir = TestTempDir::create();
        let config_dir = workspace_config_dir(temp_dir.path());
        fs::create_dir_all(&config_dir).expect("config dir should be created");
        fs::write(config_dir.join("zeta.yaml"), "").expect("workspace should be written");
        fs::write(config_dir.join("alpha.yaml"), "").expect("workspace should be written");
        fs::write(config_dir.join("ignored.txt"), "").expect("ignored file should be written");

        let entries = list_workspaces(temp_dir.path()).expect("workspaces should list");

        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.name().to_owned())
                .collect::<Vec<_>>(),
            vec!["alpha".to_owned(), "zeta".to_owned()]
        );
    }

    #[test]
    fn add_workspace_writes_template_without_overwriting_existing_file() {
        let temp_dir = TestTempDir::create();
        let editor = "true".to_owned();
        let path = add_workspace_with_editor(temp_dir.path(), "backend", editor.clone())
            .expect("workspace should be added");

        let first_content = fs::read_to_string(&path).expect("workspace should be readable");
        assert!(first_content.contains("name: backend"));

        fs::write(&path, "name: custom\n").expect("workspace should be overwritten for test");
        add_workspace_with_editor(temp_dir.path(), "backend", editor)
            .expect("existing workspace should open");

        assert_eq!(
            fs::read_to_string(&path).expect("workspace should be readable"),
            "name: custom\n"
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
                "codex-ws-workspace-test-{}-{timestamp}-{counter}",
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
