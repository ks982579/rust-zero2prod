//! src/configuration.rs
use std::path::PathBuf;

use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::{
    postgres::{PgConnectOptions, PgSslMode},
    ConnectOptions,
};

use crate::domain::SubscriberEmail;

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

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub base_url: String,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: Secret<String>,
    pub timeout_milliseconds: u64,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }
    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_milliseconds)
    }
}

/// 2 config values: Application Port; Database Connection;
#[derive(serde::Deserialize, Debug, Clone)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub email_client: EmailClientSettings,
}

/// Needs to be desearlized so it's _parent_ can also be desearlized.
#[derive(serde::Deserialize, Debug, Clone)]
pub struct DatabaseSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    pub database_name: String,
    // Determine if encrypted connection required
    pub require_ssl: bool,
}

impl DatabaseSettings {
    // Renamed from "connection_string"
    pub fn with_db(&self) -> PgConnectOptions {
        let options: PgConnectOptions = self.without_db().database(&self.database_name);
        // Book takes slight different approach,
        // But `.log_statements` consumes self, and returns ``
        options.log_statements(tracing_log::log::LevelFilter::Trace)
        // Secret::new(format!(
        //     "postgres://{}:{}@{}:{}/{}",
        //     self.username,
        //     self.password.expose_secret(),
        //     self.host,
        //     self.port,
        //     self.database_name,
        // ))
    }
    // Renamed from `connection_string_without_db(&self) -> Secret<String>`
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            // This tries to encrypt, but can fall back to unencrypted
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(&self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
        // Secret::new(format!(
        //     "postgres://{}:{}@{}:{}",
        //     self.username,
        //     self.password.expose_secret(),
        //     self.host,
        //     self.port
        // ))
    }
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
    // Environments are either "local" or "production" currently
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
        // Add in settings from environment variables (Prefix of APP)
        // eg. "APP_APPLICATION__PORT=5001" would set `Settings.application.port`
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        ) // Allows for overriding what we put in configuration files.
        .build()?;
    // Try convert configuration values it read into our settings type
    settings.try_deserialize::<Settings>()
}
