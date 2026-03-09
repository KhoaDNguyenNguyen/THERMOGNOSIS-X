// rust_core/src/flags.rs

//! # Anomaly Flag Bitmask Registry — GAP-03
//!
//! **Layer:** Governance / Audit Trail
//! **Status:** Normative — Q1 Dataset Standard
//! **Implements:** GAP-03, SPEC-AUDIT-01
//!
//! Every validation step that detects a data quality issue MUST set the corresponding
//! flag bit on the record. The `anomaly_flags: u32` field in the output Parquet schema
//! reflects the bitwise OR of all flags set during the pipeline.
//!
//! ## Bitmask Layout (u32)
//!
//! | Bit | Constant                   | Meaning                                                    |
//! |-----|----------------------------|------------------------------------------------------------|
//! |   0 | `FLAG_WF_VIOLATION`        | Wiedemann-Franz violation (κ_lattice < 0 or L out of bounds) |
//! |   1 | `FLAG_ZT_CROSSCHECK_FAIL`  | ZT cross-check MAE exceeds tolerance                       |
//! |   2 | `FLAG_UNIT_UNKNOWN`        | Unit string not found in unit_registry.toml                |
//! |   3 | `FLAG_SIGMA_RHO_INCON`     | σ·ρ consistency ratio deviates > 5% from unity             |
//! |   4 | `FLAG_LOW_CONFIDENCE_EXP`  | Experiment type classified with low confidence             |
//! |   5 | `FLAG_INTERP_INSUFFICIENT` | Fewer than 3 common grid points for ZT cross-check         |
//! |   6 | `FLAG_SEEBECK_BOUND_EXCEED`| |S| > SEEBECK_MAX_ABS_V_PER_K (1000 µV/K)                 |
//! |   7 | `FLAG_SIGMA_BOUND_EXCEED`  | σ > SIGMA_MAX_S_PER_M (10⁷ S/m)                           |
//! |   8 | `FLAG_KAPPA_BOUND_EXCEED`  | κ > KAPPA_MAX_W_PER_MK (100 W/(m·K))                      |
//! |   9 | `FLAG_FIGUREID_MISMATCH`   | figureid absent from the record's figure array             |
//! |  10 | `FLAG_DUPLICATE_SUSPECTED` | BloomFilter indicates a likely duplicate record            |
//! |  11 | `FLAG_ALGEBRAIC_REJECT`    | T ≤ 0, σ ≤ 0, or κ ≤ 0 — state is thermodynamically undefined |

/// Wiedemann-Franz violation: κ_lattice < 0 (electronic κ exceeds total κ) or
/// effective Lorenz number outside [L_MIN, L_MAX].
///
/// Both sub-cases share this flag because both imply an inconsistency in the
/// measured σ-κ pair that cannot be resolved without additional measurement data.
pub const FLAG_WF_VIOLATION: u32 = 1 << 0;

/// ZT cross-check failure: mean absolute error between computed and reported ZT
/// exceeds 0.05 (absolute) or 10% (relative), whichever is less restrictive.
///
/// Implies a possible digitization error, implicit unit mismatch, or unreported
/// temperature offset in the source publication.
pub const FLAG_ZT_CROSSCHECK_FAIL: u32 = 1 << 1;

/// Unknown unit string encountered during ingestion; the record's raw value was
/// NOT converted to SI. The measurement MUST NOT be used for quantitative analysis
/// until the unit is identified and registered in `unit_registry.toml`.
pub const FLAG_UNIT_UNKNOWN: u32 = 1 << 2;

/// Resistivity-conductivity internal inconsistency: |σ·ρ − 1| > 0.05.
/// When both σ and ρ are present for the same (sample, temperature) point,
/// they must satisfy σ = 1/ρ within 5% tolerance (accounting for measurement noise).
pub const FLAG_SIGMA_RHO_INCON: u32 = 1 << 3;

/// Experiment type classification was inconclusive. The record defaulted to
/// `ExperimentType::Experimental` but no unambiguous experimental keyword
/// was found. Results from this record should be interpreted with caution.
pub const FLAG_LOW_CONFIDENCE_EXP: u32 = 1 << 4;

/// Insufficient overlap for ZT cross-check interpolation: fewer than 3 grid
/// points where all four transport properties (S, σ, κ, ZT_reported) are
/// simultaneously available. Cross-check was skipped.
pub const FLAG_INTERP_INSUFFICIENT: u32 = 1 << 5;

/// Seebeck coefficient magnitude exceeds the physical upper bound.
/// |S| > `SEEBECK_MAX_ABS_V_PER_K` (= 1000 µV/K = 1 mV/K).
/// The record is rejected from the clean dataset.
pub const FLAG_SEEBECK_BOUND_EXCEED: u32 = 1 << 6;

/// Electrical conductivity exceeds the physical upper bound.
/// σ > `SIGMA_MAX_S_PER_M` (= 10⁷ S/m).
/// The record is rejected from the clean dataset.
pub const FLAG_SIGMA_BOUND_EXCEED: u32 = 1 << 7;

/// Thermal conductivity exceeds the physical upper bound.
/// κ > `KAPPA_MAX_W_PER_MK` (= 100 W/(m·K)).
/// The record is rejected from the clean dataset.
pub const FLAG_KAPPA_BOUND_EXCEED: u32 = 1 << 8;

/// Figure ID mismatch: the `figureid` referenced in a data point does not appear
/// in the source file's `figure[]` array, or the figure's declared property type
/// does not match the data point's `propertyid_y`. Implies a referential integrity
/// violation in the source Starrydata JSON.
pub const FLAG_FIGUREID_MISMATCH: u32 = 1 << 9;

/// Duplicate record suspected: the xxHash3-based BloomFilter (false positive rate
/// < 10⁻⁶) indicates a record with an identical deduplication key already exists
/// in the output. The record is excluded from the clean Parquet dataset.
pub const FLAG_DUPLICATE_SUSPECTED: u32 = 1 << 10;

/// Algebraic constraint violation: T ≤ 0, σ ≤ 0, or κ ≤ 0, or any input
/// is non-finite. The thermodynamic state is undefined; no derived quantity
/// (zT, L, κ_lattice) can be computed. The record is unconditionally rejected.
pub const FLAG_ALGEBRAIC_REJECT: u32 = 1 << 11;

// ============================================================================
// CONVENIENCE: Decode a bitmask to a list of human-readable flag names
// ============================================================================

/// Returns a sorted list of flag name strings corresponding to each set bit in `flags`.
///
/// Intended for the `anomaly_flags_decoded` field in `bad_records_report.jsonl`
/// and for the human-readable `filtered_vs_unfiltered_report.md`.
///
/// # Example
/// ```
/// use rust_core::flags::{FLAG_WF_VIOLATION, FLAG_SEEBECK_BOUND_EXCEED, decode_flags};
/// let flags = FLAG_WF_VIOLATION | FLAG_SEEBECK_BOUND_EXCEED;
/// let names = decode_flags(flags);
/// assert_eq!(names, vec!["FLAG_WF_VIOLATION", "FLAG_SEEBECK_BOUND_EXCEED"]);
/// ```
pub fn decode_flags(flags: u32) -> Vec<&'static str> {
    let all: &[(u32, &str)] = &[
        (FLAG_WF_VIOLATION,        "FLAG_WF_VIOLATION"),
        (FLAG_ZT_CROSSCHECK_FAIL,  "FLAG_ZT_CROSSCHECK_FAIL"),
        (FLAG_UNIT_UNKNOWN,        "FLAG_UNIT_UNKNOWN"),
        (FLAG_SIGMA_RHO_INCON,     "FLAG_SIGMA_RHO_INCON"),
        (FLAG_LOW_CONFIDENCE_EXP,  "FLAG_LOW_CONFIDENCE_EXP"),
        (FLAG_INTERP_INSUFFICIENT, "FLAG_INTERP_INSUFFICIENT"),
        (FLAG_SEEBECK_BOUND_EXCEED,"FLAG_SEEBECK_BOUND_EXCEED"),
        (FLAG_SIGMA_BOUND_EXCEED,  "FLAG_SIGMA_BOUND_EXCEED"),
        (FLAG_KAPPA_BOUND_EXCEED,  "FLAG_KAPPA_BOUND_EXCEED"),
        (FLAG_FIGUREID_MISMATCH,   "FLAG_FIGUREID_MISMATCH"),
        (FLAG_DUPLICATE_SUSPECTED, "FLAG_DUPLICATE_SUSPECTED"),
        (FLAG_ALGEBRAIC_REJECT,    "FLAG_ALGEBRAIC_REJECT"),
    ];
    all.iter()
        .filter(|(bit, _)| flags & bit != 0)
        .map(|(_, name)| *name)
        .collect()
}

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_flags_are_distinct_powers_of_two() {
        let all_flags = [
            FLAG_WF_VIOLATION, FLAG_ZT_CROSSCHECK_FAIL, FLAG_UNIT_UNKNOWN,
            FLAG_SIGMA_RHO_INCON, FLAG_LOW_CONFIDENCE_EXP, FLAG_INTERP_INSUFFICIENT,
            FLAG_SEEBECK_BOUND_EXCEED, FLAG_SIGMA_BOUND_EXCEED, FLAG_KAPPA_BOUND_EXCEED,
            FLAG_FIGUREID_MISMATCH, FLAG_DUPLICATE_SUSPECTED, FLAG_ALGEBRAIC_REJECT,
        ];
        for f in &all_flags {
            assert!(f.is_power_of_two(), "Flag {f:#010x} is not a power of two");
        }
        // Verify uniqueness: OR of all flags must equal SUM of all flags.
        let or_all: u32 = all_flags.iter().fold(0, |acc, &f| acc | f);
        let sum_all: u64 = all_flags.iter().map(|&f| f as u64).sum();
        assert_eq!(or_all as u64, sum_all, "Flag bitmasks are not all distinct");
    }

    #[test]
    fn decode_flags_returns_correct_names() {
        let flags = FLAG_WF_VIOLATION | FLAG_SEEBECK_BOUND_EXCEED;
        let names = decode_flags(flags);
        assert!(names.contains(&"FLAG_WF_VIOLATION"));
        assert!(names.contains(&"FLAG_SEEBECK_BOUND_EXCEED"));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn decode_flags_on_zero_returns_empty() {
        assert!(decode_flags(0).is_empty());
    }

    #[test]
    fn decode_flags_on_all_bits_returns_twelve_names() {
        let all = FLAG_WF_VIOLATION | FLAG_ZT_CROSSCHECK_FAIL | FLAG_UNIT_UNKNOWN
            | FLAG_SIGMA_RHO_INCON | FLAG_LOW_CONFIDENCE_EXP | FLAG_INTERP_INSUFFICIENT
            | FLAG_SEEBECK_BOUND_EXCEED | FLAG_SIGMA_BOUND_EXCEED | FLAG_KAPPA_BOUND_EXCEED
            | FLAG_FIGUREID_MISMATCH | FLAG_DUPLICATE_SUSPECTED | FLAG_ALGEBRAIC_REJECT;
        assert_eq!(decode_flags(all).len(), 12);
    }
}
