pub mod baseline;
pub mod gates;
pub mod scoring;
pub mod tamper;

pub use baseline::Baseline;
pub use gates::{evaluate, GateFailure, GateInput, GateResult};
pub use scoring::{
    adversary_pass_rate, parsimony_term, runtime_term, score, ScoreError, ScoreTerms, Weights,
};
pub use tamper::detect;
