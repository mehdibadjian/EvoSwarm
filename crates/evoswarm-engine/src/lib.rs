//! EvoSwarm search engine: model dispatch with idempotent replay, crash recovery, budget
//! enforcement and selection (AD-5, AD-6, AD-7).
//!
//! Wave 2 lands crash recovery (e1-12), budget enforcement (e1-10) and held-out selection
//! (e1-8); the generation operators (e1-3/4/5/9) arrive in Wave 3 behind the `ModelClient`
//! seam.

pub mod dispatch;
pub mod recovery;

pub use dispatch::{
    dispatch_model, sandbox_run, CompletionRequest, CompletionResponse, DispatchError, ModelClient,
};
pub use recovery::{recover, ResumableJob};
