//! Report view types (e1-11 §1.3): the structured data the audit report renders. Kept in
//! core so both the CLI report writer and any future gateway/API surface share one shape.
//! A report section whose data is absent fails generation rather than being silently omitted
//! — the audit trail must be complete or it is not trustworthy.

use serde::{Deserialize, Serialize};

/// The e1-7 weighted-score breakdown: total score S and its adversary (A), runtime (P) and
/// parsimony (Z) terms, plus the weights that combined them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub score: f64,
    pub adversary: f64,
    pub runtime: f64,
    pub parsimony: f64,
    pub w_adversary: f64,
    pub w_runtime: f64,
    pub w_parsimony: f64,
}

/// Per-role model usage and cost for the report's cost table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleUsage {
    pub role: String,
    pub model_id: String,
    pub calls: u64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost_usd: f64,
}

/// One node in the candidate lineage tree rendered in the report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineageNode {
    pub candidate_id: String,
    pub generation: u32,
    pub parent_ids: Vec<String>,
    pub model_id: String,
}

/// The winning-candidate summary the report header states, including whether selection
/// produced a verified winner or only a best-effort patch (e1-11 §4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WinnerSummary {
    pub candidate_id: String,
    pub verified: bool,
    /// Visible tests that passed.
    pub visible_passed: Vec<String>,
    /// Held-out tests that passed.
    pub held_out_passed: Vec<String>,
    /// Proposed adversary tests offered for human review (e1-9).
    pub proposed_tests: Vec<String>,
}
