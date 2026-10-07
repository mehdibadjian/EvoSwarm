//! e5-2 selection mode comparison — acceptance criteria (story §4).
//!
//! Gate: `cargo test -p evoswarm-engine --test e5_2_selection_mode_comparison`.
//!
//! Validates:
//! - AC1: When `selection = 'topk'`, parents are drawn strictly by top score.
//!   When `selection = 'mapelites'`, parents are drawn from distinct occupied archive cells
//!   preserving phenotypic diversity.
//! - AC2: Side-by-side comparison evaluates solve rates, tokens per solved task, and diversity
//!   metrics for both modes.

use evoswarm_engine::map_elites::{ArchiveEntry, GridConfig, MapElitesArchive, RecordingCellSink};
use evoswarm_engine::selection_mode::{
    compare_selection_runs, select_parents, SelectionMode, SelectionRunMetrics,
};

fn sample_entry(id: &str, score: f64, runtime_ratio: f64, diff_lines: usize) -> ArchiveEntry {
    ArchiveEntry {
        candidate_id: id.to_string(),
        score,
        generation: 1,
        runtime_ratio,
        diff_lines,
    }
}

#[test]
fn test_selection_mode_parse_and_config() {
    assert_eq!(
        "topk".parse::<SelectionMode>().unwrap(),
        SelectionMode::TopK
    );
    assert_eq!(
        "mapelites".parse::<SelectionMode>().unwrap(),
        SelectionMode::MapElites
    );
    assert_eq!(
        "map_elites".parse::<SelectionMode>().unwrap(),
        SelectionMode::MapElites
    );
    assert!("invalid_mode".parse::<SelectionMode>().is_err());
}

#[test]
fn test_topk_selection_draws_highest_scores() {
    // 4 candidates: c1 and c2 have the same phenotype (cell 0,0) with high scores.
    // c3 and c4 have different phenotypes with lower scores.
    let c1 = sample_entry("c1", 0.95, 0.2, 5); // cell (0, 0)
    let c2 = sample_entry("c2", 0.90, 0.3, 6); // cell (0, 0)
    let c3 = sample_entry("c3", 0.70, 1.5, 30); // cell (2, 1)
    let c4 = sample_entry("c4", 0.60, 2.5, 250); // cell (3, 3)

    let candidates = vec![c1.clone(), c2.clone(), c3.clone(), c4.clone()];
    let archive = MapElitesArchive::new(GridConfig::default());

    let selected = select_parents(&candidates, SelectionMode::TopK, 2, &archive);
    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0], "c1");
    assert_eq!(selected[1], "c2");
}

#[test]
fn test_mapelites_selection_draws_from_occupied_cells() {
    let c1 = sample_entry("c1", 0.95, 0.2, 5); // cell (0, 0)
    let c2 = sample_entry("c2", 0.90, 0.3, 6); // cell (0, 0) - replaces or kept in cell
    let c3 = sample_entry("c3", 0.70, 1.5, 30); // cell (2, 1)
    let c4 = sample_entry("c4", 0.60, 2.5, 250); // cell (3, 3)

    let candidates = vec![c1.clone(), c2.clone(), c3.clone(), c4.clone()];

    let sink = RecordingCellSink::default();
    let mut archive = MapElitesArchive::new(GridConfig::default());
    for c in &candidates {
        archive.archive(c.clone(), &sink);
    }

    // MAP-Elites should pick elites across distinct occupied cells:
    // (0, 0) elite is c1; (2, 1) elite is c3; (3, 3) elite is c4.
    let selected = select_parents(&candidates, SelectionMode::MapElites, 3, &archive);
    assert_eq!(selected.len(), 3);
    assert!(selected.contains(&"c1".to_string()));
    assert!(selected.contains(&"c3".to_string()));
    assert!(selected.contains(&"c4".to_string()));
    // c2 should NOT be chosen over c3/c4 because c1 already represents cell (0,0)
    assert!(!selected.contains(&"c2".to_string()));
}

#[test]
fn test_side_by_side_reporting() {
    let topk = SelectionRunMetrics {
        mode: SelectionMode::TopK,
        tasks_solved: 18,
        total_tasks: 30,
        total_tokens: 3_600_000,
        cell_coverage_pct: 25.0,
    };

    let mapelites = SelectionRunMetrics {
        mode: SelectionMode::MapElites,
        tasks_solved: 22,
        total_tasks: 30,
        total_tokens: 4_100_000,
        cell_coverage_pct: 62.5,
    };

    let report = compare_selection_runs(&topk, &mapelites);
    assert_eq!(report.topk_solve_rate, 60.0);
    assert_eq!(report.mapelites_solve_rate, 73.33333333333333);
    assert_eq!(report.solve_rate_delta, 13.333333333333329);
    assert_eq!(report.topk_tokens_per_solved, Some(200_000.0));
    assert_eq!(report.mapelites_tokens_per_solved, Some(186_363.63636363635));

    let summary = report.render_markdown();
    assert!(summary.contains("Top-K vs MAP-Elites"));
    assert!(summary.contains("60.00%"));
    assert!(summary.contains("73.33%"));
    assert!(summary.contains("25.0%"));
    assert!(summary.contains("62.5%"));
}
