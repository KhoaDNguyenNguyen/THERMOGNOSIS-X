// rust_core/tests/entropy_tests.rs

//! # INFORMATION ENTROPY L'HÔPITAL SINGULARITY TESTS
//! 
//! **Author:** Lead QA Architect / Distinguished Professor of CMS
//! **Status:** Strict Enforcing (Cannot be bypassed)
//! 
//! Asserts the rigorous mathematical boundary handling of spatial exploration
//! metrics. Specifically guarantees the engine resolves $0 \cdot \ln(0)$ limits 
//! without propagating `NaN` pollution into the Active Learning selection mechanism.

use rust_core::information_gain::{compute_information_gain_batch, GapScore};
use std::f64;

#[test]
fn test_req4_entropy_singularity_avoidance() {
    // Generate an incredibly biased topological sampling.
    // 4 data points, ALL located exactly at 150.0 K.
    let t = vec![150.0, 150.0, 150.0, 150.0];
    
    // We isolate the whole batch
    let bounds = vec![(0, t.len())];
    
    // Create 4 bins: [100-200], [200-300], [300-400], [400-500]
    let t_min = 100.0;
    let t_max = 500.0;
    let num_bins = 4;
    
    let gamma_1 = 1.0;
    let gamma_2 = 1.0;

    let result = compute_information_gain_batch(
        &t, &bounds, t_min, t_max, num_bins, gamma_1, gamma_2
    );
    
    assert!(result.is_ok(), "FATAL: Engine failed to traverse spatial topology.");
    
    let gap_scores = result.unwrap();
    assert_eq!(gap_scores.len(), 1);
    let score = &gap_scores[0];

    // Mathematical Ground Truth Analysis:
    // Bin 0 (100-200): 4 counts -> p_0 = 1.0
    // Bin 1 (200-300): 0 counts -> p_1 = 0.0
    // Bin 2 (300-400): 0 counts -> p_2 = 0.0
    // Bin 3 (400-500): 0 counts -> p_3 = 0.0
    //
    // Entropy = - (1.0 * ln(1.0)) - (0 * ln(0)) - (0 * ln(0)) - (0 * ln(0))
    // Because p_k=0 elements are safely bypassed, Entropy MUST be exactly 0.0.
    
    assert!(
        score.entropy.is_finite(), 
        "FATAL: Entropy resolved to NaN. Engine failed to traverse L'Hôpital's rule for p_k=0."
    );
    
    assert_eq!(
        score.entropy, 0.0, 
        "FATAL: Entropy of a perfectly certain, singular distribution must be absolute 0.0."
    );

    // KL Divergence Ground Truth Analysis:
    // Uniform Prior u_k = 1/4 = 0.25
    // D_KL = p_0 * ln(p_0 / u_0) = 1.0 * ln(1.0 / 0.25) = ln(4.0)
    let expected_kl = 4.0_f64.ln();
    
    assert!(
        score.kl_divergence.is_finite(),
        "FATAL: KL Divergence resolved to NaN."
    );
    
    assert!(
        (score.kl_divergence - expected_kl).abs() < f64::EPSILON,
        "FATAL: KL divergence computed incorrectly. Expected {}, got {}", expected_kl, score.kl_divergence
    );
    
    // Aggregated Score
    let expected_total = (gamma_1 * 0.0) + (gamma_2 * expected_kl);
    assert!(
        (score.total_score - expected_total).abs() < f64::EPSILON,
        "FATAL: Aggregated Gap Score functional is arithmetically corrupted."
    );
}

#[test]
fn test_vacuous_manifold_handling() {
    // Ensure that mapping over entirely empty subsets (0 observations)
    // cleanly yields an identity zero state, averting Division by Zero when calculating p_k.
    let t: Vec<f64> = vec![];
    let bounds = vec![(0, 0)];
    let result = compute_information_gain_batch(
        &t, &bounds, 100.0, 500.0, 4, 1.0, 1.0
    ).unwrap();

    let score = &result[0];
    assert_eq!(score.entropy, 0.0, "Vacuous subset failed to yield 0.0 entropy.");
    assert_eq!(score.kl_divergence, 0.0, "Vacuous subset failed to yield 0.0 KL divergence.");
    assert_eq!(score.total_score, 0.0, "Vacuous subset failed to yield 0.0 total score.");
}