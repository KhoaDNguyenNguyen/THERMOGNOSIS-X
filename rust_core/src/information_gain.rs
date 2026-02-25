// rust_core/src/information_gain.rs

//! # Thermognosis Engine - Information Gain & Data Gap Analysis
//! 
//! **Layer:** Statistical Inference / Active Learning
//! **Status:** Normative — Strict Mathematical Execution Environment
//! **Implements:** SPEC-ACTIVE-GAP, CL02-INFORMATION-GAIN-SELECTION
//! 
//! This module computes the spatial exploration entropy and topological data 
//! gaps within empirical thermoelectric measurement regimes. It strictly quantifies 
//! historical sampling bias by evaluating the Kullback-Leibler (KL) divergence 
//! of the empirical temperature distribution against an uninformative (uniform) prior.
//!
//! ## Architectural Guarantees:
//! 1. **Zero-Panic Execution:** Strictly conforms to SPEC-GOV-ERROR-HIERARCHY.
//! 2. **L'Hôpital's Singularity Resolution:** Implicitly limits $\lim_{p \to 0} p \ln(p) = 0$.
//! 3. **Zero-Copy Traversal:** Executes directly over contiguous memory slices via `rayon`.

use rayon::prelude::*;
use std::f64;

/// Struct containing the decoupled entropic evaluations for spatial data gap analysis.
/// 
/// **Implements:** CL02-INFORMATION-GAIN-SELECTION (Score Segregation)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GapScore {
    /// Shannon Entropy ($H$) of the measurement distribution.
    pub entropy: f64,
    /// KL Divergence ($D_{KL}$) strictly from a uniform distribution ($U$).
    pub kl_divergence: f64,
    /// Weighted linear aggregation: $G = \gamma_1 H + \gamma_2 D_{KL}$
    pub total_score: f64,
}

/// Computes the Information Gain and Data Gap Analysis en masse across isolated sub-manifolds.
/// 
/// Evaluates the density of the macroscopic observations over the temperature domain $T$.
/// 
/// **Math:**
/// 1. $p_k = n_k / \sum n_j$
/// 2. $H = - \sum_k p_k \ln(p_k)$
/// 3. $D_{KL}(P \parallel U) = \sum_k p_k \ln(p_k / u_k)$ where $u_k = 1 / K$
/// 4. $G = \gamma_1 H + \gamma_2 D_{KL}$
/// 
/// **Implements:** SPEC-ACTIVE-GAP
pub fn compute_information_gain_batch(
    t: &[f64],
    bounds: &[(usize, usize)],
    t_min: f64,
    t_max: f64,
    num_bins: usize,
    gamma_1: f64,
    gamma_2: f64,
) -> Result<Vec<GapScore>, crate::ThermoError> {
    // 1. Fundamental Mathematical Pre-validation
    if num_bins == 0 || t_max <= t_min {
        return Err(crate::ThermoError::NumericalInstability);
    }
    
    let len = t.len();
    
    // 2. Strict O(N) Subgraph Boundary Pre-validation 
    // Mathematically precludes out-of-bounds panics during threaded execution.
    for &(start, end) in bounds {
        if start > end || end > len {
            return Err(crate::ThermoError::DimensionMismatch(end, len));
        }
    }

    // 3. Define the Uniform Prior Density: u_k = 1.0 / K
    let u_k = 1.0 / (num_bins as f64);
    let delta = (t_max - t_min) / (num_bins as f64);

    // 4. O(M/P) Highly Parallel Entropic Evaluation
    let results: Vec<GapScore> = bounds
        .par_iter()
        .map(|&(start, end)| {
            let t_slice = &t[start..end];
            let total_counts = t_slice.len();

            // Handle vacuous sub-manifolds (0 measurements)
            if total_counts == 0 {
                return GapScore::default();
            }

            let mut counts = vec![0usize; num_bins];

            // 5. Compute the Empirical Histogram
            for &temp in t_slice {
                // Determine bin index strictly using finite mathematics
                let mut idx = ((temp - t_min) / delta).floor() as isize;
                
                // Clamping Policy: Measurements marginally outside the theoretical bounds 
                // are clamped to the terminal bins. This strictly preserves the total 
                // experimental effort metrics (n_k) and prevents probability leakage.
                if idx < 0 {
                    idx = 0;
                } else if idx >= num_bins as isize {
                    idx = (num_bins - 1) as isize;
                }
                
                counts[idx as usize] += 1;
            }

            let total_f64 = total_counts as f64;
            let mut h = 0.0;
            let mut d_kl = 0.0;

            // 6. Project Entropies safely bounding singularities
            for &count in &counts {
                if count > 0 {
                    let p_k = (count as f64) / total_f64;
                    
                    // Singularity safe execution: p_k > 0 prevents ln(0) = -inf
                    h -= p_k * p_k.ln();
                    d_kl += p_k * (p_k / u_k).ln();
                }
            }

            // 7. Compute the Aggregated Gap Score Functional
            let total_score = (gamma_1 * h) + (gamma_2 * d_kl);

            GapScore {
                entropy: h,
                kl_divergence: d_kl,
                total_score,
            }
        })
        .collect();

    Ok(results)
}