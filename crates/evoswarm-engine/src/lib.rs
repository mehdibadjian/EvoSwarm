//! EvoSwarm search engine: model dispatch with idempotent replay, crash recovery, budget
//! enforcement and selection (AD-5, AD-6, AD-7).
//!
//! Wave 2 lands crash recovery (e1-12), budget enforcement (e1-10) and held-out selection
//! (e1-8); the generation operators (e1-3/4/5/9) arrive in Wave 3 behind the `ModelClient`
//! seam.

pub mod adversary;
pub mod archive_report;
pub mod budget;
pub mod crossover;
pub mod dedup;
pub mod dispatch;
pub mod early_stop;
pub mod feedback;
pub mod map_elites;
pub mod mutation;
pub mod recovery;
pub mod seeding;
pub mod selection;
pub mod selection_mode;

pub use adversary::{
    compile_filter, generate as generate_adversary, scoreable, suspect_filter, AdversaryDeps,
    SuspectEvidence,
};
pub use archive_report::{health_report, render_heatmap, ArchiveHealth};
pub use budget::{
    BudgetCaps, BudgetExhausted, BudgetGuard, CallRequest, CrossoverBudget, ExhaustedKind,
    RoleRates, CROSSOVER_CAP_RATIO,
};
pub use crossover::{
    crossover, rank_pairs, CrossoverContext, CrossoverDeps, CrossoverError, MeasuredCandidate,
};
pub use dedup::diff_hash;
pub use dispatch::{
    dispatch_model, sandbox_run, CompletionRequest, CompletionResponse, DispatchError, ModelClient,
};
pub use early_stop::{should_stop, StopReason};
pub use feedback::{build as build_feedback, render_clamped, FailureFeedback, TRUNCATION_MARKER};
pub use map_elites::{
    ArchiveEntry, CellCoord, CellSink, GridConfig, MapElitesArchive, RecordingCellSink, GRID_DIM,
};
pub use mutation::{mutate, MutationContext, MutationDeps, MutationError};
pub use recovery::{recover, ResumableJob};
pub use seeding::{
    seed_generation_zero, EmptyMemorySeeder, MemorySeeder, SeedConfig, SeedingDeps, SeedingError,
};
pub use selection::{
    select_verified_winner, HeldOutEvaluator, ScoredCandidate, SelectionDeps, SelectionError,
};
pub use selection_mode::{
    compare_selection_runs, select_parents, SelectionComparisonReport, SelectionMode,
    SelectionRunMetrics,
};

