use thiserror::Error;

/// Environment variable used by the runtime entrypoint for apt packages.
pub const CODEX_WS_APT_PACKAGES_ENV: &str = "CODEX_WS_APT_PACKAGES";

/// Environment variable used by the runtime entrypoint for setup commands.
pub const CODEX_WS_SETUP_COMMANDS_ENV: &str = "CODEX_WS_SETUP_COMMANDS";

/// Environment variable used by the runtime entrypoint for Python version selection.
pub const CODEX_WS_PYTHON_VERSION_ENV: &str = "CODEX_WS_PYTHON_VERSION";

/// Environment variable used by the runtime entrypoint for Node.js version selection.
pub const CODEX_WS_NODE_VERSION_ENV: &str = "CODEX_WS_NODE_VERSION";

/// Environment variable used by the runtime entrypoint for Go version selection.
pub const CODEX_WS_GO_VERSION_ENV: &str = "CODEX_WS_GO_VERSION";

/// Environment variable used by the runtime entrypoint for Rust version selection.
pub const CODEX_WS_RUST_VERSION_ENV: &str = "CODEX_WS_RUST_VERSION";

/// Environment variable used by the runtime entrypoint for Java version selection.
pub const CODEX_WS_JAVA_VERSION_ENV: &str = "CODEX_WS_JAVA_VERSION";

/// Environment variable used by the runtime entrypoint for Clang version selection.
pub const CODEX_WS_CLANG_VERSION_ENV: &str = "CODEX_WS_CLANG_VERSION";

/// Environment variable used by the runtime entrypoint for C compiler version selection.
pub const CODEX_WS_C_VERSION_ENV: &str = "CODEX_WS_C_VERSION";

/// Environment variable used by the runtime entrypoint for C++ compiler version selection.
pub const CODEX_WS_CPP_VERSION_ENV: &str = "CODEX_WS_CPP_VERSION";

/// Environment variable used by the runtime entrypoint for Ruby version selection.
pub const CODEX_WS_RUBY_VERSION_ENV: &str = "CODEX_WS_RUBY_VERSION";

/// Environment variable used by the runtime entrypoint for PHP version selection.
pub const CODEX_WS_PHP_VERSION_ENV: &str = "CODEX_WS_PHP_VERSION";

/// Environment variable used by the runtime entrypoint for Deno version selection.
pub const CODEX_WS_DENO_VERSION_ENV: &str = "CODEX_WS_DENO_VERSION";

/// Environment variable used by the runtime entrypoint for Bun version selection.
pub const CODEX_WS_BUN_VERSION_ENV: &str = "CODEX_WS_BUN_VERSION";

/// Environment variable used by the runtime entrypoint for Zig version selection.
pub const CODEX_WS_ZIG_VERSION_ENV: &str = "CODEX_WS_ZIG_VERSION";

/// Environment variable used by the runtime entrypoint for .NET version selection.
pub const CODEX_WS_DOTNET_VERSION_ENV: &str = "CODEX_WS_DOTNET_VERSION";

/// Docker environment variable generated from a workspace runtime config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEnvironmentVariable {
    name: &'static str,
    value: String,
}

impl RuntimeEnvironmentVariable {
    /// Create a runtime environment variable.
    ///
    /// # Arguments
    ///
    /// * `name` - Environment variable name recognized by the runtime entrypoint.
    /// * `value` - Environment variable value passed to Docker.
    ///
    /// # Returns
    ///
    /// A runtime environment variable.
    #[must_use]
    pub fn new(name: &'static str, value: String) -> Self {
        Self { name, value }
    }

    /// Return the environment variable name.
    ///
    /// # Returns
    ///
    /// The runtime entrypoint environment variable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Return the environment variable value.
    ///
    /// # Returns
    ///
    /// The value passed to Docker.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Return the `NAME=value` form accepted by `docker run -e`.
    ///
    /// # Returns
    ///
    /// A Docker environment assignment.
    #[must_use]
    pub fn docker_assignment(&self) -> String {
        format!("{}={}", self.name, self.value)
    }
}

/// Validate apt package names from a workspace manifest.
///
/// # Arguments
///
/// * `packages` - Package names requested by `runtime.apt`.
///
/// # Returns
///
/// Trimmed package names in input order.
///
/// # Errors
///
/// Returns [`RuntimeSpecError::EmptyAptPackage`] for a blank package or
/// [`RuntimeSpecError::InvalidAptPackage`] when a package contains shell metacharacters.
pub fn validate_apt_packages(packages: Vec<String>) -> Result<Vec<String>, RuntimeSpecError> {
    let mut validated_packages = Vec::with_capacity(packages.len());
    for package in packages {
        let package = package.trim().to_owned();
        if package.is_empty() {
            return Err(RuntimeSpecError::EmptyAptPackage);
        }
        if !is_valid_apt_package(&package) {
            return Err(RuntimeSpecError::InvalidAptPackage { package });
        }
        validated_packages.push(package);
    }

    Ok(validated_packages)
}

/// Validate setup commands from a workspace manifest.
///
/// # Arguments
///
/// * `commands` - Shell commands requested by `runtime.setup`.
///
/// # Returns
///
/// Trimmed commands in input order.
///
/// # Errors
///
/// Returns [`RuntimeSpecError::EmptySetupCommand`] when a setup command is blank.
pub fn validate_setup_commands(commands: Vec<String>) -> Result<Vec<String>, RuntimeSpecError> {
    let mut validated_commands = Vec::with_capacity(commands.len());
    for command in commands {
        let command = command.trim().to_owned();
        if command.is_empty() {
            return Err(RuntimeSpecError::EmptySetupCommand);
        }
        validated_commands.push(command);
    }

    Ok(validated_commands)
}

/// Validate a runtime tool version from a workspace manifest.
///
/// # Arguments
///
/// * `tool` - Tool name used in diagnostics.
/// * `version` - Optional version requested by the workspace manifest.
///
/// # Returns
///
/// A trimmed version when one was configured.
///
/// # Errors
///
/// Returns [`RuntimeSpecError::EmptyToolVersion`] for a blank version or
/// [`RuntimeSpecError::InvalidToolVersion`] when the version contains shell metacharacters.
pub fn validate_tool_version(
    tool: RuntimeTool,
    version: Option<String>,
) -> Result<Option<RuntimeToolVersion>, RuntimeSpecError> {
    let Some(version) = version else {
        return Ok(None);
    };
    let version = version.trim().to_owned();
    if version.is_empty() {
        return Err(RuntimeSpecError::EmptyToolVersion { tool });
    }
    if !is_valid_tool_version(&version) {
        return Err(RuntimeSpecError::InvalidToolVersion { tool, version });
    }

    Ok(Some(RuntimeToolVersion::new(tool, version)))
}

/// Validate a set of requested runtime tool versions.
///
/// # Arguments
///
/// * `versions` - Runtime tool versions collected from a workspace manifest.
///
/// # Returns
///
/// Runtime tool versions in input order.
///
/// # Errors
///
/// Returns [`RuntimeSpecError::ConflictingCompilerVersions`] when `c`, `cpp`, and `clang`
/// request different LLVM Clang versions.
pub fn validate_runtime_tool_versions(
    versions: Vec<RuntimeToolVersion>,
) -> Result<Vec<RuntimeToolVersion>, RuntimeSpecError> {
    let mut clang_version: Option<&str> = None;

    for version in &versions {
        if !version.tool().uses_clang() {
            continue;
        }
        if let Some(existing_version) = clang_version
            && existing_version != version.version()
        {
            return Err(RuntimeSpecError::ConflictingCompilerVersions {
                first: existing_version.to_owned(),
                second: version.version().to_owned(),
            });
        }
        clang_version = Some(version.version());
    }

    Ok(versions)
}

fn is_valid_apt_package(package: &str) -> bool {
    package.bytes().all(|byte| {
        byte.is_ascii_alphanumeric()
            || matches!(byte, b'+' | b'-' | b'.' | b'_' | b':' | b'=' | b'~')
    })
}

fn is_valid_tool_version(version: &str) -> bool {
    version.bytes().all(|byte| {
        byte.is_ascii_alphanumeric()
            || matches!(byte, b'+' | b'-' | b'.' | b'_' | b':' | b'/' | b'@')
    })
}

/// Runtime tools that can be installed declaratively.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeTool {
    /// Python installed with uv.
    Python,
    /// Node.js installed with mise.
    Node,
    /// Go installed with mise.
    Go,
    /// Rust installed with mise.
    Rust,
    /// Java installed with mise.
    Java,
    /// Clang installed with LLVM apt packages.
    Clang,
    /// C compiler installed with LLVM apt packages.
    C,
    /// C++ compiler installed with LLVM apt packages.
    Cpp,
    /// Ruby installed with mise.
    Ruby,
    /// PHP installed with mise.
    Php,
    /// Deno installed with mise.
    Deno,
    /// Bun installed with mise.
    Bun,
    /// Zig installed with mise.
    Zig,
    /// .NET SDK installed with mise.
    Dotnet,
}

impl RuntimeTool {
    /// Return the manifest field name for this tool.
    ///
    /// # Returns
    ///
    /// Lowercase tool name used in workspace manifests.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::Node => "node",
            Self::Go => "go",
            Self::Rust => "rust",
            Self::Java => "java",
            Self::Clang => "clang",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Ruby => "ruby",
            Self::Php => "php",
            Self::Deno => "deno",
            Self::Bun => "bun",
            Self::Zig => "zig",
            Self::Dotnet => "dotnet",
        }
    }

    /// Return the environment variable consumed by the runtime entrypoint.
    ///
    /// # Returns
    ///
    /// Environment variable name for this declarative runtime tool.
    #[must_use]
    pub const fn environment_variable(self) -> &'static str {
        match self {
            Self::Python => CODEX_WS_PYTHON_VERSION_ENV,
            Self::Node => CODEX_WS_NODE_VERSION_ENV,
            Self::Go => CODEX_WS_GO_VERSION_ENV,
            Self::Rust => CODEX_WS_RUST_VERSION_ENV,
            Self::Java => CODEX_WS_JAVA_VERSION_ENV,
            Self::Clang => CODEX_WS_CLANG_VERSION_ENV,
            Self::C => CODEX_WS_C_VERSION_ENV,
            Self::Cpp => CODEX_WS_CPP_VERSION_ENV,
            Self::Ruby => CODEX_WS_RUBY_VERSION_ENV,
            Self::Php => CODEX_WS_PHP_VERSION_ENV,
            Self::Deno => CODEX_WS_DENO_VERSION_ENV,
            Self::Bun => CODEX_WS_BUN_VERSION_ENV,
            Self::Zig => CODEX_WS_ZIG_VERSION_ENV,
            Self::Dotnet => CODEX_WS_DOTNET_VERSION_ENV,
        }
    }

    const fn uses_clang(self) -> bool {
        matches!(self, Self::Clang | Self::C | Self::Cpp)
    }
}

/// One declarative runtime tool version selected for a workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeToolVersion {
    tool: RuntimeTool,
    version: String,
}

impl RuntimeToolVersion {
    /// Create a runtime tool version.
    ///
    /// # Arguments
    ///
    /// * `tool` - Runtime tool requested by the workspace.
    /// * `version` - Version string passed to the runtime installer.
    ///
    /// # Returns
    ///
    /// A runtime tool version.
    #[must_use]
    pub fn new(tool: RuntimeTool, version: String) -> Self {
        Self { tool, version }
    }

    /// Return the requested runtime tool.
    ///
    /// # Returns
    ///
    /// Runtime tool enum value.
    #[must_use]
    pub const fn tool(&self) -> RuntimeTool {
        self.tool
    }

    /// Return the requested tool version.
    ///
    /// # Returns
    ///
    /// Version string passed to the runtime installer.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Convert this runtime tool into a Docker environment variable.
    ///
    /// # Returns
    ///
    /// Runtime entrypoint environment variable.
    #[must_use]
    pub fn environment_variable(&self) -> RuntimeEnvironmentVariable {
        RuntimeEnvironmentVariable::new(self.tool.environment_variable(), self.version.clone())
    }
}

/// Errors returned while parsing workspace runtime setup.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RuntimeSpecError {
    /// An apt package entry was empty or only whitespace.
    #[error("runtime apt package cannot be empty")]
    EmptyAptPackage,

    /// An apt package entry contained characters that are unsafe for shell word splitting.
    #[error("invalid runtime apt package '{package}'")]
    InvalidAptPackage {
        /// Invalid package entry.
        package: String,
    },

    /// A setup command was empty or only whitespace.
    #[error("runtime setup command cannot be empty")]
    EmptySetupCommand,

    /// A declarative runtime tool version was empty or only whitespace.
    #[error("runtime {tool} version cannot be empty", tool = .tool.name())]
    EmptyToolVersion {
        /// Tool with an empty version.
        tool: RuntimeTool,
    },

    /// A declarative runtime tool version contained unsafe characters.
    #[error("invalid runtime {tool} version '{version}'", tool = .tool.name())]
    InvalidToolVersion {
        /// Tool with an invalid version.
        tool: RuntimeTool,
        /// Invalid version text.
        version: String,
    },

    /// Multiple C/C++ compiler aliases requested different Clang versions.
    #[error("conflicting C/C++ runtime versions '{first}' and '{second}'")]
    ConflictingCompilerVersions {
        /// First configured compiler version.
        first: String,
        /// Conflicting compiler version.
        second: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_apt_packages_accepts_common_package_syntax() {
        let packages = validate_apt_packages(vec![
            " python3 ".to_owned(),
            "libssl-dev:amd64".to_owned(),
            "nodejs=22.0.0-1nodesource1".to_owned(),
        ])
        .expect("apt packages should validate");

        assert_eq!(
            packages,
            vec![
                "python3".to_owned(),
                "libssl-dev:amd64".to_owned(),
                "nodejs=22.0.0-1nodesource1".to_owned()
            ]
        );
    }

    #[test]
    fn validate_apt_packages_rejects_shell_metacharacters() {
        let error = validate_apt_packages(vec!["python3;curl".to_owned()])
            .expect_err("shell metacharacters should fail");

        assert!(matches!(
            error,
            RuntimeSpecError::InvalidAptPackage { package } if package == "python3;curl"
        ));
    }

    #[test]
    fn validate_setup_commands_rejects_empty_commands() {
        let error =
            validate_setup_commands(vec![" ".to_owned()]).expect_err("blank command should fail");

        assert_eq!(error, RuntimeSpecError::EmptySetupCommand);
    }

    #[test]
    fn validate_tool_version_trims_versions() {
        let version = validate_tool_version(RuntimeTool::Python, Some(" 3.13 ".to_owned()))
            .expect("version should validate");

        assert_eq!(
            version,
            Some(RuntimeToolVersion::new(
                RuntimeTool::Python,
                "3.13".to_owned()
            ))
        );
    }

    #[test]
    fn validate_tool_version_rejects_shell_metacharacters() {
        let error = validate_tool_version(RuntimeTool::Go, Some("1.24;curl".to_owned()))
            .expect_err("shell metacharacters should fail");

        assert!(matches!(
            error,
            RuntimeSpecError::InvalidToolVersion {
                tool: RuntimeTool::Go,
                version
            } if version == "1.24;curl"
        ));
    }

    #[test]
    fn validate_runtime_tool_versions_accepts_matching_compiler_aliases() {
        let versions = validate_runtime_tool_versions(vec![
            RuntimeToolVersion::new(RuntimeTool::C, "20".to_owned()),
            RuntimeToolVersion::new(RuntimeTool::Cpp, "20".to_owned()),
        ])
        .expect("matching compiler aliases should validate");

        assert_eq!(versions.len(), 2);
    }

    #[test]
    fn validate_runtime_tool_versions_rejects_conflicting_compiler_aliases() {
        let error = validate_runtime_tool_versions(vec![
            RuntimeToolVersion::new(RuntimeTool::C, "20".to_owned()),
            RuntimeToolVersion::new(RuntimeTool::Cpp, "21".to_owned()),
        ])
        .expect_err("conflicting compiler aliases should fail");

        assert!(matches!(
            error,
            RuntimeSpecError::ConflictingCompilerVersions { first, second }
                if first == "20" && second == "21"
        ));
    }
}
