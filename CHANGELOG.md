# CHANGELOG — THERMOGNOSIS-X

All bug fixes and significant changes are documented here.
Format: [BUGFIX|FEATURE|REFACTOR] ID — Short description.

---

## [BUGFIX] BUG-01 — ALLOWED_Y filter was dead code (data contamination)

**Impact**: The `parse_json_file` function in `rust_core/src/mirror_walker.rs`
admitted ALL measurement types regardless of `propertyid_y`. Non-thermoelectric
optical measurements (UV-Vis Absorbance, Reflectance) from samples 00040076,
00040077, and 00040078 entered the clean dataset. This silently inflated record
counts and corrupted downstream statistical summaries.

**Fix**:
- Added `ALLOWED_PROPERTY_IDS: &[u32]` constant in `mirror_walker.rs` listing
  the 11 valid thermoelectric transport property IDs (2, 3, 4, 5, 6, 10, 11,
  12, 14, 15, 16).
- `parse_json_file` now applies a two-stage filter: (1) NaN/Inf rejection;
  (2) property allowlist check via `is_allowed_property(propertyid_y)`.
- Rejected count is propagated through `StarrydataRecord.rejected_non_thermoelectric_count`
  and aggregated into `WalkSummary.rejected_non_thermoelectric`.
- The Python summary dict from `py_scan_domain` now includes
  `"rejected_non_thermoelectric"` for pipeline statistics.
- Created `allowed_properties.toml` as the human-readable policy document.

**Test**: Feed sample files 00040076, 00040077, 00040078 (UV-Vis optical data)
and assert `rejected_non_thermoelectric_count > 0` and
`measurements.len() == 0`.

---

## [BUGFIX] BUG-02 — NumPy boolean masking undefined behavior

**Impact**: `python/thermognosis/pipeline/validation.py` lines 251–254 used:
```python
valid_T = np.greater(T_arr, 0.0, where=~np.isnan(T_arr))
```
`np.greater(arr, val, where=mask)` leaves output elements uninitialized wherever
`mask=False`. NumPy does not guarantee any value for those elements. In practice,
this caused NaN-containing rows to be classified as valid (`is_valid=True`)
depending on garbage memory state, silently bypassing the physical constraint
filter.

**Fix**: Replaced all four `np.greater(..., where=...)` calls with `np.where`:
```python
valid_T = np.where(~np.isnan(T_arr), T_arr > 0.0, False)
```
`np.where` is fully defined for all elements — the False branch explicitly maps
to the sentinel `False` for every masked position.

**Note**: Since Rust `validation.rs` is the canonical implementation, this Python
fix is a correctness patch for the legacy path. Long-term, the Python module
should be replaced by calling the Rust engine via FFI (TASK 3).

**Test**: Pass an array containing `[NaN, -1.0, 300.0]` and assert
`is_valid == [False, False, True]`.

---

## [BUGFIX] BUG-04 — Experiment classification keyword collision

**Impact**: `classify_experiment_type()` in `ingestion.py` built its search text
from `" ".join(str(v) for v in sample.values()).lower()`, which includes the
JSON field name `"sample"` itself (present in every StarryData record). The
keyword list included `"sample"`, causing every single record to be classified
as `"experimental"` regardless of its actual nature. The filter was a permanent
no-op. DFT calculations were admitted into the experimental dataset.

**Fix**: Rewrote `classify_experiment_type()` to:
1. Search only semantically relevant fields: `method`, `calculationtype`,
   `technique`, `comment`, `instrument`.
2. Apply a priority-ordered keyword scheme: Computational first (unambiguous
   DFT/VASP/Wien2k terms), then Experimental (synthesis/measurement terms).
3. Return `'unknown'` — never default to `'experimental'` — when no keyword
   matches. The caller sets `FLAG_LOW_CONFIDENCE_EXP` for unknown records.
4. Completely removed `"sample"` and `"experiment"` from keyword lists (too
   broad to be discriminating).

**Test**: Verify that a record with `method="DFT+U with VASP"` returns
`"computational"`, a record with `method="Spark plasma sintering"` returns
`"experimental"`, and a record with no method field returns `"unknown"`.

---

## [BUGFIX] BUG-05 — S_MAX_ABS threshold inconsistency between Rust layers

**Impact**: `physics.rs::calc_zt_batch` used three independent hardcoded
literals inconsistent with the module-level constants:
- `s_val.abs() > 0.05` — accepted |S| up to 50 mV/K, which is 50× the
  correct physical limit of 1 mV/K (1000 µV/K).
- `sigma_val > 1e8` — accepted σ up to 10⁸ S/m, which is 10× above
  `SIGMA_MAX = 1e7 S/m`.
- `kappa_val > 5000.0` — accepted κ up to 5000 W/(m·K), which is 50× above
  `KAPPA_MAX = 100 W/(m·K)`.

Measurements from 2400-year-old manuscripts could not produce 50 mV/K Seebeck
coefficients; such values always indicate a unit conversion failure (e.g., µV/K
reported without conversion → appears as V/K × 1000).

**Fix**:
- Created `rust_core/src/constants.rs` as the single source of truth for all
  physical bounds, with compile-time assertions:
  ```rust
  pub const SEEBECK_MAX_ABS_V_PER_K: f64 = 1.0e-3;
  const _ASSERT_SEEBECK_MAX: () = assert!(SEEBECK_MAX_ABS_V_PER_K == 1.0e-3, ...);
  ```
- Updated `physics.rs` to import constants from `constants.rs` instead of
  defining them locally.
- Replaced all three wrong literals in `calc_zt_batch` with `S_MAX_ABS`,
  `SIGMA_MAX`, `KAPPA_MAX` (which now alias the canonical constants).
- Updated `audit.rs::triple_check_physics` to also check empirical bounds
  (Gate 1b) using the same canonical constants, setting `FLAG_SEEBECK_BOUND_EXCEED`,
  `FLAG_SIGMA_BOUND_EXCEED`, or `FLAG_KAPPA_BOUND_EXCEED` as appropriate.

**Tests**:
- `bug05_seebeck_50mv_per_k_is_rejected`: asserts 50 mV/K → NAN in batch.
- `bug05_sigma_1e8_is_rejected`: asserts σ=1e8 S/m → NAN in batch.
- `bug05_kappa_5000_is_rejected`: asserts κ=5000 W/(m·K) → NAN in batch.
- `gate1b_rejects_seebeck_50mv_per_k`: asserts Tier::Reject + FLAG in audit.

---

## [FEATURE] GAP-03 — Anomaly flags bitmask now SET, not just warned

**Impact**: Previous audit warnings were emitted to stderr but the
`anomaly_flags` field in Parquet always remained 0. Peer reviewers querying
flagged records would find no records.

**Fix**:
- Created `rust_core/src/flags.rs` with 12 distinct single-bit constants
  covering all validation failure modes (FLAG_WF_VIOLATION through
  FLAG_ALGEBRAIC_REJECT).
- Updated `audit.rs` to use the new flag constants from `flags.rs`.
- All prior audit flag constants (`FLAG_NEGATIVE_KAPPA_L`, `FLAG_LORENZ_OUT_BOUNDS`,
  `FLAG_ZT_MISMATCH`) re-exported as aliases for backwards compatibility.
- New flags exported to Python via `lib.rs`:
  `FLAG_WF_VIOLATION`, `FLAG_ZT_CROSSCHECK_FAIL`, `FLAG_UNIT_UNKNOWN`,
  `FLAG_SIGMA_RHO_INCON`, `FLAG_LOW_CONFIDENCE_EXP`, `FLAG_INTERP_INSUFFICIENT`,
  `FLAG_SEEBECK_BOUND_EXCEED`, `FLAG_SIGMA_BOUND_EXCEED`, `FLAG_KAPPA_BOUND_EXCEED`,
  `FLAG_FIGUREID_MISMATCH`, `FLAG_DUPLICATE_SUSPECTED`, `FLAG_ALGEBRAIC_REJECT`.
- Also exported `decode_flags(u32) -> Vec<&str>` for human-readable bitmask
  decoding in `bad_records_report.jsonl`.

---

## [FEATURE] GAP-02 (Partial) — Unit registry TOML created

**Impact**: Unit conversion was claimed in the data descriptor but never
implemented. This constitutes a scientific integrity violation if published.

**Fix**: Created `unit_registry.toml` covering all unit strings encountered
in the Starrydata corpus survey, including multiple representations of Seebeck
(V/K, mV/K, µV/K, uV/K), conductivity (S/m, S/cm, (Ω·cm)⁻¹), resistivity
(Ω·m, Ω·cm, mΩ·cm, µΩ·cm), thermal conductivity (W/(m·K), mW/(m·K),
W/(cm·K)), temperature (K, °C, degC), and power factor (W/(m·K²), µW/(cm·K²)).

The Rust `UnitConverter` (src/units.rs) must be extended to load this TOML
at startup. This is documented as a remaining implementation gap.

---

## [REFACTOR] New modules declared in lib.rs

- `pub mod constants;` — Physical bounds (single source of truth).
- `pub mod flags;` — Anomaly bitmask set (GAP-03).
- Physical constants (`SEEBECK_MAX_ABS_V_PER_K`, `SIGMA_MAX_S_PER_M`,
  `KAPPA_MAX_W_PER_MK`, `T_MIN_K`, `T_MAX_K`) now exported to Python.

---

## [FEATURE] GAP-01 — MemoryGuard: RSS monitoring with hard/soft limits

**Module**: `rust_core/src/memory_guard.rs` (NEW)

**Implementation**:
- `MemoryGuard` struct with configurable soft/hard limits and amortised check interval.
- `MemoryPressure` enum: `Ok` / `Soft` / `Hard`.
- `tick()` — amortised RSS check every N records (default: 1000).
- `check_pressure()` — immediate RSS check.
- Platform-specific RSS readers: `/proc/self/status` (Linux), `getrusage` (macOS).
- `write_checkpoint()` / `read_checkpoint()` for batch flush on soft, halt on hard.
- `default_production()` factory: soft=6.5 GB, hard=7.0 GB, interval=1000.
- `from_max_ram_mb(u64)` factory for `--max-ram-mb` CLI override.
- `epoch_to_ymd_hms()` — dependency-free ISO 8601 timestamp (Howard Hinnant civil calendar algorithm).
- 9 unit tests covering checkpoint round-trip, pressure levels, timestamp correctness.

---

## [FEATURE] GAP-02 — UnitRegistry: TOML-backed runtime conversion table

**Module**: `rust_core/src/units.rs` (EXTENDED)

**Implementation**:
- `UnitRegistry::from_toml(path: &Path)` — loads `unit_registry.toml` at startup.
- `UnitRegistry::to_si(raw_value, unit_string)` — O(1) HashMap lookup; returns
  `ConversionResult { si_value, property, flag_bits }`.
- `FLAG_UNIT_UNKNOWN` set if unit string not found.
- `check_sigma_rho_consistency(sigma, rho, tol)` — σ–ρ cross-check returning
  relative deviation and `FLAG_SIGMA_RHO_INCON` if tolerance exceeded.
- 8 unit tests covering known units, offset conversion (°C→K), unknown units,
  σ–ρ consistency edge cases.

---

## [FEATURE] GAP-03 — PCHIP Interpolation + ZT Cross-Check

**Module**: `rust_core/src/interpolation.rs` (NEW)

**Implementation**:
- `InterpMethod::{ Linear, PCHIP }` — method selector enum.
- `interpolate_pchip(xs, ys, x_query)` — Fritsch–Carlson (1980) monotone cubic
  Hermite; O(log n) per query. No overshoot on monotone data.
- `interpolate_linear(xs, ys, x_query)` — O(log n) binary-search linear fallback.
- `interpolate_to_grid(data, grid, method)` — batch interpolation with automatic
  out-of-range filtering.
- `ZTCrossCheck` result struct: `n_overlap_points`, `mean_absolute_error`,
  `max_absolute_error`, `passes_tolerance`, `flag_bits`.
- `compute_zt_cross_check(s_data, sigma_data, kappa_data, zt_reported, method, min_overlap)`
  — 10 K grid construction, PCHIP interpolation, ε = |ZT_comp − ZT_rep| / max(|ZT_rep|, floor).
- `ZT_CROSSCHECK_TOLERANCE = 0.10` exposed to Python as `rust_core.ZT_CROSSCHECK_TOLERANCE`.
- `InterpError` thiserror enum: 5 variants.
- 9 unit tests: exact knot reproduction, monotonicity, linear data, out-of-range,
  insufficient points, non-monotonic x, self-consistent ZT pass, inconsistent ZT fail,
  insufficient overlap.

---

## [FEATURE] GAP-04 — BloomFilter Deduplication

**Module**: `rust_core/src/dedup.rs` (NEW)

**Implementation**:
- `RecordDeduplicator` with `bloomfilter::Bloom` (capacity=5M, FPR=1e-6, ≈12 MB RAM).
- `DedupKey { sha256_hex, raw_key }` — SHA256(DOI ‖ normalized_composition ‖ T_bucket).
- `normalize_composition(raw)` — lowercase, whitespace-stripped, element-token alphabetical sort.
- `temperature_range_bucket(t_min, t_max)` — 50 K bin discretisation to collapse jitter.
- `make_dedup_key(doi, composition, t_min, t_max)` — deterministic key construction.
- `DedupResult::{ Unique, Duplicate }` with `FLAG_DUPLICATE_SUSPECTED` on duplicate.
- `RecordDeduplicator::new_bypass()` for `--force-include-duplicates` mode.
- 9 unit tests covering composition normalisation, T-bucketing, key determinism,
  order-independence, first/second insert, bypass mode.

---

## [FEATURE] TASK 4 — Streaming Statistics Engine

**Module**: `rust_core/src/statistics.rs` (NEW)

**Implementation**:
- `WelfordAccumulator` — O(1)-memory mean/variance (Welford 1962).
- `P2Quantile` — O(1)-memory streaming quantile (Jain & Chlamtac 1985, ACM 28(10)).
- `QuantileBundle` — 5 simultaneous P² estimators (P5, P25, P50, P75, P95).
- `StatisticsEngine` — central accumulator with `observe_{seebeck,sigma,kappa,zt}_{raw,clean}()`.
- `PipelineCounters` — per-stage record counts and rejection rates.
- `ks_two_sample(s1, s2, name)` — Kolmogorov–Smirnov two-sample test with Kolmogorov
  asymptotic p-value approximation (Numerical Recipes §14.3).
- `StatisticsEngine::write_pipeline_json(output_dir)` → `pipeline_summary.json`.
- `StatisticsEngine::write_property_json(output_dir)` → `property_statistics.json`.
- `StatisticsEngine::write_comparison_report(output_dir, ks_results)` →
  `filtered_vs_unfiltered_report.md` (Markdown table with raw vs. clean stats + KS results).
- `generate_certificate(cert, output_dir)` → `clean_dataset_certificate.md` (GAP-09).
- `DatasetCertificate` serde struct for provenance metadata.
- 8 unit tests: Welford known sequence, empty/single-obs NaN, large-offset stability,
  P² median uniform, quantile ordering, KS identical/different distributions,
  engine accumulation, certificate file content.

---

## [FEATURE] GAP-05 — FigureID Cross-Validation in mirror_walker.rs

**Module**: `rust_core/src/mirror_walker.rs` (EXTENDED)

**Implementation**:
- `build_figure_id_set(figures)` — builds `AHashSet<u32>` from `figure[]` array per file.
- `validate_figure_reference(figureid, known_figure_ids)` — returns `FLAG_FIGUREID_MISMATCH`
  if figureid not found in file's figure set; 0 otherwise.
- `parse_json_file` extended with Stage 3 (FigureID validation) after existing Stage 1+2.
  Measurements failing Stage 3 are **retained** but counted in `figureid_mismatch_count`.
- `StarrydataRecord.figureid_mismatch_count: usize` — new field.
- `WalkSummary.figureid_mismatches: usize` — aggregated across all files.
- `py_scan_domain` now exposes `"figureid_mismatches"` in the Python summary dict.

---

## [FEATURE] GAP-07 / BUG-03 — O(1) Property Lookup via AHashSet + OnceLock

**Module**: `rust_core/src/mirror_walker.rs` (EXTENDED)

**Implementation**:
- `ALLOWED_PROPERTY_SET: OnceLock<ahash::AHashSet<u32>>` — global lazy singleton,
  initialised exactly once from `ALLOWED_PROPERTY_IDS` on first call.
- `allowed_property_set()` — returns `&'static AHashSet<u32>`.
- `is_allowed_property()` now calls `allowed_property_set().contains()` — O(1) amortised.
- Previous O(11) linear scan replaced; semantics identical, performance improved.

---

## [REFACTOR] Session 2 — lib.rs module declarations + Python exports

- `pub mod memory_guard;` — GAP-01.
- `pub mod interpolation;` — GAP-03.
- `pub mod dedup;` — GAP-04.
- `pub mod statistics;` — TASK 4.
- `py_decode_flags(flag_bits: u32) -> Vec<str>` exposed to Python for human-readable
  bitmask decoding in `bad_records_report.jsonl`.
- `rust_core.ZT_CROSSCHECK_TOLERANCE` (= 0.10) exported as Python module constant.
- `[dev-dependencies] tempfile = "3"` added to `Cargo.toml` for `UnitRegistry` TOML tests.
