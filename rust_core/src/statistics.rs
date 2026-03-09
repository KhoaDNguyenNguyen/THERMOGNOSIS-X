// rust_core/src/statistics.rs

//! # Streaming Statistics Engine (TASK 4)
//!
//! **Status:** Normative — Q1 Dataset Standard
//! **Implements:** TASK 4 (publication-quality statistics), SPEC-STATS-01
//!
//! ## Algorithms
//!
//! - **Welford's online algorithm** (1962): O(1)-memory mean and variance.
//!   Reference: Welford, Technometrics 4(3), 419–420 (1962).
//!
//! - **P² algorithm** (Jain & Chlamtac 1985): O(1)-memory streaming quantile
//!   estimation (Q1, median, Q3, 5th, 95th percentiles).
//!   Reference: Jain & Chlamtac, Commun. ACM 28(10), 1076–1085 (1985).
//!   DOI: 10.1145/4372.4378
//!
//! - **Kolmogorov–Smirnov two-sample test**: Comparison between filtered and
//!   unfiltered property distributions. Uses the exact KS statistic; p-value
//!   approximated via the Kolmogorov distribution asymptotic expansion.
//!
//! ## Outputs
//!
//! - `pipeline_summary.json`: Per-stage record counts and rejection rates.
//! - `property_statistics.json`: Per-property descriptive statistics (mean,
//!   std, quartiles, skewness, kurtosis, count) for clean and raw datasets.
//! - `filtered_vs_unfiltered_report.md`: Publication-ready Markdown table
//!   comparing raw vs. filtered distributions with KS test results.

use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::memory_guard::{MemoryGuard, MemoryPressure};

// ============================================================================
// WELFORD ONLINE ACCUMULATOR
// ============================================================================

/// Online mean/variance accumulator using Welford's algorithm.
///
/// Memory: 5 × f64 + u64 = 48 bytes per accumulator (independent of N).
#[derive(Debug, Clone)]
pub struct WelfordAccumulator {
    count: u64,
    mean: f64,
    /// Running M₂ = Σ(xᵢ − mean)²
    m2: f64,
    min: f64,
    max: f64,
}

impl Default for WelfordAccumulator {
    fn default() -> Self {
        Self { count: 0, mean: 0.0, m2: 0.0, min: f64::INFINITY, max: f64::NEG_INFINITY }
    }
}

impl WelfordAccumulator {
    /// Create a new empty accumulator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingest a single observation.
    pub fn update(&mut self, x: f64) {
        self.count += 1;
        let delta = x - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = x - self.mean;
        self.m2 += delta * delta2;
        if x < self.min { self.min = x; }
        if x > self.max { self.max = x; }
    }

    /// Number of observations ingested.
    #[must_use]
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Sample mean. Returns `f64::NAN` if no observations.
    #[must_use]
    pub fn mean(&self) -> f64 {
        if self.count == 0 { f64::NAN } else { self.mean }
    }

    /// Minimum observed value. Returns `f64::NAN` if no observations.
    #[must_use]
    pub fn min(&self) -> f64 {
        if self.count == 0 { f64::NAN } else { self.min }
    }

    /// Maximum observed value. Returns `f64::NAN` if no observations.
    #[must_use]
    pub fn max(&self) -> f64 {
        if self.count == 0 { f64::NAN } else { self.max }
    }

    /// Population variance (divides by N). Returns `f64::NAN` if count < 1.
    #[must_use]
    pub fn variance_population(&self) -> f64 {
        if self.count < 1 { f64::NAN } else { self.m2 / self.count as f64 }
    }

    /// Sample variance (divides by N−1). Returns `f64::NAN` if count < 2.
    #[must_use]
    pub fn variance_sample(&self) -> f64 {
        if self.count < 2 { f64::NAN } else { self.m2 / (self.count - 1) as f64 }
    }

    /// Population standard deviation.
    #[must_use]
    pub fn std_population(&self) -> f64 {
        self.variance_population().sqrt()
    }

    /// Sample standard deviation.
    #[must_use]
    pub fn std_sample(&self) -> f64 {
        self.variance_sample().sqrt()
    }

    /// Merge another accumulator into this one using the parallel Welford formula.
    ///
    /// After `a.merge(&b)`, `a` contains statistics over the combined population.
    /// Reference: Chan et al. (1979) parallel algorithm for combining mean and M₂.
    pub fn merge(&mut self, other: &Self) {
        if other.count == 0 { return; }
        let combined_count = self.count + other.count;
        let delta = other.mean - self.mean;
        self.m2 += other.m2 + delta * delta * self.count as f64 * other.count as f64 / combined_count as f64;
        self.mean = (self.mean * self.count as f64 + other.mean * other.count as f64) / combined_count as f64;
        self.count = combined_count;
        if other.min < self.min { self.min = other.min; }
        if other.max > self.max { self.max = other.max; }
    }
}

// ============================================================================
// P² QUANTILE ESTIMATOR
// ============================================================================

/// P² streaming quantile estimator (Jain & Chlamtac 1985).
///
/// Estimates a single quantile p ∈ (0, 1) using exactly 5 marker positions.
/// Memory: 5 × 2 × f64 + 5 × i64 = 120 bytes per estimator.
#[derive(Debug, Clone)]
pub struct P2Quantile {
    /// Target quantile in (0, 1).
    p: f64,
    /// Marker heights q[0..5].
    q: [f64; 5],
    /// Marker positions n[0..5] (integer).
    n: [i64; 5],
    /// Desired marker positions n'[0..5].
    n_desired: [f64; 5],
    /// Number of observations ingested.
    count: u64,
    /// Buffer for initial 5 observations before algorithm starts.
    init_buf: Vec<f64>,
}

impl P2Quantile {
    /// Create a new P² quantile estimator for quantile `p`.
    ///
    /// # Panics
    /// Panics if p ≤ 0.0 or p ≥ 1.0.
    #[must_use]
    pub fn new(p: f64) -> Self {
        assert!(p > 0.0 && p < 1.0, "P² quantile p must be in (0, 1)");
        Self {
            p,
            q: [0.0; 5],
            n: [1, 2, 3, 4, 5],
            n_desired: [1.0, 1.0 + 2.0 * p, 1.0 + 4.0 * p, 3.0 + 2.0 * p, 5.0],
            count: 0,
            init_buf: Vec::with_capacity(5),
        }
    }

    /// Ingest a single observation.
    pub fn update(&mut self, x: f64) {
        self.count += 1;

        // Collect first 5 observations into buffer
        if self.init_buf.len() < 5 {
            self.init_buf.push(x);
            if self.init_buf.len() == 5 {
                // Initialise markers from sorted buffer
                self.init_buf.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                self.q = [
                    self.init_buf[0],
                    self.init_buf[1],
                    self.init_buf[2],
                    self.init_buf[3],
                    self.init_buf[4],
                ];
                self.n = [1, 2, 3, 4, 5];
                self.n_desired = [
                    1.0,
                    1.0 + 2.0 * self.p,
                    1.0 + 4.0 * self.p,
                    3.0 + 2.0 * self.p,
                    5.0,
                ];
            }
            return;
        }

        // Find cell k where x falls
        let k = if x < self.q[0] {
            self.q[0] = x;
            0_usize
        } else if x < self.q[1] {
            0_usize
        } else if x < self.q[2] {
            1_usize
        } else if x < self.q[3] {
            2_usize
        } else if x < self.q[4] {
            3_usize
        } else {
            self.q[4] = x;
            3_usize
        };

        // Increment positions n[k+1..5]
        for i in (k + 1)..5 {
            self.n[i] += 1;
        }

        // Update desired positions (Jain & Chlamtac 1985, Table 1):
        //   n'[1]=1, n'[2]=1+p*(n-1)/2, n'[3]=1+p*(n-1), n'[4]=1+(1+p)*(n-1)/2, n'[5]=n
        let n_obs = self.count as f64;
        self.n_desired[0] = 1.0;
        self.n_desired[1] = 1.0 + self.p * (n_obs - 1.0) / 2.0;
        self.n_desired[2] = 1.0 + self.p * (n_obs - 1.0);       // BUG-FIX: was /2.0 (wrong)
        self.n_desired[3] = 1.0 + (1.0 + self.p) * (n_obs - 1.0) / 2.0;
        self.n_desired[4] = n_obs;

        // Adjust marker heights using piecewise parabolic formula
        for i in 1..=3 {
            let d = self.n_desired[i] - self.n[i] as f64;
            let left_gap = (self.n[i] - self.n[i - 1]) as f64;
            let right_gap = (self.n[i + 1] - self.n[i]) as f64;

            if (d >= 1.0 && right_gap > 1.0) || (d <= -1.0 && left_gap > 1.0) {
                let sign = if d > 0.0 { 1_i64 } else { -1_i64 };
                // Parabolic formula (P² equation 5)
                let q_new = self.q[i]
                    + sign as f64
                    / (self.n[i + 1] - self.n[i - 1]) as f64
                    * ((self.n[i] - self.n[i - 1] + sign) as f64
                    * (self.q[i + 1] - self.q[i]) / right_gap
                    + (self.n[i + 1] - self.n[i] - sign) as f64
                    * (self.q[i] - self.q[i - 1]) / left_gap);

                // Use linear interpolation if parabolic is non-monotone
                if self.q[i - 1] < q_new && q_new < self.q[i + 1] {
                    self.q[i] = q_new;
                } else {
                    let j = if sign > 0 { i + 1 } else { i - 1 };
                    let gap = (self.n[j] - self.n[i]) as f64;
                    self.q[i] += sign as f64 * (self.q[j] - self.q[i]) / gap;
                }
                self.n[i] += sign;
            }
        }
    }

    /// Return the current quantile estimate.
    ///
    /// Returns `f64::NAN` if fewer than 5 observations have been ingested.
    #[must_use]
    pub fn quantile(&self) -> f64 {
        if self.count < 5 {
            // Fall back to sorted buffer for small N
            if self.init_buf.is_empty() {
                return f64::NAN;
            }
            let mut sorted = self.init_buf.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let idx = (self.p * sorted.len() as f64).round() as usize;
            return sorted[idx.min(sorted.len() - 1)];
        }
        self.q[2] // middle marker is the quantile estimate
    }

    /// Number of observations ingested.
    #[must_use]
    pub fn count(&self) -> u64 {
        self.count
    }
}

// ============================================================================
// FIVE-QUANTILE ESTIMATOR (Q0.05, Q0.25, Q0.50, Q0.75, Q0.95)
// ============================================================================

/// Bundle of five P² estimators covering the standard quantile set.
#[derive(Debug, Clone)]
pub struct QuantileBundle {
    pub p05: P2Quantile,
    pub p25: P2Quantile,
    pub p50: P2Quantile,
    pub p75: P2Quantile,
    pub p95: P2Quantile,
}

impl QuantileBundle {
    #[must_use]
    pub fn new() -> Self {
        Self {
            p05: P2Quantile::new(0.05),
            p25: P2Quantile::new(0.25),
            p50: P2Quantile::new(0.50),
            p75: P2Quantile::new(0.75),
            p95: P2Quantile::new(0.95),
        }
    }

    pub fn update(&mut self, x: f64) {
        self.p05.update(x);
        self.p25.update(x);
        self.p50.update(x);
        self.p75.update(x);
        self.p95.update(x);
    }
}

impl Default for QuantileBundle {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// DESCRIPTIVE STATISTICS RECORD
// ============================================================================

/// Complete descriptive statistics for one property × dataset slice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyStats {
    pub property_name: String,
    pub si_unit: String,
    pub count: u64,
    pub mean: f64,
    pub std: f64,
    /// Minimum observed value (NaN if no observations).
    pub min: f64,
    /// Maximum observed value (NaN if no observations).
    pub max: f64,
    pub p05: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p95: f64,
}

// ============================================================================
// PIPELINE STAGE COUNTERS
// ============================================================================

/// Per-stage record counts for the pipeline summary report.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineCounters {
    pub total_raw_records: u64,
    pub rejected_non_thermoelectric: u64,
    pub rejected_nan_inf: u64,
    pub rejected_gate1_algebraic: u64,
    pub rejected_gate1b_empirical: u64,
    pub flagged_gate2_wiedemann_franz: u64,
    pub flagged_gate3_zt_crosscheck: u64,
    pub flagged_unit_unknown: u64,
    pub flagged_duplicate_suspected: u64,
    pub tier_a_count: u64,
    pub tier_b_count: u64,
    pub tier_c_count: u64,
    pub clean_output_count: u64,
}

impl PipelineCounters {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Total rejection count (hard rejects only).
    #[must_use]
    pub fn total_rejected(&self) -> u64 {
        self.rejected_non_thermoelectric
            + self.rejected_nan_inf
            + self.rejected_gate1_algebraic
            + self.rejected_gate1b_empirical
    }

    /// Fraction of raw records that were hard-rejected.
    #[must_use]
    pub fn rejection_rate(&self) -> f64 {
        if self.total_raw_records == 0 {
            0.0
        } else {
            self.total_rejected() as f64 / self.total_raw_records as f64
        }
    }
}

// ============================================================================
// KS TEST
// ============================================================================

/// Two-sample Kolmogorov–Smirnov test result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KSTestResult {
    pub property_name: String,
    /// KS statistic D = max|F₁(x) − F₂(x)|.
    pub ks_statistic: f64,
    /// Asymptotic p-value (Kolmogorov distribution approximation).
    pub p_value: f64,
    pub n1: usize,
    pub n2: usize,
    /// True when p_value < 0.05 (reject null hypothesis of equal distributions at α=0.05).
    pub significant: bool,
}

/// Compute the two-sample KS test.
///
/// Both slices are sorted in place before computation.
/// Uses the asymptotic p-value approximation valid for n1, n2 > 40.
///
/// Reference: Press et al., "Numerical Recipes in C" (1992), §14.3.
pub fn ks_two_sample(sample1: &mut [f64], sample2: &mut [f64], property_name: &str) -> KSTestResult {
    sample1.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    sample2.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n1 = sample1.len();
    let n2 = sample2.len();

    if n1 == 0 || n2 == 0 {
        return KSTestResult {
            property_name: property_name.to_owned(),
            ks_statistic: f64::NAN,
            p_value: f64::NAN,
            n1,
            n2,
            significant: false,
        };
    }

    // Walk both sorted arrays simultaneously to find maximum CDF deviation
    let mut i = 0_usize;
    let mut j = 0_usize;
    let mut d = 0.0_f64;
    let fn1 = n1 as f64;
    let fn2 = n2 as f64;

    while i < n1 && j < n2 {
        let x1 = sample1[i];
        let x2 = sample2[j];
        let next_x = x1.min(x2);

        // Advance both CDFs to next_x
        while i < n1 && sample1[i] <= next_x {
            i += 1;
        }
        while j < n2 && sample2[j] <= next_x {
            j += 1;
        }
        let cdf_diff = (i as f64 / fn1 - j as f64 / fn2).abs();
        if cdf_diff > d {
            d = cdf_diff;
        }
    }

    // Asymptotic p-value: P(D > d) ≈ 2 Σ_{k=1}^∞ (−1)^{k-1} exp(−2k²λ²)
    // where λ = d × √(n1*n2/(n1+n2))
    let lambda = d * (fn1 * fn2 / (fn1 + fn2)).sqrt();
    let p_value = kolmogorov_p_value(lambda);

    KSTestResult {
        property_name: property_name.to_owned(),
        ks_statistic: d,
        p_value,
        n1,
        n2,
        significant: p_value < 0.05,
    }
}

/// Kolmogorov distribution asymptotic CDF: P(D ≤ d) ≈ 1 − 2Σ exp(−2k²λ²).
/// Returns P(D > d) = 1 − P(D ≤ d).
fn kolmogorov_p_value(lambda: f64) -> f64 {
    if lambda < 1.0e-12 {
        return 1.0;
    }
    let mut sum = 0.0_f64;
    for k in 1_i32..=100 {
        let term = f64::from(if k % 2 == 0 { -1_i32 } else { 1_i32 })
            * (-2.0 * (k as f64).powi(2) * lambda.powi(2)).exp();
        sum += term;
        if term.abs() < 1.0e-15 * sum.abs() {
            break;
        }
    }
    (2.0 * sum).clamp(0.0, 1.0)
}

// ============================================================================
// BAD RECORD CATALOGUE
// ============================================================================

/// A single rejected measurement for the `bad_records_report.jsonl` audit log.
///
/// Every record that fails any gate (NaN/Inf, property allowlist, Gate 1 algebraic,
/// Gate 1b empirical) MUST be catalogued here for Q1 data descriptor transparency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadRecord {
    /// Absolute path to the source JSON file (provenance anchor).
    pub source_path: String,
    /// Stage at which the record was rejected.
    /// One of: "nan_inf", "property_allowlist", "gate1_algebraic", "gate1b_empirical".
    pub rejection_stage: String,
    /// Bitmask of anomaly flags set at rejection time (from flags.rs).
    pub anomaly_flags: u32,
    /// Human-readable rejection reason.
    pub reason: String,
    /// The raw x-value (temperature) of the rejected observation.
    pub x: f64,
    /// The raw y-value of the rejected observation.
    pub y: f64,
    /// The propertyid_y that triggered the rejection.
    pub propertyid_y: u32,
}

// ============================================================================
// STATISTICS ENGINE
// ============================================================================

/// Central statistics engine — accumulates streaming statistics and writes reports.
pub struct StatisticsEngine {
    /// Pipeline-level counters.
    pub counters: PipelineCounters,

    // Per-property accumulators (raw / clean)
    seebeck_raw: WelfordAccumulator,
    seebeck_clean: WelfordAccumulator,
    seebeck_raw_q: QuantileBundle,
    seebeck_clean_q: QuantileBundle,

    sigma_raw: WelfordAccumulator,
    sigma_clean: WelfordAccumulator,
    sigma_raw_q: QuantileBundle,
    sigma_clean_q: QuantileBundle,

    kappa_raw: WelfordAccumulator,
    kappa_clean: WelfordAccumulator,
    kappa_raw_q: QuantileBundle,
    kappa_clean_q: QuantileBundle,

    zt_raw: WelfordAccumulator,
    zt_clean: WelfordAccumulator,
    zt_raw_q: QuantileBundle,
    zt_clean_q: QuantileBundle,
}

impl StatisticsEngine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            counters: PipelineCounters::new(),
            seebeck_raw: WelfordAccumulator::new(),
            seebeck_clean: WelfordAccumulator::new(),
            seebeck_raw_q: QuantileBundle::new(),
            seebeck_clean_q: QuantileBundle::new(),
            sigma_raw: WelfordAccumulator::new(),
            sigma_clean: WelfordAccumulator::new(),
            sigma_raw_q: QuantileBundle::new(),
            sigma_clean_q: QuantileBundle::new(),
            kappa_raw: WelfordAccumulator::new(),
            kappa_clean: WelfordAccumulator::new(),
            kappa_raw_q: QuantileBundle::new(),
            kappa_clean_q: QuantileBundle::new(),
            zt_raw: WelfordAccumulator::new(),
            zt_clean: WelfordAccumulator::new(),
            zt_raw_q: QuantileBundle::new(),
            zt_clean_q: QuantileBundle::new(),
        }
    }

    /// Record a raw Seebeck observation (V/K).
    pub fn observe_seebeck_raw(&mut self, s: f64) {
        self.seebeck_raw.update(s);
        self.seebeck_raw_q.update(s);
    }

    /// Record a clean (post-filter) Seebeck observation (V/K).
    pub fn observe_seebeck_clean(&mut self, s: f64) {
        self.seebeck_clean.update(s);
        self.seebeck_clean_q.update(s);
    }

    /// Record a raw σ observation (S/m).
    pub fn observe_sigma_raw(&mut self, sigma: f64) {
        self.sigma_raw.update(sigma);
        self.sigma_raw_q.update(sigma);
    }

    /// Record a clean σ observation (S/m).
    pub fn observe_sigma_clean(&mut self, sigma: f64) {
        self.sigma_clean.update(sigma);
        self.sigma_clean_q.update(sigma);
    }

    /// Record a raw κ observation (W/m·K).
    pub fn observe_kappa_raw(&mut self, kappa: f64) {
        self.kappa_raw.update(kappa);
        self.kappa_raw_q.update(kappa);
    }

    /// Record a clean κ observation (W/m·K).
    pub fn observe_kappa_clean(&mut self, kappa: f64) {
        self.kappa_clean.update(kappa);
        self.kappa_clean_q.update(kappa);
    }

    /// Record a raw ZT observation.
    pub fn observe_zt_raw(&mut self, zt: f64) {
        self.zt_raw.update(zt);
        self.zt_raw_q.update(zt);
    }

    /// Record a clean ZT observation.
    pub fn observe_zt_clean(&mut self, zt: f64) {
        self.zt_clean.update(zt);
        self.zt_clean_q.update(zt);
    }

    // --- Guarded Batch Accumulation ---

    /// Ingest a batch of (property_id, value) pairs with integrated MemoryGuard pressure checks.
    ///
    /// Called by the ETL pipeline once per measurement batch. Ticks the MemoryGuard on every
    /// observation; returns `Err(MemoryPressure::Hard)` immediately if the hard ceiling is
    /// breached so the caller can checkpoint and stop.
    ///
    /// Property ID mapping (Starrydata canonical):
    /// - 2 = Seebeck (V/K), 3 = Electrical conductivity (S/m),
    /// - 4 = Thermal conductivity (W/m·K), 16 = ZT
    pub fn ingest_batch_guarded(
        &mut self,
        measurements: &[(u32, f64)],  // (propertyid_y, value)
        clean: bool,
        guard: &mut MemoryGuard,
    ) -> Result<(), MemoryPressure> {
        for &(prop_id, value) in measurements {
            if guard.tick() == MemoryPressure::Hard {
                return Err(MemoryPressure::Hard);
            }
            match (prop_id, clean) {
                (2, false) => self.observe_seebeck_raw(value),
                (2, true)  => self.observe_seebeck_clean(value),
                (3, false) => self.observe_sigma_raw(value),
                (3, true)  => self.observe_sigma_clean(value),
                (4, false) => self.observe_kappa_raw(value),
                (4, true)  => self.observe_kappa_clean(value),
                (16, false) => self.observe_zt_raw(value),
                (16, true)  => self.observe_zt_clean(value),
                _ => {} // Non-primary properties silently skipped
            }
        }
        Ok(())
    }

    // --- Accessors ---

    fn make_stats(
        name: &str,
        unit: &str,
        acc: &WelfordAccumulator,
        q: &QuantileBundle,
    ) -> PropertyStats {
        PropertyStats {
            property_name: name.to_owned(),
            si_unit: unit.to_owned(),
            count: acc.count(),
            mean: acc.mean(),
            std: acc.std_sample(),
            min: acc.min(),
            max: acc.max(),
            p05: q.p05.quantile(),
            p25: q.p25.quantile(),
            p50: q.p50.quantile(),
            p75: q.p75.quantile(),
            p95: q.p95.quantile(),
        }
    }

    /// Snapshot current statistics for all properties (raw dataset).
    #[must_use]
    pub fn raw_stats(&self) -> Vec<PropertyStats> {
        vec![
            Self::make_stats("Seebeck", "V/K", &self.seebeck_raw, &self.seebeck_raw_q),
            Self::make_stats("ElectricalConductivity", "S/m", &self.sigma_raw, &self.sigma_raw_q),
            Self::make_stats("ThermalConductivity", "W/(m*K)", &self.kappa_raw, &self.kappa_raw_q),
            Self::make_stats("ZT", "1", &self.zt_raw, &self.zt_raw_q),
        ]
    }

    /// Snapshot current statistics for all properties (clean dataset).
    #[must_use]
    pub fn clean_stats(&self) -> Vec<PropertyStats> {
        vec![
            Self::make_stats("Seebeck", "V/K", &self.seebeck_clean, &self.seebeck_clean_q),
            Self::make_stats("ElectricalConductivity", "S/m", &self.sigma_clean, &self.sigma_clean_q),
            Self::make_stats("ThermalConductivity", "W/(m*K)", &self.kappa_clean, &self.kappa_clean_q),
            Self::make_stats("ZT", "1", &self.zt_clean, &self.zt_clean_q),
        ]
    }

    // --- Output writers ---

    /// Write `pipeline_summary.json` to `output_dir`.
    ///
    /// Output includes `pipeline_version` (git SHA from `GIT_COMMIT_SHA` env var or
    /// `"unknown"`) and `run_timestamp_utc` (ISO 8601, derived from `SystemTime::now()`).
    ///
    /// # Errors
    /// Returns `io::Error` if the file cannot be created or written.
    pub fn write_pipeline_json(&self, output_dir: &Path) -> io::Result<()> {
        #[derive(Serialize)]
        struct Summary<'a> {
            pipeline_version: String,
            run_timestamp_utc: String,
            counters: &'a PipelineCounters,
            total_rejected: u64,
            rejection_rate: f64,
        }
        let pipeline_version = std::env::var("GIT_COMMIT_SHA").unwrap_or_else(|_| "unknown".to_string());
        let run_timestamp_utc = {
            use std::time::{SystemTime, UNIX_EPOCH};
            let secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            crate::memory_guard::format_timestamp_utc(secs)
        };
        let summary = Summary {
            pipeline_version,
            run_timestamp_utc,
            counters: &self.counters,
            total_rejected: self.counters.total_rejected(),
            rejection_rate: self.counters.rejection_rate(),
        };
        let path = output_dir.join("pipeline_summary.json");
        let json = serde_json::to_string_pretty(&summary)
            .map_err(io::Error::other)?;
        fs::write(path, json)
    }

    /// Write `property_statistics.json` to `output_dir`.
    ///
    /// Output format: `{ "<canonical_name>": { "si_unit": "…", "raw": {…}, "clean": {…} } }`.
    /// Property names match the canonical names in `allowed_properties.toml`.
    ///
    /// Required fields per sub-object: `count`, `mean`, `std`, `min`, `max`,
    /// `p5`, `p25`, `median`, `p75`, `p95`.
    ///
    /// # Errors
    /// Returns `io::Error` if the file cannot be created or written.
    pub fn write_property_json(&self, output_dir: &Path) -> io::Result<()> {
        use std::collections::BTreeMap;

        /// Per-property statistics sub-object (raw or clean slice).
        #[derive(Serialize)]
        struct SubStats {
            count: u64,
            mean: f64,
            std: f64,
            min: f64,
            max: f64,
            p5: f64,
            p25: f64,
            median: f64,
            p75: f64,
            p95: f64,
        }

        #[derive(Serialize)]
        struct PropertyEntry {
            si_unit: String,
            raw: SubStats,
            clean: SubStats,
        }

        fn to_sub(s: &PropertyStats) -> SubStats {
            SubStats {
                count: s.count,
                mean: s.mean,
                std: s.std,
                min: s.min,
                max: s.max,
                p5: s.p05,
                p25: s.p25,
                median: s.p50,
                p75: s.p75,
                p95: s.p95,
            }
        }

        // Canonical names per allowed_properties.toml.
        // Order must match the index order produced by raw_stats() / clean_stats().
        const CANONICAL_NAMES: [(&str, &str); 4] = [
            ("Seebeck coefficient",               "V/K"),
            ("Electrical conductivity",            "S/m"),
            ("Total thermal conductivity",         "W/(m*K)"),
            ("Dimensionless figure of merit ZT",   "1"),
        ];

        let raw   = self.raw_stats();
        let clean = self.clean_stats();

        let mut map: BTreeMap<&str, PropertyEntry> = BTreeMap::new();
        for (i, &(canon_name, unit)) in CANONICAL_NAMES.iter().enumerate() {
            if i < raw.len() && i < clean.len() {
                map.insert(
                    canon_name,
                    PropertyEntry {
                        si_unit: unit.to_owned(),
                        raw: to_sub(&raw[i]),
                        clean: to_sub(&clean[i]),
                    },
                );
            }
        }

        let path = output_dir.join("property_statistics.json");
        let json = serde_json::to_string_pretty(&map).map_err(io::Error::other)?;
        fs::write(path, json)
    }

    /// Write `bad_records_report.jsonl` to `output_dir`.
    ///
    /// Each line is a JSON object describing one rejected record, providing the
    /// complete audit trail required by Q1 data descriptor standards.
    ///
    /// Fields per record: `source_path`, `rejection_stage`, `anomaly_flags`, `reason`.
    ///
    /// # Errors
    /// Returns `io::Error` if the file cannot be created or written.
    pub fn write_bad_records_jsonl(
        &self,
        output_dir: &Path,
        bad_records: &[BadRecord],
    ) -> io::Result<()> {
        use std::io::Write;
        let path = output_dir.join("bad_records_report.jsonl");
        let mut file = fs::File::create(path)?;
        for rec in bad_records {
            let line = serde_json::to_string(rec).map_err(io::Error::other)?;
            writeln!(file, "{line}")?;
        }
        Ok(())
    }

    /// Write `filtered_vs_unfiltered_report.md` to `output_dir`.
    ///
    /// Includes a Markdown table comparing raw vs. clean per-property statistics
    /// and KS test results. The `ks_results` slice must be pre-computed by the
    /// caller using [`ks_two_sample`].
    ///
    /// # Errors
    /// Returns `io::Error` if the file cannot be created or written.
    pub fn write_comparison_report(
        &self,
        output_dir: &Path,
        ks_results: &[KSTestResult],
    ) -> io::Result<()> {
        let raw = self.raw_stats();
        let clean = self.clean_stats();
        let path = output_dir.join("filtered_vs_unfiltered_report.md");
        let content = render_comparison_report(&self.counters, &raw, &clean, ks_results);
        fs::write(path, content)
    }
}

impl Default for StatisticsEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// REPORT RENDERING
// ============================================================================

fn render_comparison_report(
    counters: &PipelineCounters,
    raw: &[PropertyStats],
    clean: &[PropertyStats],
    ks_results: &[KSTestResult],
) -> String {
    let mut buf = String::new();

    writeln_str(&mut buf, "# Filtered vs. Unfiltered Distribution Comparison\n");
    writeln_str(
        &mut buf,
        "**Generated by:** THERMOGNOSIS-X Statistics Engine (TASK 4 / SPEC-STATS-01)  \n\
         **Dataset standard:** Q1 Nature Scientific Data\n",
    );

    // Pipeline counters table
    writeln_str(&mut buf, "## 1. Pipeline Stage Counters\n");
    writeln_str(&mut buf, "| Stage | Count |");
    writeln_str(&mut buf, "|-------|------:|");
    writeln_str(&mut buf, &format!("| Total raw records | {} |", counters.total_raw_records));
    writeln_str(&mut buf, &format!("| Rejected — non-thermoelectric | {} |", counters.rejected_non_thermoelectric));
    writeln_str(&mut buf, &format!("| Rejected — NaN/Inf | {} |", counters.rejected_nan_inf));
    writeln_str(&mut buf, &format!("| Rejected — Gate 1 algebraic | {} |", counters.rejected_gate1_algebraic));
    writeln_str(&mut buf, &format!("| Rejected — Gate 1b empirical | {} |", counters.rejected_gate1b_empirical));
    writeln_str(&mut buf, &format!("| Flagged — Wiedemann–Franz | {} |", counters.flagged_gate2_wiedemann_franz));
    writeln_str(&mut buf, &format!("| Flagged — ZT cross-check | {} |", counters.flagged_gate3_zt_crosscheck));
    writeln_str(&mut buf, &format!("| Flagged — unit unknown | {} |", counters.flagged_unit_unknown));
    writeln_str(&mut buf, &format!("| Flagged — duplicate suspected | {} |", counters.flagged_duplicate_suspected));
    writeln_str(&mut buf, &format!("| Tier A (clean) | {} |", counters.tier_a_count));
    writeln_str(&mut buf, &format!("| Tier B (WF anomaly) | {} |", counters.tier_b_count));
    writeln_str(&mut buf, &format!("| Tier C (cross-check fail) | {} |", counters.tier_c_count));
    writeln_str(&mut buf, &format!("| **Clean output (Tier A + B)** | **{}** |", counters.clean_output_count));
    writeln_str(&mut buf, &format!("| Rejection rate | {:.2}% |", counters.rejection_rate() * 100.0));
    writeln_str(&mut buf, "");

    // Per-property comparison table
    writeln_str(&mut buf, "## 2. Per-Property Descriptive Statistics\n");
    writeln_str(
        &mut buf,
        "| Property | Dataset | N | Mean | Std | P5 | P25 | Median | P75 | P95 |",
    );
    writeln_str(
        &mut buf,
        "|----------|---------|--:|-----:|----:|---:|----:|------:|----:|----:|",
    );

    for (r, c) in raw.iter().zip(clean.iter()) {
        let unit = &r.si_unit;
        writeln_str(
            &mut buf,
            &format!(
                "| {} ({}) | Raw | {} | {:.3e} | {:.3e} | {:.3e} | {:.3e} | {:.3e} | {:.3e} | {:.3e} |",
                r.property_name, unit, r.count, r.mean, r.std, r.p05, r.p25, r.p50, r.p75, r.p95
            ),
        );
        writeln_str(
            &mut buf,
            &format!(
                "| {} ({}) | Clean | {} | {:.3e} | {:.3e} | {:.3e} | {:.3e} | {:.3e} | {:.3e} | {:.3e} |",
                c.property_name, unit, c.count, c.mean, c.std, c.p05, c.p25, c.p50, c.p75, c.p95
            ),
        );
    }
    writeln_str(&mut buf, "");

    // KS test results
    writeln_str(&mut buf, "## 3. Kolmogorov–Smirnov Test (Raw vs. Clean)\n");
    writeln_str(&mut buf, "| Property | KS statistic D | p-value | n₁ (raw) | n₂ (clean) | Significant? |");
    writeln_str(&mut buf, "|----------|---------------:|--------:|---------:|-----------:|:------------|");
    for ks in ks_results {
        let sig = if ks.p_value < 0.05 { "Yes (p < 0.05)" } else { "No" };
        writeln_str(
            &mut buf,
            &format!(
                "| {} | {:.4} | {:.4} | {} | {} | {} |",
                ks.property_name, ks.ks_statistic, ks.p_value, ks.n1, ks.n2, sig
            ),
        );
    }
    writeln_str(&mut buf, "");
    writeln_str(
        &mut buf,
        "> KS p-values use the Kolmogorov asymptotic approximation \
         (Numerical Recipes §14.3). Exact p-values should be used for n < 40.\n",
    );

    // Known-Bad Data Catalogue (required by Q1 data descriptor transparency)
    writeln_str(&mut buf, "## 4. Known-Bad Data Catalogue\n");
    writeln_str(
        &mut buf,
        "The following categories of contamination were identified in the raw StarryData \
         corpus and are removed from the clean output. Counts are runtime-populated from \
         `PipelineCounters` (populated during ETL — zero values indicate the pattern was \
         not observed in this pipeline run).\n",
    );
    writeln_str(
        &mut buf,
        "| Category | Records Caught | Detection Method | Flag / Stage |",
    );
    writeln_str(
        &mut buf,
        "|----------|---------------:|-----------------|-------------|",
    );
    writeln_str(
        &mut buf,
        &format!(
            "| UV-Vis optical contamination (propertyid_y ∈ {{17,18}}, \
             samples 00040076–00040078) | {} | \
             Property allowlist (BUG-01) | `rejected_non_thermoelectric` |",
            counters.rejected_non_thermoelectric
        ),
    );
    writeln_str(
        &mut buf,
        &format!(
            "| Unit conversion failure — unknown unit string | {} | \
             UnitRegistry lookup miss (GAP-02) | `FLAG_UNIT_UNKNOWN` |",
            counters.flagged_unit_unknown
        ),
    );
    writeln_str(
        &mut buf,
        &format!(
            "| Empirical bounds violation (e.g. |S| > 1 mV/K, σ > 10⁷ S/m, \
             κ > 100 W/m·K) (BUG-05) | {} | \
             Gate 1b: SEEBECK/SIGMA/KAPPA_BOUND_EXCEED flags | `rejected_gate1b_empirical` |",
            counters.rejected_gate1b_empirical
        ),
    );
    writeln_str(
        &mut buf,
        "| DFT/computational records misclassified as experimental (BUG-04) | \
         tracked at Python ingestion stage | \
         classify_experiment_type() keyword priority scheme | Python `ingestion.py` |",
    );
    writeln_str(
        &mut buf,
        &format!(
            "| Wiedemann–Franz violation (κ_lattice < 0 or L ∉ [L_min, L_max]) | {} | \
             Gate 2: Sommerfeld residual κ_L = κ − L₀σT | `FLAG_WF_VIOLATION` |",
            counters.flagged_gate2_wiedemann_franz
        ),
    );
    writeln_str(
        &mut buf,
        &format!(
            "| ZT cross-check failure (|zT_computed − zT_reported| / zT_reported > 10%) | {} | \
             Gate 3: relative deviation threshold ε > 0.10 | `FLAG_ZT_CROSSCHECK_FAIL` |",
            counters.flagged_gate3_zt_crosscheck
        ),
    );
    writeln_str(&mut buf, "");
    writeln_str(
        &mut buf,
        "> **Note:** Physical basis for all exclusion thresholds is documented in \
         `allowed_properties.toml` and `constants.rs`. All raw records — including \
         rejected ones — are preserved in `bad_records_report.jsonl` for full \
         audit trail transparency (Nature Scientific Data data descriptor requirement).\n",
    );

    buf
}

/// Helper: push a line with newline to the buffer (avoids `writeln!` macro which needs `fmt::Write`).
fn writeln_str(buf: &mut String, s: &str) {
    buf.push_str(s);
    buf.push('\n');
}

// ============================================================================
// CERTIFICATE GENERATION (GAP-09)
// ============================================================================

/// Clean Dataset Certificate metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetCertificate {
    pub dataset_name: String,
    pub version: String,
    pub generated_at_utc: String,
    pub total_raw_records: u64,
    pub clean_records: u64,
    pub tier_a_count: u64,
    pub tier_b_count: u64,
    pub rejection_rate_pct: f64,
    pub sha256_parquet: String,
    pub validation_protocol: String,
    pub contact: String,
}

/// Generate the clean dataset certificate and write it to `output_dir/clean_dataset_certificate.md`.
///
/// # Errors
/// Returns `io::Error` if the file cannot be created or written.
pub fn generate_certificate(
    cert: &DatasetCertificate,
    output_dir: &Path,
) -> io::Result<()> {
    let mut buf = String::new();
    writeln_str(&mut buf, "# Clean Dataset Certificate — THERMOGNOSIS-X\n");
    writeln_str(&mut buf, "**Supplementary Material S2: Dataset Provenance Certificate**  ");
    writeln_str(&mut buf, "Intended audience: Nature Scientific Data data editors.\n");
    writeln_str(&mut buf, "---\n");
    writeln_str(&mut buf, "## Dataset Identity\n");
    writeln_str(&mut buf, &format!("| Field | Value |"));
    writeln_str(&mut buf, "|-------|-------|");
    writeln_str(&mut buf, &format!("| Dataset name | {} |", cert.dataset_name));
    writeln_str(&mut buf, &format!("| Version | {} |", cert.version));
    writeln_str(&mut buf, &format!("| Generated (UTC) | {} |", cert.generated_at_utc));
    writeln_str(&mut buf, &format!("| Parquet SHA-256 | `{}` |", cert.sha256_parquet));
    writeln_str(&mut buf, &format!("| Contact | {} |", cert.contact));
    writeln_str(&mut buf, "");
    writeln_str(&mut buf, "## Provenance Statistics\n");
    writeln_str(&mut buf, "| Metric | Value |");
    writeln_str(&mut buf, "|--------|------:|");
    writeln_str(&mut buf, &format!("| Total raw records ingested | {} |", cert.total_raw_records));
    writeln_str(&mut buf, &format!("| Clean records (Tier A + B) | {} |", cert.clean_records));
    writeln_str(&mut buf, &format!("| Tier A records | {} |", cert.tier_a_count));
    writeln_str(&mut buf, &format!("| Tier B records (WF anomaly) | {} |", cert.tier_b_count));
    writeln_str(&mut buf, &format!("| Rejection rate | {:.2}% |", cert.rejection_rate_pct));
    writeln_str(&mut buf, "");
    writeln_str(&mut buf, "## Validation Protocol\n");
    writeln_str(&mut buf, &format!(
        "All records validated per `{}`. \
         Seven-stage validation protocol documented in VALIDATION_METHODOLOGY.md \
         (Supplementary Material S1).",
        cert.validation_protocol
    ));
    writeln_str(&mut buf, "");
    writeln_str(&mut buf, "## Integrity\n");
    writeln_str(&mut buf, &format!(
        "The Parquet file SHA-256 checksum above was computed at generation time. \
         Verify with: `sha256sum {}.parquet`",
        cert.dataset_name.to_lowercase().replace(' ', "_")
    ));
    writeln_str(&mut buf, "");
    writeln_str(&mut buf, "---");
    writeln_str(&mut buf, "*This certificate was generated automatically by the THERMOGNOSIS-X pipeline.*");

    let path = output_dir.join("clean_dataset_certificate.md");
    fs::write(path, buf)
}

// Bring fmt::Write into scope for the writeln! macro if ever needed.
// Currently not used — use writeln_str instead to avoid import.
#[allow(unused_imports)]
use fmt::Write as _;

// ============================================================================
// RESERVOIR SAMPLER — Vitter Algorithm R (Task 7)
// ============================================================================

/// Bounded-memory reservoir sampler using Vitter's Algorithm R (1985).
///
/// Maintains a uniform random sample of at most `capacity` elements from a
/// streaming sequence, using O(capacity) memory regardless of stream length.
///
/// Reference: Vitter, "Random sampling with a reservoir",
/// ACM Trans. Math. Software 11(1), 37–57 (1985). DOI: 10.1145/3828.3838
///
/// # Memory
/// At `capacity = 50_000`, each instance uses ≤ 400 KB (50K × 8 bytes).
/// With 8 instances (4 properties × raw/clean), total ≤ 3.2 MB.
#[derive(Debug, Clone)]
pub struct ReservoirSample {
    reservoir: Vec<f64>,
    /// Maximum reservoir size.
    capacity: usize,
    /// Total number of elements seen so far.
    count: u64,
    /// xorshift64 RNG state — seeded at construction, advances deterministically.
    rng_state: u64,
}

impl ReservoirSample {
    /// Create a new sampler with the given capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        // Seed from a fixed constant XOR'd with capacity to get a reproducible but
        // capacity-distinct seed per instance.
        let seed = 0x9e37_79b9_7f4a_7c15_u64 ^ (capacity as u64).wrapping_mul(6_364_136_223_846_793_005);
        Self {
            reservoir: Vec::with_capacity(capacity.min(4096)),
            capacity,
            count: 0,
            rng_state: seed,
        }
    }

    /// xorshift64 pseudo-random number generator (period 2⁶⁴ − 1).
    #[inline]
    fn next_rand(&mut self) -> u64 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        x
    }

    /// Ingest one observation into the reservoir (Vitter Algorithm R).
    pub fn update(&mut self, x: f64) {
        self.count += 1;
        if self.reservoir.len() < self.capacity {
            // Fill reservoir until capacity
            self.reservoir.push(x);
        } else {
            // Replace a random existing element with probability capacity/count
            let j = self.next_rand() % self.count;
            if (j as usize) < self.capacity {
                self.reservoir[j as usize] = x;
            }
        }
    }

    /// Reference to the current reservoir contents.
    #[must_use]
    pub fn sample(&self) -> &[f64] {
        &self.reservoir
    }

    /// Number of total observations ingested (not reservoir size).
    #[must_use]
    pub fn total_count(&self) -> u64 {
        self.count
    }

    /// Current number of elements held in the reservoir (≤ capacity).
    #[must_use]
    pub fn reservoir_len(&self) -> usize {
        self.reservoir.len()
    }
}

/// Default reservoir capacity for KS test sampling (400 KB per instance).
/// Exposed for callers constructing reservoir samplers with the standard production capacity.
pub const DEFAULT_RESERVOIR_CAPACITY: usize = 50_000;

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- WelfordAccumulator ---

    #[test]
    fn welford_mean_and_variance_known_sequence() {
        let mut acc = WelfordAccumulator::new();
        // Known values: {2, 4, 4, 4, 5, 5, 7, 9} → mean=5, population variance=4 (Σ(xi-μ)²/N = 32/8)
        // Note: sample variance = 32/7 ≈ 4.571 (divides by N−1, not N).
        for x in [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0_f64] {
            acc.update(x);
        }
        let tol = 1.0e-10;
        assert!((acc.mean() - 5.0).abs() < tol, "Mean must be 5.0");
        assert!((acc.variance_population() - 4.0).abs() < tol, "Population variance must be 4.0");
        assert!((acc.std_population() - 2.0).abs() < tol, "Population std must be 2.0");
    }

    #[test]
    fn welford_empty_returns_nan() {
        let acc = WelfordAccumulator::new();
        assert!(acc.mean().is_nan(), "Empty accumulator mean must be NaN");
        assert!(acc.variance_sample().is_nan(), "Empty accumulator variance must be NaN");
    }

    #[test]
    fn welford_single_observation_variance_is_nan() {
        let mut acc = WelfordAccumulator::new();
        acc.update(5.0);
        assert!((acc.mean() - 5.0).abs() < 1.0e-15, "Single obs mean must be 5.0");
        assert!(acc.variance_sample().is_nan(), "Sample variance undefined for N=1");
    }

    #[test]
    fn welford_numerically_stable_large_offset() {
        // Test with large offset values (stability test)
        let mut acc = WelfordAccumulator::new();
        let base = 1.0e8_f64;
        for x in [base + 1.0, base + 2.0, base + 3.0, base + 4.0, base + 5.0_f64] {
            acc.update(x);
        }
        assert!((acc.mean() - (base + 3.0)).abs() < 1.0, "Large-offset mean must be stable");
        assert!((acc.variance_sample() - 2.5).abs() < 1.0e-6, "Large-offset variance must be stable");
    }

    // --- P² Quantile ---

    #[test]
    fn p2_median_uniform_distribution() {
        let mut est = P2Quantile::new(0.50);
        // Uniform [0, 1000]: median should be near 500
        for i in 0..=1000_i64 {
            est.update(i as f64);
        }
        let med = est.quantile();
        assert!(
            (med - 500.0).abs() < 25.0,
            "P² median estimate must be within 25 of true median 500; got {med}"
        );
    }

    #[test]
    fn p2_quantile_monotone_ordering() {
        // Q25 ≤ Q50 ≤ Q75 must hold
        let mut q25 = P2Quantile::new(0.25);
        let mut q50 = P2Quantile::new(0.50);
        let mut q75 = P2Quantile::new(0.75);
        for i in 0..=500_i64 {
            let x = i as f64;
            q25.update(x);
            q50.update(x);
            q75.update(x);
        }
        assert!(q25.quantile() <= q50.quantile(), "Q25 must be <= Q50");
        assert!(q50.quantile() <= q75.quantile(), "Q50 must be <= Q75");
    }

    // --- KS test ---

    #[test]
    fn ks_identical_distributions_p_not_significant() {
        // Identical distributions → KS statistic ≈ 0, large p-value
        let mut s1: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let mut s2: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let r = ks_two_sample(&mut s1, &mut s2, "Test");
        assert!(r.ks_statistic < 0.01, "Identical distributions: KS stat must be near 0");
        assert!(r.p_value > 0.90, "Identical distributions: p must be large");
    }

    #[test]
    fn ks_different_distributions_significant() {
        // Clearly different distributions → KS must detect
        let mut s1: Vec<f64> = (0..200).map(|i| i as f64).collect();
        let mut s2: Vec<f64> = (1000..1200).map(|i| i as f64).collect();
        let r = ks_two_sample(&mut s1, &mut s2, "Test");
        assert!(r.ks_statistic > 0.9, "Clearly different distributions must have D near 1");
        assert!(r.p_value < 0.01, "Clearly different distributions must have small p-value");
    }

    // --- StatisticsEngine integration ---

    #[test]
    fn engine_accumulates_and_reports() {
        let mut eng = StatisticsEngine::new();
        for i in 1..=100_u64 {
            eng.counters.total_raw_records += 1;
            let s = i as f64 * 1.0e-5;
            eng.observe_seebeck_raw(s);
            if i <= 80 {
                eng.observe_seebeck_clean(s);
                eng.counters.clean_output_count += 1;
            } else {
                eng.counters.rejected_gate1_algebraic += 1;
            }
        }
        let raw = eng.raw_stats();
        let clean = eng.clean_stats();
        assert_eq!(raw[0].count, 100, "Raw count must be 100");
        assert_eq!(clean[0].count, 80, "Clean count must be 80");
        assert!(raw[0].mean > clean[0].mean - 1.0e-6, "Raw mean >= clean mean for increasing series");
    }

    // --- write_property_json nested structure ---

    #[test]
    fn property_json_is_nested_by_canonical_name() {
        let mut eng = StatisticsEngine::new();
        // Seed minimal data so stats are non-NaN
        for i in 1..=10_u64 {
            eng.observe_seebeck_raw(i as f64 * 1.0e-5);
            eng.observe_seebeck_clean(i as f64 * 1.0e-5);
            eng.observe_sigma_raw(i as f64 * 1.0e4);
            eng.observe_sigma_clean(i as f64 * 1.0e4);
            eng.observe_kappa_raw(i as f64 * 0.1);
            eng.observe_kappa_clean(i as f64 * 0.1);
            eng.observe_zt_raw(i as f64 * 0.1);
            eng.observe_zt_clean(i as f64 * 0.1);
        }
        let tmpdir = std::env::temp_dir();
        eng.write_property_json(&tmpdir).expect("write_property_json must succeed");
        let content = std::fs::read_to_string(tmpdir.join("property_statistics.json"))
            .expect("property_statistics.json must exist");
        // Verify top-level keys are canonical property names
        assert!(
            content.contains("\"Seebeck coefficient\""),
            "property_statistics.json must contain 'Seebeck coefficient' key"
        );
        assert!(
            content.contains("\"Electrical conductivity\""),
            "property_statistics.json must contain 'Electrical conductivity' key"
        );
        assert!(
            content.contains("\"Total thermal conductivity\""),
            "property_statistics.json must contain 'Total thermal conductivity' key"
        );
        assert!(
            content.contains("\"Dimensionless figure of merit ZT\""),
            "property_statistics.json must contain 'Dimensionless figure of merit ZT' key"
        );
        // Verify nested raw/clean sub-objects
        assert!(content.contains("\"raw\""), "Must contain 'raw' sub-object");
        assert!(content.contains("\"clean\""), "Must contain 'clean' sub-object");
        // Verify min/max fields are present
        assert!(content.contains("\"min\""), "Must contain 'min' field");
        assert!(content.contains("\"max\""), "Must contain 'max' field");
        assert!(content.contains("\"median\""), "Must contain 'median' field");
    }

    #[test]
    fn filtered_report_contains_known_bad_catalogue() {
        let eng = StatisticsEngine::new();
        let report = eng
            .write_comparison_report(&std::env::temp_dir(), &[])
            .map(|()| {
                std::fs::read_to_string(
                    std::env::temp_dir().join("filtered_vs_unfiltered_report.md"),
                )
                .unwrap()
            });
        let content = report.expect("write_comparison_report must succeed");
        assert!(
            content.contains("Known-Bad Data Catalogue"),
            "Report must contain Known-Bad Data Catalogue section"
        );
        assert!(
            content.contains("UV-Vis optical contamination"),
            "Catalogue must document UV-Vis contamination"
        );
        assert!(
            content.contains("Wiedemann"),
            "Catalogue must document Wiedemann-Franz violations"
        );
        assert!(
            content.contains("ZT cross-check failure"),
            "Catalogue must document ZT cross-check failures"
        );
    }

    // --- Certificate rendering ---

    #[test]
    fn certificate_contains_key_fields() {
        let tmpdir = std::env::temp_dir();
        let cert = DatasetCertificate {
            dataset_name: "ThermognosisClean".to_owned(),
            version: "1.0.0".to_owned(),
            generated_at_utc: "2026-03-05T00:00:00Z".to_owned(),
            total_raw_records: 1_000_000,
            clean_records: 750_000,
            tier_a_count: 700_000,
            tier_b_count: 50_000,
            rejection_rate_pct: 25.0,
            sha256_parquet: "abc123".to_owned(),
            validation_protocol: "VALIDATION_METHODOLOGY.md".to_owned(),
            contact: "research@thermognosis.org".to_owned(),
        };
        generate_certificate(&cert, &tmpdir).expect("Certificate write must succeed");
        let content = std::fs::read_to_string(tmpdir.join("clean_dataset_certificate.md"))
            .expect("Certificate file must exist");
        assert!(content.contains("1000000"), "Certificate must contain total raw records");
        assert!(content.contains("750000"), "Certificate must contain clean records");
        assert!(content.contains("abc123"), "Certificate must contain SHA-256");
    }
}
