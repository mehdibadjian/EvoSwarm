//! Lineage sink seam (e1-4 §3, e1-5 §2).
//!
//! Every child records its provenance — a `MUTATED_FROM` edge to its parent (e1-4) or a
//! `MERGED_FROM` edge to both parents with merged traits (e1-5). The graph-backed implementation
//! arrives in Epic 2 (e2-2, FalkorDB); Epic 1 verifies the operator logic against
//! `RecordingLineageSink`, an in-memory sink that captures edges for assertion. The trait is the
//! dependency-inversion point: swapping in the graph store touches neither mutation nor crossover.

use std::sync::Mutex;

use async_trait::async_trait;

/// A directed provenance edge from one candidate to another, tagged with its relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    /// The child candidate id (edge origin).
    pub from: String,
    /// The parent candidate id (edge target).
    pub to: String,
    /// `MUTATED_FROM` (single parent) or `MERGED_FROM` (crossover, one edge per parent).
    pub relation: String,
    /// Synthesiser-reported merged traits for a crossover edge; empty for a mutation edge.
    pub traits: String,
}

/// The provenance sink. `Send + Sync` so operators can record edges from concurrent tasks.
#[async_trait]
pub trait LineageSink: Send + Sync {
    /// Records a provenance edge. Implementations persist it (Epic 2) or capture it (tests).
    async fn record(&self, edge: Edge);
}

/// An in-memory lineage sink for Epic 1 and tests: captures every recorded edge.
#[derive(Default)]
pub struct RecordingLineageSink {
    edges: Mutex<Vec<Edge>>,
}

impl RecordingLineageSink {
    /// A snapshot of every edge recorded so far, in insertion order.
    pub fn edges(&self) -> Vec<Edge> {
        self.edges.lock().unwrap().clone()
    }
}

#[async_trait]
impl LineageSink for RecordingLineageSink {
    async fn record(&self, edge: Edge) {
        self.edges.lock().unwrap().push(edge);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn records_edges_in_order() {
        let sink = RecordingLineageSink::default();
        sink.record(Edge {
            from: "c1".into(),
            to: "c0".into(),
            relation: "MUTATED_FROM".into(),
            traits: String::new(),
        })
        .await;
        let edges = sink.edges();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].relation, "MUTATED_FROM");
        assert_eq!(edges[0].from, "c1");
    }
}
