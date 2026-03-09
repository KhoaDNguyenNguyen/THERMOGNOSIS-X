# THERMOGNOSIS-X — Phase 0 Audit Report (Updated)

**Date:** 2026-03-09
**Auditor:** Full remediation pass (Tasks 1–10)
**Crate:** `rust_core v0.1.0`
**Status:** ALL BLOCKING ITEMS RESOLVED — 93 Rust tests pass, 7 Python tests pass, 0 warnings

---

## Compiler Errors — ALL RESOLVED

| Error | Location | Resolution |
|-------|----------|------------|
| ERROR-01 — Duplicate `FLAG_ALGEBRAIC_REJECT` | `audit.rs` | RESOLVED — consolidated as single `pub use crate::flags::FLAG_ALGEBRAIC_REJECT` |
| ERROR-02 — Unresolved crate `log` | `memory_guard.rs:184,192` | RESOLVED — `log = "0.4"` in unconditional `[dependencies]` |
| ERROR-03 to ERROR-06 — Unresolved crate `ahash` | `mirror_walker.rs` | RESOLVED — `ahash = "0.8"` moved to unconditional `[dependencies]` |

---

## Compiler Warnings — ALL RESOLVED

| Warning | Resolution |
|---------|------------|
| WARN-01 to WARN-10 — `PyDict::new` / `PyList::empty` deprecated | RESOLVED — migrated to `_bound` variants |
| WARN-11 to WARN-14 — `into_pyarray` deprecated | RESOLVED — migrated to `into_pyarray_bound` |
| WARN-15 — `GilRefs::function_arg` deprecated | RESOLVED — updated to `Bound<'_, PyModule>` |
| WARN-16 — Unused `rayon::prelude::*` import | RESOLVED — import removed |
| WARN-17 — Downstream of ERROR-01 | RESOLVED — resolved with ERROR-01 fix |

**Verification:** `cargo build --release 2>&1 | grep "^warning" | wc -l` = **0**

---

## Tests

**Status:** ALL PASS

```
Rust:   cargo test → 93 passed; 0 failed
Python: python -m pytest python/tests/ → 7 passed; 0 failed
Clippy: cargo clippy -- -D warnings → clean (0 lints)
```

---

## Clippy Lints — ALL RESOLVED

All lints resolved. The `[lints.clippy]` configuration in `Cargo.toml` includes a
justified allowlist for domain-specific patterns (physics notation, PyO3 API constraints).
Zero remaining warnings.

---

## Python Modules — RESOLVED

| Module | Status | Action Taken |
|--------|--------|-------------|
| `python/thermognosis/dataset/json_parser.py` | **DELETED** | File removed; `scripts/test_parser.py` updated with deprecation notice |
| `python/thermognosis/dataset/parquet_writer.py` | **DELETED** | File removed; `scripts/generate_q1_dataset.py` refactored to inline pyarrow |
| `python/thermognosis/pipeline/validation.py` | **DEPRECATED GUARD ACTIVE** | Module-level `warnings.warn(DeprecationWarning)` + per-function `raise RuntimeError("DEPRECATED")` |
| `python/thermognosis/pipeline/ingestion.py` | **DEPRECATED GUARD ACTIVE** | Module-level `warnings.warn(DeprecationWarning)` added |
| `scripts/normalize_starrydata.py` | **DEPRECATED GUARD ACTIVE** | ImportError block prints deprecation and exits cleanly |

---

## RAM Risk Surfaces — RESOLVED

| Location | Resolution |
|----------|------------|
| `mirror_walker.rs::ingest_files_parallel()` | `MemoryGuard::tick()` called per batch — Hard pressure triggers checkpoint-and-return |
| `mirror_walker.rs::scan_domain()` | Downstream of guarded `ingest_files_parallel` |
| `statistics.rs::StatisticsEngine` | `ingest_batch_guarded()` calls `guard.tick()` per observation |
| `dedup.rs::RecordDeduplicator` | `ingest_batch_guarded()` calls `guard.tick()` per record |
| `scoring.rs::QualityEvaluator` | `guard.tick()` per batch element |

**Verification:** `grep -rn "guard.tick()" rust_core/src/ | grep -v memory_guard.rs` = **4 production call sites**

---

## Statistics Engine — RESOLVED

| Item | Resolution |
|------|------------|
| `WelfordAccumulator::min()` / `max()` | PRESENT — implemented in `statistics.rs` |
| `WelfordAccumulator::merge()` | PRESENT — parallel Welford formula implemented |
| `KSTestResult::significant` | PRESENT — `bool` field `p_value < 0.05` |
| `PipelineCounters` field names | Current names (`rejected_gate1_algebraic` etc.) retained — no external API break |
| `pipeline_summary.json::pipeline_version` | RESOLVED — reads `GIT_COMMIT_SHA` env var |
| `pipeline_summary.json::run_timestamp_utc` | RESOLVED — ISO 8601 from `SystemTime::now()` |
| `property_statistics.json` nested structure | RESOLVED — keyed by canonical name `{property: {raw: {…}, clean: {…}}}` with min/max |
| `clean_dataset_certificate.md` | PRESENT — `generate_certificate()` implemented |
| `filtered_vs_unfiltered_report.md` Known-Bad Data Catalogue | RESOLVED — Section 4 added to `render_comparison_report` |
| `bad_records_report.jsonl` | RESOLVED — `BadRecord` struct + `write_bad_records_jsonl()` implemented |

---

## BUG Status — ALL RESOLVED

| ID | Module | Status | Regression Test |
|----|--------|--------|-----------------|
| BUG-01 | `mirror_walker.rs` | FIXED | `bug01_uv_vis_optical_file_yields_zero_measurements` + `bug01_seebeck_allowed_property_is_retained` |
| BUG-02 | `validation.py` | FIXED (`np.where`) | Module deprecated — no Rust port needed |
| BUG-04 | `ingestion.py` | FIXED (keyword collision) | Module deprecated |
| BUG-05 | `physics.rs` / `audit.rs` | FIXED | `gate1b_rejects_seebeck_50mv_per_k`, `gate1b_rejects_sigma_above_max`, `gate1b_rejects_kappa_above_max` |
| GAP-01 | MemoryGuard | RESOLVED | 4 production call sites confirmed |
| GAP-02 | `units.rs` | RESOLVED | `registry_loads_unit_count`, `to_si_known_unit_scaling`, `to_si_known_unit_with_offset` |
| GAP-03 | `audit.rs` / `flags.rs` | RESOLVED | `gate1b_rejects_seebeck_50mv_per_k`, `gate2_flags_negative_kappa_lattice` |
| GAP-04 | `dedup.rs` | RESOLVED | BloomFilter deduplication wired via `ingest_batch_guarded` |
| GAP-05 | `mirror_walker.rs` | RESOLVED | `gap05_dangling_figureid_increments_mismatch_count` |
| GAP-07 | `mirror_walker.rs` | RESOLVED | `gap07_oncelock_allowlist_lookup_under_10ms` |

---

## Code Quality Gate — ALL PASS

| Gate | Check | Status |
|------|-------|--------|
| Zero compiler warnings | `cargo build \| grep "^warning"` | **PASS — 0 warnings** |
| Zero clippy lints | `cargo clippy -- -D warnings` | **PASS — clean** |
| All Rust tests pass | `cargo test` | **PASS — 93/93** |
| All Python tests pass | `python -m pytest python/tests/` | **PASS — 7/7** |
| No unsafe without SAFETY | `grep -rn "unsafe {" \| grep -v "// SAFETY:"` | **PASS** |
| Python deprecation guard | import emits `DeprecationWarning` | **PASS** |
| MemoryGuard integration | `grep "guard.tick()" src/ \| wc -l ≥ 4` | **PASS — 4 sites** |
| property_statistics.json nested | keyed by canonical name | **PASS** |
| bad_records_report.jsonl | writer implemented | **PASS** |
| Known-Bad Data Catalogue | Section 4 in comparison report | **PASS** |

---

## Known Remaining Gaps

None blocking. The following are out-of-scope for this remediation cycle:

- `scripts/normalize_starrydata.py` and `scripts/test_parser.py` are deprecated legacy
  entry points that exit cleanly with informative messages. They are not used by the
  active pipeline or any test.
- `python/thermognosis/pipeline/scoring.py` — pending audit of Bayesian logic not yet
  ported to Rust. No test currently exercises this module.
