# CONTRACT — Versioning and Lineage  
**Document ID:** SPEC-CONTRACT-VERSIONING  
**Layer:** spec/01_contracts  
**Status:** Normative — Version Control & Lineage Governance Contract  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Versioning and Lineage Contract (VLC)** governing:

- Entity version control
- Data evolution
- Model evolution
- Specification evolution
- Reproducibility guarantees
- Audit trail integrity

Its objectives are:

1. To ensure strict reproducibility across time.
2. To prevent silent overwriting of scientific artifacts.
3. To formalize data and model lineage.
4. To enable deterministic reconstruction of results.
5. To support long-term scalability of the system.

Scientific infrastructure without version control is epistemically unstable.

---

# 2. Foundational Principle

Every mutable entity must satisfy:

\[
\text{Entity}^{(v+1)} \neq \text{Entity}^{(v)}
\]

No entity may be modified in place.

All changes create a new version.

---

# 3. Formal Version Representation

Each versioned entity is defined as:

\[
\mathcal{E}^{(v)}
=
(\text{ID}, v, \mathcal{C}, \mathcal{L})
\]

where:

- \( \text{ID} \): stable root identifier
- \( v \): semantic version number
- \( \mathcal{C} \): content
- \( \mathcal{L} \): lineage metadata

Root ID invariant:

\[
\text{ID}^{(v)} = \text{ID}^{(v+1)}
\]

Version number distinguishes state.

---

# 4. Semantic Versioning Standard

Version format: MAJOR.MINOR.PATCH

Formal interpretation:

- MAJOR: breaking change
- MINOR: backward-compatible feature addition
- PATCH: correction or metadata update

Version ordering:

\[
v_1 < v_2 \iff
\text{lexicographic comparison}
\]

---

# 5. Entity Types Subject to Versioning

Mandatory versioning applies to:

- Material entities
- Raw measurements
- Validated measurements
- Paper entities
- Datasets
- Models
- Specifications

No exception permitted.

---

# 6. Lineage Graph

Version evolution represented as directed acyclic graph:

\[
\mathcal{G}_{\text{version}} = (\mathcal{V}, \mathcal{E})
\]

Node:

\[
\mathcal{E}^{(v)}
\]

Edge:

\[
\mathcal{E}^{(v)} \rightarrow \mathcal{E}^{(v+1)}
\]

Graph must satisfy:

\[
\text{DAG property}
\]

No cyclic versioning allowed.

---

# 7. Dataset Versioning

Dataset defined as:

\[
\mathcal{D}^{(v)}
=
\{ \mathcal{V}_i^{(v_i)} \}
\]

Dataset hash:

\[
H_{\mathcal{D}}
=
\mathrm{Hash}(\text{canonical serialization})
\]

Invariant:

\[
\mathcal{D}^{(v_1)} \neq \mathcal{D}^{(v_2)}
\Rightarrow
H_{\mathcal{D}_1} \neq H_{\mathcal{D}_2}
\]

---

# 8. Model Versioning

Model entity:

\[
\mathcal{M}^{(v)}
=
(\theta^{(v)}, \text{hyperparameters}^{(v)}, \mathcal{D}^{(v)})
\]

Model hash:

\[
H_{\mathcal{M}}
=
\mathrm{Hash}(\theta, \text{hyperparameters}, H_{\mathcal{D}})
\]

Reproducibility invariant:

\[
H_{\mathcal{M}_1} = H_{\mathcal{M}_2}
\Rightarrow
\text{Model outputs identical}
\]

---

# 9. Specification Versioning

Specification version: SPEC_VERSION = vX.Y.Z


All entities must log:

\[
\text{spec\_version}
\]

Results must be reproducible under identical spec version.

---

# 10. Lineage Metadata

Each versioned entity must record:

\[
\mathcal{L}
=
(\text{timestamp}, \text{author}, \text{reason}, \text{parent\_version})
\]

Lineage invariant:

\[
\text{parent\_version} = v
\]

except for initial creation.

---

# 11. Reproducibility Equation

A computational artifact must satisfy:

\[
\text{Artifact}
=
f(
\mathcal{D}^{(v)},
\mathcal{M}^{(v)},
\text{Spec}^{(v)},
\text{Seed}
)
\]

If all inputs identical, output must be identical.

---

# 12. Seed Versioning

Random seed:

\[
s \in \mathbb{N}
\]

Seed must be stored in lineage metadata.

Invariant:

\[
s_1 = s_2
\Rightarrow
\text{stochastic processes deterministic}
\]

---

# 13. Change Impact Analysis

If entity \( \mathcal{E}^{(v)} \) updated:

Affected set:

\[
\mathcal{A}
=
\{ X \mid X \text{ depends on } \mathcal{E}^{(v)} \}
\]

All affected artifacts must be recomputed.

---

# 14. Immutable History Rule

Deletion of historical version prohibited.

Soft-deprecation allowed:

\[
\text{status} = \text{deprecated}
\]

But entity remains accessible.

---

# 15. Backward Compatibility Constraint

For MINOR and PATCH changes:

\[
\mathcal{E}^{(v+1)} \supseteq \mathcal{E}^{(v)}
\]

Backward compatibility must be preserved.

MAJOR change may alter schema.

---

# 16. Auditability Requirement

Audit must reconstruct:

1. Exact dataset
2. Exact model parameters
3. Exact specification version
4. Exact seed
5. Exact computational environment

Failure to reconstruct indicates governance violation.

---

# 17. Hash Determinism

Hash function must satisfy:

\[
\mathrm{Hash}(x) = \mathrm{Hash}(y)
\iff
x = y
\]

Canonical serialization required.

Floating-point precision must be fixed.

---

# 18. Version Conflict Resolution

If concurrent versions exist:

Branch structure:

\[
v_1 \rightarrow v_{2a}, v_{2b}
\]

Merge allowed only if:

\[
\mathcal{C}_{2a} \cap \mathcal{C}_{2b} = \varnothing
\]

Otherwise manual resolution required.

---

# 19. Prohibited Practices

The following are forbidden:

1. In-place modification.
2. Silent metadata update.
3. Changing data without version increment.
4. Overwriting model parameters.
5. Undocumented schema change.

---

# 20. Compliance Requirement

All versioned entities must satisfy:

\[
\mathcal{E}^{(v)} \models \text{SPEC-CONTRACT-VERSIONING}
\]

Non-compliance blocks merge or ingestion.

---

# 21. Strategic Interpretation

Versioning is not administrative overhead.

It is:

- A mathematical guarantee of reproducibility,
- A structural defense against data drift,
- A foundation of scientific credibility,
- A prerequisite for Q1 publication standard.

Without strict version control,  
results cannot be defended.

---

# 22. Concluding Statement

In the Thermognosis Engine, every scientific object evolves —  
but never mutates silently.

Versioning ensures that:

- Every result can be reconstructed,
- Every correction is transparent,
- Every dataset is historically traceable,
- Every model is reproducible,
- Every decision is defensible.

Scientific integrity requires temporal integrity.

Versioning is the backbone of long-term epistemic stability.




