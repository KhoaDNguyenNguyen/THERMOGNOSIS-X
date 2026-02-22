# UNIT — Converter Specification  
**Document ID:** SPEC-UNIT-CONVERTER  
**Layer:** spec/04_unit_system  
**Status:** Normative — Deterministic Conversion and Canonicalization Protocol  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Unit Converter Specification (UCS)** governing deterministic, dimensionally validated, and reproducible transformation of physical quantities within the Thermognosis Engine.

The Unit Converter is responsible for:

1. Canonical SI normalization.
2. Exact dimensional compatibility enforcement.
3. Deterministic scaling and affine transformation.
4. Uncertainty propagation.
5. Bitwise reproducibility under version control.

Conversion is a mathematically constrained operator, not a formatting utility.

---

# 2. Formal Definition

A physical quantity is defined as:

\[
Q = (v, U, \sigma)
\]

where:

- \( v \in \mathbb{R} \) — magnitude  
- \( U \in \mathcal{R}_U \) — registered unit  
- \( \sigma \ge 0 \) — standard uncertainty  

The conversion operator is:

\[
\mathcal{C}(Q, U_t)
=
Q'
=
(v', U_t, \sigma')
\]

subject to dimensional constraint:

\[
\mathbf{d}(U) = \mathbf{d}(U_t)
\]

---

# 3. Dimensional Precondition

Let:

\[
\mathbf{d}(U) \in \mathbb{Z}^7
\]

If:

\[
\mathbf{d}(U) \neq \mathbf{d}(U_t)
\]

then:

\[
\mathcal{C}(Q, U_t) = \bot
\]

Conversion without dimensional compatibility is forbidden.

---

# 4. Scaling Transformation

For pure scaling units:

\[
v_{SI} = k_s v
\]

\[
v' = \frac{k_s}{k_t} v
\]

where:

- \( k_s \) — scaling factor of source unit  
- \( k_t \) — scaling factor of target unit  

Invertibility condition:

\[
v = \frac{k_t}{k_s} v'
\]

---

# 5. Affine Transformation

For affine units:

\[
v_{SI} = k_s v + b_s
\]

Target value:

\[
v' = \frac{v_{SI} - b_t}{k_t}
\]

Uncertainty propagation:

\[
\sigma' = \left| \frac{k_s}{k_t} \right| \sigma
\]

Offsets do not affect uncertainty.

---

# 6. Canonical SI Normalization

Internal storage requires:

\[
Q_{SI} = (v_{SI}, U_{SI})
\]

where:

\[
\mathbf{d}(U_{SI}) = \mathbf{d}(U)
\]

Canonical normalization is mandatory before:

- Aggregation,
- Model training,
- Statistical inference.

---

# 7. Derived Unit Conversion

For compound unit:

\[
U = U_1^{a_1} U_2^{a_2} \cdots U_n^{a_n}
\]

Scaling factor:

\[
k(U) = \prod_{i=1}^n k(U_i)^{a_i}
\]

Dimensional vector:

\[
\mathbf{d}(U)
=
\sum_{i=1}^n a_i \mathbf{d}(U_i)
\]

Conversion applies to aggregated scaling.

---

# 8. Floating-Point Determinism

All arithmetic must satisfy:

\[
v' = \text{round}(v', p)
\]

Default precision:

\[
p = 12 \text{ significant digits}
\]

Deterministic rounding strategy must be explicitly defined.

Cross-platform consistency required.

---

# 9. Uncertainty Propagation

For scaling:

\[
\sigma' = \left| \frac{k_s}{k_t} \right| \sigma
\]

For derived expression:

\[
z = f(x_1, \dots, x_n)
\]

First-order propagation:

\[
\sigma_z^2 =
\sum_{i=1}^n
\left(
\frac{\partial f}{\partial x_i}
\sigma_i
\right)^2
\]

Higher-order propagation may be enabled for precision-critical workflows.

---

# 10. Idempotency Requirement

Repeated conversion must satisfy:

\[
\mathcal{C}(\mathcal{C}(Q, U_t), U) = Q
\]

up to rounding tolerance.

---

# 11. Logarithmic Units

For logarithmic quantities:

\[
L = 10 \log_{10} \left( \frac{X}{X_0} \right)
\]

Conversion requires explicit reference value:

\[
X_0
\]

No implicit reference permitted.

---

# 12. Overflow and Underflow Handling

If:

\[
|v_{SI}| > 10^{308}
\quad \text{or} \quad
|v_{SI}| < 10^{-308}
\]

System must:

- Raise numerical stability warning.
- Optionally use high-precision arithmetic backend.

---

# 13. Registry Coupling

All conversions must reference:

\[
\mathcal{R}_U^{(t)}
\]

Conversion record includes:

- Source unit
- Target unit
- Registry version
- Scaling factors used

---

# 14. Deterministic Reproducibility

Given identical:

- Input quantity,
- Source unit,
- Target unit,
- Registry version,
- Precision setting,

The conversion result must satisfy:

\[
\mathcal{C}(Q, U_t) = \text{constant}
\]

Bitwise identical output required.

---

# 15. Security Constraints

The converter must:

- Operate without external API calls.
- Disallow dynamic string evaluation.
- Use immutable registry reference.
- Reject unknown unit symbols.

---

# 16. Conversion Graph Formalism

Define conversion graph:

\[
G_U = (V, E)
\]

where:

- \( V = \mathcal{R}_U \)
- \( E \) — valid dimension-preserving transformations

Conversion path must be unique and acyclic via canonical SI.

---

# 17. Performance Requirement

Time complexity for conversion:

\[
\mathcal{O}(1)
\]

per operation under registry lookup.

Derived unit decomposition must not exceed:

\[
\mathcal{O}(n)
\]

for \( n \) atomic factors.

---

# 18. Compliance with Dimensional Validation

Invariant:

\[
\text{Dimensional Validation}
\Rightarrow
\text{Conversion Allowed}
\]

No conversion without successful dimensional check.

---

# 19. Audit Trail

Each conversion event must log:

\[
(
Q_{source},
Q_{target},
k_s,
k_t,
b_s,
b_t,
\text{registry version}
)
\]

Auditability is mandatory for publication-grade workflows.

---

# 20. Formal Correctness Condition

The converter is mathematically correct if:

1. Dimensional invariance holds.
2. Invertibility holds (within tolerance).
3. Uncertainty scales consistently.
4. Canonical SI mapping is unique.
5. Output is deterministic.

---

# 21. Strategic Interpretation

The Unit Converter is the deterministic numerical backbone of the Thermognosis Engine.

Without it:

- Multi-source datasets become incomparable.
- Physical invariants collapse.
- Statistical modeling inherits hidden bias.

With it:

- Numerical normalization is provably correct.
- Cross-paper aggregation becomes valid.
- Model interpretability improves.

---

# 22. Concluding Statement

Every numerical quantity entering modeling, storage, or publication must satisfy:

\[
Q \models \text{SPEC-UNIT-CONVERTER}
\]

Conversion is not optional.

It is a mathematically enforced transformation ensuring dimensional integrity, reproducibility, and scientific credibility at scale.
