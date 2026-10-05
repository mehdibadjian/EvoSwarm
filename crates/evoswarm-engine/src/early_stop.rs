//! Plateau early-stop (e1-10 spec §2): the search terminates when the best score per
//! generation stops improving. Strictly-greater improvement is required to continue, so an
//! equal score counts as no improvement and, sustained across three generations, ends the run
//! with `plateau_early_stop`. Early stop never discards an already-verified winner; it only
//! reports the decision, leaving the caller to return the best candidate.

/// Why the loop stopped early.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// Three consecutive generations with non-increasing best scores.
    PlateauEarlyStop,
}

/// Inspects the per-generation best-score history and returns `Some(PlateauEarlyStop)` when
/// the last three generations are non-increasing, i.e. `max(S_G) <= max(S_{G-1}) <=
/// max(S_{G-2})`. Fewer than three generations can never be a plateau, so returns `None`.
pub fn should_stop(history: &[f64]) -> Option<StopReason> {
    let n = history.len();
    if n < 3 {
        return None;
    }
    let (g2, g1, g0) = (history[n - 3], history[n - 2], history[n - 1]);
    if g0 <= g1 && g1 <= g2 {
        Some(StopReason::PlateauEarlyStop)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_increasing_three_generations_is_plateau() {
        assert_eq!(should_stop(&[0.9, 0.8, 0.8]), Some(StopReason::PlateauEarlyStop));
        assert_eq!(should_stop(&[0.9, 0.8, 0.7]), Some(StopReason::PlateauEarlyStop));
        // Strictly decreasing across the window.
        assert_eq!(should_stop(&[0.5, 0.4, 0.3]), Some(StopReason::PlateauEarlyStop));
    }

    #[test]
    fn improvement_resets_the_plateau() {
        // The most recent generation improved, so the run continues.
        assert_eq!(should_stop(&[0.5, 0.5, 0.6]), None);
        assert_eq!(should_stop(&[0.9, 0.7, 0.8]), None);
    }

    #[test]
    fn fewer_than_three_generations_never_stops() {
        assert_eq!(should_stop(&[]), None);
        assert_eq!(should_stop(&[0.5]), None);
        assert_eq!(should_stop(&[0.5, 0.4]), None);
    }
}
