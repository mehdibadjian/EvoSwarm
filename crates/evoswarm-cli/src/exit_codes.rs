//! Distinct process exit codes per rejection reason (e1-1 spec §3.5) so a caller can
//! branch on the outcome without parsing stderr. The numeric values are part of the CLI
//! contract; they must never be reused across reasons.

use std::process::ExitCode as StdExitCode;

/// Every way `evoswarm run` can terminate. `Ok` is zero; each rejection reason is a
/// distinct non-zero code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitCode {
    /// Job accepted and queued.
    Ok = 0,
    /// A flag was malformed or missing (e.g. empty `--paths`).
    Validation = 2,
    /// A `--paths` entry resolved outside the repository root.
    PathEscape = 3,
    /// The baseline test command could not be executed.
    BaselineCommandFailed = 4,
    /// Baseline results differed across the three consecutive runs.
    FlakyTestDetected = 5,
    /// Every baseline test already passes and `--objective perf` was not set.
    NothingToImprove = 6,
}

impl ExitCode {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn to_std(self) -> StdExitCode {
        StdExitCode::from(self.as_u8())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_distinct_and_nonzero_for_rejections() {
        let codes = [
            ExitCode::Ok,
            ExitCode::Validation,
            ExitCode::PathEscape,
            ExitCode::BaselineCommandFailed,
            ExitCode::FlakyTestDetected,
            ExitCode::NothingToImprove,
        ];
        let mut seen = std::collections::HashSet::new();
        for c in codes {
            assert!(seen.insert(c.as_u8()), "duplicate exit code {:?}", c);
        }
        assert_eq!(ExitCode::Ok.as_u8(), 0);
        for c in codes.iter().skip(1) {
            assert_ne!(c.as_u8(), 0, "{:?} must be non-zero", c);
        }
    }
}
