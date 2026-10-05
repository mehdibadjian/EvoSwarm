//! EvoSwarm optional streaming gateway (AD-2).
//!
//! A transparent Tokio/axum reverse proxy for Claude Code traffic pointed at
//! `ANTHROPIC_BASE_URL`. It forwards Server-Sent Events byte-for-byte and never blocks on
//! swarm state (AD-2). e4-1 supplies the pass-through proxy; e4-2 adds local token/cost
//! accounting; e4-4 adds injection opt-out; e4-5 adds the p95 TTFT latency gate.

pub mod config;
pub mod latency;
pub mod proxy;
pub mod usage;

pub use config::{ConfigError, GatewayConfig};
pub use latency::{Budget, LatencyBudget};
pub use proxy::{ProxyError, ProxyState};
pub use usage::{UsageError, UsagePricing, UsageRecord, UsageStore};
