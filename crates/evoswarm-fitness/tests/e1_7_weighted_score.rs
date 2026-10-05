use evoswarm_fitness::scoring::{adversary_pass_rate, parsimony_term, runtime_term, score, ScoreTerms, Weights};

#[test]
fn test_exact_weighted_score_math() {
    let weights = Weights {
        w_a: 0.5,
        w_p: 0.3,
        w_s: 0.2,
    };
    let terms = ScoreTerms {
        adversary_pass_rate: Some(0.8),
        runtime: 0.9,
        parsimony: 0.7,
    };

    let result = score(&terms, &weights).unwrap();
    let expected = 0.5 * 0.8 + 0.3 * 0.9 + 0.2 * 0.7;
    assert_eq!(result, expected);
}

#[test]
fn test_weight_validation_at_startup() {
    let weights = Weights {
        w_a: 0.5,
        w_p: 0.35,
        w_s: 0.2,
    };
    let terms = ScoreTerms {
        adversary_pass_rate: Some(0.8),
        runtime: 0.9,
        parsimony: 0.7,
    };

    let result = score(&terms, &weights);
    assert!(result.is_err());
    if let Err(e) = result {
        let err_msg = format!("{}", e);
        assert!(err_msg.contains("1.05"), "Error should mention the sum: {}", err_msg);
    }
}

#[test]
fn test_proportional_redistribution_without_adversary() {
    let weights = Weights {
        w_a: 0.5,
        w_p: 0.3,
        w_s: 0.2,
    };
    let terms = ScoreTerms {
        adversary_pass_rate: None,
        runtime: 0.9,
        parsimony: 0.7,
    };

    let result = score(&terms, &weights).unwrap();
    let expected = 0.6 * 0.9 + 0.4 * 0.7;
    assert_eq!(result, expected);
}

#[test]
fn test_deterministic_scoring() {
    let weights = Weights {
        w_a: 0.5,
        w_p: 0.3,
        w_s: 0.2,
    };
    let terms = ScoreTerms {
        adversary_pass_rate: Some(0.8),
        runtime: 0.9,
        parsimony: 0.7,
    };

    let first = score(&terms, &weights).unwrap();
    for _ in 0..999 {
        let result = score(&terms, &weights).unwrap();
        assert_eq!(first.to_bits(), result.to_bits());
    }
}

#[test]
fn test_edge_inputs_are_clamped() {
    let weights = Weights {
        w_a: 0.5,
        w_p: 0.3,
        w_s: 0.2,
    };
    let terms = ScoreTerms {
        adversary_pass_rate: Some(0.8),
        runtime: runtime_term(100, 0),
        parsimony: parsimony_term(0),
    };

    let result = score(&terms, &weights).unwrap();
    assert!(result.is_finite());
    assert!((0.0..=1.0).contains(&result));
}

#[test]
fn test_adversary_pass_rate_none_when_zero() {
    assert_eq!(adversary_pass_rate(0, 0), None);
    assert_eq!(adversary_pass_rate(5, 0), None);
}

#[test]
fn test_runtime_term_clamped() {
    assert_eq!(runtime_term(100, 50), 1.0);
    assert_eq!(runtime_term(100, 200), 0.5);
    assert_eq!(runtime_term(100, 0), 1.0);
}

#[test]
fn test_parsimony_term_formula() {
    let result = parsimony_term(100);
    let expected = (-1.0_f64).exp();
    assert_eq!(result, expected);
}
