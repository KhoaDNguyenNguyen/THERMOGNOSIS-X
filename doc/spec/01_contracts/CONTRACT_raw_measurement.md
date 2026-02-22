# CONTRACT — Raw Measurement Entity  
**Document ID:** SPEC-CONTRACT-RAW-MEASUREMENT  
**Layer:** spec/01_contracts  
**Status:** Normative — Experimental Data Integrity Contract  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Raw Measurement Contract (RMC)** governing the ingestion, validation, storage, and traceability of primary experimental measurements within the Thermognosis Engine.

Its objectives are:

1. To formally define the atomic unit of experimental data.
2. To preserve measurement-level uncertainty structure.
3. To prevent information loss during aggregation.
4. To ensure strict linkage to material and paper entities.
5. To maintain full reproducibility from raw value to derived inference.

A raw measurement is the smallest epistemic unit of the system.  
It must be preserved with maximal fidelity.

---

# 2. Formal Definition

A raw measurement entity is defined as:

\[
\mathcal{R}
=
(\text{ID}, \text{MAT\_ID}, \text{PAPER\_ID}, \mathcal{Q}, \mathcal{U}, \mathcal{C}, \mathcal{M})
\]

where:

- \( \text{ID} \): unique measurement identifier
- \( \text{MAT\_ID} \): linked material identifier
- \( \text{PAPER\_ID} \): linked paper identifier
- \( \mathcal{Q} \): measured quantities
- \( \mathcal{U} \): uncertainty structure
- \( \mathcal{C} \): contextual metadata
- \( \mathcal{M} \): measurement method metadata

All fields mandatory unless explicitly declared optional.

---

# 3. Identity Requirements

Measurement ID format: MEAS_[HASH]

Hash computed from:

\[
\text{ID}
=
\mathrm{Hash}(
\text{MAT\_ID},
\text{PAPER\_ID},
T,
\text{measurement\_type},
\text{value}
)
\]

Identity invariant:

\[
\text{ID}_i = \text{ID}_j
\Rightarrow
\mathcal{R}_i = \mathcal{R}_j
\]

Duplicate measurement entries prohibited.

---

# 4. Measured Quantities

Quantity set:

\[
\mathcal{Q}
=
(T, S, \sigma, \kappa)
\]

Where applicable:

- Temperature \( T \)
- Seebeck coefficient \( S \)
- Electrical conductivity \( \sigma \)
- Thermal conductivity \( \kappa \)

Each quantity may be individually recorded.

Derived value:

\[
zT
=
\frac{S^2 \sigma T}{\kappa}
\]

Derived quantities must never replace raw measurements.

---

# 5. Unit Requirements

All quantities must be in SI units:

| Quantity | Unit |
|----------|------|
| \( T \) | K |
| \( S \) | V/K |
| \( \sigma \) | S/m |
| \( \kappa \) | W/(m·K) |

Unit conversion must occur before ingestion and be logged.

---

# 6. Uncertainty Structure

Uncertainty tuple:

\[
\mathcal{U}
=
(\sigma_T, \sigma_S, \sigma_\sigma, \sigma_\kappa)
\]

Constraints:

\[
\sigma_x \ge 0
\]

If covariance available:

\[
\mathbf{\Sigma}
=
\begin{bmatrix}
\sigma_T^2 & \dots \\
\dots & \sigma_\kappa^2
\end{bmatrix}
\]

Uncertainty must be stored at raw level — not aggregated.

---

# 7. Measurement Context

Context metadata:

\[
\mathcal{C}
=
(\text{pressure}, \text{atmosphere}, \text{sample\_orientation}, \text{doping\_level})
\]

All contextual fields must include units.

Context invariant:

\[
\text{Context completeness} \ge \text{Minimum required schema}
\]

---

# 8. Measurement Method Metadata

Method descriptor:

\[
\mathcal{M}
=
(\text{instrument}, \text{calibration\_date}, \text{technique}, \text{resolution})
\]

Example:

- Laser flash method
- Four-probe method

Calibration must include timestamp.

---

# 9. Physical Validity Constraints

The following must hold:

\[
T > 0
\]

\[
\sigma > 0
\]

\[
\kappa > 0
\]

Optional Wiedemann–Franz consistency:

\[
\kappa_e \le L_0 \sigma T
\]

Violation triggers governance flag, not silent correction.

---

# 10. Error Propagation Rule

If derived quantity computed:

\[
zT = \frac{S^2 \sigma T}{\kappa}
\]

Variance:

\[
\sigma_{zT}^2
=
\nabla g^T \mathbf{\Sigma} \nabla g
\]

where:

\[
g(S,\sigma,T,\kappa)
=
\frac{S^2 \sigma T}{\kappa}
\]

Raw measurements must retain original uncertainty prior to propagation.

---

# 11. Temporal Integrity

Measurement timestamp:

\[
t_{\text{meas}} \in \mathbb{R}
\]

Chronological invariant:

\[
t_{\text{meas}} \le t_{\text{ingest}}
\]

No future-dated measurement allowed.

---

# 12. Linkage Invariants

Material linkage:

\[
\text{MAT\_ID} \models \text{SPEC-CONTRACT-MATERIAL}
\]

Paper linkage:

\[
\text{PAPER\_ID} \models \text{SPEC-CONTRACT-PAPER}
\]

No orphan measurement allowed.

---

# 13. Immutability Rule

Raw measurement values immutable after ingestion.

If correction required:

\[
\mathcal{R}^{(v+1)}
\]

New version created.

Historical record preserved.

---

# 14. Aggregation Prohibition

Raw measurement must not be overwritten by:

- Averaged value
- Smoothed curve
- Model prediction

Aggregation must create new entity, not modify raw data.

---

# 15. Outlier Handling

Outlier label:

\[
\delta_i \in \{0,1\}
\]

Outlier classification must not delete measurement.

Instead:

\[
p(y_i | \theta)
=
(1 - \epsilon) \mathcal{N}(\mu_i, \sigma_i^2)
+
\epsilon \mathcal{T}(\nu)
\]

Mixture model preferred.

---

# 16. Serialization Standard

Serialized as canonical JSON.

All numeric values must include:

- Explicit precision
- Unit annotation (if external format)

Hash computed post-serialization.

---

# 17. Logging Requirements

Each ingestion must log:

- Dataset hash
- Spec version
- Ingestion timestamp
- Validation result
- Numerical stabilization (if applied)

---

# 18. Validation Checklist

Before acceptance:

- ✔ Units verified  
- ✔ Uncertainty non-negative  
- ✔ Physical constraints satisfied  
- ✔ Material linkage valid  
- ✔ Paper linkage valid  
- ✔ Context metadata complete  
- ✔ ID deterministic  

---

# 19. Failure Modes

Invalid raw measurement may cause:

- Bias in Bayesian inference
- Artificial reduction of uncertainty
- Graph misrepresentation
- Incorrect acquisition decisions

Governance must block invalid entries.

---

# 20. Cross-Layer Invariant

Raw measurement invariant:

\[
\text{Raw} \rightarrow \text{Material} \rightarrow \text{Paper}
\]

Statistical and graph layers must reference raw entity, not reconstructed value.

---

# 21. Compliance Requirement

Every raw measurement entity must satisfy:

\[
\mathcal{R} \models \text{SPEC-CONTRACT-RAW-MEASUREMENT}
\]

Non-compliance results in ingestion rejection.

---

# 22. Strategic Interpretation

Raw measurement is the foundation of epistemic credibility.

If raw data integrity fails:

- Uncertainty modeling collapses,
- Statistical inference becomes biased,
- Closed-loop selection becomes unstable,
- Publication credibility is compromised.

Preserving raw measurement integrity is non-negotiable.

---

# 23. Concluding Statement

In the Thermognosis Engine, raw measurements are first-class scientific objects.

They carry:

- Physical value,
- Structured uncertainty,
- Contextual metadata,
- Method traceability,
- Cross-layer linkage.

Only by preserving raw measurement fidelity can the system remain:

- Scientifically defensible,
- Statistically rigorous,
- Architecturally stable,
- Publication-ready.

Data integrity is the root of knowledge integrity.


