//! e5-6 archive health report — acceptance criteria (story §4).
//!
//! Renders occupancy (out of 16) and a 4×4 heatmap of the best score per cell from an
//! e5-1 [`MapElitesArchive`]. Pure logic over the in-process archive, so fully certifiable
//! here. The story suggests `tests/engine/test_archive_report.rs`; the flat target
//! `e5_6_archive_health_report` matches the gate command.

use evoswarm_engine::archive_report::{health_report, render_heatmap, ArchiveHealth};
use evoswarm_engine::map_elites::{
    ArchiveEntry, GridConfig, MapElitesArchive, RecordingCellSink, GRID_DIM,
};

fn entry(id: &str, score: f64, runtime_ratio: f64, diff_lines: usize) -> ArchiveEntry {
    ArchiveEntry {
        candidate_id: id.into(),
        score,
        generation: 0,
        runtime_ratio,
        diff_lines,
    }
}

/// Occupancy ratio: occupied cells out of the full 16, and the derived diversity fraction.
#[test]
fn test_occupancy_ratio_calculation() {
    let mut archive = MapElitesArchive::new(GridConfig::default());
    let sink = RecordingCellSink::default();

    // Empty archive: 0/16.
    let h0 = health_report(&archive);
    assert_eq!((h0.occupied, h0.total), (0, GRID_DIM * GRID_DIM));
    assert_eq!(h0.diversity, 0.0);

    // Two entries in distinct cells: 2/16.
    archive.archive(entry("a", 0.9, 0.1, 1), &sink); // cell (0,0)
    archive.archive(entry("b", 0.5, 3.0, 500), &sink); // cell (3,3)
    let h = health_report(&archive);
    assert_eq!(h.occupied, 2);
    assert_eq!(h.total, 16);
    assert!((h.diversity - 2.0 / 16.0).abs() < 1e-9);

    // A third entry into an occupied cell does not raise occupancy.
    archive.archive(entry("a-better", 0.95, 0.2, 2), &sink); // still cell (0,0)
    let h2 = health_report(&archive);
    assert_eq!(h2.occupied, 2, "replacement does not add a cell");
}

/// The 4×4 ASCII heatmap renders one cell per grid position, showing the best score where
/// occupied and an empty marker elsewhere.
#[test]
fn test_grid_heatmap_rendering() {
    let mut archive = MapElitesArchive::new(GridConfig::default());
    let sink = RecordingCellSink::default();
    archive.archive(entry("champ", 0.9, 0.1, 1), &sink); // cell (0,0)

    let grid = render_heatmap(&archive);
    let lines: Vec<&str> = grid.lines().collect();

    // The heatmap is a GRID_DIM-row block.
    assert_eq!(
        lines.len(),
        GRID_DIM,
        "one rendered row per grid row: {grid:?}"
    );

    // Row 0 (y=0) shows the champion's score in the x=0 column.
    assert!(
        lines[0].contains("0.9"),
        "row 0 carries the 0.9 score: {}",
        lines[0]
    );
    // An empty cell renders the empty marker, not a score.
    assert!(
        lines[3].contains(ArchiveHealth::EMPTY_MARKER),
        "row 3 is empty: {}",
        lines[3]
    );

    // The full report embeds the heatmap and an occupancy summary line.
    let h = health_report(&archive);
    assert!(
        h.summary.contains("1/16"),
        "summary shows occupancy: {}",
        h.summary
    );
    assert!(h.heatmap.contains(&grid), "report heatmap matches render");
}
