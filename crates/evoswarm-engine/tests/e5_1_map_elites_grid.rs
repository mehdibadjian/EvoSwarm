//! e5-1 MAP-Elites grid — acceptance criteria (story §4, AD-8).
//!
//! Red-phase tests authored before the production `map_elites` module exists. The story
//! suggests `tests/engine/test_map_elites.rs`, but Cargo only treats *flat* `tests/<name>.rs`
//! as `--test` targets, so the target is `e5_1_map_elites_grid` to match the gate command.
//!
//! The FalkorDB Cell-node write (AC3) is a SEAM: this environment has no redis-compatible
//! FalkorDB, so writes are verified against an in-process `CellSink` recorder. The pure
//! binning + replacement logic (AC1, AC2) is fully certifiable here.

use evoswarm_engine::map_elites::{
    ArchiveEntry, CellCoord, GridConfig, MapElitesArchive, RecordingCellSink, GRID_DIM,
};

/// AC1: a scored candidate is placed in a 4×4 cell by runtime-ratio bin (x) and diff-size
/// bin (y).
#[test]
fn test_cell_binning() {
    let mut archive = MapElitesArchive::new(GridConfig::default());
    let sink = RecordingCellSink::default();

    // Default edges: runtime ratio [0.5, 1.0, 2.0], diff lines [10, 50, 200].
    let fast_small = ArchiveEntry {
        candidate_id: "fast-small".into(),
        score: 0.8,
        generation: 0,
        runtime_ratio: 0.25,
        diff_lines: 5,
    };
    assert_eq!(archive.coord_for(&fast_small), CellCoord { x: 0, y: 0 });

    let mid = ArchiveEntry {
        candidate_id: "mid".into(),
        score: 0.5,
        generation: 1,
        runtime_ratio: 1.5, // in [1.0, 2.0) -> bin 2
        diff_lines: 60,     // in [50, 200)  -> bin 2
    };
    assert_eq!(archive.coord_for(&mid), CellCoord { x: 2, y: 2 });

    let slow_big = ArchiveEntry {
        candidate_id: "slow-big".into(),
        score: 0.2,
        generation: 2,
        runtime_ratio: 3.0,
        diff_lines: 500,
    };
    assert_eq!(archive.coord_for(&slow_big), CellCoord { x: 3, y: 3 });

    // Each entry lands in a distinct cell within the 4×4 grid, and each placement writes a
    // Cell node to the sink.
    for e in [fast_small, mid, slow_big] {
        let c = archive.archive(e.clone(), &sink);
        assert!(c.x < GRID_DIM && c.y < GRID_DIM);
    }
    assert_eq!(sink.writes().len(), 3);
    assert_eq!(archive.occupied_count(), 3);
}

/// AC2: a higher-scoring candidate replaces the cell's current occupant; a lower-scoring one
/// does not.
#[test]
fn test_cell_replacement() {
    let mut archive = MapElitesArchive::new(GridConfig::default());
    let sink = RecordingCellSink::default();

    let first = ArchiveEntry {
        candidate_id: "first".into(),
        score: 0.5,
        generation: 0,
        runtime_ratio: 0.25,
        diff_lines: 5,
    };
    let coord = archive.archive(first, &sink);
    assert_eq!(archive.occupant(coord).map(|o| o.candidate_id.as_str()), Some("first"));

    // Higher score in the same cell replaces the occupant.
    let better = ArchiveEntry {
        candidate_id: "better".into(),
        score: 0.9,
        generation: 1,
        runtime_ratio: 0.3, // same x bin 0
        diff_lines: 8,      // same y bin 0
    };
    let coord2 = archive.archive(better, &sink);
    assert_eq!(coord, coord2, "same cell");
    assert_eq!(archive.occupant(coord).map(|o| o.candidate_id.as_str()), Some("better"));

    // Lower score does not displace the incumbent.
    let worse = ArchiveEntry {
        candidate_id: "worse".into(),
        score: 0.1,
        generation: 2,
        runtime_ratio: 0.1,
        diff_lines: 1,
    };
    archive.archive(worse, &sink);
    assert_eq!(archive.occupant(coord).map(|o| o.candidate_id.as_str()), Some("better"));

    // Occupancy counts distinct filled cells, not archive attempts.
    assert_eq!(archive.occupied_count(), 1);
    // Sink recorded writes only for the two placements that changed the occupant.
    assert_eq!(sink.writes().len(), 2);
}

/// AC3: bin edges come from config, so changing them changes the next job's placement.
#[test]
fn test_configurable_bin_edges() {
    // A config whose runtime edges are tiny pushes almost every ratio into the top bin.
    let cfg = GridConfig {
        runtime_ratio_edges: [0.01, 0.02, 0.03],
        diff_line_edges: [10, 50, 200],
    };
    let archive = MapElitesArchive::new(cfg);
    let entry = ArchiveEntry {
        candidate_id: "x".into(),
        score: 1.0,
        generation: 0,
        runtime_ratio: 0.5, // >= 0.03 -> top bin 3
        diff_lines: 5,      // < 10    -> bin 0
    };
    assert_eq!(archive.coord_for(&entry), CellCoord { x: 3, y: 0 });
}
