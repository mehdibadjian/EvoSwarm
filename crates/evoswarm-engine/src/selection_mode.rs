//! Selection mode dispatch and comparison (e5-2, AD-8).
//!
//! Enables comparing standard Top-K parent selection against MAP-Elites phenotypic archive
//! selection.
//!
//! - **Top-K**: Draws parents strictly from highest scoring candidates regardless of phenotypic
//!   overlap, potentially collapsing diversity into local optima.
//! - **MAP-Elites**: Draws parents across distinct occupied cells of the phenotypic grid,
//!   preserving solutions across runtime and diff complexity bands.

use crate::map_elites::{ArchiveEntry, MapElitesArchive};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Selection strategy for parent sampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SelectionMode {
    #[default]
    TopK,
    MapElites,
}

impl FromStr for SelectionMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_lowercase().replace('-', "_");
        match normalized.as_str() {
            "topk" | "top_k" => Ok(Self::TopK),
            "mapelites" | "map_elites" => Ok(Self::MapElites),
            other => Err(format!("unknown selection mode '{other}', expected 'topk' or 'mapelites'")),
        }
    }
}

impl fmt::Display for SelectionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TopK => write!(f, "topk"),
            Self::MapElites => write!(f, "mapelites"),
        }
    }
}

/// Selects up to `k` parents from `candidates` according to `mode`.
///
/// Under `TopK`, candidates are ranked strictly by score descending.
/// Under `MapElites`, distinct cell elites from `archive` are drawn first, falling back to
/// top-scoring candidates only if fewer than `k` cells are occupied.
pub fn select_parents(
    candidates: &[ArchiveEntry],
    mode: SelectionMode,
    k: usize,
    archive: &MapElitesArchive,
) -> Vec<String> {
    if k == 0 || candidates.is_empty() {
        return Vec::new();
    }

    match mode {
        SelectionMode::TopK => {
            let mut sorted = candidates.to_vec();
            sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            sorted.into_iter().take(k).map(|e| e.candidate_id).collect()
        }
        SelectionMode::MapElites => {
            let mut selected = Vec::with_capacity(k);
            let mut elites: Vec<ArchiveEntry> = archive
                .cells()
                .iter()
                .filter_map(|slot| slot.clone())
                .collect();

            // Rank elites by score descending so highest-performing cell representatives are favored
            elites.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

            for elite in elites {
                if selected.len() >= k {
                    break;
                }
                selected.push(elite.candidate_id);
            }

            // If occupied cells < k, fill remainder from remaining top-scoring candidates
            if selected.len() < k {
                let mut fallback = candidates.to_vec();
                fallback.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
                for c in fallback {
                    if selected.len() >= k {
                        break;
                    }
                    if !selected.contains(&c.candidate_id) {
                        selected.push(c.candidate_id);
                    }
                }
            }

            selected
        }
    }
}

/// Metrics from a benchmark or evaluation run under a specific selection mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionRunMetrics {
    pub mode: SelectionMode,
    pub tasks_solved: usize,
    pub total_tasks: usize,
    pub total_tokens: u64,
    pub cell_coverage_pct: f64,
}

impl SelectionRunMetrics {
    pub fn solve_rate(&self) -> f64 {
        if self.total_tasks == 0 {
            0.0
        } else {
            (self.tasks_solved as f64 / self.total_tasks as f64) * 100.0
        }
    }

    pub fn tokens_per_solved(&self) -> Option<f64> {
        if self.tasks_solved == 0 {
            None
        } else {
            Some(self.total_tokens as f64 / self.tasks_solved as f64)
        }
    }
}

/// Comparative report evaluating Top-K vs MAP-Elites performance and diversity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionComparisonReport {
    pub topk_solve_rate: f64,
    pub mapelites_solve_rate: f64,
    pub solve_rate_delta: f64,
    pub topk_tokens_per_solved: Option<f64>,
    pub mapelites_tokens_per_solved: Option<f64>,
    pub topk_coverage: f64,
    pub mapelites_coverage: f64,
}

impl SelectionComparisonReport {
    pub fn render_markdown(&self) -> String {
        let topk_tokens_str = self
            .topk_tokens_per_solved
            .map(|t| format!("{t:.0}"))
            .unwrap_or_else(|| "N/A".to_string());
        let mapelites_tokens_str = self
            .mapelites_tokens_per_solved
            .map(|t| format!("{t:.0}"))
            .unwrap_or_else(|| "N/A".to_string());

        format!(
            "# Selection Mode Comparison: Top-K vs MAP-Elites\n\n\
             | Metric | Top-K | MAP-Elites | Delta |\n\
             |---|---|---|---|\n\
             | **Solve Rate** | {:.2}% | {:.2}% | {:+.2}% |\n\
             | **Tokens / Solved** | {} | {} | - |\n\
             | **Grid Coverage** | {:.1}% | {:.1}% | {:+.1}% |\n",
            self.topk_solve_rate,
            self.mapelites_solve_rate,
            self.solve_rate_delta,
            topk_tokens_str,
            mapelites_tokens_str,
            self.topk_coverage,
            self.mapelites_coverage,
            self.mapelites_coverage - self.topk_coverage
        )
    }
}

/// Evaluates two runs side by side and produces an auditable comparison report.
pub fn compare_selection_runs(
    topk: &SelectionRunMetrics,
    mapelites: &SelectionRunMetrics,
) -> SelectionComparisonReport {
    let topk_rate = topk.solve_rate();
    let mapelites_rate = mapelites.solve_rate();
    SelectionComparisonReport {
        topk_solve_rate: topk_rate,
        mapelites_solve_rate: mapelites_rate,
        solve_rate_delta: mapelites_rate - topk_rate,
        topk_tokens_per_solved: topk.tokens_per_solved(),
        mapelites_tokens_per_solved: mapelites.tokens_per_solved(),
        topk_coverage: topk.cell_coverage_pct,
        mapelites_coverage: mapelites.cell_coverage_pct,
    }
}
