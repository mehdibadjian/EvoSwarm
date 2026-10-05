pub mod baseline;
pub mod gates;
pub mod holdout;
pub mod leak_scan;
pub mod scoring;
pub mod tamper;

pub use baseline::Baseline;
pub use gates::{evaluate, GateFailure, GateInput, GateResult};
pub use holdout::{assignment_hash, partition};
pub use leak_scan::{assert_no_leakage, LeakDetected};
pub use scoring::{
    adversary_pass_rate, parsimony_term, runtime_term, score, ScoreError, ScoreTerms, Weights,
};
pub use tamper::detect;
