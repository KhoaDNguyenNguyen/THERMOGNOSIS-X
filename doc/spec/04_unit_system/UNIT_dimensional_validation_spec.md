# UNIT — Dimensional Validation Specification  
**Document ID:** SPEC-UNIT-DIM-VALIDATION  
**Layer:** spec/04_unit_system  
**Status:** Normative — System-Level Dimensional Consistency Enforcement  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **System-Level Dimensional Validation Specification (DVS)** governing formal verification of dimensional consistency across all unit operations within the Thermognosis Engine.

While extraction-layer validation ensures correctness at ingestion, this specification enforces dimensional invariants at the **unit system level**, prior to conversion, aggregation, modeling, and publication.

Dimensional validation is a mathematical constraint, not a heuristic safeguard.

---

# 2. Mathematical Foundation

Let the SI base dimension basis be:

\[
\mathcal{B} =
\{
\mathrm{L},     % length
\mathrm{M},     % mass
\mathrm{T},     % time
\mathrm{I},     % current
\mathrm{\Theta},% temperature
\mathrm{N},     % amount
\mathrm{J}      % luminous intensity
\}
\]

Define the dimensional mapping:

\[
\mathbf{d} : \mathcal{R}_U \rightarrow \mathbb{Z}^7
\]

such that:

\[
U \mapsto
\mathbf{d}(U)
=
(\alpha_1,\dots,\alpha_7)
\]

with integer exponents.

Dimensional space is a lattice:

\[
\mathcal{D} = \mathbb{Z}^7
\]

---

# 3. Core Invariant

For any physically meaningful equality:

\[
Q_1 = Q_2
\]

it must hold that:

\[
\mathbf{d}(U_1) = \mathbf{d}(U_2)
\]

Violation constitutes a structural physical inconsistency.

---

# 4. Binary Operation Validation

For addition or subtraction:

\[
z = x \pm y
\]

Validity condition:

\[
\mathbf{d}(x) = \mathbf{d}(y)
\]

If not satisfied:

\[
z = \bot
\]

Addition across mismatched dimensions is prohibited.

---

# 5. Multiplicative Closure

For multiplication:

\[
z = x \cdot y
\]

Dimensional mapping:

\[
\mathbf{d}(z)
=
\mathbf{d}(x)
+
\mathbf{d}(y)
\]

For division:

\[
z = \frac{x}{y}
\]

\[
\mathbf{d}(z)
=
\mathbf{d}(x)
-
\mathbf{d}(y)
\]

Closure under multiplication and division is mandatory.

---

# 6. Power Operation

For exponentiation:

\[
z = x^{n}
\]

where:

\[
n \in \mathbb{Z}
\]

Dimensional mapping:

\[
\mathbf{d}(z) = n \mathbf{d}(x)
\]

Non-integer exponents require:

\[
\mathbf{d}(x) = \mathbf{0}
\]

Fractional powers of dimensioned quantities are disallowed unless explicitly defined in registry.

---

# 7. Function Domain Constraints

Let:

\[
f : \mathbb{R} \rightarrow \mathbb{R}
\]

Common mathematical functions require:

### 7.1 Exponential and Logarithmic

\[
f(x) = e^x, \quad \log(x)
\]

Require:

\[
\mathbf{d}(x) = \mathbf{0}
\]

### 7.2 Trigonometric

\[
\sin(x), \cos(x)
\]

Require:

\[
\mathbf{d}(x) = \mathbf{0}
\]

Angular quantities must be dimensionless in canonical representation.

---

# 8. Affine Unit Handling

Temperature absolute scales:

\[
T_K = k T_C + b
\]

Dimensional validation must distinguish:

- Absolute temperature \( T \)
- Temperature difference \( \Delta T \)

Both share:

\[
\mathbf{d}(T) = \mathbf{d}(\Delta T) = \mathrm{\Theta}
\]

but affine metadata must be preserved.

---

# 9. Derived Quantity Validation

For general expression:

\[
z = f(x_1,\dots,x_n)
\]

System must compute:

\[
\mathbf{d}(z)
=
\Phi(\mathbf{d}(x_1),\dots,\mathbf{d}(x_n))
\]

and verify:

\[
\mathbf{d}(z)
\text{ matches declared unit dimension}
\]

---

# 10. Dimensionless Classification

A quantity is dimensionless if:

\[
\mathbf{d}(U) = \mathbf{0}
\]

Examples:

- ZT
- Efficiency
- Ratios
- Logarithmic measures

Dimensionless status must be explicitly declared and validated.

---

# 11. Model-Level Validation

Before model training:

\[
X \in \mathbb{R}^{n \times p}
\]

each feature column must satisfy:

\[
\exists U_j \in \mathcal{R}_U
\]

and:

\[
\mathbf{d}(U_j)
\text{ correctly represents physical feature}
\]

No implicit mixing of heterogeneous dimensions allowed.

---

# 12. Aggregation Constraints

For mean:

\[
\bar{x} = \frac{1}{n} \sum_{i=1}^n x_i
\]

Require:

\[
\mathbf{d}(x_i) = \mathbf{d}(x_j)
\quad \forall i,j
\]

Variance:

\[
\mathrm{Var}(x)
\]

Dimension:

\[
\mathbf{d}(\mathrm{Var}(x)) = 2 \mathbf{d}(x)
\]

Standard deviation:

\[
\mathbf{d}(\sigma_x) = \mathbf{d}(x)
\]

---

# 13. Graph and Pipeline Integration

Dimensional validation must occur at:

1. Extraction layer
2. Unit conversion layer
3. Feature engineering
4. Statistical modeling
5. Closed-loop selection

Invariant:

\[
\text{All Numeric Nodes} \models \text{DVS}
\]

---

# 14. Registry Dependency

Validation relies exclusively on:

\[
\mathcal{R}_U^{(t)}
\]

Registry version must be logged.

Dimensional vector computation must be deterministic.

---

# 15. Error Taxonomy

Dimensional system errors classified as:

- DV-01: Addition mismatch
- DV-02: Function domain violation
- DV-03: Fractional exponent misuse
- DV-04: Undeclared dimensionless quantity
- DV-05: Derived dimension inconsistency
- DV-06: Affine misuse

Each error must contain:

- Operation trace
- Operand dimensions
- Registry version

---

# 16. Formal Soundness Property

The unit system is dimensionally sound if:

\[
\forall \text{ expressions } E:
\quad
\mathbf{d}(E)
\text{ is well-defined}
\]

and:

\[
\text{no invalid operation passes validation}
\]

---

# 17. Deterministic Guarantee

Given identical:

- Unit registry version,
- Expression structure,
- Operand dimensions,

Validation result must satisfy:

\[
\text{DVS}(E) = \text{constant}
\]

No stochastic behavior permitted.

---

# 18. Computational Complexity

Dimensional validation must execute in:

\[
\mathcal{O}(n)
\]

where \( n \) is expression tree size.

Caching allowed but must preserve correctness.

---

# 19. Strategic Interpretation

Dimensional validation at the unit-system layer ensures:

- Algebraic physical correctness,
- Protection against silent dimensional drift,
- Model interpretability,
- Cross-dataset comparability.

Dimensional inconsistency is a structural error, not a formatting mistake.

---

# 20. Concluding Statement

Every arithmetic, statistical, and modeling operation within the Thermognosis Engine must satisfy:

\[
E \models \text{SPEC-UNIT-DIM-VALIDATION}
\]

Dimensional correctness is a non-negotiable invariant.

It is the mathematical foundation upon which reproducible scientific computation is built.
