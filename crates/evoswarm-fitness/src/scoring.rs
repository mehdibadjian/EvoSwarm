use thiserror::Error;

const EPSILON: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weights {
    pub w_a: f64,
    pub w_p: f64,
    pub w_s: f64,
}

impl Weights {
    pub fn validate(&self) -> Result<(), ScoreError> {
        let sum = self.w_a + self.w_p + self.w_s;
        if (sum - 1.0).abs() > EPSILON {
            return Err(ScoreError::WeightsNotNormalised {
                w_a: self.w_a,
                w_p: self.w_p,
                w_s: self.w_s,
                sum,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum ScoreError {
    #[error("weights do not sum to 1.0: w_a={w_a}, w_p={w_p}, w_s={w_s} (sum={sum})")]
    WeightsNotNormalised { w_a: f64, w_p: f64, w_s: f64, sum: f64 },
    #[error("cannot redistribute: w_p + w_s = {0}")]
    DivisionByZero(f64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoreTerms {
    pub adversary_pass_rate: Option<f64>,
    pub runtime: f64,
    pub parsimony: f64,
}

pub fn adversary_pass_rate(passed: usize, total: usize) -> Option<f64> {
    if total == 0 {
        None
    } else {
        Some(passed as f64 / total as f64)
    }
}

pub fn runtime_term(baseline_ms: u64, candidate_ms: u64) -> f64 {
    if candidate_ms == 0 {
        return 1.0;
    }
    let ratio = baseline_ms as f64 / candidate_ms as f64;
    ratio.clamp(0.0, 1.0)
}

pub fn parsimony_term(diff_lines: usize) -> f64 {
    (-(diff_lines as f64) / 100.0).exp()
}

pub fn score(terms: &ScoreTerms, weights: &Weights) -> Result<f64, ScoreError> {
    weights.validate()?;

    match terms.adversary_pass_rate {
        Some(a) => {
            Ok(weights.w_a * a + weights.w_p * terms.runtime + weights.w_s * terms.parsimony)
        }
        None => {
            let denom = weights.w_p + weights.w_s;
            if denom.abs() <= EPSILON {
                return Err(ScoreError::DivisionByZero(denom));
            }
            let wp = weights.w_p / denom;
            let ws = weights.w_s / denom;
            Ok(wp * terms.runtime + ws * terms.parsimony)
        }
    }
}
