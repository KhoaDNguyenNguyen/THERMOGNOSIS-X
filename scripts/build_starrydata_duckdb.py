#!/usr/bin/env python3
# scripts/build_starrydata_duckdb.py
#
# SPEC-INGEST-PIPELINE-01 — Starrydata Mirror → DuckDB Ingestion Engine
# ======================================================================
# Layer    : Python / ETL Pipeline
# Status   : Normative — Q1 Infrastructure Standard
# Requires : rust_core (compiled), duckdb, pandas, pyarrow, tqdm
#
# Pipeline Stages:
#   Stage 1 — Parallel JSON Ingestion (Rust)
#   Stage 2 — Deduplication & Normalization (pandas)
#   Stage 3 — SI Unit Conversion (pandas + Rust Physics Constants)
#   Stage 4 — Dimensional Loading (DuckDB)
#   Stage 5 — Triple-Gate Physics Audit (Rust, SPEC-AUDIT-01)
#   Stage 6 — Thermoelectric State Materialization (DuckDB SQL)
#   Stage 7 — Parquet Export (pyarrow)

import json
import logging
import time
from pathlib import Path

import duckdb
import numpy as np
import pandas as pd
import pyarrow as pa
import pyarrow.parquet as pq
import rust_core
from tqdm import tqdm

# =============================================================================
# CONFIGURATION
# =============================================================================

MIRROR_ROOT   = Path("starrydata_mirror")
DB_PATH       = Path("data/thermognosis.duckdb")
PARQUET_DIR   = Path("data/parquet")
DOMAINS       = ("samples", "papers", "figures")

# Temperature binning resolution (K). 10 K bins suppress digitization jitter
# from figure-extraction software while preserving physical fidelity.
T_BIN_RESOLUTION_K = 10.0

# SI conversion registry: property_id → (si_unit, multiplicative_factor).
# All Starrydata values are stored in SI units by the upstream database maintainers,
# so every factor is 1.0. The registry's purpose is solely to mark which property IDs
# are analytically known: IDs absent from this dict receive y_si = NaN in stage3,
# which propagates NULL through the S5 pivot and prevents triplet formation.
#
# BUG-FIX: property IDs 3 (σ) and 4 (κ) were previously absent, causing y_si = NaN
# for both columns and making the complete_mask identically False → 0 triplets.
#
# Source: Starrydata2 property ontology; IUPAC Green Book (2007 / 2024 update).
_SI_REGISTRY: dict[int, tuple[str, float]] = {
    1:  ("K",           1.0),  # Temperature
    2:  ("V/K",         1.0),  # Seebeck coefficient
    3:  ("S/m",         1.0),  # Electrical conductivity
    4:  ("W/(m*K)",     1.0),  # Total thermal conductivity
    5:  ("ohm*m",       1.0),  # Electrical resistivity
    6:  ("W/(m*K^2)",   1.0),  # Power factor (S²σ)
    7:  ("m^2/s",       1.0),  # Thermal diffusivity
    8:  ("J/(kg*K)",    1.0),  # Specific heat capacity (isobaric)
    9:  ("kg/m^3",      1.0),  # Mass density
    10: ("m^3/C",       1.0),  # Hall coefficient
    11: ("m^-3",        1.0),  # Carrier concentration
    12: ("m^2/(V*s)",   1.0),  # Carrier (Hall) mobility
    14: ("W/(m*K)",     1.0),  # Lattice thermal conductivity
    15: ("K^-1",        1.0),  # Figure of merit Z (= zT / T)
    16: ("1",           1.0),  # Dimensionless figure of merit ZT
}

# =============================================================================
# LOGGING
# =============================================================================

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s — %(message)s",
    datefmt="%Y-%m-%dT%H:%M:%S",
)
log = logging.getLogger("THERMOGNOSIS.INGEST")


# =============================================================================
# STAGE 1 — PARALLEL JSON INGESTION (RUST)
# =============================================================================

def stage1_ingest_domain(mirror_root: Path, domain: str) -> tuple[pd.DataFrame, pd.DataFrame, pd.DataFrame, pd.DataFrame, pd.DataFrame]:
    """
    Delegates the three-phase parallel walk (SPEC-IO-WALKER-01) to the Rust
    core. Returns five DataFrames: measurements, samples, papers, properties,
    figures extracted from every JSON file in the domain.

    The GIL is released inside `py_scan_domain` for the full shard-discovery
    and parallel-parse pipeline; Python only incurs FFI reconstruction costs.
    """
    domain_root = str(mirror_root / domain)
    log.info(f"[S1] Scanning domain '{domain}' via Rust parallel walker...")
    t0 = time.perf_counter()

    records_list, summary = rust_core.py_scan_domain(domain_root, domain)

    elapsed = time.perf_counter() - t0
    log.info(
        f"[S1] '{domain}': {summary['files_parsed']:,} files parsed, "
        f"{summary['total_measurements']:,} measurements, "
        f"{summary['files_failed']:,} failures — {elapsed:.2f}s"
    )
    if summary["files_failed"] > 0:
        for err in summary["errors"][:10]:
            log.warning(f"[S1] ParseError: {err}")

    # Flatten records into columnar lists — avoids repeated DataFrame appends
    meas_rows   = []
    sample_rows = []
    paper_rows  = []

    for rec in records_list:
        src = rec["source_file"] if "source_file" in rec else rec.get("source_path", "")
        for m in rec["measurements"]:
            m["source_domain"] = domain
            m["source_file"]   = src
            meas_rows.append(m)
        for s in rec["samples"]:
            paper_rows_from_rec = rec["papers"]  # noqa — used below
            sample_rows.append(s)
        for p in rec["papers"]:
            paper_rows.append(p)

    df_meas    = pd.DataFrame(meas_rows)    if meas_rows    else _empty_measurements()
    df_samples = pd.DataFrame(sample_rows)  if sample_rows  else _empty_samples()
    df_papers  = pd.DataFrame(paper_rows)   if paper_rows   else _empty_papers()

    return df_meas, df_samples, df_papers


def _empty_measurements() -> pd.DataFrame:
    return pd.DataFrame(columns=[
        "paperid","sampleid","figureid","x","y",
        "propertyid_x","propertyid_y","source_domain","source_file"
    ])

def _empty_samples() -> pd.DataFrame:
    return pd.DataFrame(columns=[
        "sampleid","paperid","samplename","composition","sampleinfo_json"
    ])

def _empty_papers() -> pd.DataFrame:
    return pd.DataFrame(columns=[
        "paperid","doi","title","author","author_full",
        "journal","journal_full","year","volume","pages","publisher","url"
    ])


# =============================================================================
# STAGE 2 — DEDUPLICATION & NORMALIZATION
# =============================================================================

def stage2_deduplicate(
    all_meas: pd.DataFrame,
    all_samples: pd.DataFrame,
    all_papers: pd.DataFrame,
) -> tuple[pd.DataFrame, pd.DataFrame, pd.DataFrame]:
    """
    Removes cross-domain duplicate records.

    Measurement deduplication key: (paperid, figureid, sampleid, propertyid_y, x_rounded_3sig).
    The `figureid` is the most stable anchor because each measurement originates
    from a digitized figure; the same figure appears verbatim across all three
    domain files that contain it.

    Sample deduplication key: (sampleid, paperid).
    Paper deduplication key: (paperid,).
    """
    log.info("[S2] Deduplicating across domains...")
    n_raw = len(all_meas)

    # Round x to 3 significant figures to collapse digitization floating-point noise
    all_meas["x_key"] = all_meas["x"].round(3)
    dedup_cols = ["paperid", "figureid", "sampleid", "propertyid_y", "x_key"]
    # Keep the first occurrence (samples/ domain is the canonical source)
    priority_order = {"samples": 0, "papers": 1, "figures": 2}
    all_meas["_domain_priority"] = all_meas["source_domain"].map(priority_order).fillna(9)
    all_meas = (
        all_meas
        .sort_values("_domain_priority")
        .drop_duplicates(subset=dedup_cols, keep="first")
        .drop(columns=["x_key", "_domain_priority"])
        .reset_index(drop=True)
    )
    log.info(f"[S2] Measurements: {n_raw:,} raw → {len(all_meas):,} deduplicated ({n_raw - len(all_meas):,} removed)")

    all_samples = (
        all_samples
        .drop_duplicates(subset=["sampleid", "paperid"], keep="first")
        .reset_index(drop=True)
    )
    all_papers = (
        all_papers
        .drop_duplicates(subset=["paperid"], keep="first")
        .reset_index(drop=True)
    )

    return all_meas, all_samples, all_papers


# =============================================================================
# STAGE 3 — SI UNIT NORMALIZATION
# =============================================================================

def stage3_si_normalize(df_meas: pd.DataFrame) -> pd.DataFrame:
    """
    Applies SI unit normalization to raw measurement values.

    The `y_si` column is the analytically clean value used in all downstream
    physics computations. `y_raw` is preserved for provenance and regression testing.

    This function handles the five canonical property IDs present in Starrydata.
    Unrecognized property IDs receive `y_si = NaN` and emit a warning, ensuring
    no silent corruption of the analytical pipeline.
    """
    log.info("[S3] Applying SI unit normalization...")
    df_meas["x_si"] = df_meas["x"]  # x is always Temperature (K) — already SI
    df_meas["y_si"] = np.nan

    for prop_id, (unit_si, factor) in _SI_REGISTRY.items():
        mask = df_meas["propertyid_y"] == prop_id
        df_meas.loc[mask, "y_si"] = df_meas.loc[mask, "y"] * factor

    unhandled = df_meas[df_meas["y_si"].isna()]["propertyid_y"].unique()
    if len(unhandled):
        log.warning(f"[S3] {len(unhandled)} unregistered property IDs — y_si will be NULL: {unhandled[:10]}")

    return df_meas


# =============================================================================
# STAGE 4 — DIMENSIONAL LOADING (DuckDB)
# =============================================================================

DDL = """
-- ============================================================
-- THERMOGNOSIS-X Database Schema v1.0
-- Standard: Nature Scientific Data — Data Records
-- Architecture: Snowflake Dimensional Model (3NF + Fact Star)
-- Engine: DuckDB (OLAP, columnar, embedded)
-- ============================================================

-- 4.1 Property Registry
CREATE TABLE IF NOT EXISTS dim_properties (
    property_id    INTEGER PRIMARY KEY,
    propertyname   VARCHAR NOT NULL,
    unit_raw       VARCHAR,
    unit_si        VARCHAR,
    si_factor      DOUBLE,
    quantity_type  VARCHAR
        CHECK (quantity_type IN (
            'temperature','seebeck','electrical_conductivity',
            'thermal_conductivity','electrical_resistivity',
            'power_factor','figure_of_merit','other'
        ))
);

-- 4.2 Bibliographic Dimension
CREATE TABLE IF NOT EXISTS dim_papers (
    paper_id      INTEGER PRIMARY KEY,
    doi           VARCHAR,
    title         VARCHAR,
    authors       VARCHAR,
    authors_full  VARCHAR,
    journal       VARCHAR,
    journal_full  VARCHAR,
    year          SMALLINT,
    volume        VARCHAR,
    pages         VARCHAR,
    publisher     VARCHAR,
    url           VARCHAR
);

-- 4.3 Material Sample Dimension
-- (sample_id, paper_id) form the composite natural key because sampleid
-- values are paper-scoped integers (not globally unique) in the raw corpus.
CREATE TABLE IF NOT EXISTS dim_samples (
    sample_id            INTEGER,
    paper_id             INTEGER REFERENCES dim_papers(paper_id),
    samplename           VARCHAR,
    composition          VARCHAR,
    material_family      VARCHAR,
    form                 VARCHAR,
    fabrication_process  VARCHAR,
    relative_density     VARCHAR,
    grain_size           VARCHAR,
    data_type            VARCHAR,
    thermal_measurement  VARCHAR,
    electrical_measurement VARCHAR,
    sampleinfo_json      JSON,
    PRIMARY KEY (sample_id, paper_id)
);

-- 4.4 Figure Dimension
CREATE TABLE IF NOT EXISTS dim_figures (
    figure_id      INTEGER,
    paper_id       INTEGER REFERENCES dim_papers(paper_id),
    figurename     VARCHAR,
    caption        VARCHAR,
    property_id_x  INTEGER,
    property_id_y  INTEGER,
    PRIMARY KEY (figure_id, paper_id)
);

-- 4.5 Core Measurement Fact Table
-- Grain: one physical (x,y) digitized point from one figure for one sample.
-- Surrogate key uses ROW_NUMBER() assigned during ingestion for O(1) lookup.
CREATE TABLE IF NOT EXISTS fact_measurements (
    measurement_id  BIGINT PRIMARY KEY,
    paper_id        INTEGER NOT NULL,
    sample_id       INTEGER NOT NULL,
    figure_id       INTEGER NOT NULL,
    property_id_x   INTEGER NOT NULL,
    property_id_y   INTEGER NOT NULL,
    x_raw           DOUBLE NOT NULL,
    y_raw           DOUBLE NOT NULL,
    x_si            DOUBLE,
    y_si            DOUBLE,
    source_domain   VARCHAR NOT NULL
        CHECK (source_domain IN ('samples','papers','figures')),
    source_file     VARCHAR NOT NULL
);

-- 4.6 Thermoelectric State Table (populated by Stage 5+6)
-- Grain: one temperature bin (10 K resolution) for one (sample, paper) pair.
-- This is the primary ML training surface.
CREATE TABLE IF NOT EXISTS thermoelectric_states (
    state_id        BIGINT PRIMARY KEY,
    sample_id       INTEGER NOT NULL,
    paper_id        INTEGER NOT NULL,
    T_bin_K         DOUBLE NOT NULL,
    S_si            DOUBLE,
    sigma_si        DOUBLE,
    kappa_si        DOUBLE,
    rho_si          DOUBLE,
    ZT_reported     DOUBLE,
    ZT_computed     DOUBLE,
    audit_tier      TINYINT,
    anomaly_flags   INTEGER,
    quality_class   TINYINT,
    quality_score   DOUBLE
);

-- Analytical indices for common query patterns
CREATE INDEX IF NOT EXISTS idx_meas_sample   ON fact_measurements(sample_id, paper_id);
CREATE INDEX IF NOT EXISTS idx_meas_prop_y   ON fact_measurements(property_id_y);
CREATE INDEX IF NOT EXISTS idx_states_sample ON thermoelectric_states(sample_id, paper_id);
CREATE INDEX IF NOT EXISTS idx_states_T      ON thermoelectric_states(T_bin_K);
CREATE INDEX IF NOT EXISTS idx_samples_comp  ON dim_samples(composition);
"""

# Canonical property registry derived from observed Starrydata schema
PROPERTY_REGISTRY = [
    (1,  "Temperature",              "K",          "K",         1.0,   "temperature"),
    (2,  "Seebeck coefficient",      "V*K^(-1)",   "V/K",       1.0,   "seebeck"),
    (3,  "Electrical conductivity",  "S*m^(-1)",   "S/m",       1.0,   "electrical_conductivity"),
    (4,  "Thermal conductivity",     "W*m^(-1)*K^(-1)", "W/(m*K)", 1.0, "thermal_conductivity"),
    (5,  "Electrical resistivity",   "ohm*m",      "ohm*m",     1.0,   "electrical_resistivity"),
    (6,  "Power factor",             "W*m^(-1)*K^(-2)", "W/(m*K^2)", 1.0, "power_factor"),
    (7,  "Thermal diffusivity",      "m^2*s^(-1)", "m^2/s",     1.0,   "other"),
    (8,  "Specific heat capacity",   "J*kg^(-1)*K^(-1)", "J/(kg*K)", 1.0, "other"),
    (9,  "Density",                  "kg*m^(-3)",  "kg/m^3",    1.0,   "other"),
    (10, "Hall coefficient",         "m^3*C^(-1)", "m^3/C",     1.0,   "other"),
    (11, "Carrier concentration",    "m^(-3)",     "m^-3",      1.0,   "other"),
    (12, "Carrier mobility",         "m^2*V^(-1)*s^(-1)", "m^2/(V*s)", 1.0, "other"),
    (14, "Lattice thermal conductivity", "W*m^(-1)*K^(-1)", "W/(m*K)", 1.0, "thermal_conductivity"),
    (15, "Figure of merit Z",        "K^(-1)",     "K^-1",      1.0,   "figure_of_merit"),
    (16, "ZT",                       "dimensionless", "1",       1.0,   "figure_of_merit"),
]


def _extract_sampleinfo_field(sampleinfo_json: str, key: str) -> str:
    """Safely extracts `category` from a sampleinfo JSON string by key."""
    try:
        si = json.loads(sampleinfo_json) if sampleinfo_json else {}
        entry = si.get(key, si.get(key.strip(), {}))
        return str(entry.get("category", "")).strip() if isinstance(entry, dict) else ""
    except (json.JSONDecodeError, AttributeError):
        return ""


def stage4_load_duckdb(
    con: duckdb.DuckDBPyConnection,
    df_meas: pd.DataFrame,
    df_samples: pd.DataFrame,
    df_papers: pd.DataFrame,
) -> None:
    """Loads all dimensional and fact data into DuckDB."""
    log.info("[S4] Loading dimensional tables into DuckDB...")

    # 4.1 Property registry (idempotent seed)
    prop_df = pd.DataFrame(
        PROPERTY_REGISTRY,
        columns=["property_id","propertyname","unit_raw","unit_si","si_factor","quantity_type"]
    )
    con.execute("DELETE FROM dim_properties")
    con.register("_prop_df", prop_df)
    con.execute("INSERT INTO dim_properties SELECT * FROM _prop_df")

    # 4.2 Papers
    if not df_papers.empty:
        df_papers = df_papers.rename(columns={
            "paperid": "paper_id", "author": "authors", "author_full": "authors_full",
        })
        for col in ["doi","title","authors","authors_full","journal","journal_full",
                    "volume","pages","publisher","url"]:
            if col not in df_papers.columns:
                df_papers[col] = ""
        if "year" not in df_papers.columns:
            df_papers["year"] = 0
        df_papers = df_papers[["paper_id","doi","title","authors","authors_full",
                                "journal","journal_full","year","volume","pages","publisher","url"]]
        df_papers = df_papers.drop_duplicates("paper_id")
        con.register("_papers_df", df_papers)
        con.execute("INSERT OR IGNORE INTO dim_papers SELECT * FROM _papers_df")
        log.info(f"[S4] Loaded {len(df_papers):,} papers into dim_papers")

    # 4.3 Samples — flatten sampleinfo into canonical columns
    if not df_samples.empty:
        si_json = df_samples.get("sampleinfo_json", pd.Series(["{}"] * len(df_samples)))
        df_samples["material_family"]        = si_json.apply(lambda j: _extract_sampleinfo_field(j, "MaterialFamily"))
        df_samples["form"]                   = si_json.apply(lambda j: _extract_sampleinfo_field(j, "Form"))
        df_samples["fabrication_process"]    = si_json.apply(lambda j: _extract_sampleinfo_field(j, "FabricationProcess"))
        df_samples["relative_density"]       = si_json.apply(lambda j: _extract_sampleinfo_field(j, "RelativeDensity"))
        df_samples["grain_size"]             = si_json.apply(lambda j: _extract_sampleinfo_field(j, "GrainSize"))
        df_samples["data_type"]              = si_json.apply(lambda j: _extract_sampleinfo_field(j, "DataType"))
        df_samples["thermal_measurement"]    = si_json.apply(lambda j: _extract_sampleinfo_field(j, "ThermalMeasurement"))
        df_samples["electrical_measurement"] = si_json.apply(lambda j: _extract_sampleinfo_field(j, "ElectricalMeasurement"))

        df_s = df_samples.rename(columns={"sampleid": "sample_id", "paperid": "paper_id"})
        for col in ["samplename","composition"]:
            if col not in df_s.columns:
                df_s[col] = ""
        df_s = df_s[[
            "sample_id","paper_id","samplename","composition",
            "material_family","form","fabrication_process",
            "relative_density","grain_size","data_type",
            "thermal_measurement","electrical_measurement","sampleinfo_json"
        ]].drop_duplicates(["sample_id","paper_id"])
        con.register("_samples_df", df_s)
        con.execute("INSERT OR IGNORE INTO dim_samples SELECT * FROM _samples_df")
        log.info(f"[S4] Loaded {len(df_s):,} samples into dim_samples")

    # 4.4 Measurements
    if not df_meas.empty:
        df_meas = df_meas.rename(columns={
            "paperid": "paper_id", "sampleid": "sample_id",
            "figureid": "figure_id", "propertyid_x": "property_id_x",
            "propertyid_y": "property_id_y", "x": "x_raw", "y": "y_raw",
        })
        df_meas["measurement_id"] = range(len(df_meas))
        for col in ["x_si", "y_si"]:
            if col not in df_meas.columns:
                df_meas[col] = np.nan

        df_meas = df_meas[[
            "measurement_id","paper_id","sample_id","figure_id",
            "property_id_x","property_id_y","x_raw","y_raw",
            "x_si","y_si","source_domain","source_file"
        ]]
        con.register("_meas_df", df_meas)
        con.execute("INSERT INTO fact_measurements SELECT * FROM _meas_df")
        log.info(f"[S4] Loaded {len(df_meas):,} measurements into fact_measurements")


# =============================================================================
# STAGE 5 — TRIPLE-GATE PHYSICS AUDIT (RUST, SPEC-AUDIT-01)
# =============================================================================

def stage5_physics_audit(con: duckdb.DuckDBPyConnection) -> pd.DataFrame:
    """
    Constructs thermoelectric state vectors from fact_measurements via a
    temperature-binned pivot, then passes them to the Rust Triple-Gate Audit
    (SPEC-AUDIT-01) for per-state tier assignment and anomaly flag bitmask.

    ## Temperature Binning — FLOOR-based (BUG-3 fix)
    The bin assignment is B(T) = FLOOR(T / Δ_T) × Δ_T with Δ_T = 10 K.
    Under FLOOR binning, two temperatures T_i and T_j are co-binned iff both
    lie in the same half-open interval [k·Δ_T, (k+1)·Δ_T).  This eliminates
    the ±5 K boundary-sensitivity artefact of ROUND-based binning, where
    digitization jitter of 0.1–2 K could deterministically split co-measured
    properties into adjacent bins and destroy triplet formation.

    ## Conductivity–Resistivity Duality (BUG-2 fix)
    Most Starrydata papers report ρ (property_id_y = 5), not σ directly
    (property_id_y = 3).  sigma_si is therefore derived as:
        σ = σ_direct     if property_id_y = 3 is present, else
        σ = 1 / ρ        if property_id_y = 5 is present and ρ > 0.

    ## Thermal Conductivity Coverage (BUG-2 extension)
    Total κ (property_id_y = 4) and lattice κ_L (property_id_y = 14) are
    treated as equivalent estimators when only one is available.  When both
    are present the AVG naturally provides their arithmetic mean, which is a
    conservative estimate since κ_total ≥ κ_lattice.
    """
    log.info("[S5] Constructing thermoelectric state vectors for physics audit...")

    # Emit per-property counts before pivoting to diagnose coverage issues
    coverage_query = """
        SELECT property_id_y, COUNT(*) AS n
        FROM fact_measurements
        WHERE y_si IS NOT NULL AND x_si > 0
        GROUP BY property_id_y
        ORDER BY n DESC
    """
    cov = con.execute(coverage_query).df()
    prop_map = {1:"T", 2:"S", 3:"σ", 4:"κ", 5:"ρ", 6:"PF", 14:"κ_L", 15:"Z", 16:"ZT"}
    for _, row in cov.iterrows():
        pid = int(row["property_id_y"])
        log.info(f"[S5] Coverage — prop_id {pid:>3} ({prop_map.get(pid,'?'):>4}): "
                 f"{int(row['n']):>10,} measurements with valid y_si")

    # -------------------------------------------------------------------------
    # FLOOR-based T-binning pivot.
    #
    # sigma_si: prefers direct conductivity (prop 3); falls back to 1/ρ (prop 5).
    # kappa_si: accepts both total κ (prop 4) and lattice κ_L (prop 14).
    #           AVG across both is intentional — for samples reporting only one,
    #           it equals the single available value exactly.
    # rho_si  : prefers direct resistivity (prop 5); falls back to 1/σ (prop 3).
    # ZT_reported: prefers dimensionless ZT (prop 16); derives from Z·T (prop 15).
    # -------------------------------------------------------------------------
    pivot_query = f"""
    SELECT
        sample_id,
        paper_id,
        FLOOR(x_si / {T_BIN_RESOLUTION_K}) * {T_BIN_RESOLUTION_K} AS T_bin_K,

        -- Seebeck coefficient (V/K)
        AVG(CASE WHEN property_id_y = 2 AND y_si IS NOT NULL
                 THEN y_si END)                                    AS S_si,

        -- Electrical conductivity (S/m): direct σ preferred, else 1/ρ
        AVG(CASE WHEN property_id_y = 3 AND y_si IS NOT NULL
                 THEN y_si
                 WHEN property_id_y = 5 AND y_si > 0
                 THEN 1.0 / y_si END)                              AS sigma_si,

        -- Thermal conductivity (W/m·K): total κ or lattice κ_L
        AVG(CASE WHEN property_id_y IN (4, 14) AND y_si IS NOT NULL
                 THEN y_si END)                                    AS kappa_si,

        -- Electrical resistivity (ohm·m): direct ρ preferred, else 1/σ
        AVG(CASE WHEN property_id_y = 5 AND y_si IS NOT NULL
                 THEN y_si
                 WHEN property_id_y = 3 AND y_si > 0
                 THEN 1.0 / y_si END)                              AS rho_si,

        -- Author-reported ZT: dimensionless ZT preferred, derive from Z·T
        AVG(CASE WHEN property_id_y = 16 AND y_si IS NOT NULL
                 THEN y_si
                 WHEN property_id_y = 15 AND y_si IS NOT NULL
                 THEN y_si * x_si END)                             AS ZT_reported

    FROM fact_measurements
    WHERE x_si IS NOT NULL
      AND x_si > 0
    GROUP BY sample_id, paper_id, T_bin_K
    HAVING T_bin_K > 0
    ORDER BY sample_id, paper_id, T_bin_K
    """
    pivot_df = con.execute(pivot_query).df()
    log.info(f"[S5] {len(pivot_df):,} thermoelectric state vectors constructed")

    # Per-column coverage diagnostics
    for col in ["S_si", "sigma_si", "kappa_si", "rho_si", "ZT_reported"]:
        n_valid = pivot_df[col].notna().sum()
        pct = 100 * n_valid / len(pivot_df) if len(pivot_df) else 0
        log.info(f"[S5] {col:<14}: {n_valid:>8,} states non-NULL ({pct:.1f}%)")

    # Identify states with all three properties required for zT computation
    complete_mask = (
        pivot_df["S_si"].notna()     &
        pivot_df["sigma_si"].notna() &
        pivot_df["kappa_si"].notna()
    )
    complete_df = pivot_df[complete_mask].copy()
    log.info(f"[S5] {len(complete_df):,} states have complete (S, σ, κ) triplet")

    if complete_df.empty:
        log.warning("[S5] No complete thermoelectric states found. Skipping audit.")
        pivot_df["ZT_computed"]  = np.nan
        pivot_df["audit_tier"]   = 4
        pivot_df["anomaly_flags"]= 0
        return pivot_df

    # Build contiguous C-order arrays for zero-copy FFI transfer
    S_arr     = np.ascontiguousarray(complete_df["S_si"].values,     dtype=np.float64)
    Sig_arr   = np.ascontiguousarray(complete_df["sigma_si"].values,  dtype=np.float64)
    Kap_arr   = np.ascontiguousarray(complete_df["kappa_si"].values,  dtype=np.float64)
    T_arr     = np.ascontiguousarray(complete_df["T_bin_K"].values,   dtype=np.float64)
    ZTr_arr   = np.ascontiguousarray(
        complete_df["ZT_reported"].fillna(np.nan).values, dtype=np.float64
    )

    log.info(f"[S5] Dispatching {len(complete_df):,} states to Rust Triple-Gate Arbiter...")
    t0 = time.perf_counter()
    audit_result = rust_core.audit_thermodynamics_py(
        S_arr, Sig_arr, Kap_arr, T_arr, ZTr_arr, deterministic=False
    )
    elapsed = time.perf_counter() - t0
    throughput = len(complete_df) / elapsed
    log.info(f"[S5] Audit complete: {throughput:,.0f} states/sec — {elapsed:.3f}s")

    complete_df["ZT_computed"]   = audit_result["zT_computed"]
    complete_df["audit_tier"]    = audit_result["tiers"].astype(np.int8)
    complete_df["anomaly_flags"] = audit_result["anomaly_flags"].astype(np.int32)

    # Merge audit results back onto the full pivot (incomplete states → tier=Reject)
    pivot_df = pivot_df.merge(
        complete_df[["sample_id","paper_id","T_bin_K",
                     "ZT_computed","audit_tier","anomaly_flags"]],
        on=["sample_id","paper_id","T_bin_K"], how="left"
    )
    pivot_df["audit_tier"]    = pivot_df["audit_tier"].fillna(4).astype(np.int8)
    pivot_df["anomaly_flags"] = pivot_df["anomaly_flags"].fillna(0).astype(np.int32)

    # Log audit tier distribution
    tier_dist = pivot_df["audit_tier"].value_counts().sort_index()
    tier_names = {1: "Tier-A", 2: "Tier-B", 3: "Tier-C", 4: "Reject"}
    log.info("[S5] Audit Tier Distribution:")
    for tier, count in tier_dist.items():
        pct = 100 * count / len(pivot_df)
        log.info(f"       {tier_names.get(tier, 'Unknown')}: {count:,} ({pct:.1f}%)")

    return pivot_df


# =============================================================================
# STAGE 6 — THERMOELECTRIC STATE MATERIALIZATION (DuckDB)
# =============================================================================

def stage6_materialize_states(
    con: duckdb.DuckDBPyConnection,
    states_df: pd.DataFrame,
) -> None:
    """
    Persists the audited thermoelectric state table to DuckDB.
    `quality_class` and `quality_score` are populated with NULL here;
    the SPEC-QUAL-SCORING pipeline fills them in a separate pass
    (requires completeness, credibility, and metadata vectors).
    """
    log.info("[S6] Materializing thermoelectric_states table...")
    states_df = states_df.copy()
    states_df.insert(0, "state_id", range(len(states_df)))
    states_df["quality_class"] = np.nan
    states_df["quality_score"] = np.nan

    col_order = [
        "state_id","sample_id","paper_id","T_bin_K",
        "S_si","sigma_si","kappa_si","rho_si","ZT_reported",
        "ZT_computed","audit_tier","anomaly_flags","quality_class","quality_score"
    ]
    for col in col_order:
        if col not in states_df.columns:
            states_df[col] = np.nan
    states_df = states_df[col_order]

    con.register("_states_df", states_df)
    con.execute("INSERT INTO thermoelectric_states SELECT * FROM _states_df")
    log.info(f"[S6] {len(states_df):,} thermoelectric states persisted")


# =============================================================================
# STAGE 7 — PARQUET EXPORT
# =============================================================================

def stage7_export_parquet(con: duckdb.DuckDBPyConnection, parquet_dir: Path) -> None:
    """
    Exports all tables to domain-partitioned Parquet files using ZSTD
    compression (level 9). ZSTD provides the best compression ratio for
    floating-point scientific data while maintaining decompression throughput
    compatible with DuckDB's vectorized engine.
    """
    parquet_dir.mkdir(parents=True, exist_ok=True)
    log.info("[S7] Exporting tables to Parquet...")

    tables = [
        "dim_papers", "dim_samples", "dim_properties",
        "fact_measurements", "thermoelectric_states"
    ]
    for table in tables:
        out_path = parquet_dir / f"{table}.parquet"
        con.execute(f"""
            COPY (SELECT * FROM {table})
            TO '{out_path}'
            (FORMAT PARQUET, COMPRESSION 'ZSTD', COMPRESSION_LEVEL 9)
        """)
        row_count = con.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0]
        file_size_mb = out_path.stat().st_size / 1e6
        log.info(f"[S7] {table}: {row_count:,} rows → {out_path.name} ({file_size_mb:.1f} MB)")


# =============================================================================
# MAIN ORCHESTRATOR
# =============================================================================

def main() -> None:
    t_total = time.perf_counter()
    log.info("=" * 70)
    log.info("  THERMOGNOSIS-X — Starrydata Mirror Ingestion Pipeline")
    log.info("  SPEC-INGEST-PIPELINE-01 | Nature Scientific Data Standard")
    log.info("=" * 70)

    DB_PATH.parent.mkdir(parents=True, exist_ok=True)
    con = duckdb.connect(str(DB_PATH))
    con.execute(DDL)

    # This is a full-rebuild pipeline: truncate all tables before loading so
    # re-runs do not accumulate duplicate rows or generate PK conflicts.
    # Order respects FK dependency: facts before dimensions.
    _TRUNCATE_ORDER = [
        "thermoelectric_states",
        "fact_measurements",
        "dim_figures",
        "dim_samples",
        "dim_papers",
        "dim_properties",
    ]
    for table in _TRUNCATE_ORDER:
        con.execute(f"DELETE FROM {table}")
    log.info(f"[INIT] DuckDB schema initialized and truncated at: {DB_PATH}")

    # Stage 1+2+3: Ingest, deduplicate, normalize across all three domains
    all_meas_parts   = []
    all_sample_parts = []
    all_paper_parts  = []

    for domain in DOMAINS:
        df_meas, df_samples, df_papers = stage1_ingest_domain(MIRROR_ROOT, domain)
        all_meas_parts.append(df_meas)
        all_sample_parts.append(df_samples)
        all_paper_parts.append(df_papers)

    all_meas    = pd.concat(all_meas_parts,   ignore_index=True)
    all_samples = pd.concat(all_sample_parts, ignore_index=True)
    all_papers  = pd.concat(all_paper_parts,  ignore_index=True)

    all_meas, all_samples, all_papers = stage2_deduplicate(all_meas, all_samples, all_papers)
    all_meas = stage3_si_normalize(all_meas)

    # Stage 4: Load into DuckDB
    stage4_load_duckdb(con, all_meas, all_samples, all_papers)

    # Stage 5: Physics audit → thermoelectric states
    states_df = stage5_physics_audit(con)

    # Stage 6: Persist states
    stage6_materialize_states(con, states_df)

    # Stage 7: Export Parquet
    stage7_export_parquet(con, PARQUET_DIR)

    # Final summary
    total_elapsed = time.perf_counter() - t_total
    log.info("=" * 70)
    log.info("  INGESTION COMPLETE")
    log.info(f"  Total elapsed: {total_elapsed:.1f}s")
    for table in ["fact_measurements", "thermoelectric_states", "dim_papers", "dim_samples"]:
        n = con.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0]
        log.info(f"  {table:<28}: {n:>10,} rows")
    log.info("=" * 70)
    con.close()


if __name__ == "__main__":
    main()
