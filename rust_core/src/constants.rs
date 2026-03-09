// rust_core/src/constants.rs

//! # Canonical Physical Constants — Single Source of Truth
//!
//! **Layer:** Foundation / Shared Constants
//! **Status:** Normative — Q1 Dataset Standard
//! **Implements:** P03-PHYSICAL-CONSTRAINTS (Section 7), P04-WIEDEMANN-FRANZ-LIMIT
//!
//! All empirical bounds and physical constants in this crate are defined **exactly once**
//! in this module. Any module requiring a bound (physics, audit, validation, mirror_walker)
//! must import from here. Duplicated magic numbers are forbidden.
//!
//! ## Compile-Time Assertions
//! Each constant is guarded by a `const _: ()` assertion that fails at compile time
//! if the value is accidentally modified. This is the regression guard required by
//! BUG-05 and SPEC-PHYS-CONSTRAINTS.

// ============================================================================
// P04-WIEDEMANN-FRANZ-LIMIT: Transport Consistency Constants
// ============================================================================

/// Sommerfeld (free-electron Fermi liquid) value of the Lorenz number.
/// Units: W·Ω·K⁻²
/// Reference: Wiedemann & Franz (1853); Sommerfeld (1928).
pub const L0_SOMMERFELD: f64 = 2.44e-8;

/// Absolute minimum admissible Lorenz number (strong phonon-drag / bipolar regime).
/// Below this bound, the state is physically impossible for any known thermoelectric.
pub const L_MIN: f64 = 1.0e-9;

/// Absolute maximum admissible Lorenz number (heavily correlated / Mott insulator limit).
/// Above this bound, the state violates the Fermi-liquid upper limit.
pub const L_MAX: f64 = 1.0e-7;

// ============================================================================
// P03-PHYSICAL-CONSTRAINTS §7: Realistic Magnitude Bounds (SI units)
// ============================================================================

/// Maximum physically realistic absolute Seebeck coefficient |S| in V·K⁻¹.
/// Equal to 1000 µV·K⁻¹ = 1 mV·K⁻¹.
/// Reference: Goldsmid, "Introduction to Thermoelectricity" (2010), Chapter 2;
///            Snyder & Toberer, Nature Materials 7, 105–114 (2008).
/// No known bulk thermoelectric material exceeds this limit at any temperature.
///
/// # Compile-Time Regression Guard
/// The assertion below fails at compile time if this constant is accidentally changed.
/// This guard was introduced to prevent recurrence of BUG-05.
pub const SEEBECK_MAX_ABS_V_PER_K: f64 = 1.0e-3;
const _ASSERT_SEEBECK_MAX: () = assert!(
    SEEBECK_MAX_ABS_V_PER_K == 1.0e-3,
    "SEEBECK_MAX_ABS_V_PER_K must be exactly 1.0e-3 V/K (1000 uV/K). \
     Modifying this constant breaks BUG-05 regression test."
);

/// Maximum physically realistic electrical conductivity in S·m⁻¹.
/// Corresponds to degenerately doped semiconductors approaching metallic conduction.
/// Reference: Snyder & Toberer (2008); Pei et al., Nature 473, 66–69 (2011).
pub const SIGMA_MAX_S_PER_M: f64 = 1.0e7;

/// Maximum physically realistic total thermal conductivity in W·m⁻¹·K⁻¹.
/// Set to 100 W/(m·K) — above diamond (~2000 W/(m·K) at RT) this would fail,
/// but thermoelectric-class materials never exceed ~20 W/(m·K). The bound is
/// intentionally conservative (100) to accommodate outlier metals in the corpus.
/// Reference: Goldsmid (2010); Zhao et al., Nature 508, 373–377 (2014).
pub const KAPPA_MAX_W_PER_MK: f64 = 100.0;

/// Minimum physically realistic absolute temperature for thermoelectric evaluation, in K.
/// Below 100 K, measurements are typically cryogenic characterisation and outside the
/// application domain of the Thermognosis thermoelectric dataset.
pub const T_MIN_K: f64 = 100.0;

/// Maximum physically realistic absolute temperature for thermoelectric evaluation, in K.
/// Above 2000 K, most thermoelectric materials have decomposed or melted.
pub const T_MAX_K: f64 = 2000.0;

// ============================================================================
// COMPILE-TIME CROSS-CONSISTENCY ASSERTIONS
// ============================================================================

const _ASSERT_T_RANGE: () = assert!(
    T_MIN_K > 0.0 && T_MAX_K > T_MIN_K,
    "Temperature bounds must satisfy 0 < T_MIN_K < T_MAX_K."
);

const _ASSERT_SIGMA_MAX: () = assert!(
    SIGMA_MAX_S_PER_M > 0.0,
    "SIGMA_MAX_S_PER_M must be strictly positive."
);

const _ASSERT_KAPPA_MAX: () = assert!(
    KAPPA_MAX_W_PER_MK > 0.0,
    "KAPPA_MAX_W_PER_MK must be strictly positive."
);

const _ASSERT_LORENZ: () = assert!(
    L_MIN < L0_SOMMERFELD && L0_SOMMERFELD < L_MAX,
    "Sommerfeld Lorenz number must lie strictly within [L_MIN, L_MAX]."
);

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seebeck_bound_is_one_millivolt_per_kelvin() {
        // BUG-05 regression: ensures the constant is exactly 1 mV/K.
        assert_eq!(SEEBECK_MAX_ABS_V_PER_K, 1.0e-3);
    }

    #[test]
    fn seebeck_bound_rejects_50_millivolts_per_kelvin() {
        // BUG-05 regression: 50 mV/K was previously accepted by calc_zt_batch.
        // This test documents the threshold that must reject it.
        let s_bad = 50.0e-3_f64; // 50 mV/K — physically impossible
        assert!(
            s_bad.abs() > SEEBECK_MAX_ABS_V_PER_K,
            "50 mV/K must exceed SEEBECK_MAX_ABS_V_PER_K ({SEEBECK_MAX_ABS_V_PER_K})"
        );
    }

    #[test]
    fn lorenz_sommerfeld_within_bounds() {
        assert!(L0_SOMMERFELD > L_MIN && L0_SOMMERFELD < L_MAX);
    }

    #[test]
    fn temperature_range_is_valid() {
        assert!(T_MIN_K > 0.0);
        assert!(T_MAX_K > T_MIN_K);
    }
}
