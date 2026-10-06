//! Gateway configuration (e4-1, AD-2).
//!
//! The gateway is a transparent reverse proxy for Claude Code traffic pointed at
//! `ANTHROPIC_BASE_URL`. It forwards to an upstream provider and listens for client
//! connections; nothing here couples it to the evolutionary loop (AD-2).

use std::net::SocketAddr;

use thiserror::Error;

/// Errors constructing a [`GatewayConfig`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("invalid upstream base URL {0:?}: {1}")]
    InvalidUpstreamUrl(String, String),
}

/// Immutable gateway settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayConfig {
    /// Upstream provider base URL (e.g. `https://api.anthropic.com`, or a local stub in
    /// tests). Request paths are appended to it verbatim. Must be an absolute http(s) URL.
    pub upstream_base_url: String,
    /// Socket address the gateway binds and serves on.
    pub listen_addr: SocketAddr,
    /// Master switch for exemplar injection (e4-4). `true` by default; `false` is equivalent
    /// to every request carrying `x-evoswarm-inject: off` — the injector is never consulted.
    /// Usage logging (e4-2) is unaffected either way.
    pub inject: bool,
}

impl GatewayConfig {
    /// Builds a config, validating that `upstream_base_url` is an absolute http(s) URL so a
    /// misconfiguration fails fast at startup rather than per-request.
    pub fn new(upstream_base_url: &str, listen_addr: SocketAddr) -> Result<Self, ConfigError> {
        // reqwest re-exports the `url` crate's parser; reuse it rather than adding a dep.
        let parsed = upstream_base_url.parse::<reqwest::Url>().map_err(|e| {
            ConfigError::InvalidUpstreamUrl(upstream_base_url.to_string(), e.to_string())
        })?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(ConfigError::InvalidUpstreamUrl(
                upstream_base_url.to_string(),
                format!("scheme must be http or https, got {:?}", parsed.scheme()),
            ));
        }
        Ok(Self {
            upstream_base_url: upstream_base_url.trim_end_matches('/').to_string(),
            listen_addr,
            inject: true,
        })
    }

    /// Sets the e4-4 injection master switch, returning the config for chaining. `false`
    /// bypasses the injector for every request; `true` (the default) leaves injection on and
    /// still honours the per-request `x-evoswarm-inject: off` header.
    pub fn with_inject(mut self, inject: bool) -> Self {
        self.inject = inject;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr() -> SocketAddr {
        "127.0.0.1:8080".parse().unwrap()
    }

    #[test]
    fn accepts_http_and_https() {
        assert!(GatewayConfig::new("http://127.0.0.1:9000", addr()).is_ok());
        assert!(GatewayConfig::new("https://api.anthropic.com", addr()).is_ok());
    }

    #[test]
    fn strips_trailing_slash() {
        let c = GatewayConfig::new("https://api.anthropic.com/", addr()).unwrap();
        assert_eq!(c.upstream_base_url, "https://api.anthropic.com");
    }

    #[test]
    fn rejects_non_http_scheme_and_garbage() {
        assert!(matches!(
            GatewayConfig::new("ftp://example.com", addr()),
            Err(ConfigError::InvalidUpstreamUrl(_, _))
        ));
        assert!(matches!(
            GatewayConfig::new("not a url", addr()),
            Err(ConfigError::InvalidUpstreamUrl(_, _))
        ));
    }

    /// e4-4: injection is on unless the operator turns it off, and `with_inject` is the only
    /// way to change it — the default must not silently become opt-out.
    #[test]
    fn inject_defaults_to_on_and_is_overridable() {
        let c = GatewayConfig::new("http://127.0.0.1:9000", addr()).unwrap();
        assert!(c.inject, "injection is enabled by default");

        assert!(!c.clone().with_inject(false).inject);
        assert!(c.clone().with_inject(true).inject);
        // The override is the only difference from the default config.
        assert_eq!(
            c.clone().with_inject(false),
            GatewayConfig {
                inject: false,
                ..c.clone()
            }
        );
    }
}
