// rust_core/tests/bayesian_tests.rs

//! # BAYESIAN EPISTEMIC BOUNDARY TESTS
//! 
//! **Author:** Lead QA Architect / Distinguished Professor of CMS
//! **Status:** Strict Enforcing (Cannot be bypassed)
//! 
//! Asserts the rigorous mathematical stability of the Bayesian evidential
//! aggregation layer. Any failure here indicates a collapse in the 
//! thermodynamic credibility mappings.

use rust_core::bayesian::{compute_log_posterior_batch, ThermoError};
use std::f64;

/// TOLERANCE SCALING FOR FLOATING POINT PARALLEL REDUCTIONS
const BASE_TOLERANCE: f64 = 1e-12;

#[test]
fn test_req1_numerical_limits_zero_panic() {
    // Inject near-zero values approximating deep cryogenic or vacuum transport regimes.
    let s = vec![1e-12];
    let sigma = vec![1e-12];
    let kappa = vec![1e-12];
    let t = vec![1e-12];
    let zt_obs = vec![0.1];
    let sigma_zt = vec![0.01];
    let prior = vec![1.0];
    let lambda_wf = 1.0;

    let result = compute_log_posterior_batch(
        &s, &sigma, &kappa, &t, &zt_obs, &sigma_zt, &prior, lambda_wf
    );

    // 1. Assert successful traversal without panic or Domain Mismatches
    assert!(result.is_ok(), "Engine failed to resolve valid tensor boundaries.");
    
    let (posteriors, log_posteriors) = result.unwrap();

    // 2. Assert Strict Mathematical Finiteness (No NaN, No Inf)
    assert!(posteriors[0].is_finite(), "FATAL: Posterior probability collapsed to a singularity.");
    assert!(log_posteriors[0].is_finite(), "FATAL: Log-Posterior collapsed to a singularity.");
    
    // 3. For N=1, the LSE trick must identically normalize the single posterior to exactly 1.0.
    assert!((posteriors[0] - 1.0).abs() < f64::EPSILON, "FATAL: Unitarity violation for N=1.");
}

#[test]
fn test_req2_log_sum_exp_stability_massive_scale() {
    // Evaluate across expanding topological dimensions to detect iterative precision loss.
    let dimensions = vec![10, 10_000, 1_000_000];
    
    for &n in &dimensions {
        let s = vec![1e-3; n];
        let sigma = vec![1e6; n];
        let kappa = vec![0.1; n];
        let t = vec![100.0; n];
        
        // zT_phys is analytically (1e-6 * 1e6 * 100) / 0.1 = 1000.0
        // We inject zT_obs intentionally far away (1099.9908) with sigma_zt = 1.0
        // Residual = ~99.9908. -0.5 * (99.9908)^2 ≈ -4999.08
        // Resulting log_likelihood approaches ~ -5000.0, deeply probing f64 underflow.
        let zt_obs = vec![1099.9908; n];
        let sigma_zt = vec![1.0; n];
        let prior = vec![1.0; n]; 
        let lambda_wf = 0.0; 

        let result = compute_log_posterior_batch(
            &s, &sigma, &kappa, &t, &zt_obs, &sigma_zt, &prior, lambda_wf
        );

        assert!(result.is_ok());
        let (posteriors, log_posteriors) = result.unwrap();

        // 1. Ensure sum of posteriors is strictly 1.0 despite log-likelihoods being ~ -5000.0
        // Tolerance scales by sqrt(N) due to non-deterministic thread reduction order in Rayon.
        let sum_p: f64 = posteriors.iter().sum();
        let scaled_tolerance = BASE_TOLERANCE * (n as f64).sqrt();
        assert!(
            (sum_p - 1.0).abs() < scaled_tolerance,
            "FATAL: LSE Normalization failed for N={}. Sum was {}, expected 1.0.", n, sum_p
        );

        // 2. Assert NO strictly absolute zeros (underflow prevention)
        assert!(
            posteriors[0] > 0.0, 
            "FATAL: Valid but tiny probabilities underflowed to absolute 0.0. LSE isolation breached."
        );

        // 3. Verify Log-Posterior domain depth
        assert!(
            log_posteriors[0] < -1.0, 
            "FATAL: Log-Posterior mapping failed. Expected deeply negative score, got {}", log_posteriors[0]
        );
    }
}

#[test]
fn test_req3_wiedemann_franz_constraint_enforcement() {
    // We construct two identical thermodynamic profiles, differing ONLY by their thermal conductivities.
    // L_0 = 2.44e-8. sigma = 1e5, T = 300.
    // kappa_e = 2.44e-8 * 1e5 * 300 = 0.732 W/mK
    
    let s = vec![1e-3, 1e-3];
    let sigma = vec![1e5, 1e5];
    let t = vec![300.0, 300.0];
    
    // State 0: Physically impossible. kappa = 0.1 (Violates minimum electronic component of 0.732)
    // State 1: Physically plausible. kappa = 1.0 (Valid, lattice component = 0.268)
    let kappa = vec![0.1, 1.0];
    
    // Set zT_obs to EXACTLY match zT_phys so the base Gaussian log-likelihood residual is strictly 0.0.
    // zT_phys_0 = (1e-6 * 1e5 * 300) / 0.1 = 300.0
    // zT_phys_1 = (1e-6 * 1e5 * 300) / 1.0 = 30.0
    let zt_obs = vec![300.0, 30.0];
    let sigma_zt = vec![1.0, 1.0];
    let prior = vec![1.0, 1.0];
    
    let lambda_wf = 100.0; // High penalty weight to aggressively highlight the divergence.

    let result = compute_log_posterior_batch(
        &s, &sigma, &kappa, &t, &zt_obs, &sigma_zt, &prior, lambda_wf
    );
    assert!(result.is_ok());
    
    let (posteriors, log_posteriors) = result.unwrap();

    // 1. The physically invalid state MUST have a strictly lower log_posterior than the valid state.
    assert!(
        log_posteriors[0] < log_posteriors[1],
        "FATAL EPISTEMIC BREACH: Unphysical state (WF violation) was not penalized. \
        log_P(Violating)={} >= log_P(Valid)={}", 
        log_posteriors[0], log_posteriors[1]
    );

    // 2. Since N=2, P(Valid) must dominantly outweigh P(Violating)
    assert!(
        posteriors[1] > 0.9,
        "FATAL: The evidential mass did not successfully shift to the physically plausible manifold."
    );
}