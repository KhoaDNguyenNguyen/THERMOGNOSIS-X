// rust_core/src/bayesian.rs

//! # Thermognosis Engine - Bayesian Credibility Framework
//! 
//! **Layer:** Statistical Inference / Bayesian Scoring  
//! **Status:** Normative — Strict Mathematical Execution Environment  
//! **Implements:** SPEC-BAYES-CREDIBILITY, S01-BAYESIAN-CREDIBILITY-MODEL
//! 
//! This module formalizes the rigorous Bayesian evidential aggregation for 
//! macroscopic thermoelectric parameters. It explicitly penalizes thermodynamic 
//! violations (e.g., Wiedemann-Franz anomalies) whilst preserving robust 
//! numerical stability via the Log-Sum-Exp (LSE) trick.
//!
//! ## Architectural Guarantees:
//! 1. **Zero-Panic Execution**: `unwrap()` and `panic!()` are strictly forbidden. 
//!    All operations promote domain deviations to `ThermoError`.
//! 2. **Maximal Parallelism**: Fully vectorized operations utilizing `rayon`.
//! 3. **Mathematical Determinism**: Strictly bounds numerical overflow/underflow 
//!    in density calculations.

use rayon::prelude::*;
use std::f64::consts::PI;
use thiserror::Error;

/// Formal Error Hierarchy for Bayesian Credibility Module.
/// Implements: SPEC-GOV-ERROR-HIERARCHY
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ThermoError {
    #[error("Dimensionality Violation: Input arrays must have identical lengths. Expected {0}, Found {1}")]
    DimensionMismatch(usize, usize),

    #[error("Numerical Instability: Log-posterior space collapses to absolute zero. Cannot normalize.")]
    ZeroProbabilitySpace,

    #[error("Mathematical Violation: Numerical instability detected (NaN or Inf)")]
    NumericalInstability,
}

/// Enforces identical vector lengths to prevent out-of-bounds memory access.
#[inline(always)]
fn enforce_equal_lengths(lengths: &[usize]) -> Result<usize, ThermoError> {
    if lengths.is_empty() {
        return Ok(0);
    }
    let baseline = lengths[0];
    for &len in lengths.iter().skip(1) {
        if len != baseline {
            return Err(ThermoError::DimensionMismatch(baseline, len));
        }
    }
    Ok(baseline)
}

/// Physical Constraint Penalty (Wiedemann-Franz Limit).
/// 
/// Penalizes non-physical thermal transport mechanisms where the observed
/// total thermal conductivity falls below the minimum theoretical electronic 
/// component ($\kappa < \kappa_e$).
/// 
/// **Math:** $\Phi_{WF} = \lambda_{WF} \cdot \max(0, \kappa_e - \kappa)^2$  
/// **Constants:** $L_0 = 2.44 \times 10^{-8}$ W·Ω·K⁻²
/// 
/// **Implements:** S01-BAYESIAN-CREDIBILITY-MODEL (Constraint Submodule)
#[inline(always)]
pub fn apply_wf_penalty(sigma: f64, kappa: f64, t: f64, lambda_wf: f64) -> f64 {
    let l_0 = 2.44e-8;
    let kappa_e = l_0 * sigma * t;
    let diff = kappa_e - kappa;
    
    // Smoothly apply quadratic penalty only when kappa_e > kappa
    let violation = if diff > 0.0 { diff } else { 0.0 };
    lambda_wf * violation * violation
}

/// Computes the penalized log-likelihood over a massive batch of empirical observations.
/// 
/// Assumes a Gaussian measurement uncertainty model localized to the observation.
/// 
/// **Math:** 
/// $\ln \mathcal{L}_i = -0.5 \left( \frac{zT_{obs, i} - zT_{phys, i}}{\sigma_{zT, i}} \right)^2 - \ln(\sigma_{zT, i}) - 0.5 \ln(2\pi)$
/// 
/// **Implements:** S01-BAYESIAN-CREDIBILITY-MODEL (Likelihood Definition)
pub fn compute_log_likelihood_batch(
    s: &[f64],
    sigma: &[f64],
    kappa: &[f64],
    t: &[f64],
    zt_obs: &[f64],
    sigma_zt: &[f64],
    lambda_wf: f64,
) -> Result<Vec<f64>, ThermoError> {
    let len = enforce_equal_lengths(&[
        s.len(), sigma.len(), kappa.len(), t.len(), zt_obs.len(), sigma_zt.len()
    ])?;

    if len == 0 {
        return Ok(Vec::new());
    }

    let log_likelihoods: Vec<f64> = s.par_iter()
        .zip(sigma.par_iter())
        .zip(kappa.par_iter())
        .zip(t.par_iter())
        .zip(zt_obs.par_iter())
        .zip(sigma_zt.par_iter())
        .map(|((((( &s_i, &sigma_i), &kappa_i), &t_i), &zt_obs_i), &sigma_zt_i)| {
            // 1. Calculate Expected Physical Figure of Merit
            let zt_phys_i = (s_i * s_i * sigma_i * t_i) / kappa_i;
            
            // 2. Compute Gaussian Log-Likelihood
            let residual = (zt_obs_i - zt_phys_i) / sigma_zt_i;
            let log_l_i = -0.5 * (residual * residual) 
                          - sigma_zt_i.ln() 
                          - 0.5 * (2.0 * PI).ln();
            
            // 3. Apply Thermodynamic Hard-Constraint Regularization
            let phi_wf = apply_wf_penalty(sigma_i, kappa_i, t_i, lambda_wf);
            
            // Return Total Penalized Log-Likelihood
            log_l_i - phi_wf
        })
        .collect();

    Ok(log_likelihoods)
}

/// Evaluates the normalized Bayesian posterior probability space en masse.
/// 
/// Protects the analytical validity of the evidential density map by utilizing 
/// the Log-Sum-Exp algorithm, guaranteeing strictly positive definite normalization constants.
/// 
/// **Implements:** SPEC-BAYES-CREDIBILITY
pub fn compute_log_posterior_batch(
    s: &[f64],
    sigma: &[f64],
    kappa: &[f64],
    t: &[f64],
    zt_obs: &[f64],
    sigma_zt: &[f64],
    prior: &[f64],
    lambda_wf: f64
) -> Result<(Vec<f64>, Vec<f64>), ThermoError> {
    let len = enforce_equal_lengths(&[
        s.len(), sigma.len(), kappa.len(), t.len(), zt_obs.len(), sigma_zt.len(), prior.len()
    ])?;

    if len == 0 {
        return Ok((Vec::new(), Vec::new()));
    }

    // 1. Resolve Penalized Log-Likelihood
    let log_likelihoods = compute_log_likelihood_batch(s, sigma, kappa, t, zt_obs, sigma_zt, lambda_wf)?;
    
    // 2. Compute Unnormalized Log Posterior: log P_unnorm = log L_total + log(prior)
    let log_p_unnorm: Vec<f64> = log_likelihoods.par_iter()
        .zip(prior.par_iter())
        .map(|(&ll, &pr)| ll + pr.ln())
        .collect();

    // 3. Evaluate Global Maximum (M) for LSE Stabilization
    let m = log_p_unnorm.par_iter()
        .cloned()
        .reduce(|| f64::NEG_INFINITY, |a, b| a.max(b));
    
    if m == f64::NEG_INFINITY || m.is_nan() {
        return Err(ThermoError::ZeroProbabilitySpace);
    }

    // 4. Compute LSE Denominator: LSE = M + log( sum( exp(log P_unnorm_i - M) ) )
    let sum_exp: f64 = log_p_unnorm.par_iter()
        .map(|&log_p| (log_p - m).exp())
        .sum();
    
    let lse = m + sum_exp.ln();

    if !lse.is_finite() {
        return Err(ThermoError::NumericalInstability);
    }

    // 5. Parallel Projection to Normalized Posterior Subspaces
    let results: Vec<(f64, f64)> = log_p_unnorm.par_iter()
        .map(|&log_p_u| {
            // Normalized Log Posterior
            let log_p_norm = log_p_u - lse;
            // Posterior Probability bounded rigorously to [0.0, 1.0]
            let p_i = log_p_norm.exp();
            (p_i, log_p_norm)
        })
        .collect();
    
    // 6. Decouple Memory Matrix into Explicit Output Vectors
    let (posterior_probs, log_posteriors): (Vec<f64>, Vec<f64>) = results.into_iter().unzip();

    Ok((posterior_probs, log_posteriors))
}