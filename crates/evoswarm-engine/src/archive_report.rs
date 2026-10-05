//! Archive health report (e5-6): show archive diversity per job so a developer can tell whether
//! the search stayed broad.
//!
//! Renders the e5-1 [`MapElitesArchive`] as an occupancy count (out of 16) and a 4×4 ASCII
//! heatmap of the best score per cell. Pure logic over the in-process archive, so fully
//! certifiable here.
//!
//! The heatmap shows each cell's *current* best occupant. A per-generation timeline of cell
//! scores would read the FalkorDB `Cell` node history; that half is deferred to deployment
//! alongside the e5-1 SEAM (no redis-compatible FalkorDB in this environment).

use crate::map_elites::{MapElitesArchive, GRID_DIM};

/// Rendered width of a single heatmap cell, so columns align.
const CELL_WIDTH: usize = 4;

/// A rendered archive-health view.
#[derive(Debug, Clone, PartialEq)]
pub struct ArchiveHealth {
    /// Number of occupied cells.
    pub occupied: usize,
    /// Total cells in the grid (`GRID_DIM * GRID_DIM`).
    pub total: usize,
    /// Occupancy as a fraction in `[0, 1]` — the diversity score.
    pub diversity: f64,
    /// The 4×4 ASCII heatmap.
    pub heatmap: String,
    /// One-line human-readable occupancy summary.
    pub summary: String,
}

impl ArchiveHealth {
    /// Marker rendered for an empty cell.
    pub const EMPTY_MARKER: &'static str = "-";
}

/// Computes occupancy, diversity, and the heatmap for `archive`.
pub fn health_report(archive: &MapElitesArchive) -> ArchiveHealth {
    let occupied = archive.occupied_count();
    let total = GRID_DIM * GRID_DIM;
    let diversity = occupied as f64 / total as f64;
    let heatmap = render_heatmap(archive);
    let summary = format!("archive diversity: {occupied}/{total} cells occupied");
    ArchiveHealth {
        occupied,
        total,
        diversity,
        heatmap,
        summary,
    }
}

/// Renders the archive as a `GRID_DIM`-row ASCII heatmap. Each row is a grid row (increasing
/// `y`); within a row cells run left-to-right by increasing `x`. An occupied cell shows its
/// best score to two decimals; an empty cell shows [`ArchiveHealth::EMPTY_MARKER`].
pub fn render_heatmap(archive: &MapElitesArchive) -> String {
    let cells = archive.cells();
    let rows: Vec<String> = (0..GRID_DIM)
        .map(|y| {
            let cols: Vec<String> = (0..GRID_DIM)
                .map(|x| {
                    let token = match &cells[y * GRID_DIM + x] {
                        Some(entry) => format!("{:.2}", entry.score),
                        None => ArchiveHealth::EMPTY_MARKER.to_string(),
                    };
                    format!("{token:>CELL_WIDTH$}")
                })
                .collect();
            cols.join(" ")
        })
        .collect();
    rows.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_elites::{ArchiveEntry, GridConfig, RecordingCellSink};

    fn fill(
        archive: &mut MapElitesArchive,
        sink: &RecordingCellSink,
        id: &str,
        score: f64,
        x_ratio: f64,
        y_lines: usize,
    ) {
        archive.archive(
            ArchiveEntry {
                candidate_id: id.into(),
                score,
                generation: 0,
                runtime_ratio: x_ratio,
                diff_lines: y_lines,
            },
            sink,
        );
    }

    #[test]
    fn empty_archive_is_zero_diversity() {
        let archive = MapElitesArchive::new(GridConfig::default());
        let h = health_report(&archive);
        assert_eq!(h.occupied, 0);
        assert_eq!(h.total, 16);
        assert_eq!(h.diversity, 0.0);
        assert!(h.summary.contains("0/16"));
        // Every cell renders the empty marker.
        assert!(!h.heatmap.contains("0.00"));
    }

    #[test]
    fn full_archive_is_unit_diversity() {
        let mut archive = MapElitesArchive::new(GridConfig::default());
        let sink = RecordingCellSink::default();
        // Default edges: runtime bins at [0.5,1.0,2.0], diff bins at [10,50,200].
        let ratios = [0.1, 0.75, 1.5, 3.0];
        let lines = [1usize, 20, 60, 300];
        for (i, &r) in ratios.iter().enumerate() {
            for (j, &l) in lines.iter().enumerate() {
                fill(&mut archive, &sink, &format!("c{i}{j}"), 0.5, r, l);
            }
        }
        let h = health_report(&archive);
        assert_eq!(h.occupied, 16);
        assert_eq!(h.diversity, 1.0);
    }

    #[test]
    fn heatmap_has_four_rows() {
        let mut archive = MapElitesArchive::new(GridConfig::default());
        let sink = RecordingCellSink::default();
        fill(&mut archive, &sink, "a", 0.42, 0.1, 1);
        let grid = render_heatmap(&archive);
        assert_eq!(grid.lines().count(), GRID_DIM);
        assert!(grid.contains("0.42"));
        assert!(grid.contains(ArchiveHealth::EMPTY_MARKER));
    }
}
