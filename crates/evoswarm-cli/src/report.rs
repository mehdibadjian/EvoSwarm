//! Audit report rendering (e1-11 §1.3): the markdown report is generated from the core view
//! types. Every required section is rendered and a section whose data is absent fails
//! generation rather than being silently dropped, so the audit trail is always complete.

use evoswarm_core::{LineageNode, RoleUsage, ScoreBreakdown, WinnerSummary};
use thiserror::Error;

/// The complete data set the report needs. A `None` where a section is mandatory is a
/// generation failure, not an omitted heading.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportData<'a> {
    pub job_id: &'a str,
    pub winner: &'a WinnerSummary,
    pub score: &'a ScoreBreakdown,
    pub usage: &'a [RoleUsage],
    pub lineage: &'a [LineageNode],
}

#[derive(Debug, Error)]
pub enum ReportError {
    #[error("report data missing for required section: {0}")]
    MissingSection(String),
}

/// Renders the markdown audit report. Sections: header/outcome, score breakdown, held-out and
/// visible test passes, per-role model calls/tokens/cost, lineage tree, proposed tests.
pub fn render_markdown(data: &ReportData<'_>) -> Result<String, ReportError> {
    // Lineage and score are mandatory sections; an empty lineage means the report cannot state
    // provenance, which is a failure (contract matrix: "Missing section fails report generation").
    if data.lineage.is_empty() {
        return Err(ReportError::MissingSection("lineage".into()));
    }

    let mut out = String::new();
    out.push_str(&format!("# EvoSwarm Report — job `{}`\n\n", data.job_id));

    // Header states the outcome explicitly, including the no-verified-winner path (§4).
    if data.winner.verified {
        out.push_str(&format!(
            "**Outcome:** verified winner `{}`\n\n",
            data.winner.candidate_id
        ));
    } else {
        out.push_str(&format!(
            "**Outcome:** no verified winner — best-effort patch `{}`\n\n",
            data.winner.candidate_id
        ));
    }

    // Score breakdown (S, A, P, Z) with the weights used.
    let s = data.score;
    out.push_str("## Score breakdown\n\n");
    out.push_str(&format!("- **S** (total): {:.4}\n", s.score));
    out.push_str(&format!(
        "- **A** (adversary): {:.4} (w={:.2})\n",
        s.adversary, s.w_adversary
    ));
    out.push_str(&format!(
        "- **P** (runtime): {:.4} (w={:.2})\n",
        s.runtime, s.w_runtime
    ));
    out.push_str(&format!(
        "- **Z** (parsimony): {:.4} (w={:.2})\n\n",
        s.parsimony, s.w_parsimony
    ));

    // Visible and held-out test passes.
    out.push_str("## Test results\n\n");
    out.push_str(&format!(
        "- Visible passed ({}): {}\n",
        data.winner.visible_passed.len(),
        join_or_none(&data.winner.visible_passed)
    ));
    out.push_str(&format!(
        "- Held-out passed ({}): {}\n\n",
        data.winner.held_out_passed.len(),
        join_or_none(&data.winner.held_out_passed)
    ));

    // Per-role model calls, tokens and cost.
    out.push_str("## Model usage & cost\n\n");
    out.push_str("| Role | Model | Calls | Tokens in | Tokens out | Cost (USD) |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    let mut total_cost = 0.0;
    let mut total_calls = 0u64;
    for u in data.usage {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {:.4} |\n",
            u.role, u.model_id, u.calls, u.tokens_in, u.tokens_out, u.cost_usd
        ));
        total_cost += u.cost_usd;
        total_calls += u.calls;
    }
    out.push_str(&format!(
        "| **Total** | | **{total_calls}** | | | **{total_cost:.4}** |\n\n"
    ));

    // Lineage tree.
    out.push_str("## Lineage\n\n");
    for node in data.lineage {
        let parents = if node.parent_ids.is_empty() {
            "(root)".to_string()
        } else {
            node.parent_ids.join(", ")
        };
        out.push_str(&format!(
            "- gen {} `{}` ← {} [model {}]\n",
            node.generation, node.candidate_id, parents, node.model_id
        ));
    }
    out.push('\n');

    // Proposed adversary tests for human review.
    out.push_str("## Proposed adversary tests (for review)\n\n");
    if data.winner.proposed_tests.is_empty() {
        out.push_str("_none proposed_\n");
    } else {
        for t in &data.winner.proposed_tests {
            out.push_str(&format!("- {t}\n"));
        }
    }

    Ok(out)
}

fn join_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "_none_".to_string()
    } else {
        items.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data<'a>(
        winner: &'a WinnerSummary,
        score: &'a ScoreBreakdown,
        usage: &'a [RoleUsage],
        lineage: &'a [LineageNode],
    ) -> ReportData<'a> {
        ReportData {
            job_id: "job-1",
            winner,
            score,
            usage,
            lineage,
        }
    }

    fn sample_winner(verified: bool) -> WinnerSummary {
        WinnerSummary {
            candidate_id: "c1".into(),
            verified,
            visible_passed: vec!["test_a".into()],
            held_out_passed: vec!["test_secret".into()],
            proposed_tests: vec!["test_edge_overflow".into()],
        }
    }

    fn sample_score() -> ScoreBreakdown {
        ScoreBreakdown {
            score: 0.82,
            adversary: 0.9,
            runtime: 0.7,
            parsimony: 0.85,
            w_adversary: 0.4,
            w_runtime: 0.4,
            w_parsimony: 0.2,
        }
    }

    #[test]
    fn renders_all_required_sections() {
        let w = sample_winner(true);
        let s = sample_score();
        let usage = vec![RoleUsage {
            role: "mutator".into(),
            model_id: "m1".into(),
            calls: 3,
            tokens_in: 100,
            tokens_out: 50,
            cost_usd: 0.25,
        }];
        let lineage = vec![LineageNode {
            candidate_id: "c1".into(),
            generation: 1,
            parent_ids: vec!["c0".into()],
            model_id: "m1".into(),
        }];
        let md = render_markdown(&data(&w, &s, &usage, &lineage)).expect("render");
        for section in [
            "## Score breakdown",
            "## Test results",
            "## Model usage & cost",
            "## Lineage",
            "## Proposed adversary tests",
        ] {
            assert!(md.contains(section), "missing section: {section}");
        }
        assert!(md.contains("verified winner `c1`"));
        assert!(md.contains("test_secret"));
        assert!(md.contains("0.2500"));
    }

    #[test]
    fn no_verified_winner_states_outcome() {
        let w = sample_winner(false);
        let s = sample_score();
        let lineage = vec![LineageNode {
            candidate_id: "c1".into(),
            generation: 0,
            parent_ids: vec![],
            model_id: "m1".into(),
        }];
        let md = render_markdown(&data(&w, &s, &[], &lineage)).expect("render");
        assert!(md.contains("no verified winner"));
        assert!(md.contains("best-effort patch `c1`"));
    }

    #[test]
    fn empty_lineage_fails_generation() {
        let w = sample_winner(true);
        let s = sample_score();
        let err = render_markdown(&data(&w, &s, &[], &[])).unwrap_err();
        assert!(matches!(err, ReportError::MissingSection(_)));
    }
}
