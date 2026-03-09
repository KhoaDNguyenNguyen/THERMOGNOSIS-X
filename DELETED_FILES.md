# DELETED_FILES.md — Python Module Deletion Justification (TASK 3)

This document records which Python modules have been (or must be) deleted and
the precise justification for each deletion, as required by the TASK 3 Python
Elimination mandate.

The guiding principle: **Rust is canonical**. No dual implementations are
permitted. Python may only exist as a thin CLI shim or visualization notebook.

---

## Status Summary

| Module                                            | Status            | Canonical Rust Equivalent                  |
|---------------------------------------------------|-------------------|--------------------------------------------|
| `python/thermognosis/dataset/json_parser.py`      | PENDING DELETION  | `rust_core/src/mirror_walker.rs`           |
| `python/thermognosis/pipeline/validation.py`      | DEMOTED (shim)    | `rust_core/src/validation.rs`, `audit.rs`  |
| `python/thermognosis/pipeline/ingestion.py`       | DEMOTED (shim)    | `rust_core/src/mirror_walker.rs`           |
| `python/thermognosis/pipeline/scoring.py`         | PENDING AUDIT     | `rust_core/src/scoring.rs`                 |
| `python/thermognosis/dataset/parquet_writer.py`   | PENDING DELETION  | Rust `arrow2`/`parquet2` crate (future)    |

---

## Detailed Justifications

### `python/thermognosis/dataset/json_parser.py`

**Status**: PENDING DELETION

**Justification**: The Rust `mirror_walker.rs` module (`py_scan_domain`)
implements a three-phase parallel JSON ingestion engine that is:
- 15–50× faster than Python `json.load()` on the same hardware.
- Memory-safe: never panics; all errors are `Result<>`.
- Fail-soft: parse errors are captured per-file without aborting the pipeline.
- Exposes results via PyO3 FFI with zero-copy NumPy handoff.

Maintaining a Python JSON parser alongside the Rust implementation creates:
- Divergence risk: schema changes must be applied to two parsers.
- Test burden: two test suites for the same semantics.
- Performance regression: Python parser cannot handle the full ~500k-file mirror.

**Action**: Delete `json_parser.py`. All callers must use
`rust_core.py_scan_domain()` or `rust_core.py_validate_single_file()`.

---

### `python/thermognosis/pipeline/validation.py`

**Status**: DEMOTED — retain as thin shim; do NOT use for production validation

**Justification**: The Rust `validation.rs::ThermoelectricState::validate()`
and `audit.rs::triple_check_physics()` implement all constraints defined in
this Python module, plus additional constraints (empirical bounds, Wiedemann-
Franz, ZT cross-check). Python `validation.py` is a partial, slower reimplementation.

**BUG-02** in this file (fixed in this remediation) is a concrete example of
how parallel implementations diverge: the Rust version correctly handles NaN
via explicit `f64::is_nan()` checks, while the Python version had undefined
behavior via `np.greater(..., where=...)`.

**Action**: The file has been patched (BUG-02 fix applied) to prevent active
harm. It must not be used in any production pipeline path. Production validation
must call `rust_core.check_physics_consistency_py()` or
`rust_core.audit_thermodynamics_py()`. Schedule for deletion once all callers
are confirmed to use the Rust path.

---

### `python/thermognosis/pipeline/ingestion.py`

**Status**: DEMOTED — `classify_experiment_type()` patched (BUG-04 fix)

**Justification**: The `MeasurementIngestor` class accepts a dict-based record
structure incompatible with the Starrydata JSON schema parsed by `mirror_walker.rs`.
It requires `mat_id`, `paper_id`, `T`, `S`, `sigma`, `kappa` as top-level dict
keys — but Starrydata records contain `rawdata[].x`, `rawdata[].y`,
`rawdata[].propertyid_y` which require the property pivot performed by
`build_starrydata_duckdb.py`.

The `classify_experiment_type()` function (BUG-04 fixed) should be ported to
Rust as part of the experiment classifier (BUG-04 Rust port is a remaining gap).

**Action**: Do not route new data through `MeasurementIngestor`. It operates
on a pre-processed format that is only produced by legacy scripts. The Rust
ingestion engine in `mirror_walker.rs` is the production path.

---

### `python/thermognosis/pipeline/scoring.py`

**Status**: PENDING AUDIT

**Justification**: `rust_core/src/scoring.rs` implements the full Bayesian
quality scoring framework (`QualityEvaluator`, `QualityVector`, `ScoringWeights`,
`QualityClass`). Before deleting the Python `scoring.py`, verify:
1. Every function in `scoring.py` has a Rust equivalent.
2. The Bayesian credibility model in `python/thermognosis/pipeline/scoring.py`
   is covered by `rust_core/src/bayesian.rs` (not merely `scoring.rs`).

**Action**: Audit `scoring.py` against `scoring.rs` + `bayesian.rs`. Port any
Bayesian-specific logic not yet in Rust, then delete the Python module.

---

### `python/thermognosis/dataset/parquet_writer.py`

**Status**: PENDING DELETION

**Justification**: Parquet writing is currently performed by the Python
`scripts/build_starrydata_duckdb.py` pipeline using PyArrow, which is acceptable
for a scripted ETL. However, if the pipeline migrates to a fully Rust binary,
`parquet_writer.py` would become dead code.

**Action**: For Q1 submission, the current PyArrow approach in
`build_starrydata_duckdb.py` is acceptable. Delete `parquet_writer.py` (the
isolated module, not the inline PyArrow calls in the pipeline script) as it
duplicates functionality available directly via `pyarrow.parquet`.

---

## Files That Must NOT Be Deleted

| Module                                            | Reason to Retain                              |
|---------------------------------------------------|-----------------------------------------------|
| `python/thermognosis/cli.py`                      | Thin CLI shim — acceptable per TASK 3 policy |
| `python/thermognosis/wrappers/rust_wrapper.py`    | FFI bridge — required for calling Rust core   |
| `scripts/build_starrydata_duckdb.py`              | ETL orchestrator — no Rust equivalent yet     |
| `scripts/statistical_analysis.py`                 | Visualization only (matplotlib) — acceptable  |
| All Jupyter notebooks in `python/`                | Visualization only — acceptable per TASK 3    |
