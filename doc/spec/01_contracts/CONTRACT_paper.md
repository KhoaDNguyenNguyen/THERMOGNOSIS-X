# CONTRACT — Scientific Paper Entity  
**Document ID:** SPEC-CONTRACT-PAPER  
**Layer:** spec/01_contracts  
**Status:** Normative — Publication Metadata & Knowledge Contract  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Scientific Paper Contract (SPC)** governing the formal representation of published literature within the Thermognosis Engine.

Its objectives are:

1. To provide a canonical representation of scientific publications.
2. To ensure traceable linkage between materials, measurements, and claims.
3. To formalize citation graph construction.
4. To prevent duplication and metadata drift.
5. To guarantee reproducibility and auditability of knowledge extraction.

A paper is not merely a citation string.  
It is a structured epistemic object.

---

# 2. Formal Definition

A scientific paper entity is defined as:

\[
\mathcal{P}
=
(\text{ID}, \mathcal{B}, \mathcal{A}, \mathcal{C}, \mathcal{M}, \mathcal{E}, \mathcal{R})
\]

where:

- \( \text{ID} \): unique paper identifier
- \( \mathcal{B} \): bibliographic metadata
- \( \mathcal{A} \): author set
- \( \mathcal{C} \): citation structure
- \( \mathcal{M} \): referenced materials
- \( \mathcal{E} \): extracted experimental data
- \( \mathcal{R} \): reliability and credibility descriptors

All fields mandatory unless explicitly declared optional.

---

# 3. Identity Requirements

## 3.1 Unique Identifier

Paper ID format: PAPER_[DOI_HASH]


Deterministic function:

\[
\text{ID} = \mathrm{Hash}(\text{DOI})
\]

Identity invariant:

\[
\text{DOI}_i = \text{DOI}_j
\Rightarrow
\text{ID}_i = \text{ID}_j
\]

No duplicate DOIs permitted.

---

# 4. Bibliographic Metadata

Bibliographic field:

\[
\mathcal{B}
=
(\text{title}, \text{journal}, \text{year}, \text{volume}, \text{pages})
\]

Year constraint:

\[
\text{year} \in \mathbb{N}, \quad 1900 \le \text{year} \le \text{current year}
\]

Journal name must match canonical registry.

---

# 5. Author Set

Author set:

\[
\mathcal{A}
=
\{ a_k \}_{k=1}^{n}
\]

Each author entry:

\[
a_k = (\text{name}, \text{affiliation})
\]

Ordering preserved as published.

---

# 6. Citation Structure

Citation graph representation:

\[
\mathcal{G}_{\text{cite}} = (\mathcal{V}, \mathcal{E})
\]

Paper node:

\[
v_i \in \mathcal{V}
\]

Citation edge:

\[
(v_i \rightarrow v_j)
\]

if paper \( i \) cites paper \( j \).

Adjacency matrix:

\[
A_{ij}
=
\begin{cases}
1 & \text{if citation exists} \\
0 & \text{otherwise}
\end{cases}
\]

Graph must be acyclic in temporal ordering:

\[
\text{year}(i) \ge \text{year}(j)
\]

---

# 7. Referenced Materials

Material linkage:

\[
\mathcal{M}
=
\{ \text{MAT\_ID}_k \}
\]

Invariant:

\[
\forall m \in \mathcal{M}, \quad
m \models \text{SPEC-CONTRACT-MATERIAL}
\]

No material reference allowed without validated material contract.

---

# 8. Extracted Experimental Data

Extracted dataset:

\[
\mathcal{E}
=
\{ (T_i, S_i, \sigma_i, \kappa_i, \Sigma_i) \}_{i=1}^{n}
\]

Derived:

\[
zT_i
=
\frac{S_i^2 \sigma_i T_i}{\kappa_i}
\]

Uncertainty structure:

\[
\Sigma_i
=
\text{Covariance matrix of measurements}
\]

Each data point must link to:

- Section of paper
- Figure or table reference

---

# 9. Credibility and Reliability Descriptor

Reliability descriptor:

\[
\mathcal{R}
=
(\text{measurement\_method}, \text{instrumentation}, \text{error\_model}, \text{credibility\_score})
\]

Credibility score:

\[
0 \le c \le 1
\]

Defined via Bayesian credibility model:

\[
c = f(\text{reproducibility}, \text{citation impact}, \text{error transparency})
\]

---

# 10. Error Structure Integration

Measurement variance:

\[
\sigma_{\text{meas}}^2
\]

Posterior-weighted variance:

\[
\sigma_{\text{weighted}}^2
=
\frac{1}{c} \sigma_{\text{meas}}^2
\]

Low credibility increases effective variance.

---

# 11. Provenance Traceability

Traceability invariant:

\[
\forall e \in \mathcal{E},
\exists (\text{section}, \text{figure}, \text{page})
\]

No extracted value without source reference.

---

# 12. Immutability Rule

Core bibliographic fields immutable:

- DOI
- Title
- Year
- Journal

Extracted data may be versioned but not overwritten.

---

# 13. Versioning

Paper entity version:

\[
\mathcal{P}^{(v)}
\]

Increment version when:

- New data extraction added
- Credibility updated
- Correction applied

History preserved.

---

# 14. Integration with Statistical Layer

Paper contributes to dataset:

\[
\mathcal{D}
=
\bigcup_{p \in \mathcal{P}} \mathcal{E}_p
\]

Likelihood contribution:

\[
\mathcal{L}
=
\prod_{i}
\mathcal{N}(y_i | \mu_i, \sigma_{i,\text{total}}^2)
\]

where:

\[
\sigma_{i,\text{total}}^2
=
\sigma_{\text{meas}}^2
+
\sigma_{\text{model}}^2
\]

---

# 15. Graph Centrality Metrics

Citation centrality:

\[
\text{PR}(v)
=
\frac{1-d}{N}
+
d \sum_{u \in \text{In}(v)} \frac{\text{PR}(u)}{\deg(u)}
\]

PageRank used for influence estimation.

Centrality may inform credibility prior.

---

# 16. Validation Checklist

Before acceptance:

- ✔ DOI verified  
- ✔ Metadata canonical  
- ✔ Material links validated  
- ✔ Extracted data unit-consistent  
- ✔ Uncertainty provided or estimated  
- ✔ Citation edges constructed  
- ✔ Credibility computed  

---

# 17. Failure Modes

Invalid paper entity may cause:

- Duplicate data counting
- Incorrect citation graph structure
- Inflated confidence
- Structural bias in acquisition policy

Governance must block inconsistent entries.

---

# 18. Serialization Standard

Paper entity serialized to: JSON (canonical order)

Hash computed after canonical serialization.

All numeric values stored with explicit precision.

---

# 19. Cross-Layer Invariant

Paper–Material consistency:

\[
\text{Paper} \rightarrow \text{Material} \rightarrow \text{Dataset}
\]

No data point may exist without parent paper and material linkage.

---

# 20. Compliance Requirement

Every paper object must satisfy:

\[
\mathcal{P} \models \text{SPEC-CONTRACT-PAPER}
\]

Non-compliant papers are rejected from ingestion.

---

# 21. Strategic Interpretation

The scientific paper contract transforms literature into structured knowledge.

It ensures:

- Reproducible extraction,
- Transparent uncertainty,
- Citation-aware credibility,
- Graph-consistent integration,
- Long-term maintainability.

Without this contract, literature ingestion becomes uncontrolled aggregation.

---

# 22. Concluding Statement

In the Thermognosis Engine, a paper is not text —  
it is a formal epistemic unit.

It carries:

- Verified identity,
- Structured metadata,
- Quantified uncertainty,
- Citation dynamics,
- Material linkage.

Only under this contract can the system scale to large literature corpora  
while preserving scientific rigor and publication-grade credibility.




