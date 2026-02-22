# CONTRACT — Material Entity  
**Document ID:** SPEC-CONTRACT-MATERIAL  
**Layer:** spec/01_contracts  
**Status:** Normative — Data & Identity Contract  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Material Contract (MC)** governing the formal representation, validation, and lifecycle of material entities within the Thermognosis Engine.

Its objectives are:

1. To provide a canonical definition of a material entity.
2. To ensure identity consistency across physics, statistics, and graph layers.
3. To define required attributes and invariants.
4. To guarantee traceability from raw measurement to decision layer.
5. To prevent semantic drift in material representation.

A material is not merely a label.  
It is a structured scientific object.

---

# 2. Formal Definition

A material entity is defined as a tuple:

\[
\mathcal{M}
=
(\text{ID}, \mathcal{C}, \mathcal{S}, \mathcal{P}, \mathcal{U}, \mathcal{R})
\]

where:

- \( \text{ID} \): unique material identifier
- \( \mathcal{C} \): chemical composition
- \( \mathcal{S} \): structural descriptors
- \( \mathcal{P} \): physical property dataset
- \( \mathcal{U} \): uncertainty descriptors
- \( \mathcal{R} \): reference provenance metadata

All fields are mandatory unless explicitly marked optional.

---

# 3. Identity Requirements

## 3.1 Unique Identifier

Material ID format: MAT_[HASH]


Hash must be deterministic function:

\[
\text{ID} = \mathrm{Hash}(\mathcal{C}, \mathcal{S})
\]

Identity invariant:

\[
\text{ID}_i = \text{ID}_j
\Rightarrow
(\mathcal{C}_i, \mathcal{S}_i) = (\mathcal{C}_j, \mathcal{S}_j)
\]

No duplicate semantic entities permitted.

---

# 4. Composition Field

Chemical composition:

\[
\mathcal{C}
=
\{ (E_k, x_k) \}_{k=1}^K
\]

where:

- \( E_k \): element symbol
- \( x_k \): atomic fraction

Constraint:

\[
\sum_{k=1}^K x_k = 1
\]

All fractions must be normalized.

---

# 5. Structural Descriptors

Structural descriptor set:

\[
\mathcal{S}
=
\{
\text{crystal\_structure},
\text{phase},
\text{space\_group},
\text{lattice\_parameters}
\}
\]

Lattice parameters:

\[
(a, b, c, \alpha, \beta, \gamma)
\]

Units: SI (meters, radians).

---

# 6. Physical Property Dataset

Property dataset:

\[
\mathcal{P}
=
\{ (T_i, S_i, \sigma_i, \kappa_i) \}_{i=1}^{n}
\]

Each measurement must include:

- Temperature \( T_i \)
- Seebeck coefficient \( S_i \)
- Electrical conductivity \( \sigma_i \)
- Thermal conductivity \( \kappa_i \)

Derived quantity:

\[
zT_i = \frac{S_i^2 \sigma_i T_i}{\kappa_i}
\]

---

# 7. Uncertainty Specification

For each measurement:

\[
\mathcal{U}_i
=
(\sigma_{S_i}, \sigma_{\sigma_i}, \sigma_{\kappa_i})
\]

Uncertainty invariant:

\[
\sigma_{x_i} \ge 0
\]

No negative uncertainty permitted.

Covariance optional but must follow:

\[
\mathbf{\Sigma}_i
=
\begin{bmatrix}
\sigma_{S_i}^2 & \dots \\
\dots & \sigma_{\kappa_i}^2
\end{bmatrix}
\]

---

# 8. Provenance Metadata

Reference metadata:

\[
\mathcal{R}
=
(\text{DOI}, \text{authors}, \text{year}, \text{experiment\_method})
\]

Each property entry must link to provenance.

Traceability invariant:

\[
\forall p \in \mathcal{P}, \exists r \in \mathcal{R}
\]

---

# 9. Unit Compliance

All properties must use SI units:

| Quantity | Unit |
|----------|------|
| \( S \) | V/K |
| \( \sigma \) | S/m |
| \( \kappa \) | W/(m·K) |
| \( T \) | K |

No implicit unit conversion allowed.

---

# 10. Physical Validity Constraints

The following constraints must hold:

\[
\kappa > 0
\]

\[
\sigma > 0
\]

\[
T > 0
\]

Optional Wiedemann–Franz check:

\[
\kappa_e \le L_0 \sigma T
\]

Violation must trigger governance warning.

---

# 11. Graph Integration

Each material corresponds to node:

\[
v_i \in \mathcal{V}
\]

Edge definition:

\[
(v_i \rightarrow v_j)
\]

based on:

- Shared composition
- Citation linkage
- Structural similarity

Graph embedding must preserve ID consistency.

---

# 12. Statistical Layer Contract

Material dataset used in model:

\[
\mathcal{D}_\mathcal{M}
=
\{ (x_i, y_i) \}
\]

Input features:

\[
x_i = \phi(\mathcal{C}, \mathcal{S}, T_i)
\]

Target:

\[
y_i = zT_i
\]

No feature extraction may alter original identity.

---

# 13. Immutability Rule

Core identity fields:

- Composition
- Structure

must be immutable after creation.

Property dataset may extend, but not modify historical entries.

---

# 14. Versioning

Material version:

\[
\mathcal{M}^{(v)}
\]

Increment version when:

- New measurements added.
- Structural descriptors updated.
- Provenance corrected.

Version must not overwrite history.

---

# 15. Validation Checklist

Before acceptance:

- ✔ Composition normalized  
- ✔ Units verified  
- ✔ Uncertainty non-negative  
- ✔ Physical constraints satisfied  
- ✔ Provenance present  
- ✔ ID deterministic  

---

# 16. Failure Modes

Invalid material entity may lead to:

- Incorrect zT calculation
- Inconsistent graph embedding
- Biased Bayesian inference
- Corrupted closed-loop acquisition

Governance must block invalid entries.

---

# 17. Serialization Standard

Material must serialize to: JSON (canonical order)


All numeric values stored with explicit precision.

Hash computed after canonical serialization.

---

# 18. Cross-Layer Invariant

Material identity invariant:

\[
\text{ID}_{\text{physics}}
=
\text{ID}_{\text{graph}}
=
\text{ID}_{\text{statistical}}
\]

No layer-specific alias permitted.

---

# 19. Compliance Requirement

Every material object must satisfy:

\[
\mathcal{M} \models \text{SPEC-CONTRACT-MATERIAL}
\]

Non-compliant material entities are rejected.

---

# 20. Strategic Interpretation

The material contract defines the atomic unit of the Thermognosis Engine.

It ensures that:

- Physical meaning is preserved,
- Statistical modeling remains grounded,
- Graph representation remains consistent,
- Closed-loop decisions remain traceable.

Without a strict material contract,  
the system collapses into semantic inconsistency.

---

# 21. Concluding Statement

A material entity in this system is not a spreadsheet row.

It is a formally defined scientific object with:

- Identity,
- Physical constraints,
- Uncertainty structure,
- Provenance traceability,
- Cross-layer consistency.

Only under this contract can the Thermognosis Engine remain  
scientifically credible, scalable, and publication-ready.




