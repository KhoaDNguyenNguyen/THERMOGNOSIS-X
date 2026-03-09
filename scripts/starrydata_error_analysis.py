#!/usr/bin/env python3
# scripts/starrydata_error_analysis.py
#
# SPEC-ERRVAL-01 — Starrydata Corpus Error Characterization
# =========================================================
# Layer    : Python / Technical Validation
# Status   : Normative — Q1 Nature Scientific Data "Technical Validation" Section
# Requires : duckdb, pandas, numpy, matplotlib, seaborn, scipy
#
# Purpose
# -------
# This script constitutes the primary quantitative evidence for the "Technical
# Validation" section of the THERMOGNOSIS-X manuscript. It systematically
# characterizes the three principal failure modes in the raw Starrydata corpus:
#
#   Plot A — The ZT Illusion
#       Internal inconsistency between author-reported ZT and the first-
#       principles recomputed value ZT_comp = S²σT/κ. Tier-C states flagged
#       by FLAG_ZT_MISMATCH (>10% relative deviation) are highlighted.
#
#   Plot B — Wiedemann–Franz Violation in the Semiconductor Regime
#       Histogram of the lattice thermal conductivity κ_L recomputed under
#       a conservative non-degenerate semiconductor Lorenz number L_min =
#       1.48×10⁻⁸ W·Ω·K⁻². States yielding κ_L < 0 even under this
#       minimum-κ_e assumption definitively violate thermodynamic positivity.
#
#   Plot C — Empirical Bounds: Magnitude of Digitization Artifacts
#       Log-scale distribution of raw Seebeck coefficient, electrical
#       conductivity, and thermal conductivity measurements in the un-
#       audited corpus, with engine empirical thresholds superimposed.
#       Exposes the heavy-tailed outlier population that contaminates
#       naive ML training sets.
#
# Physics Constants (canonical — must match rust_core/src/physics.rs)
# --------------------------------------------------------------------
#   L0_SOMMERFELD = 2.44×10⁻⁸ W·Ω·K⁻²   (degenerate Fermi liquid reference)
#   L_ND_MIN      = 1.48×10⁻⁸ W·Ω·K⁻²   (non-degenerate acoustic-phonon lower bound)
#   S_ENGINE_MAX  = 1000  µV/K            (SPEC-AUDIT-01 empirical ceiling)
#   SIGMA_ENGINE_MAX = 1×10⁷ S/m
#   KAPPA_ENGINE_MAX = 100 W/(m·K)
#
# Anomaly Flag Bitmask (SPEC-AUDIT-01)
# ------------------------------------
#   FLAG_NEGATIVE_KAPPA_L  = 0b0001  (bit 0)
#   FLAG_LORENZ_OUT_BOUNDS = 0b0010  (bit 1)
#   FLAG_ZT_MISMATCH       = 0b0100  (bit 2)
#   FLAG_ALGEBRAIC_REJECT  = 0b1000  (bit 3)
#
# Confidence Tier Ordinal Encoding
# ---------------------------------
#   1 = Tier A  — all three gates pass
#   2 = Tier B  — Lorenz anomaly only
#   3 = Tier C  — κ_L < 0  or  |ΔzT/zT| > 10 %
#   4 = Reject  — algebraic bounds violated (T, σ, κ ≤ 0)
#
# Output
# ------
#   figures/plotA_zt_illusion.{pdf,png}
#   figures/plotB_wf_violation.{pdf,png}
#   figures/plotC_empirical_bounds.{pdf,png}
#   figures/error_analysis_report.txt

import logging
import warnings
from pathlib import Path

import duckdb
import matplotlib as mpl
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import numpy as np
import pandas as pd
import seaborn as sns
from scipy import stats

warnings.filterwarnings("ignore", category=FutureWarning)
logging.getLogger("fontTools").setLevel(logging.WARNING)

log = logging.getLogger("starrydata_error_analysis")
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s — %(message)s",
    datefmt="%Y-%m-%dT%H:%M:%S",
)

# =============================================================================
# SECTION 1: PHYSICS CONSTANTS
# Must be kept in strict numerical agreement with rust_core/src/physics.rs.
# Any discrepancy invalidates the cross-language consistency guarantee.
# =============================================================================

# Sommerfeld Lorenz number — degenerate free-electron Fermi liquid reference.
# L₀ = (π²/3)(k_B/e)² = 2.44×10⁻⁸ W·Ω·K⁻²
L0_SOMMERFELD: float = 2.44e-8   # W·Ω·K⁻²

# Non-degenerate semiconductor lower bound for the Lorenz number.
# In the non-degenerate (Maxwell–Boltzmann) limit with purely acoustic-phonon
# scattering (energy-independent relaxation time, r = −1/2), the Lorenz number
# asymptotes to:
#   L_nd = 2(k_B/e)² ≈ 1.48×10⁻⁸ W·Ω·K⁻²
# This value is lower than L₀, yielding a *smaller* κ_e estimate and therefore
# a *larger* κ_L estimate — the most conservative (hardest to violate) bound.
# If κ_L < 0 even under L_nd, the violation is physically unambiguous.
#
# Reference: Goldsmid, H.J. "Introduction to Thermoelectricity" (2010), Ch. 2;
#            May & Snyder in "Materials, Preparation, and Characterization in
#            Thermoelectrics" (2012), p. 11.
L_ND_MIN: float = 1.48e-8        # W·Ω·K⁻²

# Engine empirical ceilings (from physics.rs: SPEC-PHYS-CONSTRAINTS)
S_ENGINE_MAX_UV_K: float     = 1_000.0   # µV/K  (|S| ceiling in SI: 1000e-6 V/K)
SIGMA_ENGINE_MAX: float      = 1e7       # S/m
KAPPA_ENGINE_MAX: float      = 100.0     # W/(m·K)

# Semiconductor "impossible Seebeck" threshold for Plot C outlier illustration.
# Single-band semiconductor models bound |S| < ~800 µV/K in the non-degenerate
# limit; values > 10,000 µV/K are definitively non-physical and indicate either
# unit transcription errors (mV/K → V/K confusion) or figure digitization flaws.
S_IMPOSSIBLE_UV_K: float     = 10_000.0  # µV/K  — "impossible spike" threshold

# Anomaly bitmask constants (must match rust_core/src/audit.rs)
FLAG_NEGATIVE_KAPPA_L : int  = 0b0001
FLAG_LORENZ_OUT_BOUNDS: int  = 0b0010
FLAG_ZT_MISMATCH      : int  = 0b0100
FLAG_ALGEBRAIC_REJECT : int  = 0b1000

# Tier ordinal → display label mapping
TIER_LABELS: dict[int, str] = {1: "Tier A", 2: "Tier B", 3: "Tier C", 4: "Reject"}
TIER_COLORS: dict[int, str] = {
    1: "#2166AC",   # deep blue — physically trustworthy
    2: "#4DAC26",   # green     — Lorenz anomaly, otherwise consistent
    3: "#F4A582",   # salmon    — κ_L < 0 or zT mismatch
    4: "#CA0020",   # crimson   — algebraic violation
}

# =============================================================================
# SECTION 2: PUBLICATION STYLE CONFIGURATION
# Nature Scientific Data mandates a minimum 7 pt font, 1-column width ≈ 88 mm,
# 2-column width ≈ 180 mm, CMYK-compatible colors, and ≥ 300 DPI raster output.
# pdf.fonttype=42 embeds TrueType fonts, satisfying IEEE/Nature PDF submission
# requirements without requiring a full LaTeX installation.
# =============================================================================

mpl.rcParams.update({
    "font.family":          "sans-serif",
    "font.sans-serif":      ["Helvetica Neue", "Arial", "DejaVu Sans"],
    "font.size":            8,
    "axes.titlesize":       9,
    "axes.labelsize":       8,
    "xtick.labelsize":      7,
    "ytick.labelsize":      7,
    "legend.fontsize":      7,
    "axes.linewidth":       0.8,
    "axes.spines.top":      False,
    "axes.spines.right":    False,
    "xtick.direction":      "out",
    "ytick.direction":      "out",
    "xtick.major.width":    0.8,
    "ytick.major.width":    0.8,
    "xtick.minor.visible":  True,
    "xtick.minor.width":    0.5,
    "ytick.minor.width":    0.5,
    "figure.dpi":           150,
    "savefig.dpi":          300,
    "savefig.bbox":         "tight",
    "savefig.pad_inches":   0.02,
    "pdf.fonttype":         42,    # TrueType embedding — Nature/IEEE requirement
    "ps.fonttype":          42,
})

# =============================================================================
# SECTION 3: PATHS AND DATA LOADING
# =============================================================================

REPO_ROOT    = Path(__file__).resolve().parent.parent
DB_PATH      = REPO_ROOT / "data" / "thermognosis.duckdb"
PARQUET_DIR  = REPO_ROOT / "data" / "parquet"
FIGURES_DIR  = REPO_ROOT / "figures"
FIGURES_DIR.mkdir(parents=True, exist_ok=True)

STATES_PARQUET = PARQUET_DIR / "thermoelectric_states.parquet"


def _save(fig: mpl.figure.Figure, name: str) -> None:
    """Persist a figure to both PDF (vector) and PNG (raster) at 300 DPI."""
    for ext in ("pdf", "png"):
        out = FIGURES_DIR / f"{name}.{ext}"
        fig.savefig(out)
        log.info("  saved → %s", out.name)
    plt.close(fig)


def load_states() -> pd.DataFrame:
    """
    Load the thermoelectric_states Parquet produced by Stage 7 of the ETL
    pipeline. Each row represents one T-binned (10 K resolution) state for
    a unique (sample_id, paper_id) pair.

    Required columns
    ----------------
    S_si, sigma_si, kappa_si : float64  — transport coefficients in SI
    T_bin_K                  : float64  — bin-centre temperature (K)
    ZT_reported              : float64  — author-reported figure of merit (NaN if absent)
    ZT_computed              : float64  — engine-recomputed S²σT/κ
    audit_tier               : int8     — ordinal tier (1–4)
    anomaly_flags            : int32    — bitmask (FLAG_* constants)
    """
    if not STATES_PARQUET.exists():
        raise FileNotFoundError(
            f"Parquet not found: {STATES_PARQUET}\n"
            "Run scripts/build_starrydata_duckdb.py first."
        )
    df = pd.read_parquet(STATES_PARQUET)
    log.info(
        "Loaded thermoelectric_states: %d rows, %d columns",
        len(df), df.shape[1],
    )
    return df


def load_raw_measurements(con: duckdb.DuckDBPyConnection) -> pd.DataFrame:
    """
    Query the raw fact_measurements table for all Seebeck (prop 2),
    electrical conductivity (prop 3), and thermal conductivity (prop 4)
    digitized points, *before* any T-binning or physics audit.

    This constitutes the unfiltered corpus against which the engine's
    empirical bounds are validated in Plot C.
    """
    query = """
        SELECT
            property_id_y,
            y_si,
            x_si   AS T_raw_K
        FROM fact_measurements
        WHERE property_id_y IN (2, 3, 4, 5)
          AND y_si IS NOT NULL
          AND y_si != 0
    """
    df = con.execute(query).df()
    log.info("Raw measurements loaded: %d rows", len(df))
    return df


# =============================================================================
# SECTION 4: PLOT A — THE ZT ILLUSION
# =============================================================================

def plot_A_zt_illusion(states: pd.DataFrame) -> None:
    """
    Scatter plot of ZT_reported (author value from source publication) versus
    ZT_computed (engine recomputation: S²σT/κ) for all thermoelectric states
    possessing both quantities.

    The ideal line y = x delineates perfect internal consistency. States
    classified as Tier-C via FLAG_ZT_MISMATCH (>10 % relative deviation)
    are rendered prominently to expose the frequency of miscalculation in
    the raw corpus.

    Physical interpretation
    -----------------------
    A systematic scatter above y = x indicates authors over-reported ZT
    (common when κ is under-estimated from incomplete figure digitization).
    A scatter below y = x indicates authors under-reported ZT or applied
    different unit conventions inconsistently across the same paper.
    Either direction of systematic bias contaminates surrogate model training.
    """
    log.info("[Plot A] Preparing ZT Illusion scatter plot ...")

    # --- Data preparation -------------------------------------------------
    # Restrict to states where both ZT values are finite, positive, and within
    # a physically plausible upper bound for semiconductors (ZT ≤ 10).
    mask_valid = (
        states["ZT_reported"].notna()
        & states["ZT_computed"].notna()
        & (states["ZT_reported"] > 0)
        & (states["ZT_computed"] > 0)
        & (states["ZT_reported"] <= 10)
        & (states["ZT_computed"] <= 10)
    )
    sub = states[mask_valid].copy()
    if len(sub) < 10:
        log.warning("[Plot A] Insufficient data (%d rows) — skipping.", len(sub))
        return
    log.info("[Plot A] %d states with both ZT values.", len(sub))

    # Classify by FLAG_ZT_MISMATCH bit (bit 2, value 4) within the anomaly_flags column
    if "anomaly_flags" in sub.columns:
        mismatch_mask = (sub["anomaly_flags"].fillna(0).astype(int) & FLAG_ZT_MISMATCH) != 0
    elif "audit_tier" in sub.columns:
        # Fallback: approximate via Tier-C classification
        mismatch_mask = sub["audit_tier"] == 3
    else:
        mismatch_mask = pd.Series(False, index=sub.index)

    sub_ok   = sub[~mismatch_mask]   # internally consistent (Tier A/B)
    sub_tier_c = sub[mismatch_mask]  # FLAG_ZT_MISMATCH set (Tier C)

    n_ok     = len(sub_ok)
    n_tierc  = len(sub_tier_c)
    frac_c   = 100 * n_tierc / max(len(sub), 1)

    log.info(
        "[Plot A] Internally consistent: %d  |  ZT-mismatch (Tier-C): %d (%.1f %%)",
        n_ok, n_tierc, frac_c,
    )

    # --- Figure construction ----------------------------------------------
    fig, ax = plt.subplots(figsize=(8.8 / 2.54, 8.0 / 2.54))

    # Ideal consistency line: y = x
    lim_max = min(sub[["ZT_reported", "ZT_computed"]].max().max() * 1.05, 10)
    id_line = np.array([0, lim_max])
    ax.plot(id_line, id_line, color="black", lw=0.8, ls="--",
            label="$y = x$ (ideal)", zorder=5)

    # ±10 % tolerance band — matches the FLAG_ZT_MISMATCH gate threshold
    ax.fill_between(id_line, 0.9 * id_line, 1.1 * id_line,
                    color="black", alpha=0.07, zorder=1,
                    label="±10 % tolerance band")

    # Plot Tier-A/B (consistent) states first, behind
    ax.scatter(
        sub_ok["ZT_reported"], sub_ok["ZT_computed"],
        c=TIER_COLORS[1], s=2.5, alpha=0.35, lw=0, rasterized=True, zorder=2,
        label=f"Tier A/B — consistent ($n$ = {n_ok:,})",
    )

    # Plot Tier-C (mismatch) states on top
    if n_tierc > 0:
        ax.scatter(
            sub_tier_c["ZT_reported"], sub_tier_c["ZT_computed"],
            c=TIER_COLORS[3], s=4, alpha=0.6, lw=0, rasterized=True, zorder=3,
            label=f"Tier C — ZT mismatch ($n$ = {n_tierc:,}, {frac_c:.1f} %)",
        )

    ax.set_xlim(0, lim_max)
    ax.set_ylim(0, lim_max)
    ax.set_aspect("equal", adjustable="box")
    ax.set_xlabel("$ZT_\\mathrm{reported}$ (author value, dimensionless)")
    ax.set_ylabel("$ZT_\\mathrm{computed}$ = $S^2 \\sigma T \\, / \\, \\kappa$ (dimensionless)")
    ax.set_title(
        "Plot A — The ZT Illusion\n"
        "Internal Inconsistency in the Raw Starrydata Corpus"
    )
    ax.legend(frameon=False, markerscale=3, loc="upper left")

    # Annotate mismatch fraction in upper right
    ax.text(
        0.97, 0.04,
        f"Tier-C mismatch fraction: {frac_c:.1f} %\n"
        f"Gate threshold: |ΔZT/ZT| > 10 %",
        transform=ax.transAxes, ha="right", va="bottom",
        fontsize=6.5, color=TIER_COLORS[3],
        bbox=dict(boxstyle="round,pad=0.25", fc="white", ec=TIER_COLORS[3],
                  lw=0.6, alpha=0.9),
    )

    fig.tight_layout()
    _save(fig, "plotA_zt_illusion")


# =============================================================================
# SECTION 5: PLOT B — WIEDEMANN–FRANZ VIOLATION IN THE SEMICONDUCTOR REGIME
# =============================================================================

def plot_B_wiedemann_franz_violation(states: pd.DataFrame) -> None:
    """
    Histogram of the residual lattice thermal conductivity κ_L, computed
    under the non-degenerate semiconductor Lorenz number lower bound:

        κ_L = κ_total − L_nd_min · σ · T

    where L_nd_min = 1.48×10⁻⁸ W·Ω·K⁻² is the non-degenerate acoustic-
    phonon scattering limit (see module-level docstring for derivation).

    This choice of L is deliberately conservative: using the *minimum*
    physically plausible Lorenz number minimises κ_e and hence maximises
    κ_L. If κ_L is still negative under this maximally-favourable assumption,
    no physically realizable Lorenz number can reconcile the reported κ and σ
    — the data definitively violates the Second Law constraint of non-negative
    lattice thermal conductivity.

    For reference, the Sommerfeld reference (L₀ = 2.44×10⁻⁸ W·Ω·K⁻²,
    appropriate for degenerate metals) is plotted alongside to illustrate the
    additional violation population captured at the metallic reference.

    The engine's stored FLAG_NEGATIVE_KAPPA_L (computed at L₀) is also shown
    to cross-validate against the in-script semiconductor re-computation.
    """
    log.info("[Plot B] Preparing Wiedemann–Franz violation histogram ...")

    # --- Data preparation -------------------------------------------------
    # Require finite, physically positive σ, κ, and T
    mask_triplet = (
        states["sigma_si"].notna()
        & states["kappa_si"].notna()
        & states["T_bin_K"].notna()
        & (states["sigma_si"] > 0)
        & (states["kappa_si"] > 0)
        & (states["T_bin_K"] > 0)
    )
    sub = states[mask_triplet].copy()
    if len(sub) < 10:
        log.warning("[Plot B] Insufficient triplet data (%d rows) — skipping.", len(sub))
        return
    log.info("[Plot B] %d states with (σ, κ, T) triplets.", len(sub))

    # Recompute κ_L under both Lorenz values
    sigma = sub["sigma_si"].to_numpy()
    kappa = sub["kappa_si"].to_numpy()
    T     = sub["T_bin_K"].to_numpy()

    kappa_e_L0    = L0_SOMMERFELD * sigma * T   # Sommerfeld (metallic reference)
    kappa_e_Lmin  = L_ND_MIN      * sigma * T   # Non-degenerate semiconductor lower bound

    kappa_L_L0   = kappa - kappa_e_L0    # κ_L at L₀
    kappa_L_Lmin = kappa - kappa_e_Lmin  # κ_L at L_nd_min (conservative — most charitable)

    # Compute violation rates
    n_viol_L0   = int(np.sum(kappa_L_L0   < 0))
    n_viol_Lmin = int(np.sum(kappa_L_Lmin < 0))
    pct_viol_L0   = 100 * n_viol_L0   / len(sub)
    pct_viol_Lmin = 100 * n_viol_Lmin / len(sub)

    log.info(
        "[Plot B] κ_L < 0 at L₀ (Sommerfeld):  %d states (%.2f %%)",
        n_viol_L0, pct_viol_L0,
    )
    log.info(
        "[Plot B] κ_L < 0 at L_nd (conservative): %d states (%.2f %%)",
        n_viol_Lmin, pct_viol_Lmin,
    )

    # Cross-check against engine FLAG_NEGATIVE_KAPPA_L
    if "anomaly_flags" in sub.columns:
        n_engine_flagged = int(
            ((sub["anomaly_flags"].fillna(0).astype(int) & FLAG_NEGATIVE_KAPPA_L) != 0).sum()
        )
        log.info(
            "[Plot B] Engine FLAG_NEGATIVE_KAPPA_L count: %d states (%.2f %%)",
            n_engine_flagged, 100 * n_engine_flagged / len(sub),
        )

    # --- Figure construction ----------------------------------------------
    # Clip for histogram readability: focus on the ±5 W/(m·K) window
    CLIP_LO, CLIP_HI = -5.0, 15.0

    kappa_L_L0_clipped   = np.clip(kappa_L_L0,   CLIP_LO, CLIP_HI)
    kappa_L_Lmin_clipped = np.clip(kappa_L_Lmin, CLIP_LO, CLIP_HI)

    fig, ax = plt.subplots(figsize=(8.8 / 2.54, 7.5 / 2.54))

    bins = np.linspace(CLIP_LO, CLIP_HI, 120)

    # Sommerfeld (L₀) distribution
    ax.hist(
        kappa_L_L0_clipped, bins=bins,
        color="#2166AC", alpha=0.45, density=True, linewidth=0,
        label=(
            f"$L = L_0 = 2.44 \\times 10^{{-8}}$ W Ω K$^{{-2}}$ (Sommerfeld)\n"
            f"Violations: {n_viol_L0:,} / {len(sub):,} ({pct_viol_L0:.1f} %)"
        ),
    )

    # Non-degenerate semiconductor lower bound (L_nd_min) distribution
    ax.hist(
        kappa_L_Lmin_clipped, bins=bins,
        color="#F4A582", alpha=0.6, density=True, linewidth=0,
        label=(
            f"$L = L_\\mathrm{{nd,min}} = 1.48 \\times 10^{{-8}}$ W Ω K$^{{-2}}$ "
            f"(non-deg. semiconductor)\n"
            f"Definitive violations: {n_viol_Lmin:,} / {len(sub):,} "
            f"({pct_viol_Lmin:.1f} %)"
        ),
    )

    # Thermodynamic positivity boundary
    ax.axvline(0, color="black", lw=1.0, ls="--", zorder=5,
               label="$\\kappa_L = 0$ (thermodynamic boundary)")

    # Shade the non-physical region
    ax.axvspan(CLIP_LO, 0, color="#CA0020", alpha=0.07, zorder=0,
               label="Non-physical region ($\\kappa_L < 0$)")

    ax.set_xlim(CLIP_LO, CLIP_HI)
    ax.set_xlabel(
        "$\\kappa_L = \\kappa_\\mathrm{total} - L \\cdot \\sigma \\cdot T$"
        "  (W m$^{-1}$ K$^{-1}$)"
    )
    ax.set_ylabel("Probability density  (a.u.)")
    ax.set_title(
        "Plot B — Wiedemann–Franz Violation (Semiconductor Regime)\n"
        "$\\kappa_L < 0$ under conservative non-degenerate $L_\\mathrm{min}$ bound"
    )
    ax.legend(frameon=False, loc="upper right", fontsize=6)

    # Annotate: conservative bound explanation
    ax.text(
        0.02, 0.97,
        "Conservative bound: $L_\\mathrm{nd,min}$ minimises $\\kappa_e$,\n"
        "maximising $\\kappa_L$. Violations are thermodynamically definitive.",
        transform=ax.transAxes, va="top", ha="left", fontsize=6,
        bbox=dict(boxstyle="round,pad=0.3", fc="white", ec="grey",
                  lw=0.5, alpha=0.9),
    )

    fig.tight_layout()
    _save(fig, "plotB_wf_violation")

    return {
        "n_total":        len(sub),
        "n_viol_L0":      n_viol_L0,
        "pct_viol_L0":    pct_viol_L0,
        "n_viol_Lmin":    n_viol_Lmin,
        "pct_viol_Lmin":  pct_viol_Lmin,
    }


# =============================================================================
# SECTION 6: PLOT C — DIGITIZATION ARTIFACTS AND EMPIRICAL BOUNDS
# =============================================================================

def plot_C_empirical_bounds(con: duckdb.DuckDBPyConnection) -> None:
    """
    Three-panel log-scale distribution of raw digitized values for Seebeck
    coefficient, electrical conductivity, and thermal conductivity — before
    any physics audit — with engine empirical threshold lines superimposed.

    The heavy-tailed outlier population visible beyond each threshold line
    constitutes the class of digitization artifacts, unit-confusion errors,
    and figure-extraction failures that contaminate naïve ML training sets.

    Physical context for each property
    -----------------------------------
    |S| > 10,000 µV/K:
        Unphysical for any crystalline semiconductor. Single-band non-
        degenerate theory bounds |S| ≲ 800 µV/K. Values at 10⁴ µV/K
        indicate mV/K → V/K confusion in source extraction, or an off-by-
        10³ scale error in the original digitization software output.

    σ > 10⁷ S/m:
        Exceeds the conductivity of copper (~6×10⁷ S/m) — implausible for
        a thermoelectric semiconductor, where phonon scattering limits σ
        to ≲ 10⁵ S/m in the operating range. Values above 10⁷ S/m indicate
        either unit confusion (S/cm → S/m × 10² factor error) or non-
        thermoelectric metallic samples mis-labelled in Starrydata.

    κ > 100 W/(m·K):
        Exceeds diamond at room temperature (~2000 W/(m·K) excluded by scope).
        For oxide and chalcogenide thermoelectrics, κ > 100 W/(m·K) is
        physically inaccessible. Such values again imply unit transcription
        errors (W/(cm·K) → W/(m·K) ×100 factor error).
    """
    log.info("[Plot C] Querying raw measurements for empirical bound analysis ...")

    # --- Query raw Seebeck measurements (property_id_y = 2) ---------------
    q_seebeck = """
        SELECT ABS(y_si) * 1e6 AS S_abs_uVK   -- convert V/K → µV/K
        FROM fact_measurements
        WHERE property_id_y = 2
          AND y_si IS NOT NULL
          AND ABS(y_si) > 1e-9               -- exclude numerical zero noise
    """

    # --- Query raw electrical conductivity (property_id_y = 3) -----------
    # Also include 1/ρ from resistivity (property_id_y = 5) for completeness
    q_sigma = """
        SELECT y_si AS sigma_Sm
        FROM fact_measurements
        WHERE property_id_y = 3
          AND y_si IS NOT NULL
          AND y_si > 0
        UNION ALL
        SELECT (1.0 / y_si) AS sigma_Sm
        FROM fact_measurements
        WHERE property_id_y = 5
          AND y_si IS NOT NULL
          AND y_si > 0
    """

    # --- Query raw thermal conductivity (property_id_y = 4) ---------------
    q_kappa = """
        SELECT y_si AS kappa_WmK
        FROM fact_measurements
        WHERE property_id_y = 4
          AND y_si IS NOT NULL
          AND y_si > 0
    """

    if not DB_PATH.exists():
        log.error(
            "[Plot C] DuckDB not found at %s — skipping raw measurement analysis.", DB_PATH
        )
        return

    df_s  = con.execute(q_seebeck).df()
    df_sg = con.execute(q_sigma).df()
    df_k  = con.execute(q_kappa).df()

    log.info(
        "[Plot C] Raw measurements retrieved: S=%d, σ=%d, κ=%d",
        len(df_s), len(df_sg), len(df_k),
    )

    # --- Statistics for report annotation --------------------------------
    n_S_impossible  = int((df_s["S_abs_uVK"]  > S_IMPOSSIBLE_UV_K).sum())
    n_S_engine_ceil = int((df_s["S_abs_uVK"]  > S_ENGINE_MAX_UV_K).sum())
    n_sg_exceed     = int((df_sg["sigma_Sm"]   > SIGMA_ENGINE_MAX).sum())
    n_k_exceed      = int((df_k["kappa_WmK"]   > KAPPA_ENGINE_MAX).sum())

    log.info(
        "[Plot C] |S| > %.0f µV/K: %d (%.3f %%)",
        S_IMPOSSIBLE_UV_K, n_S_impossible,
        100 * n_S_impossible / max(len(df_s), 1),
    )
    log.info(
        "[Plot C] |S| > %.0f µV/K (engine ceiling): %d (%.3f %%)",
        S_ENGINE_MAX_UV_K, n_S_engine_ceil,
        100 * n_S_engine_ceil / max(len(df_s), 1),
    )
    log.info(
        "[Plot C] σ > %.0e S/m: %d (%.3f %%)",
        SIGMA_ENGINE_MAX, n_sg_exceed,
        100 * n_sg_exceed / max(len(df_sg), 1),
    )
    log.info(
        "[Plot C] κ > %.0f W/(m·K): %d (%.3f %%)",
        KAPPA_ENGINE_MAX, n_k_exceed,
        100 * n_k_exceed / max(len(df_k), 1),
    )

    # --- Figure construction: 3-panel horizontal layout ------------------
    fig, axes = plt.subplots(
        1, 3, figsize=(18.0 / 2.54, 7.0 / 2.54),
        gridspec_kw={"wspace": 0.38},
    )

    # --- Panel 1: Seebeck coefficient |S| (µV/K) in log x-scale ----------
    ax = axes[0]
    S_data = df_s["S_abs_uVK"].clip(lower=0.01)
    S_log_data = np.log10(S_data[S_data > 0])

    bins_s = np.linspace(-2, 5, 100)  # log10 space: 0.01 µV/K → 100,000 µV/K
    ax.hist(S_log_data, bins=bins_s, color="#4393C3", alpha=0.7,
            density=True, linewidth=0)

    # Engine empirical ceiling: 1000 µV/K
    ax.axvline(np.log10(S_ENGINE_MAX_UV_K), color="#F4A582", lw=1.0, ls="--",
               label=f"Engine limit: {S_ENGINE_MAX_UV_K:.0f} µV/K")
    # Impossible spike threshold: 10000 µV/K
    ax.axvline(np.log10(S_IMPOSSIBLE_UV_K), color="#CA0020", lw=1.0, ls="-.",
               label=f"Impossible: {S_IMPOSSIBLE_UV_K:.0f} µV/K")

    ax.set_xlabel(r"$|S|$ (µV K$^{-1}$)")
    ax.set_ylabel("Probability density  (a.u.)")
    ax.set_title(r"Seebeck Coefficient $|S|$", fontsize=8)

    # Replace log10 x-axis with formatted tick labels
    _set_log10_xaxis(ax, lo=-2, hi=5,
                     ticks=[0, 1, 2, 3, 4, 5],
                     labels=["1", "10", "100", "1000", "10⁴", "10⁵"])

    # Annotate outlier fraction
    ax.text(
        0.97, 0.97,
        f"$n$ = {len(df_s):,}\n"
        f"> {S_ENGINE_MAX_UV_K:.0f} µV/K: {n_S_engine_ceil:,} "
        f"({100*n_S_engine_ceil/max(len(df_s),1):.2f} %)\n"
        f"> {S_IMPOSSIBLE_UV_K:.0f} µV/K: {n_S_impossible:,} "
        f"({100*n_S_impossible/max(len(df_s),1):.2f} %)",
        transform=ax.transAxes, ha="right", va="top", fontsize=5.5,
        bbox=dict(boxstyle="round,pad=0.25", fc="white", ec="grey",
                  lw=0.5, alpha=0.9),
    )
    ax.legend(frameon=False, fontsize=5.5, loc="upper left")

    # --- Panel 2: Electrical conductivity σ (S/m) -------------------------
    ax = axes[1]
    sg_data = df_sg["sigma_Sm"].clip(lower=1)
    sg_log  = np.log10(sg_data[sg_data > 0])

    bins_sg = np.linspace(0, 10, 100)   # 1 S/m → 10¹⁰ S/m
    ax.hist(sg_log, bins=bins_sg, color="#4DAC26", alpha=0.7,
            density=True, linewidth=0)

    ax.axvline(np.log10(SIGMA_ENGINE_MAX), color="#CA0020", lw=1.0, ls="--",
               label=f"Engine limit: $10^{{{int(np.log10(SIGMA_ENGINE_MAX))}}}$ S/m")

    ax.set_xlabel(r"$\sigma$ (S m$^{-1}$)")
    ax.set_title(r"Electrical Conductivity $\sigma$", fontsize=8)
    _set_log10_xaxis(ax, lo=0, hi=10,
                     ticks=[0, 2, 4, 6, 8, 10],
                     labels=["1", "10²", "10⁴", "10⁶", "10⁸", "10¹⁰"])

    ax.text(
        0.97, 0.97,
        f"$n$ = {len(df_sg):,}\n"
        f"> $10^{{{int(np.log10(SIGMA_ENGINE_MAX))}}}$ S/m: {n_sg_exceed:,} "
        f"({100*n_sg_exceed/max(len(df_sg),1):.2f} %)",
        transform=ax.transAxes, ha="right", va="top", fontsize=5.5,
        bbox=dict(boxstyle="round,pad=0.25", fc="white", ec="grey",
                  lw=0.5, alpha=0.9),
    )
    ax.legend(frameon=False, fontsize=5.5, loc="upper right")

    # --- Panel 3: Thermal conductivity κ [W/(m·K)] ------------------------
    ax = axes[2]
    k_data = df_k["kappa_WmK"].clip(lower=1e-4)
    k_log  = np.log10(k_data[k_data > 0])

    bins_k = np.linspace(-4, 4, 100)   # 0.0001 → 10,000 W/(m·K)
    ax.hist(k_log, bins=bins_k, color="#762A83", alpha=0.7,
            density=True, linewidth=0)

    ax.axvline(np.log10(KAPPA_ENGINE_MAX), color="#CA0020", lw=1.0, ls="--",
               label=f"Engine limit: {KAPPA_ENGINE_MAX:.0f} W/(m·K)")

    ax.set_xlabel(r"$\kappa$ (W m$^{-1}$ K$^{-1}$)")
    ax.set_title(r"Thermal Conductivity $\kappa$", fontsize=8)
    _set_log10_xaxis(ax, lo=-4, hi=4,
                     ticks=[-4, -2, 0, 2, 4],
                     labels=["10⁻⁴", "10⁻²", "1", "10²", "10⁴"])

    ax.text(
        0.97, 0.97,
        f"$n$ = {len(df_k):,}\n"
        f"> {KAPPA_ENGINE_MAX:.0f} W/(m·K): {n_k_exceed:,} "
        f"({100*n_k_exceed/max(len(df_k),1):.2f} %)",
        transform=ax.transAxes, ha="right", va="top", fontsize=5.5,
        bbox=dict(boxstyle="round,pad=0.25", fc="white", ec="grey",
                  lw=0.5, alpha=0.9),
    )
    ax.legend(frameon=False, fontsize=5.5, loc="upper right")

    # --- Shared super-title -----------------------------------------------
    fig.suptitle(
        "Plot C — Digitization Artifacts: Raw Corpus Transport Property Distributions\n"
        "(log-scale; red dashed lines = engine empirical bounds from SPEC-PHYS-CONSTRAINTS)",
        fontsize=8, y=1.01,
    )

    fig.tight_layout()
    _save(fig, "plotC_empirical_bounds")

    return {
        "n_S":             len(df_s),
        "n_S_impossible":  n_S_impossible,
        "n_S_engine_ceil": n_S_engine_ceil,
        "n_sigma":         len(df_sg),
        "n_sigma_exceed":  n_sg_exceed,
        "n_kappa":         len(df_k),
        "n_kappa_exceed":  n_k_exceed,
    }


def _set_log10_xaxis(
    ax: mpl.axes.Axes,
    lo: float,
    hi: float,
    ticks: list,
    labels: list,
) -> None:
    """
    Configure a log10-scale x-axis on an axis whose data is already in log10
    form (i.e., the histogram was computed on log10(values)), replacing
    numeric tick positions with human-readable SI notation.
    """
    ax.set_xlim(lo, hi)
    ax.set_xticks(ticks)
    ax.set_xticklabels(labels, rotation=30, ha="right")


# =============================================================================
# SECTION 7: SUMMARY REPORT
# =============================================================================

def write_error_report(
    states: pd.DataFrame,
    wf_stats: dict | None,
    bound_stats: dict | None,
) -> None:
    """
    Write a structured plain-text summary of the corpus error characterization
    to figures/error_analysis_report.txt for inclusion as Supplementary Data
    in the Nature Scientific Data submission.
    """
    n_total   = len(states)
    tier_dist = (
        states["audit_tier"].value_counts().sort_index()
        if "audit_tier" in states.columns
        else {}
    )

    # ZT-mismatch statistics
    if "anomaly_flags" in states.columns:
        n_zt_mismatch = int(
            ((states["anomaly_flags"].fillna(0).astype(int) & FLAG_ZT_MISMATCH) != 0).sum()
        )
        n_neg_kappa_l = int(
            ((states["anomaly_flags"].fillna(0).astype(int) & FLAG_NEGATIVE_KAPPA_L) != 0).sum()
        )
        n_algebraic   = int(
            ((states["anomaly_flags"].fillna(0).astype(int) & FLAG_ALGEBRAIC_REJECT) != 0).sum()
        )
    else:
        n_zt_mismatch = n_neg_kappa_l = n_algebraic = -1

    lines = [
        "=" * 72,
        "THERMOGNOSIS-X  —  Starrydata Corpus Error Analysis Report",
        "SPEC-ERRVAL-01  |  Nature Scientific Data — Technical Validation",
        "=" * 72,
        "",
        f"Total thermoelectric states analysed:  {n_total:>12,}",
        "",
        "Confidence Tier Distribution (Triple-Gate Physics Arbiter):",
    ]
    for tier in [1, 2, 3, 4]:
        cnt = int(tier_dist.get(tier, 0))
        pct = 100 * cnt / max(n_total, 1)
        label = TIER_LABELS.get(tier, f"Tier {tier}")
        lines.append(f"  {label}:  {cnt:>10,}  ({pct:6.2f} %)")

    lines += [
        "",
        "Anomaly Flag Breakdown (bitmask, non-exclusive):",
        f"  FLAG_ALGEBRAIC_REJECT   (Gate 1):  {n_algebraic:>10,}  "
        f"({100*n_algebraic/max(n_total,1):.2f} %)",
        f"  FLAG_NEGATIVE_KAPPA_L   (Gate 2):  {n_neg_kappa_l:>10,}  "
        f"({100*n_neg_kappa_l/max(n_total,1):.2f} %)",
        f"  FLAG_ZT_MISMATCH        (Gate 3):  {n_zt_mismatch:>10,}  "
        f"({100*n_zt_mismatch/max(n_total,1):.2f} %)",
        "",
    ]

    if wf_stats:
        lines += [
            "Wiedemann–Franz Analysis (Plot B):",
            f"  States with (σ, κ, T) triplet:    {wf_stats['n_total']:>10,}",
            f"  κ_L < 0 at L₀ (Sommerfeld):       {wf_stats['n_viol_L0']:>10,}  "
            f"({wf_stats['pct_viol_L0']:.2f} %)",
            f"  κ_L < 0 at L_nd_min (definitive): {wf_stats['n_viol_Lmin']:>10,}  "
            f"({wf_stats['pct_viol_Lmin']:.2f} %)",
            f"  L₀ = {L0_SOMMERFELD:.2e} W·Ω·K⁻² (Sommerfeld reference)",
            f"  L_nd_min = {L_ND_MIN:.2e} W·Ω·K⁻² (non-degenerate semiconductor lower bound)",
            "",
        ]

    if bound_stats:
        lines += [
            "Empirical Bound Violations in Raw Corpus (Plot C):",
            f"  |S| > {S_ENGINE_MAX_UV_K:.0f} µV/K (engine ceiling):  "
            f"{bound_stats['n_S_engine_ceil']:>10,} / {bound_stats['n_S']:,}  "
            f"({100*bound_stats['n_S_engine_ceil']/max(bound_stats['n_S'],1):.3f} %)",
            f"  |S| > {S_IMPOSSIBLE_UV_K:.0f} µV/K (impossible spike): "
            f"{bound_stats['n_S_impossible']:>10,} / {bound_stats['n_S']:,}  "
            f"({100*bound_stats['n_S_impossible']/max(bound_stats['n_S'],1):.3f} %)",
            f"  σ > {SIGMA_ENGINE_MAX:.0e} S/m (engine ceiling):       "
            f"{bound_stats['n_sigma_exceed']:>10,} / {bound_stats['n_sigma']:,}  "
            f"({100*bound_stats['n_sigma_exceed']/max(bound_stats['n_sigma'],1):.3f} %)",
            f"  κ > {KAPPA_ENGINE_MAX:.0f} W/(m·K) (engine ceiling):        "
            f"{bound_stats['n_kappa_exceed']:>10,} / {bound_stats['n_kappa']:,}  "
            f"({100*bound_stats['n_kappa_exceed']/max(bound_stats['n_kappa'],1):.3f} %)",
            "",
        ]

    lines += ["=" * 72, ""]
    report = "\n".join(lines)

    out = FIGURES_DIR / "error_analysis_report.txt"
    out.write_text(report, encoding="utf-8")
    log.info("Report saved → %s", out.name)
    print(report)


# =============================================================================
# SECTION 8: MAIN ENTRY POINT
# =============================================================================

def main() -> None:
    log.info("=== THERMOGNOSIS-X  Starrydata Error Analysis (SPEC-ERRVAL-01) ===")

    # Load thermoelectric states (post-audit Parquet)
    states = load_states()

    # Connect to DuckDB for raw measurement access (Plot C)
    con = None
    if DB_PATH.exists():
        con = duckdb.connect(str(DB_PATH), read_only=True)
        log.info("Connected to DuckDB: %s", DB_PATH)
    else:
        log.warning(
            "DuckDB not found at %s — Plot C will be skipped. "
            "Run build_starrydata_duckdb.py first.", DB_PATH
        )

    # -------------------------------------------------------------------
    log.info("=== Generating Plot A: The ZT Illusion ===")
    plot_A_zt_illusion(states)

    # -------------------------------------------------------------------
    log.info("=== Generating Plot B: Wiedemann–Franz Violation ===")
    wf_stats = plot_B_wiedemann_franz_violation(states)

    # -------------------------------------------------------------------
    log.info("=== Generating Plot C: Empirical Bounds and Digitization Artifacts ===")
    bound_stats = None
    if con is not None:
        bound_stats = plot_C_empirical_bounds(con)
        con.close()
    else:
        log.warning("[Plot C] Skipped — DuckDB unavailable.")

    # -------------------------------------------------------------------
    log.info("=== Writing summary report ===")
    write_error_report(states, wf_stats, bound_stats)

    log.info(
        "=== Error analysis complete. All outputs in %s ===", FIGURES_DIR
    )


if __name__ == "__main__":
    main()
