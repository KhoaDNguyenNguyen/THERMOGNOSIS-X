# ACQ — Checksum and Integrity Specification  
**Document ID:** SPEC-ACQ-CHECKSUM  
**Layer:** spec/02_acquisition  
**Status:** Normative — Data Integrity & Acquisition Consistency Protocol  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Acquisition Checksum Specification (ACS)** governing integrity verification during data ingestion, transformation, and closed-loop acquisition within the Thermognosis Engine.

Its objectives are:

1. To guarantee deterministic identity of acquired data.
2. To prevent silent corruption or duplication.
3. To ensure reproducibility across distributed environments.
4. To formalize integrity checks at each acquisition stage.
5. To provide mathematically sound artifact verification.

Data acquisition without integrity guarantees undermines scientific credibility.

---

# 2. Foundational Principle

Every acquired artifact must satisfy:

\[
\text{Checksum}(X) = H(X)
\]

where:

- \( X \) = canonical serialized content,
- \( H \) = deterministic cryptographic hash function.

Integrity invariant:

\[
H(X_1) = H(X_2) \iff X_1 = X_2
\]

---

# 3. Canonical Serialization

Before checksum computation, artifact must undergo canonical serialization:

\[
X_{\text{canon}} = \mathcal{S}(X)
\]

where:

- Key ordering deterministic.
- Floating-point precision fixed.
- Encoding standardized (UTF-8).
- Units normalized to SI.

Serialization function \( \mathcal{S} \) must be deterministic.

---

# 4. Hash Function Definition

Hash function:

\[
H : \{0,1\}^* \rightarrow \{0,1\}^{256}
\]

Recommended implementation: SHA-256


Properties required:

1. Collision resistance
2. Preimage resistance
3. Deterministic output

---

# 5. Checksum for Raw Measurement

For raw measurement \( \mathcal{R} \):

\[
H_{\mathcal{R}} = H(\mathcal{S}(\mathcal{R}))
\]

Invariant:

\[
\mathcal{R}_1 \neq \mathcal{R}_2
\Rightarrow
H_{\mathcal{R}_1} \neq H_{\mathcal{R}_2}
\]

Checksum stored at ingestion.

---

# 6. Checksum for Dataset

Dataset:

\[
\mathcal{D}
=
\{ \mathcal{V}_i \}_{i=1}^n
\]

Dataset checksum:

\[
H_{\mathcal{D}}
=
H\left(
\mathcal{S}(
\text{sorted}(\{H_{\mathcal{V}_i}\})
)
\right)
\]

Sorting required to ensure order invariance.

---

# 7. Model Artifact Checksum

Model entity:

\[
\mathcal{M}
=
(\theta, \lambda, H_{\mathcal{D}}, \text{spec\_version}, s)
\]

Model checksum:

\[
H_{\mathcal{M}}
=
H(\mathcal{S}(\mathcal{M}))
\]

Reproducibility condition:

\[
H_{\mathcal{M}_1} = H_{\mathcal{M}_2}
\Rightarrow
\text{identical model state}
\]

---

# 8. Acquisition Step Integrity

Closed-loop acquisition step:

\[
\mathcal{A}_t
=
(\mathcal{D}_t, \mathcal{M}_t, a(x), x_{t+1})
\]

Step checksum:

\[
H_{\mathcal{A}_t}
=
H(\mathcal{S}(\mathcal{A}_t))
\]

Ensures deterministic acquisition history.

---

# 9. Integrity Verification Rule

Upon retrieval:

\[
H_{\text{stored}} \stackrel{?}{=} H(\mathcal{S}(X))
\]

If mismatch:

- Artifact rejected.
- Integrity violation logged.
- Governance alert triggered.

---

# 10. Floating-Point Precision Standard

All numeric values must satisfy:

\[
x \mapsto \text{round}(x, p)
\]

where:

\[
p = 12 \text{ decimal places (default)}
\]

Prevents non-deterministic hash changes.

---

# 11. Unit Normalization

Before hashing:

\[
\text{Value}_{\text{normalized}}
=
\text{convert\_to\_SI}(\text{Value})
\]

Unit inconsistency prohibited.

---

# 12. Incremental Checksum Strategy

For streaming ingestion:

\[
H_{n}
=
H(H_{n-1} \parallel H_{\mathcal{R}_n})
\]

Allows incremental verification.

---

# 13. Dependency Integrity

Dependency graph:

\[
\mathcal{G} = (\mathcal{V}, \mathcal{E})
\]

Integrity invariant:

\[
H_{\mathcal{D}} = f(\{H_{\mathcal{R}_i}\})
\]

No downstream artifact may exist without upstream checksum validation.

---

# 14. Version-Checksum Coupling

Versioned entity:

\[
\mathcal{E}^{(v)}
\]

Checksum must include:

\[
v
\]

Invariant:

\[
v_1 \neq v_2
\Rightarrow
H_1 \neq H_2
\]

---

# 15. Collision Handling Policy

If:

\[
H(X_1) = H(X_2)
\land
X_1 \neq X_2
\]

System must:

- Trigger cryptographic alert.
- Escalate governance review.
- Suspend ingestion pipeline.

Collision probability assumed negligible under SHA-256.

---

# 16. Logging Requirements

Every checksum computation must log:

- Artifact ID
- Checksum value
- Timestamp
- Spec version
- Environment hash

---

# 17. Audit Trail

Acquisition audit record:

\[
\mathcal{T}
=
\{ H_{\mathcal{A}_t} \}_{t=1}^T
\]

Complete audit trail must be reconstructible.

---

# 18. Distributed Consistency Requirement

In distributed environment:

\[
H_{\text{node}_i}(X)
=
H_{\text{node}_j}(X)
\]

Cross-node mismatch indicates environment inconsistency.

---

# 19. Prohibited Practices

The following are strictly forbidden:

1. Hashing non-canonical serialization.
2. Ignoring checksum mismatch.
3. Partial artifact hashing.
4. Modifying artifact after checksum.
5. Using non-cryptographic hash functions.

---

# 20. Compliance Requirement

All acquisition artifacts must satisfy:

\[
X \models \text{SPEC-ACQ-CHECKSUM}
\]

Non-compliance blocks ingestion and closed-loop progression.

---

# 21. Strategic Interpretation

Checksum integrity is not administrative redundancy.

It is:

- A cryptographic guarantee of identity,
- A structural defense against corruption,
- A prerequisite for reproducibility,
- A foundation for auditability.

Without integrity verification,  
closed-loop science becomes unverifiable.

---

# 22. Concluding Statement

In the Thermognosis Engine, every acquired artifact carries a cryptographic identity.

Checksums ensure that:

- Data cannot mutate silently,
- Models remain reproducible,
- Acquisition remains deterministic,
- Audit trails remain complete.

Integrity is a mathematical property, not a procedural formality.

Scientific rigor requires cryptographic discipline.


