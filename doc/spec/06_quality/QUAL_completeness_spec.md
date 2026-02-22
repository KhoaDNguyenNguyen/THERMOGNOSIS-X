# QUAL — Completeness Specification  
**Document ID:** SPEC-QUAL-COMPLETENESS  
**Layer:** spec/06_quality  
**Status:** Normative — Data and Model Completeness Governance Framework  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Completeness Specification (CS)** governing the formal criteria by which datasets, extracted records, physical models, and derived quantities are considered *complete* within the Thermognosis Engine.

Completeness is not merely the absence of missing fields.  
It is the satisfaction of structural, semantic, physical, and statistical sufficiency conditions required for:

- Reproducible scientific inference,
- Physically valid modeling,
- Publication-grade reporting,
- Closed-loop optimization.

No dataset shall enter the modeling or optimization layer unless it satisfies this specification.

---

# 2. Formal Definition of Completeness

Let a record be defined as:

\[
\mathcal{R} = (\mathbf{x}, \mathbf{m}, \mathbf{u}, \mathbf{c})
\]

where:

- \( \mathbf{x} \) — physical variables  
- \( \mathbf{m} \) — metadata  
- \( \mathbf{u} \) — uncertainty information  
- \( \mathbf{c} \) — contextual descriptors  

A record is complete if:

\[
\mathcal{C}(\mathcal{R}) = 1
\]

where:

\[
\mathcal{C} : \mathcal{D} \rightarrow \{0,1\}
\]

is the completeness predicate satisfying all conditions in Sections 3–10.

---

# 3. Structural Completeness

All mandatory schema fields must be present.

Let required field set:

\[
\mathcal{F}_{\text{req}}
\]

Structural completeness condition:

\[
\mathcal{F}_{\text{req}} \subseteq \text{Fields}(\mathcal{R})
\]

Missing required field ⇒ QUAL-COMP-01 violation.

---

# 4. Dimensional Completeness

Every physical quantity must have:

- Numerical value,
- Unit,
- Valid dimensional vector.

Formally:

\[
\forall x_i \in \mathbf{x} :
\mathbf{d}(x_i) \neq \varnothing
\]

Dimensionless quantities must be explicitly declared.

---

# 5. Uncertainty Completeness

Each reported physical quantity must include uncertainty unless:

- The value is theoretical and exact,
- Explicit waiver is documented.

Formal requirement:

\[
\forall x_i \in \mathbf{x} :
\sigma_i \text{ defined}
\]

Missing uncertainty ⇒ QUAL-COMP-02.

---

# 6. Physical Dependency Completeness

Derived quantities must include all dependent variables.

Example — thermoelectric figure of merit:

\[
ZT =
\frac{S^2 \sigma T}{\kappa}
\]

Completeness requires presence of:

- \( S \),
- \( \sigma \),
- \( T \),
- \( \kappa \).

If \( ZT \) reported without components ⇒ QUAL-COMP-03.

---

# 7. Domain Coverage Completeness

For functional data:

\[
f : \Omega \rightarrow \mathbb{R}
\]

Domain coverage condition:

\[
\Omega_{\text{observed}} \supseteq \Omega_{\text{required}}
\]

Example:

Temperature-dependent transport property must cover:

\[
T_{\min} \le T \le T_{\max}
\]

Insufficient range ⇒ QUAL-COMP-04.

---

# 8. Sampling Density Completeness

For discrete samples:

\[
\{x_i\}_{i=1}^n
\]

Sampling density condition:

\[
\max_i (x_{i+1} - x_i) \le \Delta_{\text{max}}
\]

Threshold determined by:

- Expected physical variation scale,
- Nyquist-like smoothness criterion.

Undersampling ⇒ QUAL-COMP-05.

---

# 9. Metadata Completeness

Mandatory metadata fields:

- Source reference
- Experimental method
- Material identity
- Measurement conditions
- Version timestamp

Formal condition:

\[
|\mathbf{m}_{\text{required}}| = |\mathbf{m}|
\]

Missing metadata ⇒ QUAL-COMP-06.

---

# 10. Contextual Completeness

Context must include:

- Phase information (if relevant),
- Composition details,
- Boundary conditions,
- Sample preparation notes.

Without context, physical interpretation is ambiguous.

---

# 11. Cross-Field Logical Completeness

Logical dependencies must be satisfied.

Example:

If:

\[
\text{Measurement Type} = \text{Hall}
\]

Then:

- Carrier concentration,
- Mobility,
- Magnetic field strength

must be present.

Violation ⇒ QUAL-COMP-07.

---

# 12. Graph Completeness (Knowledge Graph Layer)

Let data dependency graph:

\[
G = (V, E)
\]

Completeness requires:

\[
\forall v \in V_{\text{derived}},
\quad
\exists \text{ path from base measurements}
\]

Disconnected node ⇒ incomplete provenance.

---

# 13. Statistical Completeness

For aggregated results:

Sample size:

\[
n \ge n_{\text{min}}
\]

Confidence interval must be computable:

\[
\sigma_{\bar{x}} =
\frac{\sigma}{\sqrt{n}}
\]

If not computable ⇒ QUAL-COMP-08.

---

# 14. Temporal Completeness (Version Control)

Every record must include:

\[
t_{\text{created}}, \quad t_{\text{modified}}
\]

Missing timestamps ⇒ QUAL-COMP-09.

---

# 15. Completeness Score

Define completeness score:

\[
S_{\text{comp}} =
\frac{N_{\text{valid}}}{N_{\text{required}}}
\]

Acceptance threshold:

\[
S_{\text{comp}} = 1.0
\]

Partial completeness not permitted for modeling stage.

---

# 16. Determinism Requirement

Completeness evaluation must be:

- Deterministic,
- Version-stable,
- Schema-consistent.

Same record must always yield same completeness verdict.

---

# 17. Automated Validation Pipeline

Validation order:

1. Structural validation  
2. Unit validation  
3. Dimensional validation  
4. Uncertainty presence  
5. Physical dependency graph  
6. Domain coverage  
7. Sampling density  
8. Metadata audit  

Early failure halts downstream computation.

---

# 18. Error Classification

- QUAL-COMP-01: Missing required field
- QUAL-COMP-02: Missing uncertainty
- QUAL-COMP-03: Missing physical dependency
- QUAL-COMP-04: Insufficient domain coverage
- QUAL-COMP-05: Undersampling
- QUAL-COMP-06: Missing metadata
- QUAL-COMP-07: Logical inconsistency
- QUAL-COMP-08: Insufficient statistical basis
- QUAL-COMP-09: Missing temporal metadata

All violations must log record identifier.

---

# 19. Formal Soundness Condition

A dataset is complete if:

\[
\mathcal{C}(\mathcal{R}) = 1
\]

and:

- Physical constraints satisfied,
- Uncertainty defined,
- Provenance traceable.

Completeness is prerequisite for quality.

---

# 20. Strategic Interpretation

Completeness enforcement ensures:

- No hidden assumptions,
- No missing dependency variables,
- No untraceable data,
- No under-specified results.

Scientific credibility requires informational sufficiency.

---

# 21. Concluding Statement

All datasets and derived records must satisfy:

\[
\mathcal{R} \models \text{SPEC-QUAL-COMPLETENESS}
\]

Incomplete records are not eligible for modeling, optimization, or publication reporting.

Completeness is the foundation of scientific reliability.
