# CONTRACT — Validated Measurement Entity  
**Document ID:** SPEC-CONTRACT-VALIDATED-MEASUREMENT  
**Layer:** spec/01_contracts  
**Status:** Normative — Post-Validation Data Contract  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Validated Measurement Contract (VMC)** governing the transformation of raw experimental measurements into statistically and physically validated data points suitable for inference and closed-loop decision-making.

Its objectives are:

1. To formalize the transition from raw data to inference-ready data.
2. To enforce physical consistency constraints.
3. To integrate uncertainty propagation and credibility weighting.
4. To preserve traceability to raw measurement entities.
5. To prevent silent data manipulation during preprocessing.

A validated measurement is not a corrected value.  
It is a formally certified measurement with explicit validation metadata.

---

# 2. Relationship to Raw Measurement

Each validated measurement derives from exactly one raw measurement:

\[
\mathcal{V} \rightarrow \mathcal{R}
\]

where:

\[
\mathcal{R} \models \text{SPEC-CONTRACT-RAW-MEASUREMENT}
\]

Raw data must remain immutable.  
Validation creates a new entity; it never overwrites raw data.

---

# 3. Formal Definition

A validated measurement is defined as:

\[
\mathcal{V}
=
(\text{ID}, \text{RAW\_ID}, \mathcal{Q}^*, \mathcal{U}^*, \mathcal{W}, \mathcal{F}, \mathcal{L})
\]

where:

- \( \text{ID} \): unique validated measurement identifier
- \( \text{RAW\_ID} \): reference to raw measurement
- \( \mathcal{Q}^* \): validated quantity set
- \( \mathcal{U}^* \): propagated uncertainty structure
- \( \mathcal{W} \): credibility weight
- \( \mathcal{F} \): validation flags
- \( \mathcal{L} \): lineage metadata

---

# 4. Identity Requirements

Validated measurement ID format: VAL_[HASH]

Hash computed from:

\[
\text{ID}
=
\mathrm{Hash}(\text{RAW\_ID}, \mathcal{F}, \mathcal{W})
\]

Invariant:

\[
\text{RAW\_ID}_i = \text{RAW\_ID}_j
\land
\mathcal{F}_i = \mathcal{F}_j
\Rightarrow
\text{ID}_i = \text{ID}_j
\]

---

# 5. Validated Quantities

Validated quantity set:

\[
\mathcal{Q}^*
=
(T, S, \sigma, \kappa, zT)
\]

Derived value:

\[
zT
=
\frac{S^2 \sigma T}{\kappa}
\]

Derived quantities must be explicitly marked as computed.

---

# 6. Uncertainty Propagation

If raw covariance:

\[
\mathbf{\Sigma}
\]

Validated covariance:

\[
\mathbf{\Sigma}^*
=
\nabla g^T \mathbf{\Sigma} \nabla g
\]

where:

\[
g(S,\sigma,T,\kappa)
=
\frac{S^2 \sigma T}{\kappa}
\]

If no covariance provided:

Assume independence:

\[
\sigma_{zT}^2
=
zT^2
\left(
4\frac{\sigma_S^2}{S^2}
+
\frac{\sigma_\sigma^2}{\sigma^2}
+
\frac{\sigma_\kappa^2}{\kappa^2}
\right)
\]

Uncertainty propagation must be logged.

---

# 7. Credibility Weight

Credibility weight:

\[
0 \le w \le 1
\]

Derived from:

\[
w = f(\text{paper credibility}, \text{measurement consistency}, \text{method reliability})
\]

Effective variance:

\[
\sigma_{\text{effective}}^2
=
\frac{1}{w} \sigma_{\text{propagated}}^2
\]

Lower credibility increases variance.

---

# 8. Physical Validation Rules

Validated measurement must satisfy:

\[
T > 0
\]

\[
\sigma > 0
\]

\[
\kappa > 0
\]

Optional consistency:

Wiedemann–Franz bound:

\[
\kappa_e \le L_0 \sigma T
\]

Violation triggers flag in \( \mathcal{F} \), not automatic correction.

---

# 9. Outlier Assessment

Outlier probability:

\[
p_{\text{outlier}} = 1 - w
\]

Robust likelihood representation:

\[
p(y_i | \theta)
=
(1 - \epsilon) \mathcal{N}(\mu_i, \sigma_i^2)
+
\epsilon \mathcal{T}(\nu)
\]

Outlier label stored but value preserved.

---

# 10. Validation Flags

Flag structure:

\[
\mathcal{F}
=
\{
\text{unit\_validated},
\text{physical\_consistent},
\text{uncertainty\_propagated},
\text{outlier\_flag}
\}
\]

Each flag Boolean.

Validation invariant:

\[
\text{unit\_validated} = \text{True}
\]

must hold for acceptance.

---

# 11. Lineage Metadata

Lineage structure:

\[
\mathcal{L}
=
(\text{validation\_timestamp}, \text{algorithm\_version}, \text{spec\_version})
\]

Ensures reproducibility.

---

# 12. Statistical Layer Compatibility

Validated dataset:

\[
\mathcal{D}_{\text{validated}}
=
\{ (x_i, y_i, \sigma_i^2, w_i) \}
\]

Weighted likelihood:

\[
\mathcal{L}
=
\prod_i
\mathcal{N}(y_i | \mu_i, \sigma_{i,\text{effective}}^2)
\]

Validated measurements are admissible inputs for inference.

---

# 13. Immutability Rule

Validated measurement immutable after creation.

If re-validation required:

\[
\mathcal{V}^{(v+1)}
\]

New version created.

Historical lineage preserved.

---

# 14. Cross-Layer Invariants

Traceability invariant:

\[
\text{Validated} \rightarrow \text{Raw} \rightarrow \text{Material} \rightarrow \text{Paper}
\]

No validated entity may exist without valid upstream linkage.

---

# 15. Aggregation Rule

Aggregation (e.g., temperature binning) must create new dataset entity.

Validated measurements remain atomic.

---

# 16. Logging Requirements

Each validation must log:

- Raw ID
- Validation flags
- Credibility weight
- Propagation method
- Numerical stabilization parameter
- Timestamp

---

# 17. Validation Checklist

Before acceptance:

- ✔ Raw linkage valid  
- ✔ Units confirmed  
- ✔ Physical constraints checked  
- ✔ Uncertainty propagated  
- ✔ Credibility weight computed  
- ✔ Flags recorded  
- ✔ ID deterministic  

---

# 18. Failure Modes

Improper validation may cause:

- Artificial zT inflation
- Overconfident inference
- Biased acquisition policy
- Loss of reproducibility

Governance must reject non-compliant validated measurements.

---

# 19. Compliance Requirement

Every validated measurement must satisfy:

\[
\mathcal{V} \models \text{SPEC-CONTRACT-VALIDATED-MEASUREMENT}
\]

Non-compliance results in exclusion from modeling layer.

---

# 20. Strategic Interpretation

Validation is not cosmetic cleaning.

It is:

- A mathematical certification process,
- A physical consistency check,
- A credibility integration step,
- A bridge between experiment and inference.

Without rigorous validation,  
statistical modeling becomes epistemically unstable.

---

# 21. Concluding Statement

In the Thermognosis Engine, validated measurements are certified scientific data points.

They carry:

- Verified physical consistency,
- Explicit uncertainty propagation,
- Quantified credibility weighting,
- Immutable traceability,
- Cross-layer compatibility.

Only under this contract can the system maintain  
statistical rigor, physical realism, and publication-grade reliability.


