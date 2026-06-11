use std::path::Path;

use rusqlite::Connection;
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

/// Codex provider configuration loaded from the local configuration database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexProvider {
    name: String,
    auth_json: String,
    config_toml: String,
}

impl CodexProvider {
    /// Create a Codex provider configuration.
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable provider name from the configuration database.
    /// * `auth_json` - Auth JSON payload for the Codex CLI.
    /// * `config_toml` - Runtime TOML configuration payload for the Codex CLI.
    ///
    /// # Returns
    ///
    /// A provider value with owned fields.
    #[must_use]
    pub fn new(name: String, auth_json: String, config_toml: String) -> Self {
        Self {
            name,
            auth_json,
            config_toml,
        }
    }

    /// Return the provider name.
    ///
    /// # Returns
    ///
    /// The provider name as a borrowed string slice.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the provider auth JSON payload.
    ///
    /// # Returns
    ///
    /// The provider auth JSON payload as a borrowed string slice.
    #[must_use]
    pub fn auth_json(&self) -> &str {
        &self.auth_json
    }

    /// Return the provider config TOML payload.
    ///
    /// # Returns
    ///
    /// The provider config TOML payload as a borrowed string slice.
    #[must_use]
    pub fn config_toml(&self) -> &str {
        &self.config_toml
    }
}

/// Errors returned while loading Codex provider configuration.
#[derive(Debug, Error)]
pub enum ProviderError {
    /// The configuration database could not be opened or queried.
    #[error("configuration database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// The provider settings JSON did not match the expected shape.
    #[error("invalid settings JSON for provider '{provider_name}': {source}")]
    SettingsJson {
        /// Provider name associated with the invalid settings payload.
        provider_name: String,
        /// JSON parsing error returned by `serde_json`.
        source: serde_json::Error,
    },

    /// The providers table does not expose a supported Codex configuration shape.
    #[error("providers table must contain a settings_config column")]
    UnsupportedSchema,
}

#[derive(Debug, Deserialize)]
struct ProviderSettingsConfig {
    auth: Value,
    config: String,
}

/// Load all Codex providers from a local configuration database.
///
/// # Arguments
///
/// * `database_path` - Path to the SQLite database containing a `providers` table.
///
/// # Returns
///
/// A vector of Codex provider configurations, preserving database row order.
///
/// # Errors
///
/// Returns [`ProviderError::Database`] when the database cannot be opened or queried.
/// Returns [`ProviderError::SettingsJson`] when a Codex provider has invalid settings JSON.
pub fn load_codex_providers(database_path: &Path) -> Result<Vec<CodexProvider>, ProviderError> {
    let connection = Connection::open(database_path)?;
    load_codex_providers_from_connection(&connection)
}

/// Load all Codex providers from an existing SQLite connection.
///
/// # Arguments
///
/// * `connection` - Open SQLite connection containing a `providers` table.
///
/// # Returns
///
/// A vector of Codex provider configurations, preserving database row order.
///
/// # Errors
///
/// Returns [`ProviderError::Database`] when the table cannot be queried.
/// Returns [`ProviderError::SettingsJson`] when a Codex provider has invalid settings JSON.
pub fn load_codex_providers_from_connection(
    connection: &Connection,
) -> Result<Vec<CodexProvider>, ProviderError> {
    let columns = provider_table_columns(connection)?;
    if columns.iter().any(|column| column == "settings_config") {
        return load_codex_providers_from_settings_config_column(connection);
    }

    Err(ProviderError::UnsupportedSchema)
}

fn provider_table_columns(connection: &Connection) -> Result<Vec<String>, ProviderError> {
    let mut statement = connection.prepare("PRAGMA table_info(providers)")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    let mut columns = Vec::new();
    for row in rows {
        columns.push(row?);
    }
    Ok(columns)
}

fn load_codex_providers_from_settings_config_column(
    connection: &Connection,
) -> Result<Vec<CodexProvider>, ProviderError> {
    let mut statement = connection.prepare(
        "SELECT name, settings_config FROM providers WHERE app_type = ?1 ORDER BY rowid ASC",
    )?;
    let rows = statement.query_map(["codex"], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut providers = Vec::new();
    for row in rows {
        let (name, settings_json) = row?;
        let settings = parse_settings(&name, &settings_json)?;
        let auth_json = serde_json::to_string_pretty(&settings.auth).map_err(|source| {
            ProviderError::SettingsJson {
                provider_name: name.clone(),
                source,
            }
        })?;
        providers.push(CodexProvider::new(name, auth_json, settings.config));
    }

    Ok(providers)
}

fn parse_settings(
    provider_name: &str,
    settings_json: &str,
) -> Result<ProviderSettingsConfig, ProviderError> {
    serde_json::from_str(settings_json).map_err(|source| ProviderError::SettingsJson {
        provider_name: provider_name.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection_with_providers() -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory SQLite should open");
        connection
            .execute(
                "CREATE TABLE providers (
                    name TEXT NOT NULL,
                    app_type TEXT NOT NULL,
                    settings_config TEXT NOT NULL
                )",
                [],
            )
            .expect("providers table should be created");
        connection
    }

    #[test]
    fn load_codex_providers_filters_and_maps_rows() {
        let connection = connection_with_providers();
        connection
            .execute(
                "INSERT INTO providers (name, app_type, settings_config) VALUES (?1, ?2, ?3)",
                [
                    "primary",
                    "codex",
                    r#"{"auth":{"OPENAI_API_KEY":"test-key"},"config":"model = \"gpt-5.5\"\n"}"#,
                ],
            )
            .expect("codex provider row should insert");
        connection
            .execute(
                "INSERT INTO providers (name, app_type, settings_config) VALUES (?1, ?2, ?3)",
                [
                    "other",
                    "claude",
                    r#"{"auth":{"OPENAI_API_KEY":"ignored"},"config":"ignored"}"#,
                ],
            )
            .expect("non-codex provider row should insert");

        let providers =
            load_codex_providers_from_connection(&connection).expect("providers should load");

        assert_eq!(
            providers,
            vec![CodexProvider::new(
                "primary".to_owned(),
                "{\n  \"OPENAI_API_KEY\": \"test-key\"\n}".to_owned(),
                "model = \"gpt-5.5\"\n".to_owned()
            )]
        );
    }

    #[test]
    fn load_codex_providers_reports_invalid_settings_json() {
        let connection = connection_with_providers();
        connection
            .execute(
                "INSERT INTO providers (name, app_type, settings_config) VALUES (?1, ?2, ?3)",
                ["broken", "codex", "{}"],
            )
            .expect("broken provider row should insert");

        let error = load_codex_providers_from_connection(&connection)
            .expect_err("invalid settings should fail");

        assert!(matches!(
            error,
            ProviderError::SettingsJson { provider_name, .. } if provider_name == "broken"
        ));
    }
}
