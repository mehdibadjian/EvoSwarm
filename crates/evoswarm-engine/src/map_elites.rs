//! MAP-Elites archive (e5-1, AD-8): keep the best candidate of each phenotypic kind so the
//! search does not collapse onto a single approach.
//!
//! The archive is a 4×4 phenotypic grid. The x-axis bins a candidate's **runtime relative to
//! baseline**; the y-axis bins its **diff size in changed lines**. Each cell holds at most one
//! occupant — the highest-scoring candidate that has landed in that cell. When a new candidate
//! scores higher than the incumbent it replaces it; otherwise it is discarded.
//!
//! Bin edges live in [`GridConfig`], so the next job picks up changed edges without a rebuild
//! (AC3).
//!
//! **FalkorDB seam.** In production each placement writes a `Cell` node to FalkorDB. This
//! environment has no redis-compatible FalkorDB, so the write is modelled behind the
//! [`CellSink`] trait and verified with the in-process [`RecordingCellSink`]; a live FalkorDB
//! backs the same trait in deployment. The binning and replacement logic is pure and fully
//! exercised here.

use std::sync::Mutex;

/// Grid dimension per axis (the archive is `GRID_DIM × GRID_DIM`).
pub const GRID_DIM: usize = 4;

/// Number of bin edges per axis. `GRID_DIM` bins need `GRID_DIM - 1` interior edges.
const EDGE_COUNT: usize = GRID_DIM - 1;

/// A cell coordinate in the grid. `x` is the runtime-ratio bin, `y` the diff-size bin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellCoord {
    pub x: usize,
    pub y: usize,
}

/// Per-job grid configuration: the interior bin edges for each axis.
///
/// A value's bin is the count of edges it is greater than or equal to, so edges must be sorted
/// ascending. Three edges yield four bins: `< e0`, `[e0, e1)`, `[e1, e2)`, `>= e2`.
#[derive(Debug, Clone, PartialEq)]
pub struct GridConfig {
    /// Interior edges for the runtime-relative-to-baseline axis (x).
    pub runtime_ratio_edges: [f64; EDGE_COUNT],
    /// Interior edges for the changed-lines axis (y).
    pub diff_line_edges: [usize; EDGE_COUNT],
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            runtime_ratio_edges: [0.5, 1.0, 2.0],
            diff_line_edges: [10, 50, 200],
        }
    }
}

/// A scored candidate offered to the archive, carrying the two phenotype measurements that
/// decide its cell.
#[derive(Debug, Clone, PartialEq)]
pub struct ArchiveEntry {
    pub candidate_id: String,
    pub score: f64,
    pub generation: u32,
    /// Runtime relative to the baseline (1.0 = same speed as baseline).
    pub runtime_ratio: f64,
    /// Number of changed lines in the diff.
    pub diff_lines: usize,
}

/// Sink for `Cell` node writes. The production implementation persists to FalkorDB; the test
/// seam records writes in-process.
pub trait CellSink: Send + Sync {
    /// Records that `entry` now occupies `coord`.
    fn write_cell(&self, coord: CellCoord, entry: &ArchiveEntry);
}

/// In-process [`CellSink`] used to verify placements where no FalkorDB is available.
#[derive(Debug, Default)]
pub struct RecordingCellSink {
    writes: Mutex<Vec<(CellCoord, ArchiveEntry)>>,
}

impl RecordingCellSink {
    /// A snapshot of every Cell-node write, in order.
    pub fn writes(&self) -> Vec<(CellCoord, ArchiveEntry)> {
        self.writes.lock().expect("sink poisoned").clone()
    }
}

impl CellSink for RecordingCellSink {
    fn write_cell(&self, coord: CellCoord, entry: &ArchiveEntry) {
        self.writes.lock().expect("sink poisoned").push((coord, entry.clone()));
    }
}

/// The MAP-Elites archive: a `GRID_DIM × GRID_DIM` grid of optional occupants plus the config
/// that maps a candidate to its cell.
pub struct MapElitesArchive {
    config: GridConfig,
    cells: Vec<Option<ArchiveEntry>>,
}

impl MapElitesArchive {
    /// Creates an empty archive with `config`'s bin edges.
    pub fn new(config: GridConfig) -> Self {
        Self {
            config,
            cells: vec![None; GRID_DIM * GRID_DIM],
        }
    }

    /// The grid configuration in use (for the archive-health report, e5-6).
    pub fn config(&self) -> &GridConfig {
        &self.config
    }

    /// Computes the cell for `entry` without mutating the archive.
    pub fn coord_for(&self, entry: &ArchiveEntry) -> CellCoord {
        CellCoord {
            x: bin_f64(entry.runtime_ratio, &self.config.runtime_ratio_edges),
            y: bin_usize(entry.diff_lines, &self.config.diff_line_edges),
        }
    }

    /// Places `entry` in its cell, replacing the incumbent when `entry` scores strictly higher.
    /// Writes a Cell node to `sink` only when the occupant actually changes. Returns the cell
    /// coordinate the entry was mapped to (regardless of whether it won the cell).
    pub fn archive<S: CellSink + ?Sized>(&mut self, entry: ArchiveEntry, sink: &S) -> CellCoord {
        let coord = self.coord_for(&entry);
        let idx = self.index(coord);
        let slot = &mut self.cells[idx];
        let replaces = match slot {
            None => true,
            Some(incumbent) => is_higher(entry.score, incumbent.score),
        };
        if replaces {
            *slot = Some(entry.clone());
            sink.write_cell(coord, &entry);
        }
        coord
    }

    /// The current occupant of `coord`, if any.
    pub fn occupant(&self, coord: CellCoord) -> Option<&ArchiveEntry> {
        self.cells[self.index(coord)].as_ref()
    }

    /// Every cell occupant in row-major order, as `Some`/`None` per cell. Used by the
    /// archive-health report (e5-6) to render occupancy and per-cell best scores.
    pub fn cells(&self) -> &[Option<ArchiveEntry>] {
        &self.cells
    }

    /// Number of occupied cells (out of `GRID_DIM * GRID_DIM`).
    pub fn occupied_count(&self) -> usize {
        self.cells.iter().filter(|c| c.is_some()).count()
    }

    fn index(&self, coord: CellCoord) -> usize {
        coord.y * GRID_DIM + coord.x
    }
}

/// True when `candidate` strictly beats `incumbent`. Incomparable scores (NaN) never displace
/// an incumbent, so a malformed measurement cannot evict a good candidate.
fn is_higher(candidate: f64, incumbent: f64) -> bool {
    candidate > incumbent
}

/// Bin index for a floating measurement: the count of edges it is `>=`.
fn bin_f64(value: f64, edges: &[f64; EDGE_COUNT]) -> usize {
    edges.iter().filter(|&&e| value >= e).count()
}

/// Bin index for an integer measurement: the count of edges it is `>=`.
fn bin_usize(value: usize, edges: &[usize; EDGE_COUNT]) -> usize {
    edges.iter().filter(|&&e| value >= e).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bin_f64_respects_edges() {
        let edges = [0.5, 1.0, 2.0];
        assert_eq!(bin_f64(0.0, &edges), 0);
        assert_eq!(bin_f64(0.49, &edges), 0);
        assert_eq!(bin_f64(0.5, &edges), 1); // >= lower edge
        assert_eq!(bin_f64(1.99, &edges), 2);
        assert_eq!(bin_f64(2.0, &edges), 3);
        assert_eq!(bin_f64(100.0, &edges), 3);
    }

    #[test]
    fn bin_usize_respects_edges() {
        let edges = [10, 50, 200];
        assert_eq!(bin_usize(0, &edges), 0);
        assert_eq!(bin_usize(10, &edges), 1);
        assert_eq!(bin_usize(199, &edges), 2);
        assert_eq!(bin_usize(200, &edges), 3);
    }

    #[test]
    fn nan_never_displaces_incumbent() {
        let mut archive = MapElitesArchive::new(GridConfig::default());
        let sink = RecordingCellSink::default();
        let good = ArchiveEntry {
            candidate_id: "good".into(),
            score: 0.5,
            generation: 0,
            runtime_ratio: 0.1,
            diff_lines: 1,
        };
        let coord = archive.archive(good, &sink);
        let nan = ArchiveEntry {
            candidate_id: "nan".into(),
            score: f64::NAN,
            generation: 1,
            runtime_ratio: 0.1,
            diff_lines: 1,
        };
        archive.archive(nan, &sink);
        assert_eq!(archive.occupant(coord).unwrap().candidate_id, "good");
        assert_eq!(sink.writes().len(), 1, "NaN placement wrote no Cell node");
    }

    #[test]
    fn index_is_row_major_within_bounds() {
        let archive = MapElitesArchive::new(GridConfig::default());
        for y in 0..GRID_DIM {
            for x in 0..GRID_DIM {
                let i = archive.index(CellCoord { x, y });
                assert!(i < GRID_DIM * GRID_DIM);
            }
        }
    }
}
