// rust_core/src/dedup.rs

//! # Record Deduplication Engine (GAP-04)
//!
//! **Status:** Normative — Q1 Dataset Standard
//! **Implements:** GAP-04 (BloomFilter deduplication), SPEC-DEDUP-01
//!
//! ## Design
//!
//! Deduplication key: SHA256(DOI ‖ normalized_composition ‖ T_range_bucket)
//!
//! A BloomFilter (capacity 5,000,000, FPR ≤ 10⁻⁶, ≈12 MB RAM) provides O(1)
//! amortized duplicate detection. SHA256 is used as the dedup key hash to
//! minimise false-positive collisions beyond the BloomFilter's guaranteed FPR.
//!
//! False positives are possible at FPR=10⁻⁶. Records flagged as duplicates
//! are written to `duplicates.jsonl` with `FLAG_DUPLICATE_SUSPECTED` set.
//! The `--force-include-duplicates` CLI flag bypasses this filter for
//! reproducibility audits.
//!
//! ## Temperature Bucketing
//!
//! Temperature ranges are discretised into 50 K bins to avoid treating
//! datasets with slightly different measurement endpoints as non-duplicates.
//! Bucket index = floor(T_min / 50) * 50, floor(T_max / 50) * 50.

use bloomfilter::Bloom;
use sha2::{Digest, Sha256};

use crate::flags::FLAG_DUPLICATE_SUSPECTED;
use crate::memory_guard::{MemoryGuard, MemoryPressure};

// ============================================================================
// CONSTANTS
// ============================================================================

/// BloomFilter capacity: maximum expected unique records in corpus.
const BLOOM_CAPACITY: usize = 5_000_000;

/// BloomFilter false positive rate ≤ 10⁻⁶ per SPEC-DEDUP-01.
const BLOOM_FPR: f64 = 1.0e-6;

/// Temperature bin width for range bucketing (K).
const T_BUCKET_K: f64 = 50.0;

// ============================================================================
// TYPES
// ============================================================================

/// Result of a deduplication check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DedupResult {
    /// Record appears unique — insert into BloomFilter and pass through.
    Unique,
    /// Record is a suspected duplicate — set `FLAG_DUPLICATE_SUSPECTED`.
    Duplicate,
}

/// Per-record deduplication key metadata.
#[derive(Debug, Clone)]
pub struct DedupKey {
    /// Hex-encoded SHA256 digest of the canonical key string.
    pub sha256_hex: String,
    /// Canonical key string before hashing (for audit log).
    pub raw_key: String,
}

// ============================================================================
// RECORD DEDUPLICATOR
// ============================================================================

/// Streaming deduplicator using a BloomFilter for O(1) membership tests.
///
/// # Memory
/// A 5M-capacity BloomFilter at FPR=10⁻⁶ requires approximately 12 MB RAM.
/// This is negligible relative to the 8 GB hard ceiling enforced by `MemoryGuard`.
pub struct RecordDeduplicator {
    bloom: Bloom<String>,
    /// Number of `Unique` decisions emitted.
    pub unique_count: u64,
    /// Number of `Duplicate` decisions emitted.
    pub duplicate_count: u64,
    /// Whether to bypass the filter (for reproducibility audits).
    pub bypass: bool,
}

impl RecordDeduplicator {
    /// Construct a new `RecordDeduplicator` with the canonical production parameters.
    ///
    /// Capacity = 5,000,000 records; FPR ≤ 10⁻⁶.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bloom: Bloom::new_for_fp_rate(BLOOM_CAPACITY, BLOOM_FPR),
            unique_count: 0,
            duplicate_count: 0,
            bypass: false,
        }
    }

    /// Construct with `bypass = true` (for `--force-include-duplicates`).
    #[must_use]
    pub fn new_bypass() -> Self {
        Self {
            bloom: Bloom::new_for_fp_rate(1, 0.5), // minimal bloom, never queried
            unique_count: 0,
            duplicate_count: 0,
            bypass: true,
        }
    }

    /// Check whether a record's dedup key has been seen before.
    ///
    /// - If the key is NOT in the BloomFilter, insert it and return `Unique`.
    /// - If the key IS in the BloomFilter (may be false positive), return `Duplicate`.
    ///
    /// When `bypass = true`, always returns `Unique` without consulting the filter.
    pub fn check_and_insert(&mut self, key: &DedupKey) -> DedupResult {
        if self.bypass {
            self.unique_count += 1;
            return DedupResult::Unique;
        }
        if self.bloom.check(&key.sha256_hex) {
            self.duplicate_count += 1;
            DedupResult::Duplicate
        } else {
            self.bloom.set(&key.sha256_hex);
            self.unique_count += 1;
            DedupResult::Unique
        }
    }

    /// Process a batch of dedup keys with integrated MemoryGuard pressure checks.
    ///
    /// Returns a parallel `Vec<DedupResult>` for each key. Stops early and returns
    /// `Err(MemoryPressure::Hard)` with a partial results vector if the hard memory
    /// ceiling is breached.
    pub fn ingest_batch_guarded(
        &mut self,
        keys: &[DedupKey],
        guard: &mut MemoryGuard,
    ) -> Result<Vec<DedupResult>, (Vec<DedupResult>, MemoryPressure)> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            if guard.tick() == MemoryPressure::Hard {
                return Err((results, MemoryPressure::Hard));
            }
            results.push(self.check_and_insert(key));
        }
        Ok(results)
    }

    /// Return the `FLAG_DUPLICATE_SUSPECTED` flag value if the result is `Duplicate`.
    pub fn flag_for(result: &DedupResult) -> u32 {
        match result {
            DedupResult::Duplicate => FLAG_DUPLICATE_SUSPECTED,
            DedupResult::Unique => 0,
        }
    }
}

impl Default for RecordDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// KEY CONSTRUCTION
// ============================================================================

/// Normalise a chemical composition string for deduplication.
///
/// Normalisation steps:
/// 1. Convert to lowercase.
/// 2. Remove all whitespace.
/// 3. Sort element tokens alphabetically (so "Bi2Te3" == "Te3Bi2").
///
/// Element token splitting: naive split on uppercase letter boundaries,
/// suitable for standard Hill notation compositions present in Starrydata.
#[must_use]
pub fn normalize_composition(raw: &str) -> String {
    // Strip whitespace, lowercase
    let compact: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.is_empty() {
        return String::new();
    }

    // Split on uppercase letters to extract element tokens.
    // Example: "Bi2Te3" → ["Bi2", "Te3"]
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    for ch in compact.chars() {
        if ch.is_uppercase() && !current.is_empty() {
            tokens.push(current.to_lowercase());
            current = String::new();
        }
        current.push(ch);
    }
    if !current.is_empty() {
        tokens.push(current.to_lowercase());
    }

    tokens.sort_unstable();
    tokens.join("")
}

/// Build the temperature range bucket string.
///
/// Discretises [T_min, T_max] into 50 K bins to collapse digitisation
/// jitter in endpoint temperatures.
///
/// Returns `"Tmin_{lo}_Tmax_{hi}"` where lo and hi are multiples of 50 K.
#[must_use]
pub fn temperature_range_bucket(t_min: f64, t_max: f64) -> String {
    let lo = (t_min / T_BUCKET_K).floor() as i64 * T_BUCKET_K as i64;
    let hi = (t_max / T_BUCKET_K).floor() as i64 * T_BUCKET_K as i64;
    format!("Tmin_{lo}_Tmax_{hi}")
}

/// Construct a `DedupKey` from the canonical components.
///
/// Key string: `"{doi}|{normalized_composition}|{t_bucket}"`
/// SHA256 is computed over the UTF-8 encoding of this string.
///
/// # Parameters
/// - `doi`: Digital Object Identifier from the paper record. May be empty if unknown.
/// - `composition`: Raw composition string (will be normalised internally).
/// - `t_min`: Minimum measurement temperature in the dataset (K).
/// - `t_max`: Maximum measurement temperature in the dataset (K).
#[must_use]
pub fn make_dedup_key(doi: &str, composition: &str, t_min: f64, t_max: f64) -> DedupKey {
    let norm_comp = normalize_composition(composition);
    let t_bucket = temperature_range_bucket(t_min, t_max);
    let raw_key = format!("{doi}|{norm_comp}|{t_bucket}");

    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    let digest = hasher.finalize();
    let sha256_hex = hex::encode(digest);

    DedupKey { sha256_hex, raw_key }
}

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- normalize_composition ---

    #[test]
    fn normalize_reorders_elements_alphabetically() {
        let a = normalize_composition("Bi2Te3");
        let b = normalize_composition("Te3Bi2");
        assert_eq!(a, b, "Bi2Te3 and Te3Bi2 must normalize identically");
    }

    #[test]
    fn normalize_strips_whitespace() {
        let a = normalize_composition("Bi 2 Te 3");
        let b = normalize_composition("Bi2Te3");
        assert_eq!(a, b, "Whitespace should not affect normalization");
    }

    #[test]
    fn normalize_case_insensitive_ordering() {
        let a = normalize_composition("PbTe");
        let b = normalize_composition("TePb");
        assert_eq!(a, b, "PbTe and TePb must normalize identically");
    }

    #[test]
    fn normalize_empty_string() {
        assert_eq!(normalize_composition(""), "", "Empty string must normalize to empty");
    }

    // --- temperature_range_bucket ---

    #[test]
    fn t_bucket_rounds_to_50k() {
        // 302 K → floor(302/50)*50 = 300 (same bin as 300)
        // 802 K → floor(802/50)*50 = 800 (same bin as 800; 798 would floor to 750, a different bin)
        let b1 = temperature_range_bucket(302.0, 802.0);
        let b2 = temperature_range_bucket(300.0, 800.0);
        assert_eq!(b1, b2, "302..802 should bucket to same as 300..800");
    }

    #[test]
    fn t_bucket_distinct_for_different_ranges() {
        let b1 = temperature_range_bucket(300.0, 600.0);
        let b2 = temperature_range_bucket(300.0, 900.0);
        assert_ne!(b1, b2, "Different T-max must produce different buckets");
    }

    // --- make_dedup_key ---

    #[test]
    fn dedup_key_deterministic() {
        let k1 = make_dedup_key("10.1038/nmat2090", "Bi2Te3", 300.0, 800.0);
        let k2 = make_dedup_key("10.1038/nmat2090", "Bi2Te3", 300.0, 800.0);
        assert_eq!(k1.sha256_hex, k2.sha256_hex, "Dedup key must be deterministic");
    }

    #[test]
    fn dedup_key_different_doi_differs() {
        let k1 = make_dedup_key("10.1038/nmat2090", "Bi2Te3", 300.0, 800.0);
        let k2 = make_dedup_key("10.1038/nature09996", "Bi2Te3", 300.0, 800.0);
        assert_ne!(k1.sha256_hex, k2.sha256_hex, "Different DOI must produce different key");
    }

    #[test]
    fn dedup_key_composition_order_independent() {
        let k1 = make_dedup_key("10.1038/nmat2090", "Bi2Te3", 300.0, 800.0);
        let k2 = make_dedup_key("10.1038/nmat2090", "Te3Bi2", 300.0, 800.0);
        assert_eq!(
            k1.sha256_hex, k2.sha256_hex,
            "Composition order must not affect dedup key"
        );
    }

    // --- RecordDeduplicator ---

    #[test]
    fn first_insert_is_unique() {
        let mut dedup = RecordDeduplicator::new();
        let key = make_dedup_key("10.1038/nmat2090", "Bi2Te3", 300.0, 800.0);
        assert_eq!(dedup.check_and_insert(&key), DedupResult::Unique);
        assert_eq!(dedup.unique_count, 1);
        assert_eq!(dedup.duplicate_count, 0);
    }

    #[test]
    fn second_insert_same_key_is_duplicate() {
        let mut dedup = RecordDeduplicator::new();
        let key = make_dedup_key("10.1038/nmat2090", "Bi2Te3", 300.0, 800.0);
        dedup.check_and_insert(&key);
        let result = dedup.check_and_insert(&key);
        assert_eq!(result, DedupResult::Duplicate);
        assert_eq!(dedup.duplicate_count, 1);
    }

    #[test]
    fn different_keys_are_unique() {
        let mut dedup = RecordDeduplicator::new();
        let k1 = make_dedup_key("10.1038/nmat2090", "Bi2Te3", 300.0, 800.0);
        let k2 = make_dedup_key("10.1038/nature09996", "PbTe", 400.0, 900.0);
        assert_eq!(dedup.check_and_insert(&k1), DedupResult::Unique);
        assert_eq!(dedup.check_and_insert(&k2), DedupResult::Unique);
        assert_eq!(dedup.unique_count, 2);
    }

    #[test]
    fn bypass_mode_always_returns_unique() {
        let mut dedup = RecordDeduplicator::new_bypass();
        let key = make_dedup_key("10.1038/nmat2090", "Bi2Te3", 300.0, 800.0);
        dedup.check_and_insert(&key);
        let result = dedup.check_and_insert(&key);
        assert_eq!(result, DedupResult::Unique, "Bypass mode must always return Unique");
        assert_eq!(dedup.duplicate_count, 0);
    }

    #[test]
    fn flag_for_duplicate_is_set() {
        assert_ne!(
            RecordDeduplicator::flag_for(&DedupResult::Duplicate) & FLAG_DUPLICATE_SUSPECTED,
            0,
            "FLAG_DUPLICATE_SUSPECTED must be set for Duplicate result"
        );
    }

    #[test]
    fn flag_for_unique_is_zero() {
        assert_eq!(
            RecordDeduplicator::flag_for(&DedupResult::Unique),
            0,
            "No flag bits should be set for Unique result"
        );
    }

    #[test]
    fn sha256_hex_is_64_chars() {
        let key = make_dedup_key("10.1038/nmat2090", "Bi2Te3", 300.0, 800.0);
        assert_eq!(key.sha256_hex.len(), 64, "SHA256 hex must be 64 characters");
    }
}
