use thiserror::Error;

use crate::config::Role;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("mandatory model role is missing from config: {role}")]
    MissingRole { role: Role },

    #[error("invalid value for role {role} field '{field}': {reason}")]
    InvalidField {
        role: Role,
        field: &'static str,
        reason: String,
    },

    #[error("failed to read config: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse config TOML: {0}")]
    Parse(#[from] toml::de::Error),
}
