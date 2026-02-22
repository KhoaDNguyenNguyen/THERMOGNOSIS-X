# EXT — Text Extractor Specification  
**Document ID:** SPEC-EXT-TEXT-EXTRACTOR  
**Layer:** spec/03_extraction  
**Status:** Normative — Deterministic Scientific Text Extraction Protocol  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Text Extractor Specification (TES-X)** governing deterministic extraction, normalization, and structural segmentation of textual content from validated PDF documents and related scientific sources.

The Text Extractor provides the foundational textual representation required for:

- Material parsing,
- Property extraction,
- Citation graph construction,
- Semantic indexing,
- Knowledge graph population.

Text extraction is not mere string recovery.  
It is a controlled reconstruction of semantic structure under strict determinism and provenance constraints.

---

# 2. Formal Problem Definition

Let validated PDF artifact be:

\[
\mathcal{P}
\]

Let raw byte stream:

\[
B = (b_1, b_2, \dots, b_n)
\]

Text extraction defines mapping:

\[
\mathcal{E}_X : \mathcal{P} \rightarrow T
\]

where:

\[
T = \{ s_1, s_2, \dots, s_m \}
\]

and each \( s_i \) is a structured textual unit (paragraph, sentence, or tokenized segment).

---

# 3. Extraction Layers

Text extraction consists of layered transformation:

\[
\mathcal{E}_X =
\mathcal{N} \circ \mathcal{S} \circ \mathcal{R}
\]

Where:

- \( \mathcal{R} \): Raw text recovery from PDF objects.
- \( \mathcal{S} \): Structural segmentation.
- \( \mathcal{N} \): Normalization.

Each layer must be deterministic.

---

# 4. Raw Text Recovery

Let text objects be:

\[
\mathcal{O}_{text} =
\{ O_i \subset \mathcal{P} \}
\]

Recovered string:

\[
T_{raw} =
\bigcup_i \text{decode}(O_i)
\]

Constraints:

- Respect reading order inferred from layout.
- Preserve Unicode encoding (UTF-8).
- No heuristic content alteration.

---

# 5. Layout-Aware Ordering

Let each text block have bounding box:

\[
B_i = (x_i, y_i, w_i, h_i)
\]

Ordering rule:

Primary sort:

\[
y_i \downarrow
\]

Secondary sort:

\[
x_i \uparrow
\]

Deterministic ordering invariant:

\[
\text{Order}(B_i) = \text{stable}
\]

---

# 6. Structural Segmentation

Segmentation function:

\[
\mathcal{S}(T_{raw}) \rightarrow 
\{
\text{Title},
\text{Abstract},
\text{Sections},
\text{References}
\}
\]

Each section must be explicitly labeled.

Section boundary detection based on:

- Font size,
- Boldness,
- Pattern matching (e.g., numbering),
- Layout features.

Segmentation must be deterministic.

---

# 7. Sentence Tokenization

Sentence tokenizer:

\[
\mathcal{T}_s : s_i \rightarrow \{ s_{i1}, \dots, s_{ik} \}
\]

Constraints:

- Preserve decimal numbers.
- Avoid splitting chemical formulas.
- Preserve equation references.

Determinism condition:

\[
\mathcal{T}_s(s; v) = \text{stable}
\]

---

# 8. Mathematical Expression Handling

Inline mathematical expression:

\[
E \subset T_{raw}
\]

Extractor must:

1. Preserve LaTeX-equivalent representation if available.
2. Encode equation position metadata.

Example:

\[
\kappa = \frac{1}{3} C_v v l
\]

Stored as structured equation block.

No symbolic simplification at extraction stage.

---

# 9. Unicode Normalization

Normalization function:

\[
\mathcal{N}_u : T \rightarrow T'
\]

Apply:

- NFC normalization,
- Removal of non-printable control characters,
- Standardization of quotation marks.

Invariant:

\[
\mathcal{N}_u(\mathcal{N}_u(T)) = \mathcal{N}_u(T)
\]

(Idempotency)

---

# 10. Hyphenation Correction

Line-break hyphen rule:

If:

\[
\text{word-}\n\text{continuation}
\]

Then merge to:

\[
\text{wordcontinuation}
\]

Only if dictionary or lexical validation confirms.

No speculative merging permitted.

---

# 11. Reference Section Isolation

Reference detection function:

\[
\mathcal{R}_f(T) \rightarrow T_{ref}
\]

References must be isolated from main text.

Each reference assigned structured record:

\[
R_i =
(\text{authors}, \text{title}, \text{journal}, \text{year})
\]

---

# 12. Provenance Encoding

Each textual unit must store:

\[
\mathcal{P}
=
(
\text{PDF\_checksum},
\text{page\_number},
\text{bounding\_box},
\text{extractor\_version}
)
\]

Traceability is mandatory.

---

# 13. Deterministic Reproducibility

Given identical:

- PDF artifact,
- Extractor version,
- Configuration parameters,

Must satisfy:

\[
\mathcal{E}_X(\mathcal{P}) = T
\]

Bitwise-identical output required.

---

# 14. Quality Metrics

Define extraction completeness ratio:

\[
\rho =
\frac{|T_{extracted}|}{|T_{expected}|}
\]

Where \( |T_{expected}| \) approximated via:

- Page text density heuristic,
- Character count estimation.

If:

\[
\rho < \rho_{min}
\]

Flag for review.

---

# 15. Error Classification

Extraction failure types:

\[
F \in
\{
\text{CorruptedTextObject},
\text{OrderingFailure},
\text{EncodingError},
\text{SegmentationFailure}
\}
\]

Each must generate structured log entry.

---

# 16. Security Constraints

Text extraction must operate in:

- Sandboxed PDF parsing environment.
- No external API calls.
- No code execution from embedded objects.
- Memory-limited process.

---

# 17. Hashing and Identity

Text artifact checksum:

\[
H_T = H(\mathcal{S}(T))
\]

Identity invariant:

\[
H_{T_1} = H_{T_2}
\iff
T_1 = T_2
\]

Text must be content-addressable.

---

# 18. Version Coupling

Text artifact must record:

\[
\text{text\_extractor\_version}
\]

Parser updates require version increment.

---

# 19. Compliance Requirement

Extracted text \( T \) accepted only if:

\[
T \models \text{SPEC-ACQ-CHECKSUM}
\land
T \models \text{SPEC-CONTRACT-VERSIONING}
\]

Failure blocks downstream semantic processing.

---

# 20. Strategic Interpretation

The Text Extractor forms the semantic substrate of the Thermognosis Engine.

It ensures:

- Deterministic textual reconstruction,
- Structural segmentation,
- Mathematical expression preservation,
- Unicode normalization,
- Full provenance traceability.

Text is the carrier of scientific reasoning.  
Its extraction must preserve structural and semantic integrity.

---

# 21. Concluding Statement

In the Thermognosis Engine, textual extraction must be:

- Deterministic,
- Structurally segmented,
- Encoding-consistent,
- Cryptographically identifiable,
- Fully reproducible.

Text integrity is foundational to semantic parsing, graph construction, and closed-loop reasoning.

The Text Extractor safeguards the transition from document artifact to structured scientific knowledge.
