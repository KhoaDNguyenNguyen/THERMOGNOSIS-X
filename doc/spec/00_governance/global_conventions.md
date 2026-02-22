# Global Conventions Specification  
**Document ID:** SPEC-GOV-GLOBAL-CONVENTIONS  
**Layer:** spec/00_governance  
**Status:** Normative — System-Wide Semantic and Mathematical Conventions  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Global Conventions (GC)** governing notation, units, symbols, naming standards, indexing rules, and formal semantics across the Thermognosis Engine.

Its objectives are:

1. To eliminate ambiguity across physics, statistics, graph, and closed-loop layers.
2. To enforce mathematical consistency.
3. To standardize symbol usage in all specifications and code.
4. To guarantee interoperability between modules.
5. To maintain publication-level academic rigor.

This document is binding for all specification files and implementation modules.

---

# 2. Mathematical Style Convention

## 2.1 Symbol Categories

We adopt the following semantic partition:

- Scalars: italic lowercase \( x, y, z, T, S \)
- Vectors: bold lowercase \( \mathbf{x}, \mathbf{y} \)
- Matrices: bold uppercase \( \mathbf{K}, \mathbf{\Sigma} \)
- Sets: calligraphic \( \mathcal{D}, \mathcal{M}, \mathcal{G} \)
- Operators: uppercase Roman \( \mathrm{IG}, \mathrm{KL} \)
- Random variables: uppercase italic \( X, Y \)

No symbol may change meaning across documents.

---

## 2.2 Function Convention

Function notation:

\[
f : \mathcal{X} \rightarrow \mathcal{Y}
\]

Explicit domain and codomain required for all core mappings.

---

# 3. Indexing Convention

## 3.1 Data Indexing

Dataset:

\[
\mathcal{D} = \{ (x_i, y_i) \}_{i=1}^n
\]

Index \( i \) always denotes observation index.

---

## 3.2 Time Indexing

Closed-loop iteration:

\[
\mathcal{D}_t
\]

Time index always subscript \( t \).

---

## 3.3 Model Indexing

Model class:

\[
\mathcal{M}_k
\]

Index \( k \) reserved for model variants.

---

# 4. Units Convention

All physical quantities must use SI units unless explicitly stated.

| Quantity | Symbol | Unit |
|----------|--------|------|
| Seebeck coefficient | \( S \) | V/K |
| Electrical conductivity | \( \sigma \) | S/m |
| Thermal conductivity | \( \kappa \) | W/(m·K) |
| Temperature | \( T \) | K |
| Figure of merit | \( zT \) | dimensionless |

No mixed unit expressions allowed.

---

# 5. zT Convention

Definition:

\[
zT = \frac{S^2 \sigma T}{\kappa}
\]

Rules:

- Always dimensionless.
- Must use consistent SI units.
- No unit conversion inside formula without documentation.

---

# 6. Probability Convention

## 6.1 Distribution Notation

Gaussian:

\[
X \sim \mathcal{N}(\mu, \sigma^2)
\]

Multivariate Gaussian:

\[
\mathbf{X} \sim \mathcal{N}(\boldsymbol{\mu}, \mathbf{\Sigma})
\]

---

## 6.2 Conditional Notation

Conditional probability:

\[
p(y \mid x, \theta)
\]

Posterior:

\[
p(\theta \mid \mathcal{D})
\]

No overloaded shorthand permitted.

---

# 7. Entropy and Information

Entropy:

\[
H[p] = - \int p(x) \log p(x) dx
\]

KL divergence:

\[
\mathrm{KL}(p \| q)
=
\int p(x) \log \frac{p(x)}{q(x)} dx
\]

Logarithm base must be specified (default: natural log).

---

# 8. Covariance Convention

Covariance matrix:

\[
\mathbf{\Sigma}
=
\mathbb{E}[(\mathbf{X} - \boldsymbol{\mu})(\mathbf{X} - \boldsymbol{\mu})^T]
\]

Diagonal elements:

\[
\mathbf{\Sigma}_{ii} = \sigma_i^2
\]

Correlation coefficient:

\[
\rho_{ij}
=
\frac{\Sigma_{ij}}{\sigma_i \sigma_j}
\]

---

# 9. Matrix Operations Convention

Inverse:

\[
\mathbf{K}^{-1}
\]

Log-determinant:

\[
\log |\mathbf{K}|
\]

Stabilized inversion:

\[
\mathbf{K} \leftarrow \mathbf{K} + \epsilon \mathbf{I}
\]

Regularization parameter \( \epsilon \) must be logged.

---

# 10. Complexity Notation

Time complexity:

\[
\mathcal{O}(n^3)
\]

Space complexity:

\[
\mathcal{O}(n^2)
\]

Asymptotic notation must use standard Big-O formalism.

---

# 11. Graph Convention

Graph:

\[
\mathcal{G} = (\mathcal{V}, \mathcal{E})
\]

Nodes:

\[
v \in \mathcal{V}
\]

Edges:

\[
(v_i \rightarrow v_j) \in \mathcal{E}
\]

Adjacency matrix:

\[
A_{ij}
=
\begin{cases}
1 & \text{if edge exists} \\
0 & \text{otherwise}
\end{cases}
\]

---

# 12. Loss and Likelihood Convention

Likelihood:

\[
\mathcal{L}(\theta)
=
\prod_i p(y_i | x_i, \theta)
\]

Log-likelihood:

\[
\log \mathcal{L}(\theta)
=
\sum_i \log p(y_i | x_i, \theta)
\]

Weighted likelihood:

\[
\mathcal{L}_w
=
\prod_i p(y_i | x_i, \theta)^{w_i}
\]

---

# 13. Error Propagation Convention

First-order propagation:

\[
\sigma_z^2
=
\sum_i
\left(
\frac{\partial z}{\partial x_i}
\right)^2
\sigma_{x_i}^2
\]

Covariance-aware form:

\[
\sigma_z^2
=
\nabla g^T \mathbf{\Sigma} \nabla g
\]

---

# 14. Random Seed Convention

Random seed symbol:

\[
s \in \mathbb{N}
\]

Seed must be stored for reproducibility.

---

# 15. Naming Convention (Code Level)

- Snake case for functions.
- PascalCase for classes.
- ALL_CAPS for constants.
- Module names reflect spec ID.

Example:

`S02_weighted_likelihood.py`

---

# 16. File Naming Convention

Specification files: [LAYER][IDENTIFIER][topic].md


Examples:

P01_thermoelectric_equations.md
S03_uncertainty_weighting.md
CL02_information_gain_selection.md



---

# 17. Theorem Reference Convention

All formal claims must reference registry ID:

\[
\text{Theorem T3}
\]

Implementation must include docstring tag: Implements: T3


---

# 18. Figure Convention

Figures must:

- Label axes with units.
- Distinguish uncertainty types.
- Include dataset version.
- Avoid ambiguous legends.

---

# 19. Logging Convention

All modules must log:

1. Dataset hash.
2. Spec version.
3. Random seed.
4. Numerical stabilization parameter.
5. Execution timestamp.

---

# 20. Default Assumptions

Unless otherwise stated:

- Gaussian noise model.
- Independence of measurements.
- SI units.
- Natural logarithm.
- Deterministic seed.

Any deviation must be explicitly declared.

---

# 21. Consistency Invariant

Across entire system:

\[
\text{Symbol Meaning} = \text{Constant}
\]

No symbol reuse with different semantics permitted.

---

# 22. Governance Audit Rule

Periodic audit must verify:

- Notation consistency.
- Unit compliance.
- Mathematical symbol uniformity.
- Log completeness.
- Seed determinism.

---

# 23. Strategic Interpretation

Global conventions are not cosmetic formatting rules.

They are:

- The semantic backbone of the system,
- The glue between mathematics and implementation,
- The guarantee of interoperability across modules.

Without global conventions,  
large-scale scientific systems become inconsistent and fragile.

---

# 24. Concluding Statement

The Thermognosis Engine is a mathematically grounded scientific infrastructure.

Global conventions ensure that:

- Every equation means exactly what it states,
- Every symbol carries one meaning,
- Every module interprets data identically.

Scientific rigor requires semantic discipline.

Consistency is not optional —  
it is structural integrity.




