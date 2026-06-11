use thiserror::Error;

/// Environment variable used by the runtime entrypoint for apt packages.
pub const CODEX_WS_APT_PACKAGES_ENV: &str = "CODEX_WS_APT_PACKAGES";

/// Environment variable used by the runtime entrypoint for setup commands.
pub const CODEX_WS_SETUP_COMMANDS_ENV: &str = "CODEX_WS_SETUP_COMMANDS";

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

fn is_valid_apt_package(package: &str) -> bool {
    package.bytes().all(|byte| {
        byte.is_ascii_alphanumeric()
            || matches!(byte, b'+' | b'-' | b'.' | b'_' | b':' | b'=' | b'~')
    })
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
}
