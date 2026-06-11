use std::collections::HashSet;

use thiserror::Error;

/// One supported language runtime selected for a workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLanguageVersion {
    language: RuntimeLanguage,
    version: String,
}

impl RuntimeLanguageVersion {
    /// Parse a runtime language specification.
    ///
    /// # Arguments
    ///
    /// * `spec` - Runtime spec in `language:version` form.
    ///
    /// # Returns
    ///
    /// A validated runtime language version.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeSpecError`] when the spec format, language, or version is unsupported.
    pub fn parse(spec: &str) -> Result<Self, RuntimeSpecError> {
        let Some((language_text, version_text)) = spec.split_once(':') else {
            return Err(RuntimeSpecError::InvalidFormat {
                spec: spec.to_owned(),
            });
        };
        let language = RuntimeLanguage::parse(language_text.trim())?;
        let version = version_text.trim();
        if version.is_empty() {
            return Err(RuntimeSpecError::InvalidFormat {
                spec: spec.to_owned(),
            });
        }

        if !language.supports_version(version) {
            return Err(RuntimeSpecError::UnsupportedVersion {
                language,
                version: version.to_owned(),
            });
        }

        Ok(Self {
            language,
            version: version.to_owned(),
        })
    }

    /// Return the selected runtime language.
    ///
    /// # Returns
    ///
    /// The supported runtime language.
    #[must_use]
    pub const fn language(&self) -> RuntimeLanguage {
        self.language
    }

    /// Return the selected runtime version.
    ///
    /// # Returns
    ///
    /// The exact version string supported by Codex Universal.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Convert this runtime spec into a Docker environment variable.
    ///
    /// # Returns
    ///
    /// The `CODEX_ENV_*` variable consumed by Codex Universal.
    #[must_use]
    pub fn environment_variable(&self) -> RuntimeEnvironmentVariable {
        RuntimeEnvironmentVariable::new(self.language.environment_variable(), self.version.clone())
    }
}

/// Docker environment variable generated from a workspace runtime spec.
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
    /// * `name` - Environment variable name recognized by Codex Universal.
    /// * `value` - Runtime version value.
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
    /// The `CODEX_ENV_*` variable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Return the environment variable value.
    ///
    /// # Returns
    ///
    /// The selected runtime version.
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

/// Supported Codex Universal runtime languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeLanguage {
    /// Python via `CODEX_ENV_PYTHON_VERSION`.
    Python,
    /// Node.js via `CODEX_ENV_NODE_VERSION`.
    Node,
    /// Rust via `CODEX_ENV_RUST_VERSION`.
    Rust,
    /// Go via `CODEX_ENV_GO_VERSION`.
    Go,
    /// Swift via `CODEX_ENV_SWIFT_VERSION`.
    Swift,
    /// Ruby via `CODEX_ENV_RUBY_VERSION`.
    Ruby,
    /// PHP via `CODEX_ENV_PHP_VERSION`.
    Php,
    /// Java via `CODEX_ENV_JAVA_VERSION`.
    Java,
}

impl RuntimeLanguage {
    /// Parse a supported runtime language or alias.
    ///
    /// # Arguments
    ///
    /// * `language` - Language name from a workspace runtime spec.
    ///
    /// # Returns
    ///
    /// A supported runtime language.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeSpecError::UnsupportedLanguage`] when the language is unknown.
    pub fn parse(language: &str) -> Result<Self, RuntimeSpecError> {
        match language.to_ascii_lowercase().as_str() {
            "python" | "python3" | "py" => Ok(Self::Python),
            "node" | "nodejs" | "javascript" | "js" => Ok(Self::Node),
            "rust" | "rustlang" => Ok(Self::Rust),
            "go" | "golang" => Ok(Self::Go),
            "swift" => Ok(Self::Swift),
            "ruby" | "rb" => Ok(Self::Ruby),
            "php" => Ok(Self::Php),
            "java" | "jdk" => Ok(Self::Java),
            _ => Err(RuntimeSpecError::UnsupportedLanguage {
                language: language.to_owned(),
            }),
        }
    }

    /// Return the Codex Universal environment variable for this language.
    ///
    /// # Returns
    ///
    /// The `CODEX_ENV_*` variable name.
    #[must_use]
    pub const fn environment_variable(self) -> &'static str {
        match self {
            Self::Python => "CODEX_ENV_PYTHON_VERSION",
            Self::Node => "CODEX_ENV_NODE_VERSION",
            Self::Rust => "CODEX_ENV_RUST_VERSION",
            Self::Go => "CODEX_ENV_GO_VERSION",
            Self::Swift => "CODEX_ENV_SWIFT_VERSION",
            Self::Ruby => "CODEX_ENV_RUBY_VERSION",
            Self::Php => "CODEX_ENV_PHP_VERSION",
            Self::Java => "CODEX_ENV_JAVA_VERSION",
        }
    }

    /// Return the canonical language name.
    ///
    /// # Returns
    ///
    /// Lowercase language name used in error messages.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::Node => "node",
            Self::Rust => "rust",
            Self::Go => "go",
            Self::Swift => "swift",
            Self::Ruby => "ruby",
            Self::Php => "php",
            Self::Java => "java",
        }
    }

    /// Return supported versions for this language.
    ///
    /// # Returns
    ///
    /// Versions supported by the current Codex Universal support matrix.
    #[must_use]
    pub const fn supported_versions(self) -> &'static [&'static str] {
        match self {
            Self::Python => &["3.10", "3.11.12", "3.12", "3.13", "3.14.0"],
            Self::Node => &["18", "20", "22"],
            Self::Rust => &[
                "1.83.0", "1.84.1", "1.85.1", "1.86.0", "1.87.0", "1.88.0", "1.89.0", "1.90",
                "1.91.1", "1.92.0", "1.93.0", "1.94.0", "1.95.0",
            ],
            Self::Go => &["1.22.12", "1.23.8", "1.24.3", "1.25.1"],
            Self::Swift => &["5.10", "6.1", "6.2"],
            Self::Ruby => &["3.2.3", "3.3.8", "3.4.4"],
            Self::Php => &["8.4", "8.3", "8.2"],
            Self::Java => &["25", "24", "23", "22", "21", "17", "11"],
        }
    }

    fn supports_version(self, version: &str) -> bool {
        self.supported_versions().contains(&version)
    }
}

/// Validate a list of runtime language specs.
///
/// # Arguments
///
/// * `specs` - Runtime specs in `language:version` form.
///
/// # Returns
///
/// Runtime language versions in the same order as the input.
///
/// # Errors
///
/// Returns [`RuntimeSpecError`] when any spec is invalid or configures a language twice.
pub fn parse_runtime_specs(
    specs: &[String],
) -> Result<Vec<RuntimeLanguageVersion>, RuntimeSpecError> {
    let mut languages = HashSet::with_capacity(specs.len());
    let mut runtimes = Vec::with_capacity(specs.len());

    for spec in specs {
        let runtime = RuntimeLanguageVersion::parse(spec)?;
        if !languages.insert(runtime.language()) {
            return Err(RuntimeSpecError::DuplicateLanguage {
                language: runtime.language(),
            });
        }
        runtimes.push(runtime);
    }

    Ok(runtimes)
}

/// Errors returned while parsing workspace runtime specs.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RuntimeSpecError {
    /// Runtime spec did not use `language:version` form.
    #[error("invalid runtime spec '{spec}', expected language:version")]
    InvalidFormat {
        /// Invalid runtime spec.
        spec: String,
    },

    /// Runtime language is not supported by Codex Universal.
    #[error("unsupported runtime language '{language}'")]
    UnsupportedLanguage {
        /// Unsupported language name.
        language: String,
    },

    /// Runtime version is not supported for a language.
    #[error(
        "unsupported {language} runtime version '{version}', supported versions: {supported_versions}",
        language = .language.name(),
        supported_versions = .language.supported_versions().join(", ")
    )]
    UnsupportedVersion {
        /// Runtime language.
        language: RuntimeLanguage,
        /// Unsupported version.
        version: String,
    },

    /// A language was configured more than once.
    #[error("runtime language '{language}' was configured more than once", language = .language.name())]
    DuplicateLanguage {
        /// Duplicated language.
        language: RuntimeLanguage,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_runtime_spec_maps_golang_to_codex_env_go_version() {
        let runtime =
            RuntimeLanguageVersion::parse("golang:1.25.1").expect("runtime spec should parse");

        assert_eq!(runtime.language(), RuntimeLanguage::Go);
        assert_eq!(runtime.version(), "1.25.1");
        assert_eq!(
            runtime.environment_variable(),
            RuntimeEnvironmentVariable::new("CODEX_ENV_GO_VERSION", "1.25.1".to_owned())
        );
    }

    #[test]
    fn parse_runtime_specs_rejects_unsupported_versions() {
        let error = RuntimeLanguageVersion::parse("go:1.99.0")
            .expect_err("unsupported version should fail");

        assert!(matches!(
            error,
            RuntimeSpecError::UnsupportedVersion {
                language: RuntimeLanguage::Go,
                version
            } if version == "1.99.0"
        ));
    }

    #[test]
    fn parse_runtime_specs_rejects_duplicate_languages() {
        let error = parse_runtime_specs(&["node:20".to_owned(), "nodejs:22".to_owned()])
            .expect_err("duplicate language should fail");

        assert!(matches!(
            error,
            RuntimeSpecError::DuplicateLanguage {
                language: RuntimeLanguage::Node
            }
        ));
    }
}
