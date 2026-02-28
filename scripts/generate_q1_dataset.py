#!/usr/bin/env python3
"""
Thermognosis Engine — Q1 Dataset Generation Orchestrator
=========================================================
Document ID  : SPEC-PIPELINE-Q1
Layer        : Orchestration / Data Publication
Status       : Normative — Q1 Standard (Nature Scientific Data / Cell Patterns)
Author       : Thermognosis Research Group

Description
-----------
This script executes the full Q1 dataset generation pipeline, transforming
raw or normalised thermoelectric property data into a rigourously audited,
publication-ready Parquet dataset.

Every thermodynamic state passes through the Triple-Gate Physics Arbiter
implemented in the Rust core (SPEC-AUDIT-01), which assigns a ``ConfidenceTier``
and ``AnomalyFlags`` bitmask to each record without discarding any data.

Pipeline Stages
---------------
1. Source loading  — Read an intermediate Parquet or CSV file.
2. Column mapping  — Resolve S, σ, κ, T, zT column names.
3. Physics audit   — Dispatch all states through the Rust Triple-Gate engine.
4. Telemetry       — Emit an academic-grade audit report to stdout/logging.
5. Serialisation   — Persist the enriched dataset to columnar Parquet with
                     the strict THERMOGNOSIS_SCHEMA (v2.0.0).

Usage
-----
    python scripts/generate_q1_dataset.py \\
        --input  dataset/starry.parquet \\
        --output dataset/processed/q1_thermoelectric_dataset.parquet \\
        --batch-size 50000 \\
        --deterministic

Column Name Aliases
-------------------
The script accepts flexible column name variants for each physical quantity
(see COLUMN_ALIASES below).  Pass ``--col-*`` flags to override auto-detection.
"""

from __future__ import annotations

import argparse
import logging
import sys
import time
from pathlib import Path
from typing import Dict, Optional, Tuple

import numpy as np
import pandas as pd

# ---------------------------------------------------------------------------
# Locate the project root and inject it into sys.path so the package can be
# imported regardless of the working directory.
# ---------------------------------------------------------------------------
_SCRIPT_DIR  = Path(__file__).resolve().parent
_PROJECT_ROOT = _SCRIPT_DIR.parent
sys.path.insert(0, str(_PROJECT_ROOT / "python"))

from thermognosis.wrappers.rust_wrapper import RustCore, RustCoreError
from thermognosis.dataset.parquet_writer import (
    DataPointRecord,
    write_parquet,
    ParquetWriteError,
    SchemaValidationError,
)

# ---------------------------------------------------------------------------
# Logging Setup
# ---------------------------------------------------------------------------
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s | %(levelname)-8s | %(name)s | %(message)s",
    datefmt="%Y-%m-%dT%H:%M:%S",
    stream=sys.stdout,
)
logger = logging.getLogger("thermognosis.q1_pipeline")

# ---------------------------------------------------------------------------
# Physical Constants & Tier Labels
# ---------------------------------------------------------------------------
TIER_LABELS: Dict[int, str] = {1: "A", 2: "B", 3: "C", 4: "Reject"}

# Bitmask flag descriptions for the telemetry report
FLAG_DESCRIPTIONS: Dict[int, str] = {
    0b0001: "Negative κ_lattice (FLAG_NEGATIVE_KAPPA_L)",
    0b0010: "Lorenz number out of bounds (FLAG_LORENZ_OUT_BOUNDS)",
    0b0100: "zT cross-check mismatch > 10% (FLAG_ZT_MISMATCH)",
    0b1000: "Algebraic rejection — unphysical state (FLAG_ALGEBRAIC_REJECT)",
}

# ---------------------------------------------------------------------------
# Flexible Column Name Auto-Detection
# ---------------------------------------------------------------------------
COLUMN_ALIASES: Dict[str, Tuple[str, ...]] = {
    "s":           ("seebeck", "S", "seebeck_coefficient", "Seebeck",
                    "seebeck_uV_K", "S_V_K"),
    "sigma":       ("sigma", "electrical_conductivity", "conductivity",
                    "sigma_S_m", "elec_cond"),
    "kappa":       ("kappa", "thermal_conductivity", "kappa_total",
                    "kappa_W_mK", "therm_cond"),
    "t":           ("T", "temperature", "temp", "T_K", "Temperature"),
    "zt_reported": ("ZT", "zt", "zT", "figure_of_merit", "ZT_reported",
                    "zT_reported", "ZT_dimensionless"),
}


def _resolve_column(df: pd.DataFrame, quantity: str, override: Optional[str]) -> Optional[str]:
    """Return the first matching column name for a physical quantity."""
    if override and override in df.columns:
        return override
    for alias in COLUMN_ALIASES.get(quantity, ()):
        if alias in df.columns:
            return alias
    return None


def _load_source(path: Path) -> pd.DataFrame:
    """Load the raw source file (Parquet or CSV) into a DataFrame."""
    suffix = path.suffix.lower()
    if suffix == ".parquet":
        logger.info("Loading source Parquet file: %s", path)
        return pd.read_parquet(path)
    elif suffix in (".csv", ".tsv"):
        sep = "\t" if suffix == ".tsv" else ","
        logger.info("Loading source CSV file: %s", path)
        return pd.read_csv(path, sep=sep, low_memory=False)
    else:
        raise ValueError(
            f"Unsupported source format '{suffix}'. Expected .parquet, .csv, or .tsv."
        )


def _extract_arrays(
    df: pd.DataFrame,
    col_s: str,
    col_sigma: str,
    col_kappa: str,
    col_t: str,
    col_zt: Optional[str],
) -> Tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
    """
    Extract physical property arrays from the DataFrame with safe casting.

    All arrays are coerced to float64 C-contiguous layout to satisfy the
    Rust FFI zero-copy constraint (SPEC-GOV-CODE-GENERATION-PROTOCOL).
    """
    def _safe_f64(col: str) -> np.ndarray:
        return np.ascontiguousarray(pd.to_numeric(df[col], errors="coerce").values, dtype=np.float64)

    s_arr     = _safe_f64(col_s)
    sigma_arr = _safe_f64(col_sigma)
    kappa_arr = _safe_f64(col_kappa)
    t_arr     = _safe_f64(col_t)
    zt_arr    = _safe_f64(col_zt) if col_zt else np.full(len(df), np.nan, dtype=np.float64)

    return s_arr, sigma_arr, kappa_arr, t_arr, zt_arr


def _emit_audit_telemetry(
    df: pd.DataFrame,
    audit: Dict[str, np.ndarray],
    elapsed_s: float,
) -> None:
    """
    Emit a structured, academic-grade telemetry report to the logging subsystem.

    The report summarises tier distributions, anomaly flag frequencies, and key
    physical statistics to satisfy Q1 data publication disclosure requirements.
    """
    n       = len(audit["tier"])
    tiers   = audit["tier"]
    flags   = audit["anomaly_flags"]
    zt      = audit["zT_computed"]

    # --- Tier distribution ---
    tier_counts = {k: int(np.sum(tiers == k)) for k in (1, 2, 3, 4)}
    tier_a_pct  = 100.0 * tier_counts[1] / n if n > 0 else 0.0
    reject_pct  = 100.0 * tier_counts[4] / n if n > 0 else 0.0

    # --- Anomaly flag breakdown (non-rejected states only) ---
    valid_mask  = tiers < 4
    valid_flags = flags[valid_mask]
    flag_counts = {
        bit: int(np.sum((valid_flags & bit) != 0))
        for bit in FLAG_DESCRIPTIONS
    }

    # --- zT statistics (Tier A & B only — physically consistent states) ---
    ab_mask = (tiers == 1) | (tiers == 2)
    zt_ab   = zt[ab_mask & np.isfinite(zt)]

    sep = "─" * 78

    logger.info(sep)
    logger.info("  THERMOGNOSIS ENGINE — TRIPLE-GATE PHYSICS AUDIT REPORT")
    logger.info("  Document ID: SPEC-AUDIT-01 | Pipeline: SPEC-PIPELINE-Q1")
    logger.info(sep)
    logger.info("  DATASET OVERVIEW")
    logger.info("    Total states processed : %10d", n)
    logger.info("    Elapsed time           : %10.2f s  (%.1f k states/s)",
                elapsed_s, n / elapsed_s / 1000 if elapsed_s > 0 else float("inf"))
    logger.info(sep)
    logger.info("  CONFIDENCE TIER DISTRIBUTION")
    for tier_int, label in TIER_LABELS.items():
        count = tier_counts.get(tier_int, 0)
        pct   = 100.0 * count / n if n > 0 else 0.0
        bar   = "█" * int(pct / 2)
        logger.info("    Tier %-6s │ %8d │ %5.1f%% │ %s", label, count, pct, bar)
    logger.info(sep)
    logger.info("  ANOMALY FLAG BREAKDOWN  (physically computable states only)")
    for bit, desc in FLAG_DESCRIPTIONS.items():
        count = flag_counts.get(bit, 0)
        pct   = 100.0 * count / max(int(np.sum(valid_mask)), 1)
        logger.info("    %-55s │ %7d │ %5.1f%%", desc, count, pct)
    logger.info(sep)
    logger.info("  FIGURE OF MERIT STATISTICS  (Tier A + B, physically consistent)")
    if len(zt_ab) > 0:
        logger.info("    Count   : %d", len(zt_ab))
        logger.info("    Mean zT : %.4f", float(np.mean(zt_ab)))
        logger.info("    Median  : %.4f", float(np.median(zt_ab)))
        logger.info("    Max zT  : %.4f", float(np.max(zt_ab)))
        logger.info("    Std dev : %.4f", float(np.std(zt_ab)))
        p95 = float(np.percentile(zt_ab, 95))
        logger.info("    P95 zT  : %.4f", p95)
    else:
        logger.warning("    No physically consistent (Tier A/B) states found — check input data quality.")
    logger.info(sep)
    logger.info(
        "  PUBLICATION QUALITY ASSESSMENT: %.1f%% Tier A (high confidence), "
        "%.1f%% rejected.",
        tier_a_pct,
        reject_pct,
    )
    logger.info(sep)


def _build_records(df: pd.DataFrame, audit: Dict[str, np.ndarray]) -> list:
    """
    Fuse the raw DataFrame rows with their audit trail vectors to produce
    a stream of ``DataPointRecord`` objects for the Parquet writer.

    This function operates in a single O(N) pass and emits records lazily
    to preserve the O(B) memory guarantee of the streaming writer.
    """
    tiers   = audit["tier"]
    flags   = audit["anomaly_flags"]
    zt_c    = audit["zT_computed"]
    kl      = audit["kappa_lattice"]
    lor     = audit["lorenz_number"]
    cce     = audit["zT_cross_check_error"]

    # Prefer 'composition' / 'material' / 'formula' columns in that order.
    comp_col = next(
        (c for c in ("composition", "material", "formula", "compound") if c in df.columns),
        None,
    )
    pid_col  = next((c for c in ("paper_id", "ref_id", "reference_id") if c in df.columns), None)
    sx_col   = next((c for c in ("property_x", "prop_x", "quantity_x") if c in df.columns), None)
    sy_col   = next((c for c in ("property_y", "prop_y", "quantity_y") if c in df.columns), None)
    ux_col   = next((c for c in ("unit_x",) if c in df.columns), None)
    uy_col   = next((c for c in ("unit_y",) if c in df.columns), None)
    x_col    = next((c for c in ("x",) if c in df.columns), None)
    y_col    = next((c for c in ("y",) if c in df.columns), None)

    for i, row in enumerate(df.itertuples(index=False)):
        tier_val = int(tiers[i])
        flag_val = int(flags[i])

        # Nullable float fields — use Python None for NaN to get Arrow null
        def _f64_or_none(v: float) -> Optional[float]:
            return None if not np.isfinite(v) else float(v)

        yield DataPointRecord(
            sample_id   = int(getattr(row, "sample_id", i)),
            composition = str(getattr(row, comp_col, "unknown")) if comp_col else "unknown",
            paper_id    = int(getattr(row, pid_col,  0))          if pid_col  else 0,
            property_x  = str(getattr(row, sx_col,  ""))          if sx_col   else "",
            property_y  = str(getattr(row, sy_col,  ""))          if sy_col   else "",
            unit_x      = str(getattr(row, ux_col,  ""))          if ux_col   else "",
            unit_y      = str(getattr(row, uy_col,  ""))          if uy_col   else "",
            x           = float(getattr(row, x_col, 0.0))         if x_col    else 0.0,
            y           = float(getattr(row, y_col, 0.0))         if y_col    else 0.0,
            # Q1 Audit Trail
            confidence_tier      = tier_val,
            anomaly_flags        = flag_val,
            zT_computed          = _f64_or_none(zt_c[i]),
            kappa_lattice        = _f64_or_none(kl[i]),
            lorenz_number        = _f64_or_none(lor[i]),
            zT_cross_check_error = _f64_or_none(cce[i]),
        )


def main(args: argparse.Namespace) -> int:
    """
    Main pipeline entry point.

    Returns
    -------
    int
        Exit code — 0 on success, non-zero on failure.
    """
    input_path  = Path(args.input).resolve()
    output_path = Path(args.output).resolve()

    if not input_path.exists():
        logger.critical("Source file not found: %s", input_path)
        return 1

    # -----------------------------------------------------------------------
    # Stage 1: Source Loading
    # -----------------------------------------------------------------------
    try:
        df = _load_source(input_path)
    except Exception as exc:
        logger.critical("Failed to load source data: %s", exc)
        return 1

    logger.info("Loaded %d rows and %d columns from '%s'.",
                len(df), len(df.columns), input_path.name)
    logger.debug("Available columns: %s", list(df.columns))

    # -----------------------------------------------------------------------
    # Stage 2: Column Mapping
    # -----------------------------------------------------------------------
    col_s     = _resolve_column(df, "s",           args.col_s)
    col_sigma = _resolve_column(df, "sigma",        args.col_sigma)
    col_kappa = _resolve_column(df, "kappa",        args.col_kappa)
    col_t     = _resolve_column(df, "t",            args.col_t)
    col_zt    = _resolve_column(df, "zt_reported",  args.col_zt)

    missing = [name for name, col in
               [("S", col_s), ("sigma", col_sigma), ("kappa", col_kappa), ("T", col_t)]
               if col is None]
    if missing:
        logger.critical(
            "Cannot resolve mandatory columns for: %s. "
            "Use --col-* flags or ensure columns match known aliases. "
            "Available columns: %s",
            missing, list(df.columns),
        )
        return 1

    if col_zt is None:
        logger.warning(
            "No 'zT_reported' column found — Gate 3 (cross-validation) will be skipped. "
            "All cross_check_error values will be NaN."
        )

    logger.info(
        "Column mapping resolved: S='%s', σ='%s', κ='%s', T='%s', zT_reported=%s",
        col_s, col_sigma, col_kappa, col_t,
        f"'{col_zt}'" if col_zt else "None (Gate 3 disabled)",
    )

    # -----------------------------------------------------------------------
    # Stage 3: Array Extraction & Physics Audit
    # -----------------------------------------------------------------------
    try:
        s_arr, sigma_arr, kappa_arr, t_arr, zt_arr = _extract_arrays(
            df, col_s, col_sigma, col_kappa, col_t, col_zt
        )
    except Exception as exc:
        logger.critical("Array extraction failed: %s", exc)
        return 1

    logger.info(
        "Initialising Rust Core (deterministic=%s) …", args.deterministic
    )
    try:
        rc = RustCore(deterministic=args.deterministic)
    except RustCoreError as exc:
        logger.critical("Rust backend unavailable: %s", exc)
        return 1

    logger.info(
        "Dispatching %d thermodynamic states through the Triple-Gate Physics Arbiter …",
        len(df),
    )
    t0 = time.perf_counter()
    try:
        audit = rc.audit_thermodynamic_states(
            s_arr, sigma_arr, kappa_arr, t_arr,
            zt_reported=zt_arr,
        )
    except RustCoreError as exc:
        logger.critical("Physics audit failed: %s", exc)
        return 1
    elapsed = time.perf_counter() - t0

    # -----------------------------------------------------------------------
    # Stage 4: Telemetry Report
    # -----------------------------------------------------------------------
    _emit_audit_telemetry(df, audit, elapsed)

    # -----------------------------------------------------------------------
    # Stage 5: Persist to Parquet
    # -----------------------------------------------------------------------
    output_path.parent.mkdir(parents=True, exist_ok=True)

    logger.info(
        "Serialising enriched dataset to '%s' (batch_size=%d, compression=snappy) …",
        output_path, args.batch_size,
    )

    try:
        write_parquet(
            records    = _build_records(df, audit),
            output_path = output_path,
            batch_size  = args.batch_size,
        )
    except (ParquetWriteError, SchemaValidationError) as exc:
        logger.critical("Parquet serialisation failed: %s", exc)
        return 1

    logger.info(
        "Q1 dataset successfully written to '%s'. "
        "Total pipeline time: %.2f s.",
        output_path, time.perf_counter() - t0,
    )
    return 0


# ---------------------------------------------------------------------------
# CLI Argument Parser
# ---------------------------------------------------------------------------

def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Thermognosis Q1 Dataset Generation Pipeline\n"
            "Applies Triple-Gate Physics Audit (SPEC-AUDIT-01) to produce a\n"
            "publication-ready Apache Parquet dataset."
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )

    # --- I/O ---
    io_group = parser.add_argument_group("I/O")
    io_group.add_argument(
        "--input", "-i",
        default=str(_PROJECT_ROOT / "dataset" / "starry.parquet"),
        metavar="PATH",
        help="Path to the source Parquet or CSV file "
             "(default: dataset/starry.parquet).",
    )
    io_group.add_argument(
        "--output", "-o",
        default=str(_PROJECT_ROOT / "dataset" / "processed" / "q1_thermoelectric_dataset.parquet"),
        metavar="PATH",
        help="Output path for the publication-ready Parquet file.",
    )
    io_group.add_argument(
        "--batch-size",
        type=int,
        default=50_000,
        metavar="N",
        help="Row-group batch size for the Parquet writer (default: 50000).",
    )

    # --- Execution ---
    exec_group = parser.add_argument_group("Execution")
    exec_group.add_argument(
        "--deterministic",
        action="store_true",
        default=False,
        help="Force strictly ordered sequential iteration (disables Rayon "
             "work-stealing) for reproducible audit trails.",
    )

    # --- Column overrides ---
    col_group = parser.add_argument_group("Column Name Overrides")
    col_group.add_argument("--col-s",     default=None, metavar="NAME",
                           help="Seebeck coefficient column name.")
    col_group.add_argument("--col-sigma", default=None, metavar="NAME",
                           help="Electrical conductivity column name.")
    col_group.add_argument("--col-kappa", default=None, metavar="NAME",
                           help="Total thermal conductivity column name.")
    col_group.add_argument("--col-t",     default=None, metavar="NAME",
                           help="Absolute temperature column name.")
    col_group.add_argument("--col-zt",    default=None, metavar="NAME",
                           help="Reported zT column name (optional; "
                                "enables Gate 3 cross-validation).")

    return parser


if __name__ == "__main__":
    parser = _build_parser()
    args   = parser.parse_args()
    sys.exit(main(args))
