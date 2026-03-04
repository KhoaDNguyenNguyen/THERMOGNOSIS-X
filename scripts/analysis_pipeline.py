#!/usr/bin/env python3
# scripts/analysis_pipeline.py
#
# SPEC-ANALYSIS-01 — Golden Triplet Analysis Pipeline
# ====================================================
# Layer    : Python / Statistical Analysis
# Status   : Normative — Q1 Nature Scientific Data Supplementary Analysis
# Target   : 66,468 Golden Triplets (S, σ, κ all non-null) from thermoelectric_states
#
# Outputs (saved to figures/):
#   figA — zT distribution (KDE + histogram)
#   figB — Property correlation heatmap (S, σ, κ, T, zT)
#   figC — zT vs T scatter with waste-heat recovery zone shaded (323K–773K)
#
# Executive summary saved to: figures/golden_triplet_report.txt

import logging
import warnings
from pathlib import Path

import matplotlib as mpl
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import numpy as np
import pandas as pd
import seaborn as sns
from scipy import stats

warnings.filterwarnings("ignore", category=FutureWarning)
logging.getLogger("fontTools").setLevel(logging.WARNING)

log = logging.getLogger("analysis_pipeline")
logging.basicConfig(level=logging.INFO, format="%(levelname)s  %(message)s")

# =============================================================================
# PUBLICATION STYLE — mirrors statistical_analysis.py settings
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
    "xtick.major.width":    0.8,
    "ytick.major.width":    0.8,
    "figure.dpi":           150,
    "savefig.dpi":          300,
    "savefig.bbox":         "tight",
    "savefig.pad_inches":   0.02,
    "pdf.fonttype":         42,
    "ps.fonttype":          42,
})

# Audit-tier colormap (Tier 1=A best → Tier 4=Reject)
TIER_COLORS = {1: "#2166AC", 2: "#4DAC26", 3: "#F4A582", 4: "#CA0020"}
TIER_LABELS = {1: "Tier A", 2: "Tier B", 3: "Tier C", 4: "Reject"}

# Waste-heat recovery window
WHR_T_MIN = 323.0   # K  (~50 °C)
WHR_T_MAX = 773.0   # K  (~500 °C)
ZT_MONEY_MAKER = 0.8

# =============================================================================
# PATHS
# =============================================================================
REPO_ROOT  = Path(__file__).resolve().parent.parent
PARQUET_DIR = REPO_ROOT / "data" / "parquet"
FIGURES_DIR = REPO_ROOT / "figures"
FIGURES_DIR.mkdir(parents=True, exist_ok=True)


def _save(fig: plt.Figure, name: str) -> None:
    for ext in ("pdf", "png"):
        p = FIGURES_DIR / f"{name}.{ext}"
        fig.savefig(p)
        log.info("  saved → %s", p.name)
    plt.close(fig)


# =============================================================================
# DATA LOADING
# =============================================================================

def load_golden_triplets() -> pd.DataFrame:
    """
    Load thermoelectric_states.parquet and isolate Golden Triplets:
    rows where S_si, sigma_si, and kappa_si are all non-null.
    Also computes zT explicitly as S²σT/κ for cross-validation.
    """
    parquet_path = PARQUET_DIR / "thermoelectric_states.parquet"
    if not parquet_path.exists():
        raise FileNotFoundError(
            f"Parquet not found: {parquet_path}\n"
            "Run scripts/build_starrydata_duckdb.py first."
        )

    df = pd.read_parquet(parquet_path)
    log.info("Loaded thermoelectric_states: %d rows", len(df))

    # Golden Triplet mask
    mask = df["S_si"].notna() & df["sigma_si"].notna() & df["kappa_si"].notna()
    gt = df[mask].copy()
    log.info("Golden Triplets (S, σ, κ all non-null): %d", len(gt))

    # Explicit zT = S² σ T / κ  (formula cross-check)
    gt["zT_computed"] = (
        gt["S_si"] ** 2 * gt["sigma_si"] * gt["T_bin_K"] / gt["kappa_si"]
    )

    # Use stored ZT if available; fall back to computed
    if "ZT_si" in gt.columns:
        gt["zT"] = gt["ZT_si"].where(gt["ZT_si"].notna(), gt["zT_computed"])
    else:
        gt["zT"] = gt["zT_computed"]

    # Physical bounds filter: discard non-physical extremes
    gt = gt[(gt["zT_computed"] >= 0) & (gt["zT_computed"] <= 20)]
    log.info("After physical-bounds filter (0 ≤ zT ≤ 20): %d", len(gt))

    return gt


# =============================================================================
# EXECUTIVE SUMMARY
# =============================================================================

def compute_summary(gt: pd.DataFrame, all_states: pd.DataFrame) -> dict:
    whr = gt[(gt["T_bin_K"] >= WHR_T_MIN) & (gt["T_bin_K"] <= WHR_T_MAX)]
    money_makers = whr[whr["zT_computed"] >= ZT_MONEY_MAKER]

    tier_dist = gt["audit_tier"].value_counts().sort_index() if "audit_tier" in gt.columns else {}

    return {
        "total_states":          len(all_states),
        "golden_triplets":       len(gt),
        "triplet_fraction_pct":  100.0 * len(gt) / len(all_states),
        "whr_triplets":          len(whr),
        "money_makers":          len(money_makers),
        "money_maker_pct_whr":   100.0 * len(money_makers) / max(len(whr), 1),
        "zT_median":             gt["zT_computed"].median(),
        "zT_p95":                gt["zT_computed"].quantile(0.95),
        "zT_max":                gt["zT_computed"].max(),
        "T_median_K":            gt["T_bin_K"].median(),
        "tier_dist":             tier_dist,
        "formula_residual_mae":  (
            (gt["zT_computed"] - gt["ZT_si"]).abs().mean()
            if "ZT_si" in gt.columns else float("nan")
        ),
    }


def write_report(summary: dict, path: Path) -> None:
    lines = [
        "=" * 70,
        "THERMOGNOSIS-X  —  Golden Triplet Analysis Report",
        "SPEC-ANALYSIS-01 | Target: Q1 Nature Scientific Data",
        "=" * 70,
        "",
        f"Total thermoelectric states (all tiers):  {summary['total_states']:>12,}",
        f"Golden Triplets (S, σ, κ non-null):       {summary['golden_triplets']:>12,}",
        f"  Triplet yield:                          {summary['triplet_fraction_pct']:>11.2f} %",
        "",
        f"Waste-heat recovery window ({WHR_T_MIN:.0f}–{WHR_T_MAX:.0f} K):",
        f"  States in window:                       {summary['whr_triplets']:>12,}",
        f"  'Money-maker' (zT ≥ {ZT_MONEY_MAKER:.1f}):              "
        f"{summary['money_makers']:>12,}",
        f"  Money-maker fraction of window:         {summary['money_maker_pct_whr']:>11.2f} %",
        "",
        "zT statistics (physical-bounds filtered):",
        f"  Median zT:                              {summary['zT_median']:>12.4f}",
        f"  95th percentile zT:                     {summary['zT_p95']:>12.4f}",
        f"  Maximum zT:                             {summary['zT_max']:>12.4f}",
        f"  Median T (K):                           {summary['T_median_K']:>12.1f}",
        "",
        "Formula cross-validation (S²σT/κ vs stored ZT):",
        f"  MAE:                                    {summary['formula_residual_mae']:>12.6f}",
        "",
    ]

    if summary["tier_dist"] is not None and len(summary["tier_dist"]) > 0:
        lines.append("Audit tier distribution (Golden Triplets):")
        for tier, count in summary["tier_dist"].items():
            label = TIER_LABELS.get(tier, f"Tier {tier}")
            lines.append(f"  {label}:  {count:>10,}  ({100*count/summary['golden_triplets']:.1f} %)")
        lines.append("")

    lines += ["=" * 70, ""]
    report_text = "\n".join(lines)
    path.write_text(report_text, encoding="utf-8")
    log.info("Report saved → %s", path.name)
    print(report_text)


# =============================================================================
# FIGURE A — zT Distribution (KDE + Histogram)
# =============================================================================

def fig_A_zt_distribution(gt: pd.DataFrame) -> None:
    """
    Histogram + KDE of the zT distribution across all Golden Triplets.
    Vertical lines mark median, p95, and the money-maker threshold.
    """
    zT = gt["zT_computed"].clip(upper=6)  # clip extreme outliers for readability

    fig, ax = plt.subplots(figsize=(8.8 / 2.54, 7 / 2.54))
    ax.hist(zT, bins=120, density=True, color="#4393C3", alpha=0.45,
            linewidth=0, label="Histogram")

    kde_x = np.linspace(0, zT.max(), 500)
    kde = stats.gaussian_kde(zT.dropna(), bw_method="scott")
    ax.plot(kde_x, kde(kde_x), color="#2166AC", lw=1.2, label="KDE")

    med = zT.median()
    p95 = zT.quantile(0.95)
    ax.axvline(med, color="#4DAC26", lw=0.9, ls="--", label=f"Median ({med:.3f})")
    ax.axvline(p95, color="#F4A582", lw=0.9, ls=":",  label=f"95th pct ({p95:.3f})")
    ax.axvline(ZT_MONEY_MAKER, color="#CA0020", lw=1.0, ls="-.",
               label=f"Money-maker (zT={ZT_MONEY_MAKER})")

    ax.set_xlabel("Figure of merit $zT$ (dimensionless)")
    ax.set_ylabel("Probability density")
    ax.set_title(f"Fig. A — $zT$ Distribution\n({len(gt):,} Golden Triplets)")
    ax.legend(frameon=False)
    fig.tight_layout()
    _save(fig, "figA_zt_distribution")


# =============================================================================
# FIGURE B — Property Correlation Heatmap
# =============================================================================

def fig_B_correlation_heatmap(gt: pd.DataFrame) -> None:
    """
    Pearson correlation heatmap of (S, σ, κ, T, zT).
    Lower triangle only; cells with |r| > 0.7 and p < 0.01 bolded.
    """
    cols = {
        "S_si":        "$S$ (V K⁻¹)",
        "sigma_si":    "$\\sigma$ (S m⁻¹)",
        "kappa_si":    "$\\kappa$ (W m⁻¹K⁻¹)",
        "T_bin_K":     "$T$ (K)",
        "zT_computed": "$zT$",
    }
    sub = gt[list(cols.keys())].dropna()
    if len(sub) < 30:
        log.warning("Insufficient data for Fig B — skipping")
        return

    corr = sub.rename(columns=cols).corr(method="pearson")
    pval = pd.DataFrame(
        [[stats.pearsonr(sub[c1], sub[c2])[1] for c2 in cols] for c1 in cols],
        index=list(cols.values()), columns=list(cols.values()),
    )

    n = len(corr)
    mask = np.triu(np.ones((n, n), dtype=bool), k=1)

    fig, ax = plt.subplots(figsize=(8.8 / 2.54, 7.5 / 2.54))
    sns.heatmap(
        corr, mask=mask, ax=ax,
        cmap="RdBu_r", vmin=-1, vmax=1, center=0,
        annot=True, fmt=".2f", annot_kws={"size": 6},
        linewidths=0.5, linecolor="white",
        cbar_kws={"shrink": 0.8, "label": "Pearson $r$"},
        square=True,
    )

    # Bold high-correlation cells — iterate in seaborn's row-major annotation order
    ann_cells = [(i, j) for i in range(n) for j in range(n) if not mask[i, j]]
    for text, (i, j) in zip(ax.texts, ann_cells):
        if abs(corr.iloc[i, j]) > 0.7 and pval.iloc[i, j] < 0.01:
            text.set_fontweight("bold")

    ax.set_title("Fig. B — Transport Property Correlation Matrix\n(Golden Triplets, Pearson $r$)")
    fig.tight_layout()
    _save(fig, "figB_correlation_heatmap")


# =============================================================================
# FIGURE C — zT vs T Scatter with WHR Zone
# =============================================================================

def fig_C_zt_vs_T(gt: pd.DataFrame) -> None:
    """
    zT vs temperature scatter plot.
    Waste-heat recovery zone (323–773 K) shaded in pale orange.
    Points colored by audit tier; money-maker threshold marked.
    Hexbin used for high-density regions; tier A/B points overlaid individually.
    """
    df = gt[gt["zT_computed"] <= 6].copy()

    fig, ax = plt.subplots(figsize=(8.8 / 2.54, 7 / 2.54))

    # Shaded WHR zone
    ax.axvspan(WHR_T_MIN, WHR_T_MAX, color="#FEE08B", alpha=0.35, zorder=0,
               label=f"WHR zone ({WHR_T_MIN:.0f}–{WHR_T_MAX:.0f} K)")

    # Money-maker threshold
    ax.axhline(ZT_MONEY_MAKER, color="#CA0020", lw=0.8, ls="--",
               label=f"$zT$ = {ZT_MONEY_MAKER} (money-maker)", zorder=3)

    # Plot by tier
    if "audit_tier" in df.columns:
        for tier in [4, 3, 2, 1]:  # plot lower tiers first (behind)
            sub = df[df["audit_tier"] == tier]
            if len(sub) == 0:
                continue
            ax.scatter(
                sub["T_bin_K"], sub["zT_computed"],
                c=TIER_COLORS[tier], s=1.5, alpha=0.4, lw=0, rasterized=True,
                label=TIER_LABELS[tier], zorder=tier,
            )
    else:
        hb = ax.hexbin(df["T_bin_K"], df["zT_computed"],
                       gridsize=80, cmap="Blues", bins="log", mincnt=1)
        plt.colorbar(hb, ax=ax, label="log₁₀(count)")

    ax.set_xlabel("Temperature $T$ (K)")
    ax.set_ylabel("Figure of merit $zT$")
    ax.set_xlim(df["T_bin_K"].min() - 20, df["T_bin_K"].max() + 20)
    ax.set_ylim(bottom=-0.05)
    ax.xaxis.set_major_locator(ticker.MultipleLocator(200))
    ax.legend(frameon=False, markerscale=4, fontsize=6)
    ax.set_title(f"Fig. C — $zT$ vs. $T$ by Audit Tier\n({len(df):,} Golden Triplets)")
    fig.tight_layout()
    _save(fig, "figC_zt_vs_T")


# =============================================================================
# MAIN
# =============================================================================

def main() -> None:
    log.info("=== THERMOGNOSIS-X  Golden Triplet Analysis Pipeline ===")

    # Load full states for denominator counts
    parquet_path = PARQUET_DIR / "thermoelectric_states.parquet"
    all_states = pd.read_parquet(parquet_path)

    gt = load_golden_triplets()

    summary = compute_summary(gt, all_states)
    report_path = FIGURES_DIR / "golden_triplet_report.txt"
    write_report(summary, report_path)

    log.info("Generating Fig A — zT distribution")
    fig_A_zt_distribution(gt)

    log.info("Generating Fig B — correlation heatmap")
    fig_B_correlation_heatmap(gt)

    log.info("Generating Fig C — zT vs T scatter")
    fig_C_zt_vs_T(gt)

    log.info("=== Pipeline complete. All outputs in %s ===", FIGURES_DIR)


if __name__ == "__main__":
    main()
