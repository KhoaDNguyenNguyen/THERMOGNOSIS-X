// rust_core/src/audit.rs

//! # SPEC-AUDIT-01 — Triple-Gate Epistemic Physics Validation Engine
//!
//! **Layer:** Physical Arbiter / Epistemic Quality Gate
//! **Status:** Normative — Q1 Dataset Standard (Nature Scientific Data / Cell Patterns)
//! **Implements:** SPEC-AUDIT-01, P04-WIEDEMANN-FRANZ-LIMIT, P03-PHYSICAL-CONSTRAINTS
//!
//! ## Architectural Design
//!
//! Rather than silently dropping unphysical states, this engine assigns a
//! `ConfidenceTier` and a `AnomalyFlags` bitmask to every thermodynamic state,
//! preserving the complete audit trail required by Q1 publication standards.
//!
//! ## Gate Protocol
//!
//! Every state is evaluated through three strictly ordered gates:
//! 1. **Gate 1 (Algebraic Bounds)**: Hard positivity invariants — T, σ, κ > 0.
//! 2. **Gate 2 (Wiedemann–Franz Consistency)**: Lorenz number bounds and
//!    lattice thermal conductivity positivity under the Sommerfeld reference.
//! 3. **Gate 3 (zT Cross-Validation)**: Relative deviation between computed
//!    and externally reported zT values, where available.
//!
//! ## Mathematical Definitions
//! - $zT = S^2 \sigma T / \kappa$
//! - $L   = \kappa / (\sigma T)$ (effective Lorenz number)
//! - $\kappa_{lattice} = \kappa - L_0 \sigma T$ (residual lattice contribution)

use rayon::prelude::*;
use thiserror::Error;

use crate::constants::{L0_SOMMERFELD, L_MIN, L_MAX, SEEBECK_MAX_ABS_V_PER_K, SIGMA_MAX_S_PER_M, KAPPA_MAX_W_PER_MK, T_MIN_K, T_MAX_K};
use crate::flags::{
    FLAG_WF_VIOLATION, FLAG_ZT_CROSSCHECK_FAIL,
    FLAG_SEEBECK_BOUND_EXCEED, FLAG_SIGMA_BOUND_EXCEED, FLAG_KAPPA_BOUND_EXCEED,
    FLAG_TEMPERATURE_OUT_OF_RANGE,
};

// Re-export canonical flags under the legacy names used by existing Python consumers.
// These aliases ensure that scripts checking `rust_core.FLAG_NEGATIVE_KAPPA_L` continue
// to work after the transition to the unified flags.rs bitmask set.
//
// Note: The legacy 4-bit scheme (0b0001 … 0b1000) has been replaced by the
// authoritative GAP-03 scheme. Python consumers must update to the new constants
// exported from lib.rs; these re-exports will be removed in a future release.
pub use crate::flags::FLAG_WF_VIOLATION as FLAG_NEGATIVE_KAPPA_L;
pub use crate::flags::FLAG_ZT_CROSSCHECK_FAIL as FLAG_ZT_MISMATCH;
// FIX: ERROR-01 — consolidated as a single pub use (no separate private use).
// `pub use` makes FLAG_ALGEBRAIC_REJECT both available internally and re-exported.
pub use crate::flags::FLAG_ALGEBRAIC_REJECT;

/// Legacy alias — Lorenz-out-of-bounds is now merged into FLAG_WF_VIOLATION (bit 0).
/// Both sub-violations of the Wiedemann-Franz consistency check share a single bit
/// in the GAP-03 canonical flag scheme.
pub const FLAG_LORENZ_OUT_BOUNDS: u32 = FLAG_WF_VIOLATION;

// ============================================================================
// SECTION 2: CONFIDENCE TIER ORDINAL ENUM
// SPEC-AUDIT-01 (Section 3: Tier Classification)
// ============================================================================

/// Ordinal epistemic quality classification assigned to every thermodynamic state.
///
/// Discriminant values are serialized directly as `int8` in the Parquet audit schema.
/// Tiers are strictly ordered by physical trustworthiness.
///
/// # Tier Semantics
/// | Tier       | u8 | Physical Interpretation                                    |
/// |------------|----|------------------------------------------------------------|
/// | `TierA`    |  1 | All three gates pass — full physical consistency            |
/// | `TierB`    |  2 | Lorenz number anomalous; κ_lattice and zT remain consistent |
/// | `TierC`    |  3 | κ_lattice < 0 *or* zT cross-check deviation > 10%          |
/// | `Reject`   |  4 | Algebraic bounds violated — state is thermodynamically undefined |
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfidenceTier {
    TierA  = 1,
    TierB  = 2,
    TierC  = 3,
    Reject = 4,
}

// ============================================================================
// SECTION 3: PHYSICS AUDIT RECORD (FFI OUTPUT STRUCT)
// SPEC-AUDIT-01 (Section 5: Record Layout)
// ============================================================================

/// Complete physics audit record for a single thermodynamic state.
///
/// This `#[repr(C)]` struct is the atomic unit of the Q1 audit trail. It carries
/// both the derived physical quantities (computed independently of the source data)
/// and the epistemic verdict (tier + bitmask flags).
///
/// NaN values in the float fields denote quantities that could not be computed due
/// to a Gate 1 rejection or an unavailable reported value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicsAuditRecord {
    /// Computed figure of merit: zT = S² σ T / κ. NaN on Gate 1 failure.
    pub zt_computed: f64,
    /// Lattice thermal conductivity: κ_lattice = κ − L₀σT. NaN on Gate 1 failure.
    pub kappa_lattice: f64,
    /// Effective Lorenz number: L = κ / (σT). NaN on Gate 1 failure.
    pub lorenz_number: f64,
    /// Relative zT deviation: |zT_computed − zT_reported| / |zT_reported|.
    /// NaN when no reliable reported value is available.
    pub cross_check_error: f64,
    /// Ordinal confidence tier (1=A, 2=B, 3=C, 4=Reject). Serializes to int8.
    pub tier: u8,
    /// Bitmask of detected anomaly flags. See FLAG_* constants.
    pub anomaly_flags: u32,
}

// ============================================================================
// SECTION 4: FORMAL ERROR HIERARCHY
// SPEC-GOV-ERROR-HIERARCHY
// ============================================================================

/// Formal error types for the Triple-Gate audit engine.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum AuditError {
    /// Raised when input slices have differing lengths, violating the batching contract.
    ///
    /// **AUDI-01**: All physical property slices must be identically shaped.
    #[error(
        "AUDI-01: Dimensionality Mismatch — all input slices must have identical lengths. \
         Got s={0}, sigma={1}, kappa={2}, t={3}, zt_reported={4}"
    )]
    DimensionMismatch(usize, usize, usize, usize, usize),
}

// ============================================================================
// SECTION 5: TRIPLE-GATE PHYSICS CHECKER (SCALAR KERNEL)
// SPEC-AUDIT-01 (Section 6: Gate Evaluation Protocol)
// ============================================================================

/// Evaluates a single thermodynamic state through the three-stage epistemic gate system.
///
/// This function is the scalar kernel dispatched in parallel by `audit_dataset_batch_par`.
/// It is infallible — all physical violations are encoded into the returned
/// `PhysicsAuditRecord` rather than propagated as errors, enabling branchless
/// parallel execution without short-circuit overhead.
///
/// # Gate 1 — Algebraic Bounds (Hard Physical Invariants)
/// Enforces: T > 0, σ > 0, κ > 0, all values finite.
/// Failure: immediate `ConfidenceTier::Reject` with `FLAG_ALGEBRAIC_REJECT`.
/// All derived float fields are set to `NaN`.
///
/// # Gate 2 — Wiedemann–Franz Consistency
/// Computes:
/// - L = κ / (σT)                    (effective Lorenz number)
/// - κ_lattice = κ − L₀ σ T          (residual lattice contribution under Sommerfeld)
///
/// - `L ∉ [L_MIN, L_MAX]`  → `FLAG_LORENZ_OUT_BOUNDS` → downgrade `TierA` → `TierB`
/// - `κ_lattice < 0`        → `FLAG_NEGATIVE_KAPPA_L`  → hard downgrade to `TierC`
///
/// # Gate 3 — zT Cross-Validation
/// Evaluates: ε = |zT_computed − zT_reported| / |zT_reported|
/// (skipped when `zt_reported` is non-finite or effectively zero)
///
/// - ε > 0.10 → `FLAG_ZT_MISMATCH` → hard downgrade to `TierC`
///
/// # Arguments
/// - `s`: Seebeck coefficient (V/K)
/// - `sigma`: Electrical conductivity (S/m)
/// - `kappa`: Total thermal conductivity (W m⁻¹ K⁻¹)
/// - `t`: Absolute temperature (K)
/// - `zt_reported`: Externally reported dimensionless figure of merit (NaN if absent)
///
/// # Implements
/// SPEC-AUDIT-01, P04-WIEDEMANN-FRANZ-LIMIT, P03-PHYSICAL-CONSTRAINTS
#[inline(always)]
pub fn triple_check_physics(
    s: f64,
    sigma: f64,
    kappa: f64,
    t: f64,
    zt_reported: f64,
) -> PhysicsAuditRecord {
    let mut flags: u32 = 0;
    let mut tier = ConfidenceTier::TierA;

    // -------------------------------------------------------------------------
    // GATE 1: Algebraic Bounds (Hard Thermodynamic Invariants)
    // Constraint: T > 0, σ > 0, κ > 0 and all values must be finite.
    // Any violation is immediately fatal — the state is thermodynamically undefined.
    // -------------------------------------------------------------------------
    if !s.is_finite()
        || !sigma.is_finite()
        || !kappa.is_finite()
        || !t.is_finite()
        || t <= 0.0
        || sigma <= 0.0
        || kappa <= 0.0
    {
        flags |= FLAG_ALGEBRAIC_REJECT;
        return PhysicsAuditRecord {
            zt_computed: f64::NAN,
            kappa_lattice: f64::NAN,
            lorenz_number: f64::NAN,
            cross_check_error: f64::NAN,
            tier: ConfidenceTier::Reject as u8,
            anomaly_flags: flags,
        };
    }

    // -------------------------------------------------------------------------
    // GATE 1b: Empirical Bounds Check (P03-PHYSICAL-CONSTRAINTS §7)
    // Checks |S|, σ, and κ against the canonical constants from constants.rs.
    // Records violating these bounds receive a per-property flag (GAP-03) and
    // are unconditionally rejected (Tier::Reject). This gate is positioned after
    // the algebraic check because the bounds are meaningless on NaN/Inf values.
    // -------------------------------------------------------------------------
    if s.abs() > SEEBECK_MAX_ABS_V_PER_K {
        flags |= FLAG_SEEBECK_BOUND_EXCEED;
        flags |= FLAG_ALGEBRAIC_REJECT;
        return PhysicsAuditRecord {
            zt_computed: f64::NAN,
            kappa_lattice: f64::NAN,
            lorenz_number: f64::NAN,
            cross_check_error: f64::NAN,
            tier: ConfidenceTier::Reject as u8,
            anomaly_flags: flags,
        };
    }
    if sigma > SIGMA_MAX_S_PER_M {
        flags |= FLAG_SIGMA_BOUND_EXCEED;
        flags |= FLAG_ALGEBRAIC_REJECT;
        return PhysicsAuditRecord {
            zt_computed: f64::NAN,
            kappa_lattice: f64::NAN,
            lorenz_number: f64::NAN,
            cross_check_error: f64::NAN,
            tier: ConfidenceTier::Reject as u8,
            anomaly_flags: flags,
        };
    }
    if kappa > KAPPA_MAX_W_PER_MK {
        flags |= FLAG_KAPPA_BOUND_EXCEED;
        flags |= FLAG_ALGEBRAIC_REJECT;
        return PhysicsAuditRecord {
            zt_computed: f64::NAN,
            kappa_lattice: f64::NAN,
            lorenz_number: f64::NAN,
            cross_check_error: f64::NAN,
            tier: ConfidenceTier::Reject as u8,
            anomaly_flags: flags,
        };
    }

    // -------------------------------------------------------------------------
    // TEMPERATURE DOMAIN CHECK (soft flag — VALIDATION_METHODOLOGY.md §9)
    // T ∉ [T_MIN_K, T_MAX_K] = [100 K, 2000 K]:
    //   - NOT a hard reject: zT, L, and κ_lattice are still computed.
    //   - Downgrades the record to at most TierC.
    // Physical rationale: below 100 K, measurements are cryogenic
    // characterisation outside the practical application domain; above 2000 K,
    // most thermoelectric materials have decomposed or melted.
    // Per VALIDATION_METHODOLOGY.md §9: "flagged but not automatically rejected".
    // -------------------------------------------------------------------------
    if t < T_MIN_K || t > T_MAX_K {
        flags |= FLAG_TEMPERATURE_OUT_OF_RANGE;
        // Soft downgrade: only worsen the tier, never improve it.
        if tier < ConfidenceTier::TierC {
            tier = ConfidenceTier::TierC;
        }
    }

    // -------------------------------------------------------------------------
    // GATE 2: Transport Phenomenological Checks (Wiedemann–Franz Consistency)
    // Effective Lorenz number L = κ / (σT) — total ratio.
    // Sommerfeld lattice residual: κ_lattice = κ − L₀ σ T
    // -------------------------------------------------------------------------
    let lorenz_number = kappa / (sigma * t);
    let kappa_e = L0_SOMMERFELD * sigma * t;
    let kappa_lattice = kappa - kappa_e;

    if lorenz_number < L_MIN || lorenz_number > L_MAX {
        flags |= FLAG_WF_VIOLATION;
        // Downgrade from TierA → TierB only if no more severe flag has been raised yet.
        if tier == ConfidenceTier::TierA {
            tier = ConfidenceTier::TierB;
        }
    }

    if kappa_lattice < 0.0 {
        flags |= FLAG_WF_VIOLATION;
        // Hard downgrade to TierC: electronic conduction exceeds total κ.
        // This supersedes a TierB assignment.
        tier = ConfidenceTier::TierC;
    }

    // -------------------------------------------------------------------------
    // GATE 3: zT Cross-Validation (External Consistency Check)
    // zT = S² σ T / κ
    // Deviation threshold: ε > 10% → FLAG_ZT_CROSSCHECK_FAIL
    // -------------------------------------------------------------------------
    let zt_computed = (s * s * sigma * t) / kappa;

    let cross_check_error = if zt_reported.is_finite() && zt_reported.abs() > f64::EPSILON {
        let rel_deviation = (zt_computed - zt_reported).abs() / zt_reported.abs();
        if rel_deviation > 0.10 {
            flags |= FLAG_ZT_CROSSCHECK_FAIL;
            // Hard downgrade to TierC regardless of prior Gate 2 result.
            tier = ConfidenceTier::TierC;
        }
        rel_deviation
    } else {
        // No reliable reported value available — Gate 3 is skipped without penalty.
        f64::NAN
    };

    PhysicsAuditRecord {
        zt_computed,
        kappa_lattice,
        lorenz_number,
        cross_check_error,
        tier: tier as u8,
        anomaly_flags: flags,
    }
}

// ============================================================================
// SECTION 6: PARALLEL BATCH AUDIT ENGINE
// SPEC-AUDIT-01 (Section 7: Batch Computation Protocol)
// ============================================================================

/// Executes the Triple-Gate physics audit over an entire dataset using Rayon parallelism.
///
/// Dispatches `triple_check_physics` over all N states concurrently, exploiting
/// lock-free work-stealing to achieve O(N/p) wall-clock complexity on p logical cores.
/// The per-record kernel is fully pure and free of shared mutable state, guaranteeing
/// race-free execution without any synchronisation overhead.
///
/// # Arguments
/// - `s`: Seebeck coefficient array (V/K), length N.
/// - `sigma`: Electrical conductivity array (S/m), length N.
/// - `kappa`: Total thermal conductivity array (W m⁻¹ K⁻¹), length N.
/// - `t`: Absolute temperature array (K), length N.
/// - `zt_reported`: Externally reported zT array, length N. Pass `f64::NAN` where unavailable.
///
/// # Errors
/// Returns `AuditError::DimensionMismatch` if the input slices have differing lengths.
///
/// # Implements
/// SPEC-AUDIT-01 (Section 7), SPEC-GOV-ERROR-HIERARCHY
pub fn audit_dataset_batch_par(
    s: &[f64],
    sigma: &[f64],
    kappa: &[f64],
    t: &[f64],
    zt_reported: &[f64],
) -> Result<Vec<PhysicsAuditRecord>, AuditError> {
    let n = s.len();
    if sigma.len() != n || kappa.len() != n || t.len() != n || zt_reported.len() != n {
        return Err(AuditError::DimensionMismatch(
            n,
            sigma.len(),
            kappa.len(),
            t.len(),
            zt_reported.len(),
        ));
    }

    let records: Vec<PhysicsAuditRecord> = s
        .par_iter()
        .zip(sigma.par_iter())
        .zip(kappa.par_iter())
        .zip(t.par_iter())
        .zip(zt_reported.par_iter())
        .map(|((((si, sigi), kapi), ti), zti)| {
            triple_check_physics(*si, *sigi, *kapi, *ti, *zti)
        })
        .collect();

    Ok(records)
}

// ============================================================================
// SECTION 7: UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const L0: f64 = L0_SOMMERFELD;

    fn canonical_state() -> (f64, f64, f64, f64) {
        // BiTe-class material: S=200µV/K, σ=1e5 S/m, κ=1.5 W/mK, T=300 K
        let s = 200e-6;
        let sigma = 1e5;
        let kappa = 1.5;
        let t = 300.0;
        (s, sigma, kappa, t)
    }

    #[test]
    fn tier_a_on_canonical_state() {
        let (s, sigma, kappa, t) = canonical_state();
        let expected_zt = s * s * sigma * t / kappa;
        let rec = triple_check_physics(s, sigma, kappa, t, expected_zt);

        // Tier A: all gates pass with exact zT match
        assert_eq!(rec.tier, ConfidenceTier::TierA as u8);
        assert_eq!(rec.anomaly_flags, 0);
        assert!((rec.zt_computed - expected_zt).abs() < 1e-15);
    }

    #[test]
    fn gate1_rejects_non_positive_temperature() {
        let (s, sigma, kappa, _) = canonical_state();
        let rec = triple_check_physics(s, sigma, kappa, -1.0, f64::NAN);

        assert_eq!(rec.tier, ConfidenceTier::Reject as u8);
        assert_ne!(rec.anomaly_flags & FLAG_ALGEBRAIC_REJECT, 0);
        assert!(rec.zt_computed.is_nan());
    }

    #[test]
    fn gate1_rejects_negative_kappa() {
        let (s, sigma, _, t) = canonical_state();
        let rec = triple_check_physics(s, sigma, -0.5, t, f64::NAN);

        assert_eq!(rec.tier, ConfidenceTier::Reject as u8);
        assert_ne!(rec.anomaly_flags & FLAG_ALGEBRAIC_REJECT, 0);
    }

    #[test]
    fn gate1_rejects_nan_input() {
        let rec = triple_check_physics(f64::NAN, 1e5, 1.5, 300.0, f64::NAN);
        assert_eq!(rec.tier, ConfidenceTier::Reject as u8);
    }

    #[test]
    fn gate2_flags_negative_kappa_lattice() {
        // kappa_e = L0 * sigma * T = 2.44e-8 * 1e10 * 300 = 73.2 >> kappa
        // Artificially make kappa < kappa_e — this also makes sigma > SIGMA_MAX.
        // Use a sigma at exactly SIGMA_MAX so the empirical bound does NOT reject,
        // allowing us to test Gate 2's kappa_lattice check independently.
        let sigma = 1e7_f64; // exactly SIGMA_MAX — passes empirical bound check
        let kappa = 0.01;    // far below kappa_e = L0 * 1e7 * 300 ≈ 0.073
        let t = 300.0;
        let s = 10e-6;
        let rec = triple_check_physics(s, sigma, kappa, t, f64::NAN);

        assert_ne!(rec.anomaly_flags & FLAG_WF_VIOLATION, 0, "FLAG_WF_VIOLATION must be set");
        assert_eq!(rec.tier, ConfidenceTier::TierC as u8);
    }

    #[test]
    fn gate3_flags_zt_crosscheck_fail_above_10pct() {
        let (s, sigma, kappa, t) = canonical_state();
        let zt_computed = s * s * sigma * t / kappa;
        // Inflate reported ZT by 25% above computed.
        // cross_check_error = |zt_c - zt_r| / zt_r = 0.25*zt / (1.25*zt) = 0.20 exactly.
        let zt_reported = zt_computed * 1.25;
        let rec = triple_check_physics(s, sigma, kappa, t, zt_reported);

        assert_ne!(rec.anomaly_flags & FLAG_ZT_CROSSCHECK_FAIL, 0);
        assert_eq!(rec.tier, ConfidenceTier::TierC as u8);
        assert!((rec.cross_check_error - 0.20).abs() < 1e-10);
    }

    #[test]
    fn gate3_skipped_when_zt_reported_is_nan() {
        let (s, sigma, kappa, t) = canonical_state();
        let rec = triple_check_physics(s, sigma, kappa, t, f64::NAN);

        assert_eq!(rec.anomaly_flags & FLAG_ZT_CROSSCHECK_FAIL, 0);
        assert!(rec.cross_check_error.is_nan());
    }

    /// BUG-05 regression via audit path: 50 mV/K must be rejected as Tier::Reject
    /// with FLAG_SEEBECK_BOUND_EXCEED set, not silently passed as Tier A.
    #[test]
    fn gate1b_rejects_seebeck_50mv_per_k() {
        let s = 50.0e-3; // 50 mV/K — far above 1 mV/K limit
        let rec = triple_check_physics(s, 1e5, 1.5, 300.0, f64::NAN);

        assert_eq!(rec.tier, ConfidenceTier::Reject as u8);
        assert_ne!(rec.anomaly_flags & FLAG_SEEBECK_BOUND_EXCEED, 0);
        assert!(rec.zt_computed.is_nan());
    }

    #[test]
    fn gate1b_rejects_sigma_above_max() {
        let rec = triple_check_physics(200e-6, 2e7, 1.5, 300.0, f64::NAN);
        assert_eq!(rec.tier, ConfidenceTier::Reject as u8);
        assert_ne!(rec.anomaly_flags & FLAG_SIGMA_BOUND_EXCEED, 0);
    }

    #[test]
    fn gate1b_rejects_kappa_above_max() {
        let rec = triple_check_physics(200e-6, 1e5, 500.0, 300.0, f64::NAN);
        assert_eq!(rec.tier, ConfidenceTier::Reject as u8);
        assert_ne!(rec.anomaly_flags & FLAG_KAPPA_BOUND_EXCEED, 0);
    }

    /// P0-3: T = 50 K is below T_MIN_K (100 K).
    /// The state is algebraically valid (T > 0) but outside the operational domain.
    /// Expectation: FLAG_TEMPERATURE_OUT_OF_RANGE set; tier = TierC; zT is finite.
    /// Physics parameters chosen so that Gate 2 passes at T=50K:
    ///   κ=0.3 W/mK, σ=1e5 S/m → L=0.3/(1e5·50)=6e-8 ∈ [L_MIN, L_MAX];
    ///   κ_lattice = 0.3 − 2.44e-8·1e5·50 = 0.3 − 0.122 = 0.178 ≥ 0.
    #[test]
    fn temperature_below_100k_sets_out_of_range_flag() {
        let rec = triple_check_physics(200e-6, 1e5, 0.3, 50.0, f64::NAN);

        assert_ne!(
            rec.anomaly_flags & FLAG_TEMPERATURE_OUT_OF_RANGE, 0,
            "FLAG_TEMPERATURE_OUT_OF_RANGE must be set for T=50 K"
        );
        assert_eq!(
            rec.tier, ConfidenceTier::TierC as u8,
            "Record must be downgraded to TierC for T=50 K"
        );
        // Soft flag: zT must still be computed, not NaN.
        assert!(
            rec.zt_computed.is_finite(),
            "zT must remain finite — T=50 K is a soft flag, not a hard reject"
        );
        // Hard-reject flags must NOT be set.
        assert_eq!(rec.anomaly_flags & FLAG_ALGEBRAIC_REJECT, 0);
    }

    /// P0-3: T = 2500 K is above T_MAX_K (2000 K).
    /// Same expectation: FLAG_TEMPERATURE_OUT_OF_RANGE; tier = TierC; zT finite.
    /// Physics parameters chosen so that Gate 2 passes at T=2500K:
    ///   κ=10 W/mK, σ=1e5 S/m → L=10/(1e5·2500)=4e-8 ∈ [L_MIN, L_MAX];
    ///   κ_lattice = 10 − 2.44e-8·1e5·2500 = 10 − 6.1 = 3.9 ≥ 0.
    #[test]
    fn temperature_above_2000k_sets_out_of_range_flag() {
        let rec = triple_check_physics(200e-6, 1e5, 10.0, 2500.0, f64::NAN);

        assert_ne!(
            rec.anomaly_flags & FLAG_TEMPERATURE_OUT_OF_RANGE, 0,
            "FLAG_TEMPERATURE_OUT_OF_RANGE must be set for T=2500 K"
        );
        assert_eq!(
            rec.tier, ConfidenceTier::TierC as u8,
            "Record must be downgraded to TierC for T=2500 K"
        );
        assert!(
            rec.zt_computed.is_finite(),
            "zT must remain finite — T=2500 K is a soft flag, not a hard reject"
        );
        assert_eq!(rec.anomaly_flags & FLAG_ALGEBRAIC_REJECT, 0);
    }

    #[test]
    fn batch_par_returns_dimension_error_on_mismatched_slices() {
        let s = vec![1.0_f64; 10];
        let sigma = vec![1.0_f64; 9]; // Wrong length
        let kappa = vec![1.0_f64; 10];
        let t = vec![300.0_f64; 10];
        let zt = vec![f64::NAN; 10];

        let result = audit_dataset_batch_par(&s, &sigma, &kappa, &t, &zt);
        assert!(matches!(result, Err(AuditError::DimensionMismatch(10, 9, 10, 10, 10))));
    }

    #[test]
    fn batch_par_matches_scalar_kernel() {
        let (s, sigma, kappa, t) = canonical_state();
        let n = 8;
        let s_vec = vec![s; n];
        let sigma_vec = vec![sigma; n];
        let kappa_vec = vec![kappa; n];
        let t_vec = vec![t; n];
        let zt_vec = vec![f64::NAN; n];

        let records = audit_dataset_batch_par(&s_vec, &sigma_vec, &kappa_vec, &t_vec, &zt_vec)
            .expect("Batch audit should not fail on valid inputs");

        let scalar_rec = triple_check_physics(s, sigma, kappa, t, f64::NAN);
        for rec in &records {
            assert_eq!(rec.tier, scalar_rec.tier);
            assert_eq!(rec.anomaly_flags, scalar_rec.anomaly_flags);
            assert!((rec.zt_computed - scalar_rec.zt_computed).abs() < 1e-15);
        }
    }

    #[test]
    fn kappa_lattice_is_correct_under_sommerfeld() {
        let (s, sigma, kappa, t) = canonical_state();
        let rec = triple_check_physics(s, sigma, kappa, t, f64::NAN);

        let expected_kappa_l = kappa - L0 * sigma * t;
        assert!((rec.kappa_lattice - expected_kappa_l).abs() < 1e-14);
    }
}
