# EXT — Table Extractor Specification  
**Document ID:** SPEC-EXT-TABLE-EXTRACTOR  
**Layer:** spec/03_extraction  
**Status:** Normative — Structured Tabular Data Extraction Protocol  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Table Extractor Specification (TES)** governing deterministic extraction of structured tabular data from validated PDF documents and other scientific sources.

The Table Extractor converts visual or semi-structured tables into formally defined, schema-compliant datasets suitable for:

- Material-property ingestion,
- Model training,
- Statistical analysis,
- Knowledge graph integration.

Tabular extraction must satisfy structural rigor comparable to experimental measurement acquisition.

---

# 2. Formal Problem Definition

Let validated document page image be:

\[
I : \Omega \subset \mathbb{R}^2 \rightarrow \mathbb{R}^3
\]

Let detected table region be:

\[
\Omega_T \subset \Omega
\]

Extraction mapping:

\[
\mathcal{E}_T : \Omega_T \rightarrow \mathcal{T}
\]

where:

\[
\mathcal{T} =
\{ r_i \}_{i=1}^{m}
\]

and each row:

\[
r_i = (c_{i1}, c_{i2}, \dots, c_{in})
\]

with \( n \) columns.

---

# 3. Table Detection

Table detection function:

\[
\mathcal{D}_T(I) \rightarrow \Omega_T
\]

Detection strategies may include:

- Line detection (Hough transform),
- Grid structure recognition,
- Text alignment clustering.

Detection must be deterministic:

\[
\mathcal{D}_T(I; v) = \Omega_T
\]

for fixed extractor version \( v \).

---

# 4. Structural Reconstruction

Extracted table must satisfy rectangular structure:

\[
\exists m,n \in \mathbb{N}
\quad
\text{s.t.}
\quad
|\mathcal{T}| = m
\quad
\land
\quad
|r_i| = n
\]

Merged cells must be expanded into normalized form.

---

# 5. Header Identification

Header row(s):

\[
H = (h_1, h_2, \dots, h_n)
\]

Header mapping function:

\[
\mathcal{M}_H : H \rightarrow \mathcal{O}
\]

where \( \mathcal{O} \) is domain ontology.

Each column must map to defined semantic type.

Unmapped header → flagged.

---

# 6. Data Type Inference

Cell content:

\[
c_{ij}
\]

Type inference:

\[
\mathcal{T}(c_{ij}) \in
\{
\text{numeric},
\text{categorical},
\text{text},
\text{uncertainty},
\text{unit}
\}
\]

Numeric parsing must support:

\[
v \pm \sigma
\]

Extract:

\[
(v, \sigma)
\]

---

# 7. Unit Normalization

If column header contains unit \( u \):

\[
v^{SI} =
\text{convert\_to\_SI}(v, u)
\]

Normalization invariant:

\[
\text{store}(v^{SI}, \text{SI})
\]

Original unit preserved in metadata.

---

# 8. Multi-Page Tables

If table spans pages:

\[
\mathcal{T} =
\bigcup_{k=1}^{p}
\mathcal{T}^{(k)}
\]

Concatenation allowed only if:

\[
H^{(k)} = H^{(k+1)}
\]

Header consistency required.

---

# 9. Missing Value Handling

If cell empty:

\[
c_{ij} = \varnothing
\]

Represent as:

\[
\text{NULL}
\]

No silent imputation allowed at extraction stage.

---

# 10. Numeric Precision Constraint

Let original precision be:

\[
d = \text{decimal places in source}
\]

Extractor must preserve:

\[
\text{round}(v, d)
\]

No artificial precision enhancement allowed.

---

# 11. Uncertainty Propagation

If table contains derived value:

\[
z = f(x, y)
\]

Uncertainty:

\[
\sigma_z^2
=
\left(
\frac{\partial f}{\partial x}
\right)^2
\sigma_x^2
+
\left(
\frac{\partial f}{\partial y}
\right)^2
\sigma_y^2
\]

Extractor does not recompute unless explicitly specified.  
It only captures reported uncertainty.

---

# 12. OCR Integration

If table image-based:

Text extraction:

\[
T = \mathcal{O}(\Omega_T)
\]

OCR confidence score:

\[
\gamma \in [0,1]
\]

If:

\[
\gamma < \gamma_{\min}
\]

Table flagged for review.

---

# 13. Provenance Metadata

Each extracted table must record:

\[
\mathcal{P}
=
(
\text{PDF\_checksum},
\text{page\_range},
\text{table\_index},
\text{extractor\_version}
)
\]

Checksum of structured table:

\[
H_{\mathcal{T}} = H(\mathcal{S}(\mathcal{T}))
\]

---

# 14. Deterministic Output Requirement

Given identical:

- Input document,
- Extraction parameters,
- Extractor version,

Must satisfy:

\[
\mathcal{E}_T(I) = \mathcal{T}
\]

Bitwise-identical structured output required.

---

# 15. Schema Validation

Extracted table must satisfy:

\[
\mathcal{T} \models \mathcal{S}_{schema}
\]

Schema derived from ontology.

Violation → rejection or flagged.

---

# 16. Conflict Detection

If duplicate row entries:

\[
r_i = r_j
\quad
\land
\quad
i \neq j
\]

Duplicates must be:

- Retained if present in source.
- Annotated with row index.

No deduplication without governance.

---

# 17. Error Classification

Extraction failures:

\[
F \in
\{
\text{StructureFailure},
\text{HeaderAmbiguity},
\text{OCRLowConfidence},
\text{UnitUnknown},
\text{SchemaViolation}
\}
\]

Each logged with full context.

---

# 18. Security Constraints

Table extraction must operate in:

- Sandboxed PDF/image environment.
- Memory-limited execution.
- No external network access.
- Deterministic runtime.

---

# 19. Compliance Requirement

Extracted table \( \mathcal{T} \) accepted only if:

\[
\mathcal{T} \models \text{SPEC-ACQ-CHECKSUM}
\land
\mathcal{T} \models \text{SPEC-CONTRACT-RAW-MEASUREMENT}
\land
\mathcal{T} \models \text{SPEC-CONTRACT-VERSIONING}
\]

Non-compliance blocks ingestion.

---

# 20. Strategic Interpretation

The Table Extractor transforms structured scientific evidence into machine-readable datasets.

It guarantees:

- Structural integrity,
- Unit consistency,
- Precision preservation,
- Deterministic reconstruction,
- Full provenance traceability.

Tables are compressed knowledge.  
Their extraction must preserve scientific fidelity.

---

# 21. Concluding Statement

In the Thermognosis Engine, tabular data extraction must be:

- Structurally validated,
- Ontology-aligned,
- Numerically precise,
- Uncertainty-aware,
- Cryptographically identifiable,
- Fully reproducible.

Tabular evidence underpins statistical modeling and graph construction.

Its extraction must meet the highest academic and infrastructural standards.
