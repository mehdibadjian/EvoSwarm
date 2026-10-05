use std::fmt;
use std::path::Path;

use serde::Deserialize;

use crate::error::ConfigError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Mutator,
    Synthesiser,
    Adversary,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Role::Mutator => "mutator",
            Role::Synthesiser => "synthesiser",
            Role::Adversary => "adversary",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleConfig {
    pub provider: String,
    pub model_id: String,
    pub max_tokens: u32,
    pub temperature: f64,
    pub cost_per_million_input: f64,
    pub cost_per_million_output: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct RolesConfig {
    mutator: Option<RoleConfig>,
    synthesiser: Option<RoleConfig>,
    adversary: Option<RoleConfig>,
}

impl RolesConfig {
    fn take(&mut self, role: Role) -> Option<RoleConfig> {
        match role {
            Role::Mutator => self.mutator.take(),
            Role::Synthesiser => self.synthesiser.take(),
            Role::Adversary => self.adversary.take(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct ModelsConfig {
    roles: RolesConfig,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct FileConfig {
    models: ModelsConfig,
}

/// Validated, dispatch-ready configuration holding all three mandatory roles.
///
/// Roles are stored in `[Mutator, Synthesiser, Adversary]` order so `role()` is
/// an infallible index rather than a lookup that can miss: a `ModelConfig`
/// cannot exist unless every mandatory role was present and valid.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelConfig {
    roles: [RoleConfig; 3],
}

impl ModelConfig {
    pub fn role(&self, role: Role) -> &RoleConfig {
        let index = match role {
            Role::Mutator => 0,
            Role::Synthesiser => 1,
            Role::Adversary => 2,
        };
        &self.roles[index]
    }
}

/// Reads and validates `path`. Any missing role or out-of-range field is a hard
/// error naming the role and field, so an invalid config can never reach a
/// running daemon or the e1-10 budget projection.
pub fn load(path: &Path) -> Result<ModelConfig, ConfigError> {
    let text = std::fs::read_to_string(path)?;
    let parsed: FileConfig = toml::from_str(&text)?;
    validate(parsed)
}

fn validate(mut file: FileConfig) -> Result<ModelConfig, ConfigError> {
    let mutator = take_validated(&mut file, Role::Mutator)?;
    let synthesiser = take_validated(&mut file, Role::Synthesiser)?;
    let adversary = take_validated(&mut file, Role::Adversary)?;
    Ok(ModelConfig {
        roles: [mutator, synthesiser, adversary],
    })
}

fn take_validated(file: &mut FileConfig, role: Role) -> Result<RoleConfig, ConfigError> {
    let cfg = file
        .models
        .roles
        .take(role)
        .ok_or(ConfigError::MissingRole { role })?;
    validate_role(role, &cfg)?;
    Ok(cfg)
}

fn validate_role(role: Role, cfg: &RoleConfig) -> Result<(), ConfigError> {
    let invalid = |field: &'static str, reason: String| ConfigError::InvalidField {
        role,
        field,
        reason,
    };

    if cfg.model_id.is_empty() {
        return Err(invalid("model_id", "must not be empty".to_string()));
    }
    if cfg.model_id.chars().any(char::is_whitespace) {
        return Err(invalid(
            "model_id",
            format!("must not contain whitespace: {:?}", cfg.model_id),
        ));
    }
    if !(0.0..=2.0).contains(&cfg.temperature) {
        return Err(invalid(
            "temperature",
            format!("{} outside [0.0, 2.0]", cfg.temperature),
        ));
    }
    if cfg.max_tokens == 0 {
        return Err(invalid("max_tokens", "must be > 0".to_string()));
    }
    if cfg.cost_per_million_input < 0.0 {
        return Err(invalid(
            "cost_per_million_input",
            format!("{} must be >= 0.0", cfg.cost_per_million_input),
        ));
    }
    if cfg.cost_per_million_output < 0.0 {
        return Err(invalid(
            "cost_per_million_output",
            format!("{} must be >= 0.0", cfg.cost_per_million_output),
        ));
    }
    Ok(())
}
