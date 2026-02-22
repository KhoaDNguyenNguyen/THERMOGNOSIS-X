# ACQ — PDF Validator Specification  
**Document ID:** SPEC-ACQ-PDF-VALIDATOR  
**Layer:** spec/02_acquisition  
**Status:** Normative — Scientific PDF Structural & Semantic Validation Protocol  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **PDF Validator Specification (PVS)** governing structural, cryptographic, and semantic validation of PDF documents ingested into the Thermognosis Engine.

The PDF Validator is responsible for ensuring that every scientific document:

1. Is structurally well-formed.
2. Is cryptographically verified.
3. Is not corrupted or truncated.
4. Is semantically parsable.
5. Is suitable for deterministic downstream processing.

A corrupted or malformed PDF introduces epistemic instability into the knowledge graph.  
Validation is therefore mandatory prior to parsing or indexing.

---

# 2. Scope

The validator applies to:

- Research papers.
- Technical reports.
- Preprints.
- Supplementary materials.
- Dataset documentation distributed as PDF.

It does **not** perform scientific correctness validation.  
It verifies structural and acquisition integrity only.

---

# 3. Formal Model

Let downloaded byte stream be:

\[
B = (b_1, b_2, \dots, b_n)
\]

Let parsed PDF object be:

\[
\mathcal{P} = \mathcal{V}(B)
\]

Validation function:

\[
\mathcal{V} : B \rightarrow 
\begin{cases}
\mathcal{P}, & \text{if valid} \\
\bot, & \text{if invalid}
\end{cases}
\]

Where \( \bot \) denotes rejection.

---

# 4. Structural Validation

A valid PDF must satisfy:

1. Header consistency:

\[
b_1 \dots b_k = \text{\%PDF-x.y}
\]

2. Cross-reference table exists.
3. Trailer dictionary present.
4. EOF marker:

\[
\text{\%\%EOF}
\]

Structural completeness condition:

\[
\exists \text{ xref } \land \exists \text{ trailer } \land \exists \text{ EOF}
\]

Failure of any condition results in rejection.

---

# 5. Byte-Level Integrity

Checksum invariant:

\[
H_B = H(B)
\]

Must match checksum stored during download.

If:

\[
H_{\text{stored}} \neq H(B)
\]

Validation fails immediately.

---

# 6. File Size Constraint

Let file size be:

\[
S = |B|
\]

Constraints:

\[
S_{\min} \le S \le S_{\max}
\]

Where:

- \( S_{\min} \) prevents empty or truncated files.
- \( S_{\max} \) prevents resource exhaustion attacks.

---

# 7. Encrypted PDF Policy

If PDF is encrypted:

\[
\text{Encrypted} = \text{True}
\]

Validator must check:

1. Whether decryption key is available.
2. Whether document is legally accessible.

If no authorized access:

\[
\mathcal{V}(B) = \bot
\]

Unauthorized bypass is strictly prohibited.

---

# 8. Embedded Object Inspection

PDF object set:

\[
\mathcal{O} = \{ O_i \}_{i=1}^m
\]

Validator must inspect:

- Embedded scripts
- Executable attachments
- Suspicious object streams

If:

\[
\exists O_i \in \mathcal{O}
\text{ s.t. } O_i \text{ executable}
\]

Then:

\[
\mathcal{V}(B) = \bot
\]

Sandbox isolation required.

---

# 9. Parsing Determinism

Parsing function:

\[
\mathcal{P} = \mathcal{V}(B)
\]

Must satisfy determinism:

\[
\mathcal{V}(B; v_1) = \mathcal{V}(B; v_2)
\]

Given identical validator version and environment.

Parser version must be logged.

---

# 10. Text Extractability

Let extracted text be:

\[
T = \mathcal{E}(\mathcal{P})
\]

Validator must confirm:

\[
|T| > 0
\]

If no extractable text:

- Flag as image-only PDF.
- Route to OCR pipeline.
- Do not reject automatically.

---

# 11. Page Consistency

Let number of pages be:

\[
N = |\text{Pages}(\mathcal{P})|
\]

Constraint:

\[
N \ge 1
\]

Page objects must be reachable from document root.

---

# 12. Metadata Extraction

Validator must extract:

\[
\mathcal{M} =
(\text{Title}, \text{Author}, \text{CreationDate}, \text{Producer})
\]

Metadata hash:

\[
H_{\mathcal{M}} = H(\mathcal{S}(\mathcal{M}))
\]

Stored for provenance tracking.

---

# 13. Duplicate Detection

If:

\[
H_{B_1} = H_{B_2}
\]

Documents considered identical.

No duplicate storage permitted.

---

# 14. Truncation Detection

Let expected length from download metadata:

\[
L_{\text{expected}}
\]

Actual length:

\[
L_{\text{actual}}
\]

Invariant:

\[
L_{\text{actual}} = L_{\text{expected}}
\]

Additionally, EOF marker must appear within last 1024 bytes.

---

# 15. Malformed Object Handling

If object parsing fails:

\[
\exists O_i \text{ s.t. parse}(O_i) = \bot
\]

Validator must:

- Attempt recovery (limited attempts).
- If recovery fails → reject.

Silent object skipping prohibited.

---

# 16. Security Isolation

PDF parsing must occur in:

- Sandboxed environment.
- Memory-limited process.
- No filesystem write permissions.
- No network access.

Execution invariant:

\[
\text{Sandboxed} = \text{True}
\]

---

# 17. Logging Requirements

Validation event log must include:

- File checksum
- Validation status
- Structural checks passed/failed
- Parser version
- Environment fingerprint
- Timestamp

Log must be immutable.

---

# 18. Failure Classification

Let failure class:

\[
F \in \{
\text{ChecksumMismatch},
\text{MalformedStructure},
\text{EncryptedUnauthorized},
\text{TruncatedFile},
\text{SecurityRisk}
\}
\]

Each failure produces structured audit entry.

---

# 19. Compliance Requirement

A PDF artifact \( B \) is accepted only if:

\[
B \models \text{SPEC-ACQ-DOWNLOADER}
\land
B \models \text{SPEC-ACQ-CHECKSUM}
\land
\mathcal{V}(B) \neq \bot
\]

Otherwise ingestion is blocked.

---

# 20. Strategic Interpretation

The PDF Validator ensures that:

- No corrupted documents enter the graph.
- No malicious payload contaminates the system.
- No truncated files propagate silently.
- No parsing ambiguity destabilizes extraction.

It is a structural firewall for scientific documents.

---

# 21. Concluding Statement

In the Thermognosis Engine, every PDF must be:

- Structurally sound,
- Cryptographically verified,
- Securely parsed,
- Deterministically reproducible,
- Fully auditable.

Scientific knowledge begins with document integrity.

The PDF Validator safeguards the epistemic boundary between external content and internal reasoning systems.
