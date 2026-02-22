# EXT — Material Parser Specification  
**Document ID:** SPEC-EXT-MATERIAL-PARSER  
**Layer:** spec/03_extraction  
**Status:** Normative — Structured Material Entity Extraction Protocol  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Material Parser Specification (MPS)** governing the deterministic extraction, normalization, and structuring of material-related entities from validated scientific documents.

The Material Parser transforms unstructured textual and tabular content into formally defined material entities compliant with:

- SPEC-CONTRACT-MATERIAL  
- SPEC-CONTRACT-RAW-MEASUREMENT  
- SPEC-CONTRACT-VERSIONING  

Material parsing is not simple named-entity recognition.  
It is a constrained semantic reconstruction process under domain ontology and physicochemical rules.

---

# 2. Formal Problem Definition

Let extracted document text be:

\[
T = \{ s_1, s_2, \dots, s_n \}
\]

Let domain ontology be:

\[
\mathcal{O}
=
(\mathcal{E}, \mathcal{R}, \mathcal{C})
\]

where:

- \( \mathcal{E} \): entity types,
- \( \mathcal{R} \): relations,
- \( \mathcal{C} \): constraints.

Material parser defines mapping:

\[
\mathcal{P} : T \rightarrow \mathcal{M}
\]

where:

\[
\mathcal{M}
=
\{ M_i \}_{i=1}^{k}
\]

Each \( M_i \) is a structured material entity.

---

# 3. Material Entity Definition

A material entity is defined as:

\[
M =
(
\text{name},
\text{composition},
\text{structure},
\text{processing},
\text{properties},
\text{provenance}
)
\]

All fields optional but schema must be respected.

---

# 4. Composition Parsing

Chemical composition string:

\[
S_c
\]

Parser must produce normalized representation:

\[
\text{Comp} =
\{ (e_j, \alpha_j) \}
\]

where:

- \( e_j \) = chemical element,
- \( \alpha_j \in \mathbb{R}^+ \) = stoichiometric coefficient.

Normalization condition:

\[
\sum_j \alpha_j = 1
\quad
\text{(if atomic fraction format)}
\]

Example normalization:

\[
\text{Bi}_2\text{Te}_3
\rightarrow
\{ (\text{Bi}, 2/5), (\text{Te}, 3/5) \}
\]

---

# 5. Structure Parsing

Structure information:

\[
S_s
\]

Must map to:

\[
\text{Structure} \in
\{
\text{crystal\_system},
\text{space\_group},
\text{phase}
\}
\]

Consistency constraint:

\[
\text{space\_group} \Rightarrow \text{crystal\_system}
\]

Ontology validation required.

---

# 6. Property Association

Property extraction:

\[
\mathcal{Q}
=
\{ (q_i, v_i, u_i) \}
\]

where:

- \( q_i \): property type,
- \( v_i \): value,
- \( u_i \): unit.

Normalization:

\[
v_i^{SI} =
\text{convert\_to\_SI}(v_i, u_i)
\]

Units must be standardized prior to storage.

---

# 7. Contextual Disambiguation

If material mention ambiguous:

\[
\text{"PbTe"}
\]

Parser must consider context window:

\[
W = \{ s_{k-r}, \dots, s_{k+r} \}
\]

Disambiguation function:

\[
\mathcal{D}(S_c, W) \rightarrow \text{unique material ID}
\]

Ambiguity unresolved → flagged.

---

# 8. Table Parsing

Tabular data:

\[
\mathcal{T} =
\{ r_i \}_{i=1}^{m}
\]

Each row parsed into structured material-property tuple.

Constraint:

\[
|\text{columns}| \ge 2
\]

Table header must map to ontology terms.

---

# 9. Deterministic Parsing

Given identical:

- Text \( T \),
- Parser version \( v \),
- Ontology \( \mathcal{O} \),

Must satisfy:

\[
\mathcal{P}(T; v, \mathcal{O})
=
\mathcal{M}
\]

Bitwise-identical output required.

---

# 10. Uncertainty Extraction

If property reported with uncertainty:

\[
v \pm \sigma
\]

Parser must record:

\[
\sigma
\]

If not provided:

\[
\sigma = \text{None}
\]

No artificial uncertainty inference allowed at parsing stage.

---

# 11. Temperature and Condition Association

Properties must bind to conditions:

\[
(q, v, T, P, \text{environment})
\]

Example:

\[
\kappa = 1.5 \, \text{W/mK at } 300\,K
\]

Temperature must be explicitly stored.

Missing condition flagged as incomplete.

---

# 12. Provenance Encoding

Each parsed material entity must store:

\[
\mathcal{P}
=
(
\text{PDF\_checksum},
\text{page\_number},
\text{section},
\text{sentence\_index}
)
\]

Provenance mandatory.

---

# 13. Conflict Detection

If two statements:

\[
v_1 \neq v_2
\]

For same material and property under identical conditions:

Conflict flag:

\[
\Delta v = |v_1 - v_2|
\]

Conflict stored; not auto-resolved.

---

# 14. Ontology Compliance

Parsed entity must satisfy:

\[
M \models \mathcal{O}
\]

Violation → rejection or flagged for review.

---

# 15. Error Handling

Failure types:

\[
F \in
\{
\text{InvalidComposition},
\text{UnitUnknown},
\text{AmbiguousEntity},
\text{OntologyMismatch}
\}
\]

Each failure must produce structured log entry.

---

# 16. Hashing and Identity

Material entity checksum:

\[
H_M = H(\mathcal{S}(M))
\]

Identity invariant:

\[
H_{M_1} = H_{M_2}
\iff
M_1 = M_2
\]

Content-addressable storage required.

---

# 17. Version Coupling

Parsed entity must include:

\[
\text{parser\_version}
\]

Change in parser logic → new version required.

---

# 18. Security Constraints

Parser must operate in:

- Sandboxed environment.
- No external API calls.
- No dynamic code execution.
- Deterministic runtime environment.

---

# 19. Compliance Requirement

Material entity \( M \) accepted only if:

\[
M \models \text{SPEC-CONTRACT-MATERIAL}
\land
M \models \text{SPEC-ACQ-CHECKSUM}
\land
M \models \text{SPEC-CONTRACT-VERSIONING}
\]

Non-compliance blocks ingestion.

---

# 20. Strategic Interpretation

The Material Parser transforms narrative scientific language into structured knowledge.

It ensures:

- Ontological consistency,
- Stoichiometric correctness,
- Unit normalization,
- Condition-aware property binding,
- Deterministic reproducibility.

Parsing is semantic reconstruction under formal constraints.

---

# 21. Concluding Statement

In the Thermognosis Engine, material extraction must be:

- Ontology-aligned,
- Unit-consistent,
- Provenance-bound,
- Cryptographically identifiable,
- Deterministically reproducible.

Material knowledge is foundational to the graph model.

Its extraction must meet the highest academic and infrastructural standards.
