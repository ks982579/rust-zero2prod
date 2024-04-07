//! src/configuration.rs
use std::path::PathBuf;

use secrecy::{ExposeSecret, Secret};

const APP_ENVIRONMENT: &str = "APP_ENVIRONMENT";

/// Possible runtime Environments for application.
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;
    // type Error is formality for what is returned in event of conversion error.

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. \
                Please use either `local` or `production`.",
                other
            )),
        }
    }
}

/// 2 config values: Application Port; Database Connection;
#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
}

/// Needs to be desearlized so it's _parent_ can also be desearlized.
#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path: PathBuf =
        std::env::current_dir().expect("Failed to determine current directory?");
    let configuration_directory: PathBuf = base_path.join("configuration");

    // Detect running environment and default to "local"
    let environment: Environment = std::env::var(APP_ENVIRONMENT)
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT");
    let environment_file: String = format!("{}.yaml", environment.as_str());

    // Initialise our configuration reader
    let settings: config::Config = config::Config::builder()
        // Add configuration values rom file `configuration.yaml`.
        .add_source(config::File::from(
            configuration_directory.join("base.yaml"),
        ))
        .add_source(config::File::from(
            configuration_directory.join(environment_file),
        ))
        .build()?;
    // Try convert configuration values it read into our settings type
    settings.try_deserialize::<Settings>()
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name,
        ))
    }
    pub fn connection_string_without_db(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port
        ))
    }
}
