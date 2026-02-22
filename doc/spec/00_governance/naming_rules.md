# Naming Rules Specification  
**Document ID:** SPEC-GOV-NAMING-RULES  
**Layer:** spec/00_governance  
**Status:** Normative — System-Wide Naming and Identifier Governance  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Naming Rules (NR)** governing identifiers across:

- Specification documents
- Source code
- Mathematical notation
- Data artifacts
- Experimental records
- Figures and tables
- Theorem registry

Its objectives are:

1. To ensure semantic consistency across the entire system.
2. To eliminate ambiguity between mathematical and computational objects.
3. To preserve traceability from theory to implementation.
4. To maintain long-term scalability of the architecture.
5. To satisfy publication-level rigor and auditability.

Naming is not cosmetic.  
It is a structural encoding of meaning.

---

# 2. Foundational Principle

Each identifier must satisfy:

\[
\text{Identifier} \leftrightarrow \text{Unique Semantic Object}
\]

No two distinct semantic objects may share the same identifier within the same scope.

---

# 3. Global Identifier Uniqueness

Let:

\[
\mathcal{I} = \{ \text{all identifiers} \}
\]

The following invariant must hold:

\[
\forall i, j \in \mathcal{I}, \quad
i = j \Rightarrow \text{Semantics}(i) = \text{Semantics}(j)
\]

No overloaded meaning allowed.

---

# 4. Mathematical Symbol Naming Rules

## 4.1 Scalar Variables

Scalars must use:

\[
x, y, z, T, S, \sigma, \kappa
\]

Rules:

- Italic in documentation.
- Single-letter for atomic physical quantities.
- Descriptive subscripts permitted:

\[
\sigma_{\text{meas}}, \quad \sigma_{\text{model}}
\]

---

## 4.2 Vectors and Matrices

Vectors:

\[
\mathbf{x}, \mathbf{y}
\]

Matrices:

\[
\mathbf{K}, \mathbf{\Sigma}
\]

Bold formatting mandatory in LaTeX documentation.

---

## 4.3 Sets and Spaces

Sets:

\[
\mathcal{D}, \mathcal{M}, \mathcal{G}
\]

Calligraphic notation reserved exclusively for:

- Datasets
- Model classes
- Graph structures

---

# 5. Reserved Symbol Registry

The following symbols are globally reserved:

| Symbol | Meaning |
|--------|---------|
| \( S \) | Seebeck coefficient |
| \( \sigma \) | Electrical conductivity |
| \( \kappa \) | Thermal conductivity |
| \( T \) | Temperature |
| \( zT \) | Thermoelectric figure of merit |
| \( \mathcal{D} \) | Dataset |
| \( \theta \) | Model parameters |
| \( \mathbf{K} \) | Kernel matrix |
| \( \mathbf{\Sigma} \) | Covariance matrix |
| \( \mathrm{IG} \) | Information gain |

These meanings must never change.

---

# 6. File Naming Convention

All specification files must follow: [LAYER][INDEX][topic].md

Examples:

P01_thermoelectric_equations.md
S04_outlier_modeling.md
G02_citation_graph_dynamics.md
CL03_convergence_conditions.md


Where:

- `P` = Physics
- `S` = Statistical
- `G` = Graph
- `CL` = Closed Loop
- `SPEC-GOV` = Governance

---

# 7. Module Naming Convention (Code)

## 7.1 File Names

Snake case only:

weighted_likelihood.py
error_propagation.py
material_identity_graph.py


No camelCase in file names.

---

## 7.2 Class Names

PascalCase:

WeightedLikelihoodModel
MaterialIdentityGraph
InformationGainSelector


---

## 7.3 Function Names

Snake case:

compute_information_gain()
propagate_uncertainty()
regularize_kernel_matrix()


Verb-first naming preferred.

---

## 7.4 Constants

All caps:

DEFAULT_SEED
NUMERICAL_EPSILON
MAX_ITERATIONS

---

# 8. Index Naming Convention

Observation index:

\[
i = 1, \dots, n
\]

Time index:

\[
t = 0, 1, 2, \dots
\]

Model index:

\[
k = 1, \dots, K
\]

Graph node index:

\[
v_i \in \mathcal{V}
\]

Index symbols must not be reused for different semantic roles in same derivation.

---

# 9. Parameter Naming Convention

Parameter vector:

\[
\theta
\]

Individual parameter:

\[
\theta_j
\]

Hyperparameters:

\[
\lambda, \alpha, \beta
\]

Posterior:

\[
p(\theta \mid \mathcal{D})
\]

Never mix parameter and hyperparameter notation.

---

# 10. Dataset Naming Convention

Dataset:

\[
\mathcal{D}_t
=
\{ (x_i, y_i) \}_{i=1}^{n_t}
\]

Versioned dataset:

\[
\mathcal{D}^{(v)}
\]

Dataset hash must be recorded in logs.

---

# 11. Experiment Naming Convention

Experiment ID format: EXP_[YYYYMMDD]_[SHORT_DESCRIPTION]


Example: EXP_20260221_zT_uncertainty_study

All experimental artifacts must include experiment ID.

---

# 12. Graph Naming Convention

Graph object:

\[
\mathcal{G} = (\mathcal{V}, \mathcal{E})
\]

Node:

\[
v_i
\]

Edge:

\[
(v_i \rightarrow v_j)
\]

Adjacency matrix:

\[
A_{ij}
\]

Graph-related code classes must include suffix `Graph`.

---

# 13. Acquisition Policy Naming

Acquisition function:

\[
a(x)
\]

Information gain:

\[
\mathrm{IG}(x)
\]

Upper confidence bound:

\[
\mathrm{UCB}(x)
\]

Functions must use descriptive names:

compute_information_gain()
select_next_experiment()


---

# 14. Error Term Naming

Measurement error:

\[
\epsilon_{\text{meas}}
\]

Model error:

\[
\epsilon_{\text{model}}
\]

Total error:

\[
\epsilon_{\text{total}}
\]

Do not use ambiguous names like `err`, `e1`, `noise`.

---

# 15. Logging Identifier Convention

All logs must include keys:

spec_version
dataset_hash
random_seed
commit_hash
numerical_epsilon



Key names standardized.

---

# 16. Prohibited Naming Practices

The following are forbidden:

1. Abbreviations without definition.
2. Single-letter variables in code (except loop indices).
3. Reusing symbol for different meaning.
4. Changing symbol meaning across files.
5. Implicit renaming during refactoring.

---

# 17. Refactoring Rule

If renaming required:

1. Update specification.
2. Update code.
3. Update tests.
4. Update documentation.
5. Log change in governance registry.

Renaming without traceability prohibited.

---

# 18. Cross-Layer Naming Consistency

If specification defines:

\[
\mathbf{K}
\]

Code must use: K or: kernel_matrix 
but not arbitrary alias.

Mapping must be documented.

---

# 19. Theorem Naming Convention

Theorem registry: T1, T2, T3, ...

Reference format:

\[
\text{Theorem T3}
\]

Code docstring: Implements: T3

---

# 20. Version Tag Naming

Specification version: v1.0.0

Semantic versioning:

\[
\text{MAJOR.MINOR.PATCH}
\]

Breaking change increments MAJOR.

---

# 21. Consistency Invariant

Naming consistency invariant:

\[
\text{Name Stability} \Rightarrow \text{Semantic Stability}
\]

Frequent renaming indicates architectural instability.

---

# 22. Governance Audit

Audit checklist:

- Identifier uniqueness
- Symbol consistency
- File naming compliance
- Code-style compliance
- Mapping between spec and code verified

Non-compliance blocks merge.

---

# 23. Strategic Interpretation

Naming encodes epistemology.

A stable naming system:

- Preserves mathematical clarity,
- Reduces cognitive load,
- Enables large-scale collaboration,
- Supports reproducibility,
- Strengthens publication credibility.

Without naming discipline,  
scientific software degenerates into ambiguity.

---

# 24. Concluding Statement

The Thermognosis Engine is a mathematically structured scientific system.

Naming rules ensure that:

- Every symbol carries one meaning,
- Every module reflects formal definitions,
- Every experiment remains traceable,
- Every result is interpretable.

Precision in naming is precision in thought.

In a research-grade infrastructure,  
identifiers are part of the theory.














