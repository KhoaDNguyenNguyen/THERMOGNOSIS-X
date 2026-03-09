//! # P01 & P04 — Fundamental Thermoelectric Physics Core
//!
//! **Layer:** Theory / Physics / Electronic Transport Consistency
//! **Status:** Normative — Governing Physical Law
//! **Dependencies:** SPEC-PHYS-CONSTRAINTS, P01-THERMOELECTRIC-EQUATIONS,
//!                   P03-PHYSICAL-CONSTRAINTS, P04-WIEDEMANN-FRANZ-LIMIT
//!
//! This module formalizes the governing macroscopic transport equations and
//! constitutive relations for the Thermognosis Engine. It acts as an absolute
//! enforcement barrier against unphysical states, executing linear response
//! theory and thermodynamic bounds checks.
//!
//! ## Constant Strategy (BUG-05 Fix)
//! All empirical bounds are imported from `crate::constants` — the single
//! source of truth for physical limits. Local re-exports with short names are
//! provided for ergonomics within this module. `calc_zt_batch` previously used
//! three distinct hardcoded literals that were inconsistent with the module-level
//! constants; this has been corrected.
//!
//! No silent data mutation (clamping) is permitted. All values are strictly
//! rejected via `Result::Err` or mapped to `f64::NAN` in batch paths.

use rayon::prelude::*;
use thiserror::Error;

use crate::constants::{
    L0_SOMMERFELD as _L0, L_MIN as _L_MIN, L_MAX as _L_MAX,
    SEEBECK_MAX_ABS_V_PER_K, SIGMA_MAX_S_PER_M, KAPPA_MAX_W_PER_MK,
    T_MIN_K, T_MAX_K,
};

// ============================================================================
// Module-level re-exports of canonical constants (ergonomic aliases)
// These names are part of the public API exported to Python via lib.rs.
// ============================================================================

/// Sommerfeld value for the Lorenz number. Units: W·Ω·K⁻².
pub const L0_SOMMERFELD: f64 = _L0;

/// Minimum permissible Lorenz number bound (absolute lower limit).
pub const L_MIN: f64 = _L_MIN;

/// Maximum permissible Lorenz number bound (absolute upper limit).
pub const L_MAX: f64 = _L_MAX;

/// Maximum physically realistic |S|. Alias for `SEEBECK_MAX_ABS_V_PER_K`.
/// Kept for backwards compatibility with internal callers.
pub const S_MAX_ABS: f64 = SEEBECK_MAX_ABS_V_PER_K;

/// Maximum physically realistic σ. Alias for `SIGMA_MAX_S_PER_M`.
pub const SIGMA_MAX: f64 = SIGMA_MAX_S_PER_M;

/// Maximum physically realistic κ. Alias for `KAPPA_MAX_W_PER_MK`.
pub const KAPPA_MAX: f64 = KAPPA_MAX_W_PER_MK;

/// Minimum temperature for thermoelectric evaluation. Alias for `T_MIN_K`.
pub const T_MIN: f64 = T_MIN_K;

/// Maximum temperature for thermoelectric evaluation. Alias for `T_MAX_K`.
pub const T_MAX: f64 = T_MAX_K;

// ============================================================================
// FORMAL ERROR HIERARCHY
// ============================================================================

/// Formal Error Hierarchy for physical constraint violations in the Thermognosis Engine.
/// Implements: SPEC-PHYS-CONSTRAINTS (PCON-02, PCON-03, PCON-04, PCON-06)
#[derive(Error, Debug, Clone, PartialEq)]
pub enum PhysicsError {
    #[error("PCON-02 Inequality Violation: Absolute temperature T ({0}) must be strictly positive")]
    NegativeOrZeroTemperature(f64),

    #[error("PCON-02 Inequality Violation: Thermal conductivity kappa ({0}) must be strictly positive")]
    NegativeOrZeroThermalConductivity(f64),

    #[error("PCON-02 Inequality Violation: Electrical conductivity sigma ({0}) must be strictly positive")]
    NegativeOrZeroElectricalConductivity(f64),

    #[error("PCON-03 Empirical Bound Violation: {0}")]
    PhysicalBoundViolation(String),

    #[error("PCON-03 Bound Violation: Lorenz number L ({0}) outside admissible range (1e-9, 1e-7)")]
    LorenzNumberOutOfBounds(f64),

    #[error("PCON-04 Coupling Inconsistency: Lattice thermal conductivity kappa_l ({0}) must be >= 0")]
    NegativeLatticeThermalConductivity(f64),

    #[error("PCON-02 Inequality Violation: Figure of merit zT ({0}) must be >= 0")]
    NegativeFigureOfMerit(f64),

    #[error("PCON-06 Dimensional Inconsistency: Vector sizes do not match for batch computation")]
    MismatchedArrayLengths,
}

/// FFI-compatible struct containing the decomposed thermal conductivity values.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThermalConductivityDecomposition {
    /// Electronic thermal conductivity (kappa_e) [W/m-K]
    pub kappa_e: f64,
    /// Lattice thermal conductivity (kappa_l) [W/m-K]
    pub kappa_l: f64,
    /// Enforced Lorenz number (L) [W Ω K^-2]
    pub l: f64,
}

// ============================================================================
// CORE COMPUTATIONAL OPERATORS
// ============================================================================

/// Validates empirical magnitudes of macroscopic parameters against established bounds.
/// 
/// **Academically Rigorous Rejection Strategy:**
/// We strictly reject boundary-violating vectors via `Result::Err` rather than silently clamping 
/// them to the maximal bounds. Clamping artificially injects $0$ gradients and infinite density spikes 
/// into the Bayesian active learning manifold, fatally polluting the entropy calculations of `IG(v)`.
/// 
/// **Implements:** P03-PHYSICAL-CONSTRAINTS (Section 7)
#[inline]
pub fn validate_empirical_bounds(s: f64, sigma: f64, kappa: f64, t: f64) -> Result<(), PhysicsError> {
    if s.abs() > S_MAX_ABS {
        return Err(PhysicsError::PhysicalBoundViolation(format!("|S| exceeds {S_MAX_ABS} V/K")));
    }
    if sigma > SIGMA_MAX {
        return Err(PhysicsError::PhysicalBoundViolation(format!("sigma exceeds {SIGMA_MAX} S/m")));
    }
    if kappa > KAPPA_MAX {
        return Err(PhysicsError::PhysicalBoundViolation(format!("kappa exceeds {KAPPA_MAX} W/mK")));
    }
    if t < T_MIN || t > T_MAX {
        return Err(PhysicsError::PhysicalBoundViolation(format!("T ({t} K) lies outside realistic operational regime [{T_MIN}, {T_MAX}]")));
    }
    Ok(())
}

/// Strictly constrained computation of the dimensionless Thermoelectric Figure of Merit (zT).
///
/// Ensures hard physical thermodynamic bounds are satisfied *prior* to mathematical evaluation,
/// protecting the system from singularities (Division by Zero) and entropy production violations.
///
/// **Math:** $zT = \frac{S^2 \sigma T}{\kappa}$
/// **Implements:** P01-THERMOELECTRIC-EQUATIONS, P03-PHYSICAL-CONSTRAINTS
#[inline(always)]
pub fn compute_zt(s: f64, sigma: f64, kappa: f64, t: f64) -> Result<f64, PhysicsError> {
    // Stage 1: Hard Thermodynamic & Entropy Constraints (P03 - Level 1)
    if kappa <= 0.0 { 
        return Err(PhysicsError::NegativeOrZeroThermalConductivity(kappa)); 
    }
    if t <= 0.0 { 
        return Err(PhysicsError::NegativeOrZeroTemperature(t)); 
    }
    if sigma <= 0.0 { 
        return Err(PhysicsError::NegativeOrZeroElectricalConductivity(sigma)); 
    }

    // Stage 2: Empirical Bounded Plausibility Constraints (P03 - Level 3)
    validate_empirical_bounds(s, sigma, kappa, t)?;

    // Stage 3: Phenomenological Evaluation (P01)
    let z_t = (s * s * sigma * t) / kappa;

    // Stage 4: Resultant Positivity Invariant check
    if z_t < 0.0 {
        return Err(PhysicsError::NegativeFigureOfMerit(z_t));
    }

    Ok(z_t)
}

/// Strictly isolated calculation of the localized Lorenz Number.
///
/// Extracted to enforce isolation of transport constraints independent of thermal decomposition.
/// 
/// **Math:** $L = \frac{\kappa_e}{\sigma T}$
#[inline(always)]
pub fn compute_lorenz(sigma: f64, kappa_e: f64, t: f64) -> Result<f64, PhysicsError> {
    if sigma <= 0.0 { 
        return Err(PhysicsError::NegativeOrZeroElectricalConductivity(sigma)); 
    }
    if t <= 0.0 { 
        return Err(PhysicsError::NegativeOrZeroTemperature(t)); 
    }
    
    Ok(kappa_e / (sigma * t))
}

/// Performs strictly constrained Wiedemann-Franz decomposition of thermal conductivity.
///
/// **Math:** $\kappa_e = L \sigma T$, $\kappa_l = \kappa - \kappa_e$
/// **Implements:** P04-WIEDEMANN-FRANZ-LIMIT
#[inline]
pub fn wiedemann_franz_decomposition(
    sigma: f64,
    kappa: f64,
    t: f64,
    l: Option<f64>,
) -> Result<ThermalConductivityDecomposition, PhysicsError> {
    // Stage 1: Absolute Physical Limits
    if sigma <= 0.0 { return Err(PhysicsError::NegativeOrZeroElectricalConductivity(sigma)); }
    if kappa <= 0.0 { return Err(PhysicsError::NegativeOrZeroThermalConductivity(kappa)); }
    if t <= 0.0 { return Err(PhysicsError::NegativeOrZeroTemperature(t)); }

    // Stage 2: Lorenz Domain Check
    let l_actual = l.unwrap_or(L0_SOMMERFELD);
    if l_actual <= L_MIN || l_actual >= L_MAX {
        return Err(PhysicsError::LorenzNumberOutOfBounds(l_actual));
    }

    // Stage 3: Decomposition
    let kappa_e = l_actual * sigma * t;
    let kappa_l = kappa - kappa_e;

    // Stage 4: Sub-lattice Constraint Check
    if kappa_l < 0.0 {
        return Err(PhysicsError::NegativeLatticeThermalConductivity(kappa_l));
    }

    Ok(ThermalConductivityDecomposition {
        kappa_e,
        kappa_l,
        l: l_actual,
    })
}

// ============================================================================
// MASSIVE MACROSCOPIC BATCH OPERATORS
// ============================================================================

/// Highly optimized batch computation of Figure of Merit (zT) using parallel iterators.
/// Designed for zero-copy FFI invocation over vast macroscopic parameter arrays.
///
/// **Implements:** SPEC-PHYS-CONSTRAINTS, P01-THERMOELECTRIC-EQUATIONS
// Trong rust_core/src/physics.rs

/// Highly optimized batch computation of Figure of Merit (zT) using parallel iterators.
///
/// Returns `f64::NAN` for any data point that fails a physical constraint rather than
/// propagating an error, enabling branchless parallel evaluation of large arrays.
///
/// ## BUG-05 Fix
/// The empirical bounds in this function previously used three independent hardcoded
/// literals that were inconsistent with the module-level constants `S_MAX_ABS`,
/// `SIGMA_MAX`, and `KAPPA_MAX`. Specifically:
///   - `s_val.abs() > 0.05`    was 50 mV/K — 50× the correct limit of 1 mV/K.
///   - `sigma_val > 1e8`       was 10× too permissive relative to `SIGMA_MAX = 1e7`.
///   - `kappa_val > 5000.0`    was 50× too permissive relative to `KAPPA_MAX = 100.0`.
///
/// All three now use the canonical constants imported from `crate::constants`.
///
/// ## Physical Rejection Criteria (in order)
/// 1. **P03 Positivity**: T > 0, κ > 0, σ > 0 (all must be finite).
/// 2. **P04 Wiedemann-Franz**: L = κ/(σT) must lie in `[L_MIN, L_MAX]`.
/// 3. **P03 Empirical Bounds**: |S| ≤ `S_MAX_ABS`, σ ≤ `SIGMA_MAX`, κ ≤ `KAPPA_MAX`.
/// 4. **P01 ZT Positivity and Finiteness**: zT ∈ [0, 4].
///
/// **Implements:** SPEC-PHYS-CONSTRAINTS, P01-THERMOELECTRIC-EQUATIONS
pub fn calc_zt_batch(
    s: &[f64],
    sigma: &[f64],
    kappa: &[f64],
    t: &[f64],
) -> Result<Vec<f64>, PhysicsError> {
    let len = s.len();
    if sigma.len() != len || kappa.len() != len || t.len() != len {
        return Err(PhysicsError::MismatchedArrayLengths);
    }

    let results: Vec<f64> = s.par_iter()
        .zip(sigma)
        .zip(kappa)
        .zip(t)
        .map(|(((si, sigi), kapi), ti)| {
            let s_val = *si;
            let sigma_val = *sigi;
            let kappa_val = *kapi;
            let t_val = *ti;

            // Stage 1 — P03 Positivity + Finiteness (hard thermodynamic invariants)
            if !s_val.is_finite()
                || !sigma_val.is_finite()
                || !kappa_val.is_finite()
                || !t_val.is_finite()
                || t_val <= 0.0
                || kappa_val <= 0.0
                || sigma_val <= 0.0
            {
                return f64::NAN;
            }

            // Stage 2 — P04 Wiedemann-Franz Limit: L = κ/(σT) ∈ [L_MIN, L_MAX]
            let implied_l = kappa_val / (sigma_val * t_val);
            if implied_l < L_MIN || implied_l > L_MAX {
                return f64::NAN;
            }

            // Stage 3 — P03 Empirical Bounds (BUG-05 corrected to use canonical constants)
            // |S| must not exceed 1000 µV/K = 1 mV/K = 1e-3 V/K (S_MAX_ABS).
            // σ must not exceed 10^7 S/m (SIGMA_MAX).
            // κ must not exceed 100 W/(m·K) (KAPPA_MAX).
            if s_val.abs() > S_MAX_ABS || sigma_val > SIGMA_MAX || kappa_val > KAPPA_MAX {
                return f64::NAN;
            }

            // Stage 4 — P01 Evaluation: zT = S²σT/κ
            let zt = (s_val * s_val * sigma_val * t_val) / kappa_val;

            // Stage 5 — P05 Resultant Positivity and physical upper bound
            // zT ≤ 4 is a practical limit; no bulk material exceeds ~3.5 as of 2025.
            if !zt.is_finite() || zt < 0.0 || zt > 4.0 {
                return f64::NAN;
            }

            zt
        })
        .collect();

    Ok(results)
}

// ============================================================================
// BUG-05 REGRESSION TESTS FOR calc_zt_batch
// ============================================================================

#[cfg(test)]
mod calc_zt_batch_tests {
    use super::*;

    /// Canonical BiTe reference: S=200µV/K, σ=1e5 S/m, κ=1.5 W/(m·K), T=300 K.
    fn canonical() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        (vec![200e-6], vec![1e5], vec![1.5], vec![300.0])
    }

    #[test]
    fn canonical_state_produces_finite_zt() {
        let (s, sig, kap, t) = canonical();
        let r = calc_zt_batch(&s, &sig, &kap, &t).unwrap();
        assert!(r[0].is_finite() && r[0] > 0.0, "Canonical state must yield finite positive zT");
    }

    /// BUG-05 regression: 50 mV/K was previously accepted by the wrong `0.05` literal.
    /// It must now produce NAN because |S| = 0.05 V/K > S_MAX_ABS = 1e-3 V/K.
    #[test]
    fn bug05_seebeck_50mv_per_k_is_rejected() {
        let s = vec![50.0e-3_f64]; // 50 mV/K — physically impossible for any thermoelectric
        let sig = vec![1e5_f64];
        let kap = vec![1.5_f64];
        let t   = vec![300.0_f64];
        let r = calc_zt_batch(&s, &sig, &kap, &t).unwrap();
        assert!(
            r[0].is_nan(),
            "50 mV/K Seebeck must be rejected (NAN). BUG-05 regression. Got: {}",
            r[0]
        );
    }

    /// BUG-05 regression: sigma=1e8 S/m was previously accepted by the wrong `1e8` literal.
    /// It must now produce NAN because σ = 1e8 > SIGMA_MAX = 1e7.
    #[test]
    fn bug05_sigma_1e8_is_rejected() {
        let s   = vec![200e-6_f64];
        let sig = vec![1e8_f64]; // 10× above SIGMA_MAX
        let kap = vec![1.5_f64];
        let t   = vec![300.0_f64];
        let r = calc_zt_batch(&s, &sig, &kap, &t).unwrap();
        assert!(r[0].is_nan(), "sigma=1e8 S/m must be rejected. BUG-05 regression.");
    }

    /// BUG-05 regression: kappa=5000 W/(m·K) was previously accepted by the wrong `5000.0` literal.
    /// It must now produce NAN because κ = 5000 >> KAPPA_MAX = 100.
    #[test]
    fn bug05_kappa_5000_is_rejected() {
        let s   = vec![200e-6_f64];
        let sig = vec![1e5_f64];
        let kap = vec![5000.0_f64]; // 50× above KAPPA_MAX
        let t   = vec![300.0_f64];
        let r = calc_zt_batch(&s, &sig, &kap, &t).unwrap();
        assert!(r[0].is_nan(), "kappa=5000 W/(m·K) must be rejected. BUG-05 regression.");
    }

    #[test]
    fn negative_temperature_is_rejected() {
        let s   = vec![200e-6_f64];
        let sig = vec![1e5_f64];
        let kap = vec![1.5_f64];
        let t   = vec![-50.0_f64];
        let r = calc_zt_batch(&s, &sig, &kap, &t).unwrap();
        assert!(r[0].is_nan());
    }

    #[test]
    fn mismatched_lengths_return_error() {
        let s   = vec![200e-6_f64; 5];
        let sig = vec![1e5_f64; 4]; // wrong length
        let kap = vec![1.5_f64; 5];
        let t   = vec![300.0_f64; 5];
        assert!(calc_zt_batch(&s, &sig, &kap, &t).is_err());
    }
}

/// Highly optimized batch computation of the Wiedemann-Franz decomposition.
///
/// **Implements:** SPEC-PHYS-CONSTRAINTS, P04-WIEDEMANN-FRANZ-LIMIT
pub fn wiedemann_franz_batch(
    sigma: &[f64],
    kappa: &[f64],
    t: &[f64],
    l: Option<f64>,
) -> Result<Vec<ThermalConductivityDecomposition>, PhysicsError> {
    let len = sigma.len();
    if kappa.len() != len || t.len() != len {
        return Err(PhysicsError::MismatchedArrayLengths);
    }

    sigma.par_iter()
        .zip(kappa)
        .zip(t)
        .map(|((sigi, kapi), ti)| wiedemann_franz_decomposition(*sigi, *kapi, *ti, l))
        .collect()
}