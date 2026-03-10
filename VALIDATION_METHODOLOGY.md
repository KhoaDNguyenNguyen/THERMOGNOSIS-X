# VALIDATION METHODOLOGY — THERMOGNOSIS-X

**Intended audience**: Peer reviewers for Nature Scientific Data / npj Computational Materials.
This document serves as Supplementary Material S1: Data Validation Protocol.

---

## 1. Overview

The THERMOGNOSIS-X pipeline applies a seven-stage validation protocol to every
data point ingested from the Starrydata mirror corpus before it enters the clean
thermoelectric dataset. This document enumerates each constraint, its physical
basis, the primary literature source, the threshold value used, and the anomaly
flag assigned upon violation.

All numerical thresholds are encoded in `rust_core/src/constants.rs` (single
source of truth) and are referenced identically by the batch-compute kernel
(`physics.rs::calc_zt_batch`) and the Triple-Gate Physics Arbiter
(`audit.rs::triple_check_physics`). No threshold appears more than once in the
codebase.

---

## 2. Stage 1 — Thermoelectric Property Allowlist (BUG-01 / GAP-01)

**Constraint**: Only measurements whose `propertyid_y` belongs to the set
`{2, 3, 4, 5, 6, 10, 11, 12, 14, 15, 16}` are admitted.

**Physical basis**: The Starrydata corpus contains measurements of optical
(Absorbance, Reflectance, UV-Vis), mechanical, and thermal-diffusivity
properties alongside electronic transport properties. These non-thermoelectric
measurements are irrelevant to ZT calculation and would corrupt statistical
summaries if included.

**Implementation**: `rust_core/src/mirror_walker.rs::is_allowed_property()`.

**Rejection flag**: None (record is silently excluded; counted in
`rejected_non_thermoelectric_count`).

**Policy document**: `allowed_properties.toml`.

---

## 3. Stage 2 — NaN/Inf Artifact Rejection

**Constraint**: Every data point must satisfy `x.is_finite() && y.is_finite()`.

**Physical basis**: Figure-digitization software (WebPlotDigitizer and similar
tools) occasionally produces ±∞ or NaN outputs at graph boundary singularities.
These are computational artifacts, not physical measurements.

**Implementation**: `mirror_walker.rs::parse_json_file()` stage 1 filter.

**Rejection flag**: Counted in `rejected_non_thermoelectric_count` (shares
counter with allowlist rejections for pipeline statistics simplicity).

---

## 4. Stage 3 — Algebraic Positivity Constraints (Gate 1)

**Constraint**: All of the following must hold:
- `T > 0` (absolute temperature is strictly positive)
- `σ > 0` (electrical conductivity is strictly positive)
- `κ > 0` (thermal conductivity is strictly positive)
- All values must be finite (no NaN, ±Inf)

**Physical basis**: These follow directly from the second law of thermodynamics.
Negative absolute temperature violates Boltzmann statistics. Negative conductivity
or thermal conductivity is thermodynamically forbidden in passive materials.
Reference: Callen, "Thermodynamics and an Introduction to Thermostatistics"
(2nd ed., 1985), Chapter 2.

**Threshold**: Any violation → unconditional rejection.

**Implementation**: `audit.rs::triple_check_physics()` Gate 1.

**Rejection flag**: `FLAG_ALGEBRAIC_REJECT` (bit 11).

---

## 5. Stage 4 — Empirical Magnitude Bounds (Gate 1b)

These bounds are P03 Level-3 constraints: they reject physically impossible
values not covered by the algebraic positivity check.

### 5.1 Seebeck Coefficient

**Constraint**: |S| ≤ 1000 µV/K = 1 mV/K = 1×10⁻³ V/K.

**Physical basis**: No bulk thermoelectric material has been reported to exceed
this value. The theoretical upper limit for a parabolic band semiconductor is
of order 1 mV/K. Values exceeding this threshold in the raw data virtually always
indicate a unit conversion failure (e.g., µV/K value stored without converting
to V/K: 500 µV/K → 500×10⁻⁶ V/K stored as 500 → interpreted as 500 V/K).
Reference: Goldsmid, "Introduction to Thermoelectricity" (2010), Chapter 2;
Snyder & Toberer, Nature Materials 7, 105–114 (2008). DOI: 10.1038/nmat2090.

**Threshold**: `SEEBECK_MAX_ABS_V_PER_K = 1.0e-3 V/K`

**Implementation**: `audit.rs::triple_check_physics()` Gate 1b; `constants.rs`.

**Rejection flag**: `FLAG_SEEBECK_BOUND_EXCEED` (bit 6) + `FLAG_ALGEBRAIC_REJECT`.

### 5.2 Electrical Conductivity

**Constraint**: σ ≤ 1×10⁷ S/m.

**Physical basis**: The highest-conductivity thermoelectric materials are
degenerately doped semiconductors approaching metallic conduction. No reported
thermoelectric exceeds ~10⁶ S/m at room temperature. The bound 10⁷ S/m
corresponds to the conductivity of copper, which is an extreme upper limit.
Values above this indicate a unit error (S/cm stored without ×100 conversion).
Reference: Pei et al., Nature 473, 66–69 (2011). DOI: 10.1038/nature09996.

**Threshold**: `SIGMA_MAX_S_PER_M = 1.0e7 S/m`

**Implementation**: `audit.rs::triple_check_physics()` Gate 1b.

**Rejection flag**: `FLAG_SIGMA_BOUND_EXCEED` (bit 7) + `FLAG_ALGEBRAIC_REJECT`.

### 5.3 Thermal Conductivity

**Constraint**: κ ≤ 100 W/(m·K).

**Physical basis**: The highest thermal conductivity of any thermoelectric material
is approximately 20 W/(m·K) (single-crystal silicon at room temperature is ~150
W/(m·K) but is not a thermoelectric material). Setting 100 W/(m·K) provides
generous headroom for outlier alloys and doped semiconductors. Values above
this bound indicate mW/(m·K) data stored without the ×10⁻³ conversion.
Reference: Zhao et al., Nature 508, 373–377 (2014). DOI: 10.1038/nature13184.

**Threshold**: `KAPPA_MAX_W_PER_MK = 100.0 W/(m·K)`

**Implementation**: `audit.rs::triple_check_physics()` Gate 1b.

**Rejection flag**: `FLAG_KAPPA_BOUND_EXCEED` (bit 8) + `FLAG_ALGEBRAIC_REJECT`.

---

## 6. Stage 5 — Wiedemann–Franz Consistency (Gate 2)

**Constraint**: The effective Lorenz number L = κ/(σT) must lie in
[L_MIN, L_MAX] = [1×10⁻⁹, 1×10⁻⁷] W·Ω·K⁻².

Additionally, the lattice thermal conductivity κ_L = κ − L₀σT must satisfy
κ_L ≥ 0, where L₀ = 2.44×10⁻⁸ W·Ω·K⁻² is the Sommerfeld value.

**Physical basis**: The Wiedemann–Franz law states that in a Fermi liquid,
L ≈ L₀ = π²kB²/(3e²) = 2.44×10⁻⁸ W·Ω·K⁻². Deviations by orders of magnitude
from L₀ indicate either bipolar diffusion, phonon drag, or inconsistent σ/κ
measurements from different samples. κ_L < 0 is a physical impossibility:
it would imply that the electronic contribution exceeds the total conductivity.
Reference: Wiedemann & Franz (1853); Sommerfeld (1928); Slack, in "CRC Handbook
of Thermoelectrics" (1995), Chapter 34.

**Thresholds**:
- `L_MIN = 1.0e-9 W·Ω·K⁻²`
- `L_MAX = 1.0e-7 W·Ω·K⁻²`
- `L0_SOMMERFELD = 2.44e-8 W·Ω·K⁻²`

**Implementation**: `audit.rs::triple_check_physics()` Gate 2.

**Flags**: `FLAG_WF_VIOLATION` (bit 0). Downgrade: TierA → TierB (Lorenz only)
or TierA/B → TierC (κ_L < 0).

---

## 7. Stage 6 — ZT Cross-Validation (Gate 3)

**Constraint**: When a reported ZT value is available, the relative deviation
between the computed ZT and the reported ZT must satisfy:
ε = |ZT_computed − ZT_reported| / |ZT_reported| ≤ 0.10 (10%)

**Physical basis**: ZT_computed = S²σT/κ is computed from the individually
reported transport coefficients. ZT_reported is the value stated by the source
paper. If these deviate by more than 10%, either the transport coefficients
and the ZT value come from different samples/conditions, or there is a systematic
error in one or more values (e.g., unit mismatch in σ, temperature averaging
inconsistency).
Reference: Kim et al., Science 348, 109–114 (2015). DOI: 10.1126/science.aaa6166.

**Threshold**: 10% relative deviation.

**Implementation**: `audit.rs::triple_check_physics()` Gate 3.

**Flag**: `FLAG_ZT_CROSSCHECK_FAIL` (bit 1). Downgrade: TierA/B → TierC.

---

## 8. Confidence Tier Classification

Every validated measurement receives a `ConfidenceTier`:

| Tier     | u8 | Condition                                                     |
|----------|----|---------------------------------------------------------------|
| `TierA`  |  1 | All three gates pass; anomaly_flags = 0.                     |
| `TierB`  |  2 | Lorenz number anomalous; κ_L ≥ 0 and ZT cross-check passes. |
| `TierC`  |  3 | κ_L < 0 OR ZT cross-check deviation > 10%.                  |
| `Reject` |  4 | Gate 1 or Gate 1b violation (algebraic or empirical bound).  |

Only TierA and TierB records enter the primary clean Parquet dataset.
TierC records are included in an annotated dataset with full flag metadata.
Rejected records are written exclusively to `bad_records_report.jsonl`.

---

## 9. Temperature Domain Constraints

**Constraint**: T ∈ [100 K, 2000 K].

**Physical basis**: Below 100 K, thermoelectric measurements typically represent
cryogenic characterisation outside the practical application domain (refrigerators
and generators operate between ~200 K and ~1200 K for most reported materials).
Above 2000 K, virtually all thermoelectric materials have decomposed or melted.
Records with T outside this range are **flagged but not automatically rejected**.
They are classified as `TierC` if all other gates pass (soft flag).

**Thresholds**: `T_MIN_K = 100.0 K`, `T_MAX_K = 2000.0 K` (defined in `constants.rs`).

**Flag**: `FLAG_TEMPERATURE_OUT_OF_RANGE` (bit 12). Setting this flag downgrades
the record to at most `TierC`; no derived quantities (zT, L, κ_lattice) are
suppressed. The flag is a soft warning, not a hard reject.

**Implementation**: `audit.rs::triple_check_physics()`, temperature domain check
block (after Gate 1b, before Gate 2). `physics.rs::validate_empirical_bounds()`
also enforces this range for the scalar `compute_zt()` path; the Triple-Gate
Arbiter (`audit.rs`) is the single canonical authority for T-range policy in
the Q1 pipeline (`generate_q1_dataset.py`).

**Unit tests**: `audit.rs::temperature_below_100k_sets_out_of_range_flag`,
`audit.rs::temperature_above_2000k_sets_out_of_range_flag`.

---

## 10. Deduplication Protocol (GAP-04)

**Method**: xxHash3-based BloomFilter (capacity = 5,000,000, FPR ≤ 10⁻⁶,
≈12 MB RAM). Deduplication key: SHA256(DOI ‖ normalized_composition ‖
temperature_range_bucket).

**Flag**: `FLAG_DUPLICATE_SUSPECTED` (bit 10). Flagged records are excluded from
the clean dataset and written to `duplicates.jsonl`.

**Note**: BloomFilter false positives are possible at FPR=10⁻⁶. The
`--force-include-duplicates` CLI flag bypasses this filter for reproducibility
audits.

---

## 10b. Seebeck Coefficient Sign Convention (Known Limitation)

The pipeline validates the **magnitude** |S| against the empirical upper bound
(Stage 4, Gate 1b: |S| ≤ 1000 µV/K). It does **not** enforce sign convention
consistency between p-type (S > 0) and n-type (S < 0) materials.

**Implication for reviewers**: Records with physically implausible sign
combinations (e.g., a positive S assigned to a material known to be n-type)
are not flagged by the current implementation. The Starrydata corpus does not
include carrier-type annotations on a per-measurement basis, making automated
sign-consistency enforcement infeasible without additional metadata.

**Recommendation**: Downstream analyses that distinguish p-type from n-type
branches should apply a sign filter post-hoc using the `composition` field
or literature-derived carrier-type labels. This limitation should be disclosed
in the dataset description.

**Future work**: A carrier-type annotation field (`carrier_type`: p/n/unknown)
could be added to the output schema in a future pipeline version, enabling
sign-consistency validation.

---

## 11. Unit Conversion

All property values should be converted to SI units before any physics check.
The conversion registry is defined in `unit_registry.toml`, which maps every
known Starrydata unit string to its SI equivalent via an affine transformation
(si_value = raw_value × factor + offset).

**Current implementation status (GAP-02)**:
- TOML registry: **complete** (`unit_registry.toml`; 40+ unit strings mapped).
- Rust `UnitConverter` integration: **not yet wired** into the main Q1
  generation pipeline (`generate_q1_dataset.py` / `mirror_walker.rs` full path).
- Current workaround: The Starrydata mirror data is assumed to be pre-normalized
  to SI units by the ingestion pipeline (`scripts/run_starrydata_fast.py`).
  Records with unrecognized unit strings **currently pass through without
  `FLAG_UNIT_UNKNOWN` being set** — this is a known gap.
- `FLAG_UNIT_UNKNOWN` (bit 2) is defined in `flags.rs` and will be set once
  the UnitConverter is wired into the pipeline in a future release.

**Audit trail**: When the UnitConverter integration is complete, the full
conversion audit trail will be written to `conversion_audit.jsonl`.

---

## 13. Default Measurement Uncertainty Assumption

When measurement uncertainties are not explicitly reported in the source
publication, the pipeline applies a default relative uncertainty of **5%** to
all four transport properties (S, σ, κ, T). This assumption is used in the
first-order error propagation of zT (P02-ZT-ERROR-PROPAGATION) in
`orchestrator.py`.

**Physical justification**: The 5% value is a conservative upper bound consistent
with:
- WebPlotDigitizer digitization uncertainty: ~2–3% for well-defined curves,
  up to 5% near axis boundaries (Rohatgi, 2022).
- ZEM-3 (ULVAC) Seebeck/electrical conductivity measurement system: ≤5%
  combined relative uncertainty.
- LFA 457 (Netzsch) laser flash thermal diffusivity system: ≤3% reproducibility.

Reference: Rohatgi, A. (2022). WebPlotDigitizer v4.6. Pacifica, CA, USA.
Available at: https://automeris.io/WebPlotDigitizer

**Implementation**: `orchestrator.py::DEFAULT_RELATIVE_UNCERTAINTY = 0.05`.

**Reproducibility**: The `--uncertainty-pct` flag in `generate_q1_dataset.py`
(default: 5.0) logs the assumed percentage in the telemetry report and is
available for sensitivity analysis. Supplementary analysis with 2% and 10%
assumptions is recommended.

---

## 12. Implementation References

| Constraint                      | Module                              | Constant / Function                          |
|---------------------------------|-------------------------------------|----------------------------------------------|
| Property allowlist              | `mirror_walker.rs`                  | `ALLOWED_PROPERTY_IDS`, `is_allowed_property` |
| Physical bounds (all)           | `constants.rs`                      | `SEEBECK_MAX_ABS_V_PER_K`, `SIGMA_MAX_S_PER_M`, `KAPPA_MAX_W_PER_MK` |
| Lorenz bounds                   | `constants.rs`                      | `L_MIN`, `L_MAX`, `L0_SOMMERFELD`           |
| Gate 1: Positivity              | `audit.rs::triple_check_physics()`  | Gate 1 block                                |
| Gate 1b: Empirical bounds       | `audit.rs::triple_check_physics()`  | Gate 1b block                               |
| Gate 2: Wiedemann-Franz         | `audit.rs::triple_check_physics()`  | Gate 2 block                                |
| Gate 3: ZT cross-check          | `audit.rs::triple_check_physics()`  | Gate 3 block                                |
| ZT batch computation            | `physics.rs::calc_zt_batch()`       | Uses canonical constants                     |
| Temperature domain (soft flag)  | `audit.rs::triple_check_physics()`  | `FLAG_TEMPERATURE_OUT_OF_RANGE` (bit 12)    |
| Flag definitions                | `flags.rs`                          | All `FLAG_*` constants (13 flags, bits 0–12) |
| NumPy constraint evaluation     | `validation.py::ThermoelectricValidator.validate()` | BUG-02 corrected |
| Experiment classification       | `ingestion.py::classify_experiment_type()` | BUG-04 corrected            |
