# UNIT — Registry Specification  
**Document ID:** SPEC-UNIT-SYSTEM-REGISTRY  
**Layer:** spec/04_unit_system  
**Status:** Normative — Authoritative Unit System Definition  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Unit System Registry Specification (USRS)** governing the formal, version-controlled, and mathematically rigorous registry of all physical units recognized by the Thermognosis Engine at the unit-system layer.

The registry is the unique authoritative mapping between:

- Unit symbols,
- Canonical names,
- Dimensional vectors,
- Scaling and affine transformation parameters,
- Canonical SI representatives,
- Version and integrity metadata.

No unit operation is valid outside this registry.

---

# 2. Formal Structure of the Registry

Let the registry be defined as:

\[
\mathcal{R}_U^{(t)} = \{ U_i \}_{i=1}^{N}
\]

where \( t \) denotes registry version.

Each unit entry is defined as:

\[
U_i =
(
\text{name}_i,
\text{symbol}_i,
\mathbf{d}_i,
k_i,
b_i,
\text{category}_i,
\text{metadata}_i
)
\]

with:

- \( \mathbf{d}_i \in \mathbb{Z}^7 \) — dimensional vector  
- \( k_i \in \mathbb{R}^+ \) — multiplicative scaling factor to canonical SI  
- \( b_i \in \mathbb{R} \) — affine offset (if applicable)  

---

# 3. Dimensional Basis

All dimensional vectors are expressed in the SI basis:

\[
\mathcal{B} =
\{
\mathrm{L},
\mathrm{M},
\mathrm{T},
\mathrm{I},
\mathrm{\Theta},
\mathrm{N},
\mathrm{J}
\}
\]

A unit’s dimensional representation:

\[
\mathbf{d}(U)
=
(\alpha_1,\dots,\alpha_7)
\quad
\alpha_i \in \mathbb{Z}
\]

Floating or non-integer exponents are prohibited.

---

# 4. Canonical SI Representative

For each dimensional vector:

\[
\mathbf{d}
\]

there exists a unique canonical SI unit:

\[
U_{SI}(\mathbf{d})
\]

such that:

\[
\mathbf{d}(U_{SI}) = \mathbf{d}
\]

Uniqueness constraint:

\[
\forall U_i, U_j:
\mathbf{d}(U_i) = \mathbf{d}(U_j) = \mathbf{d}
\Rightarrow
U_{SI}(\mathbf{d}) \text{ is unique}
\]

This ensures deterministic normalization.

---

# 5. Scaling Law

Conversion to SI is defined as:

\[
v_{SI} = k_i v + b_i
\]

Two categories exist:

### 5.1 Pure Scaling Units

\[
b_i = 0
\]

Example:

\[
1 \ \mathrm{mm} = 10^{-3} \ \mathrm{m}
\]

### 5.2 Affine Units

\[
b_i \neq 0
\]

Example:

\[
T_K = T_C + 273.15
\]

Affine units must explicitly declare affine status.

---

# 6. Symbol Uniqueness Constraint

For registry integrity:

\[
\text{symbol}_i = \text{symbol}_j
\Rightarrow
i = j
\]

Duplicate symbols are forbidden.

Aliases must be explicitly mapped but reference a single canonical entry.

---

# 7. Derived Unit Construction

Derived units are defined algebraically:

\[
U =
\prod_{k=1}^{m} U_k^{\beta_k}
\]

with:

\[
\beta_k \in \mathbb{Z}
\]

Dimensional vector:

\[
\mathbf{d}(U)
=
\sum_{k=1}^{m} \beta_k \mathbf{d}(U_k)
\]

Scaling factor:

\[
k(U)
=
\prod_{k=1}^{m} k_k^{\beta_k}
\]

Registry must support symbolic decomposition.

---

# 8. Dimensionless Units

A unit is dimensionless if:

\[
\mathbf{d}(U) = \mathbf{0}
\]

Dimensionless entries must still define:

\[
k_i
\]

and be explicitly categorized.

---

# 9. Registry Versioning

Registry version:

\[
\mathcal{R}_U^{(t)}
\]

Versioning follows semantic structure:

- MAJOR — dimensional change
- MINOR — new unit addition
- PATCH — metadata correction

Every unit conversion and validation must record:

\[
t
\]

---

# 10. Deterministic Mapping Requirement

Given:

- Symbol,
- Registry version,

mapping must satisfy:

\[
\text{symbol} \rightarrow
(\mathbf{d}, k, b)
\]

Deterministically.

No runtime mutation allowed.

---

# 11. Integrity Hash

Registry must be cryptographically verifiable.

Define:

\[
H_{\mathcal{R}} = H(\text{serialized registry})
\]

All runtime operations must reference:

\[
H_{\mathcal{R}}
\]

to guarantee reproducibility.

---

# 12. Numerical Stability Constraints

Scaling factors must satisfy:

\[
10^{-300} < k_i < 10^{300}
\]

If outside range:

- High-precision arithmetic required,
- Explicit stability warning logged.

---

# 13. Closure Property

Registry must be closed under:

1. Multiplication
2. Division
3. Integer exponentiation

Formally:

\[
\forall U_a, U_b \in \mathcal{R}_U
\]

if:

\[
\mathbf{d}(U_c)
=
\mathbf{d}(U_a)
\pm
\mathbf{d}(U_b)
\]

then:

\[
U_c
\]

must map to canonical SI representation.

---

# 14. Interoperability Constraints

Registry must align with:

- Dimensional Validation Specification
- Unit Converter Specification
- Raw Measurement Contract
- Validated Measurement Contract
- Statistical Modeling Layer

Invariant:

\[
\text{All Units Used} \subseteq \mathcal{R}_U
\]

---

# 15. Error Taxonomy

Registry errors classified as:

- UR-01: Unknown symbol
- UR-02: Duplicate symbol
- UR-03: Invalid dimensional vector
- UR-04: Missing canonical SI representative
- UR-05: Inconsistent affine declaration

All errors must include:

- Symbol,
- Registry version,
- Execution context.

---

# 16. Auditability

For any stored quantity:

\[
Q = (v, U)
\]

system must reconstruct:

1. Registry entry,
2. Dimensional vector,
3. Scaling parameters,
4. Canonical SI value.

Audit path must be complete.

---

# 17. Formal Consistency Condition

Registry is consistent if:

\[
\forall U_i \in \mathcal{R}_U:
\mathbf{d}(U_i) \in \mathbb{Z}^7
\]

and:

\[
\exists ! \ U_{SI}(\mathbf{d})
\]

for each dimensional vector.

---

# 18. Strategic Interpretation

The Unit System Registry is not an implementation detail.

It is:

- The mathematical backbone of dimensional correctness,
- The anchor of deterministic conversion,
- The structural guarantee of reproducible scientific computation.

Without a rigorously governed registry, dimensional integrity collapses.

---

# 19. Concluding Statement

All unit definitions within the Thermognosis Engine must satisfy:

\[
U \models \text{SPEC-UNIT-SYSTEM-REGISTRY}
\]

The registry enforces:

- Dimensional invariance,
- Deterministic normalization,
- Version traceability,
- Cryptographic integrity.

It is the foundational layer upon which all physically meaningful computation depends.
