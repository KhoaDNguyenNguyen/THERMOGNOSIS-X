// rust_core/src/ranking_core.rs

//! # Thermognosis Engine - Graph-Based Material Ranking Core
//! 
//! **Layer:** Graph Theory / Statistical Aggregation
//! **Status:** Normative — Strict Mathematical Execution Environment
//! **Implements:** SPEC-GRAPH-RANK, G03-EMBEDDING-RANK-THEORY
//! 
//! This module orchestrates the citation-aware, entropy-regularized material 
//! ranking algorithms. It acts natively over contiguous FFI memory slices, 
//! delivering O(1) bridging overhead and maximum parallel scaling via `rayon`.
//!
//! ## Architectural Guarantees:
//! 1. **Zero-Panic Execution:** Strictly conforms to SPEC-GOV-ERROR-HIERARCHY. 
//!    Bounds and dimensional invariants are pre-validated to mathematically preclude panics.
//! 2. **L'Hôpital's Singularity Resolution:** Implicitly resolves the P*ln(P) 
//!    removable singularity at P=0, guaranteeing dense matrix stability.
//! 3. **Zero-Copy Traversal:** Allocates zero transitional 2D vectors. Employs 
//!    tuple-bound slicing directly upon monolithic memory buffers.

use rayon::prelude::*;
use std::f64;

/// Computes the Information Entropy $H_m$ over a discrete probability distribution.
/// 
/// **Math:** $H_m = - \sum P_i \ln(P_i)$
/// 
/// **Implements:** G03-EMBEDDING-RANK-THEORY (Entropy Regularization)
/// 
/// Strictly limits the domain execution bounds to $P_i > \epsilon$ to resolve 
/// the analytical asymptote at $P=0$, preserving stability.
#[inline(always)]
pub fn compute_entropy(p: &[f64]) -> f64 {
    p.iter().fold(0.0, |acc, &p_i| {
        // Enforce the analytical limit: lim_{P->0} P ln(P) = 0
        if p_i > f64::EPSILON {
            acc - p_i * p_i.ln()
        } else {
            acc
        }
    })
}

/// Evaluates the aggregated ranking functional for a singular material topology.
/// 
/// **Implements:** SPEC-GRAPH-RANK
/// 
/// 1. **Citation-Aware Weighting**: $w_i = 1.0 + \alpha \ln(1.0 + c_i)$
/// 2. **Weighted Aggregation**: $R_m = \frac{\sum w_i P_i zT_i}{\sum w_i}$
/// 3. **Final Regularization**: $R_{final, m} = R_m - \beta H_m$
#[inline]
pub fn compute_single_material_rank(
    p: &[f64],
    zt: &[f64],
    c: &[f64],
    alpha: f64,
    beta: f64,
) -> f64 {
    if p.is_empty() {
        return 0.0;
    }

    let mut sum_w_p_zt = 0.0;
    let mut sum_w = 0.0;

    for i in 0..p.len() {
        // Guarantee logarithmic domain safety (c_i >= 0)
        let c_i = if c[i] > 0.0 { c[i] } else { 0.0 };
        
        let w_i = 1.0 + alpha * (1.0 + c_i).ln();

        sum_w_p_zt += w_i * p[i] * zt[i];
        sum_w += w_i;
    }

    // Avert theoretical Division-by-Zero in vacuous manifolds
    let r_m = if sum_w > f64::EPSILON {
        sum_w_p_zt / sum_w
    } else {
        0.0
    };

    let h_m = compute_entropy(p);

    // Apply Lagrangian entropy regularization
    r_m - beta * h_m
}

/// Computes the global material ranking manifold concurrently over massive datasets.
/// 
/// Receives memory-mapped flat tensors and extracts materialized sub-graphs 
/// utilizing precise spatial bounds `(start, end)`.
/// 
/// **Implements:** SPEC-GOV-CODE-GENERATION-PROTOCOL, G03-EMBEDDING-RANK-THEORY
pub fn compute_material_rank_batch(
    p: &[f64],
    zt: &[f64],
    c: &[f64],
    material_bounds: &[(usize, usize)],
    alpha: f64,
    beta: f64,
) -> Result<Vec<f64>, crate::ThermoError> {
    let len = p.len();

    // 1. O(1) Tensor Dimensionality Synchronization
    if zt.len() != len {
        return Err(crate::ThermoError::DimensionMismatch(len, zt.len()));
    }
    if c.len() != len {
        return Err(crate::ThermoError::DimensionMismatch(len, c.len()));
    }

    // 2. Strict O(N) Subgraph Boundary Pre-validation 
    // Fails fast prior to threaded execution to mathematically preclude bounds panics
    for &(start, end) in material_bounds {
        if start > end || end > len {
            // Reusing DimensionMismatch semantically to denote topological bounds deviation
            return Err(crate::ThermoError::DimensionMismatch(end, len));
        }
    }

    // 3. O(M/P) Highly Parallel Graph Evaluation
    // Relinquishes the GIL equivalents & engages work-stealing over hardware threads
    let ranks: Vec<f64> = material_bounds
        .par_iter()
        .map(|&(start, end)| {
            let p_slice = &p[start..end];
            let zt_slice = &zt[start..end];
            let c_slice = &c[start..end];

            compute_single_material_rank(p_slice, zt_slice, c_slice, alpha, beta)
        })
        .collect();

    Ok(ranks)
}