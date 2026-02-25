//! # P01 & P04 — Fundamental Thermoelectric Physics Core
//! 
//! **Layer:** Theory / Physics / Electronic Transport Consistency
//! **Status:** Normative — Governing Physical Law
//! **Dependencies:** SPEC-PHYS-CONSTRAINTS, P01-THERMOELECTRIC-EQUATIONS, P03-PHYSICAL-CONSTRAINTS, P04-WIEDEMANN-FRANZ-LIMIT
//!
//! This module formalizes the governing macroscopic transport equations and constitutive 
//! relations for the Thermognosis Engine. It acts as an absolute enforcement barrier against 
//! unphysical states, executing linear response theory and thermodynamic bounds.
//!
//! All FFI invocations from Python to this core execute purely deterministic `f64` mathematics.
//! No silent data mutation (e.g., clamping) is permitted, preserving the topological integrity 
//! of the downstream Bayesian surrogate space.

use rayon::prelude::*;
use thiserror::Error;

// ============================================================================
// P04-WIEDEMANN-FRANZ-LIMIT: Transport Consistency Constants
// ============================================================================

/// Sommerfeld value for the Lorenz number under the free-electron Fermi-liquid approximation.
/// Units: W Ω K^-2
pub const L0_SOMMERFELD: f64 = 2.44e-8;

/// Minimum permissible bound for the Lorenz number (L).
pub const L_MIN: f64 = 1e-9;

/// Maximum permissible bound for the Lorenz number (L).
pub const L_MAX: f64 = 1e-7;

// ============================================================================
// P03-PHYSICAL-CONSTRAINTS: Section 7 - Realistic Magnitude Constraints
// ============================================================================

/// Maximum physically realistic absolute Seebeck coefficient (|S|) in V/K. (1000 µV/K)
pub const S_MAX_ABS: f64 = 1000e-6; 

/// Maximum physically realistic electrical conductivity (sigma) in S/m.
pub const SIGMA_MAX: f64 = 1e7;

/// Maximum physically realistic total thermal conductivity (kappa) in W/(m·K).
pub const KAPPA_MAX: f64 = 100.0;

/// Minimum thermodynamic domain evaluation bound for Temperature (T) in K.
pub const T_MIN: f64 = 100.0;

/// Maximum thermodynamic domain evaluation bound for Temperature (T) in K.
pub const T_MAX: f64 = 2000.0;

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

    let l0_sommerfeld = 2.44e-8; // Số Lorenz chuẩn của kim loại
    let l_min = 1e-9;            // Giới hạn dưới tuyệt đối cho bán dẫn (P04)

    // Trả về mảng kết quả: Nếu vi phạm vật lý, trả về f64::NAN
    let results: Vec<f64> = s.par_iter()
        .zip(sigma)
        .zip(kappa)
        .zip(t)
        .map(|(((si, sigi), kapi), ti)| {
            let s_val = *si;
            let sigma_val = *sigi;
            let kappa_val = *kapi;
            let t_val = *ti;

            // 1. P03: Positivity Constraints
            if t_val <= 0.0 || kappa_val <= 0.0 || sigma_val <= 0.0 {
                return f64::NAN;
            }

            // 2. P04: Wiedemann-Franz Limit (kappa_e = L * sigma * T)
            let implied_l = kappa_val / (sigma_val * t_val);
            if implied_l < l_min {
                return f64::NAN;
            }

            // 3. Empirical bounds
            if s_val.abs() > 0.05 || sigma_val > 1e8 || kappa_val > 5000.0 {
                return f64::NAN;
            }

            // 4. Calculate ZT
            let zt = (s_val * s_val * sigma_val * t_val) / kappa_val;

            if !zt.is_finite() || zt < 0.0 || zt > 4.0 {
                return f64::NAN;
            }

            zt
        })
        .collect();

    Ok(results)
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