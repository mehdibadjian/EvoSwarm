//! SQLite job ledger (AD-7): the crash-resilient record of every job's state,
//! parameters, generation progress and model-call idempotency keys.
//!
//! Durability is the whole feature: the connection runs in WAL mode with
//! `synchronous=FULL` so a completed transition survives a power cut, and every
//! state change is committed atomically. e1-1 opens the ledger and records the
//! queued ticket; e1-12 extends it with generations and the call cache.

pub mod cache;
pub mod generations;
pub mod ledger;

pub use cache::CachedCall;
pub use generations::GenerationStatus;
pub use ledger::{JobLedger, JobRecord, LedgerError, RepoRoot};
