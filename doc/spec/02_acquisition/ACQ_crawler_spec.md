# ACQ — Crawler Specification  
**Document ID:** SPEC-ACQ-CRAWLER  
**Layer:** spec/02_acquisition  
**Status:** Normative — Structured Knowledge Acquisition Protocol  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Crawler Specification (CS)** governing automated acquisition of external knowledge sources into the Thermognosis Engine.

The crawler is not a generic scraper. It is a **controlled epistemic intake mechanism** designed to:

1. Acquire structured scientific information.
2. Preserve provenance and traceability.
3. Ensure deterministic reproducibility.
4. Respect ethical and legal boundaries.
5. Maintain integrity and checksum compliance.

Acquisition without governance leads to epistemic contamination.

---

# 2. Foundational Principle

Let external source space be:

\[
\mathcal{S}_{ext}
\]

Crawler defines a mapping:

\[
\mathcal{C} : \mathcal{S}_{ext} \rightarrow \mathcal{A}
\]

where:

- \( \mathcal{A} \) = structured acquisition artifact.

The crawler must be:

\[
\text{Deterministic} \land \text{Auditable} \land \text{Reproducible}
\]

---

# 3. Scope of Crawling

Crawler is restricted to:

1. Scientific publications.
2. Public datasets.
3. Institutional repositories.
4. Open-access archives.
5. Approved data endpoints.

Crawling of:

- Personal data,
- Restricted databases,
- Non-consensual content,

is strictly prohibited.

---

# 4. Acquisition Pipeline

Crawler pipeline defined as:

\[
\mathcal{C}
=
\mathcal{P}_5 \circ \mathcal{P}_4 \circ \mathcal{P}_3 \circ \mathcal{P}_2 \circ \mathcal{P}_1
\]

Where:

1. \( \mathcal{P}_1 \): Source discovery
2. \( \mathcal{P}_2 \): Content retrieval
3. \( \mathcal{P}_3 \): Parsing and structuring
4. \( \mathcal{P}_4 \): Normalization
5. \( \mathcal{P}_5 \): Validation & checksum

---

# 5. Source Discovery

Discovery function:

\[
\mathcal{P}_1(q)
=
\{ s_i \}_{i=1}^n
\]

where \( q \) is structured query.

Query must be:

- Parameterized
- Versioned
- Logged

Discovery log must include:

- Query hash
- Timestamp
- Spec version

---

# 6. Content Retrieval

Retrieval function:

\[
\mathcal{P}_2(s)
=
X
\]

Constraints:

1. Respect robots.txt.
2. Rate-limited access.
3. Retry policy with exponential backoff:

\[
t_k = t_0 \cdot 2^k
\]

4. Timeout limits enforced.

---

# 7. Parsing and Structuring

Raw content:

\[
X
\]

Parsed representation:

\[
\tilde{X} = \mathcal{P}_3(X)
\]

Parser must extract:

- Metadata
- Numerical values
- Units
- Citations
- Provenance

Parsing must be deterministic.

---

# 8. Normalization

Normalization function:

\[
\mathcal{N}(\tilde{X})
=
X_{norm}
\]

Includes:

1. Unit normalization (SI standard).
2. Encoding normalization (UTF-8).
3. Floating-point precision standardization.
4. Field canonicalization.

Invariant:

\[
\mathcal{N}(\mathcal{N}(X)) = \mathcal{N}(X)
\]

(Idempotency requirement)

---

# 9. Provenance Encoding

Each acquired artifact must store:

\[
\mathcal{P}
=
(\text{source\_url}, \text{retrieval\_time}, \text{crawler\_version}, H(X))
\]

Provenance invariant:

\[
\text{Artifact} \rightarrow \mathcal{P}
\]

No artifact may exist without provenance.

---

# 10. Checksum Compliance

After normalization:

\[
H_X = H(\mathcal{S}(X_{norm}))
\]

Checksum must comply with:

SPEC-ACQ-CHECKSUM.

Artifact rejected if checksum verification fails.

---

# 11. Deduplication Rule

Duplicate detection:

\[
H_{X_1} = H_{X_2}
\Rightarrow
X_1 \equiv X_2
\]

Duplicates must:

- Not create new root ID.
- Increment reference counter only.

---

# 12. Error Handling

Let failure probability be:

\[
P_f = P(\text{retrieval error})
\]

Retry policy:

\[
\text{Retry if } P_f > 0 \land k < k_{max}
\]

Persistent failure:

- Log incident.
- Mark source as unstable.
- Do not silently ignore.

---

# 13. Temporal Consistency

If source is dynamic:

\[
X(t_1) \neq X(t_2)
\]

Crawler must:

- Timestamp every retrieval.
- Treat content at different times as different versions.

Temporal integrity invariant:

\[
t_1 \neq t_2
\Rightarrow
H_{X(t_1)} \neq H_{X(t_2)}
\]

---

# 14. Rate Limiting

Let request frequency be:

\[
f = \frac{n}{\Delta t}
\]

Constraint:

\[
f \le f_{max}
\]

Defined per source policy.

Violation constitutes governance breach.

---

# 15. Distributed Crawling Consistency

For distributed crawler nodes:

\[
\mathcal{C}_i(q)
=
\mathcal{C}_j(q)
\]

Given identical:

- Query
- Spec version
- Time window

Outputs must be identical.

---

# 16. Graph Integration

Crawler output integrated into knowledge graph:

\[
\mathcal{G}_{t+1}
=
\mathcal{G}_t \cup \{X_{norm}\}
\]

Edge creation must preserve:

- Citation structure
- Dependency structure
- Version lineage

---

# 17. Security Constraints

Crawler must prevent:

1. Script execution from remote content.
2. Arbitrary code injection.
3. File system escape.
4. Unauthorized access tokens.

Sandboxed execution required.

---

# 18. Logging Requirements

Each crawl event logs:

- Query ID
- Source ID
- Response status
- Checksum
- Parsing status
- Validation status
- Execution environment hash

---

# 19. Prohibited Practices

The following are strictly forbidden:

1. Silent content modification.
2. Ignoring robots.txt.
3. Overwriting previously crawled artifacts.
4. Non-deterministic parsing.
5. Non-canonical normalization.

---

# 20. Compliance Requirement

Crawler artifact \( X \) must satisfy:

\[
X \models \text{SPEC-ACQ-CHECKSUM}
\land
X \models \text{SPEC-CONTRACT-VERSIONING}
\]

Failure blocks ingestion.

---

# 21. Strategic Interpretation

The crawler is not a data vacuum.

It is:

- A governed epistemic interface,
- A deterministic knowledge intake mechanism,
- A cryptographically verified acquisition system,
- A foundational layer for closed-loop learning.

Without disciplined crawling,  
the knowledge graph becomes unstable.

---

# 22. Concluding Statement

The Thermognosis Crawler operates under strict mathematical and governance constraints.

It ensures that:

- Every external artifact is traceable,
- Every retrieval is reproducible,
- Every parsing step is deterministic,
- Every integration is verifiable.

Scientific acquisition must be controlled, auditable, and formally specified.

Crawler governance is a prerequisite for credible autonomous intelligence.
