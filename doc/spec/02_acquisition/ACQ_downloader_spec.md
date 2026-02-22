# ACQ — Downloader Specification  
**Document ID:** SPEC-ACQ-DOWNLOADER  
**Layer:** spec/02_acquisition  
**Status:** Normative — Deterministic Artifact Retrieval Protocol  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Downloader Specification (DS)** governing deterministic retrieval of digital artifacts from remote or distributed sources into the Thermognosis Engine.

The downloader is a low-level acquisition primitive responsible for:

1. Byte-level integrity preservation.
2. Deterministic transfer behavior.
3. Secure transport.
4. Version-aware retrieval.
5. Checksum-verified persistence.

The downloader is not a convenience utility.  
It is a **cryptographically disciplined transport layer** for scientific artifacts.

---

# 2. Foundational Principle

Let remote resource be:

\[
R(u, t)
\]

where:

- \( u \) = resource locator (URL or endpoint),
- \( t \) = retrieval timestamp.

Downloader defines mapping:

\[
\mathcal{D} : (u, t) \rightarrow B
\]

where \( B \) is the exact byte stream.

Integrity invariant:

\[
B_{\text{stored}} = B_{\text{received}}
\]

No transformation is permitted at transport layer.

---

# 3. Deterministic Retrieval

Given identical:

- URL \( u \),
- Headers \( h \),
- Time window \( t \),
- Downloader version \( v \),

Downloader must satisfy:

\[
\mathcal{D}(u, t; h, v)
=
B
\]

If resource changes over time:

\[
R(u, t_1) \neq R(u, t_2)
\]

Artifacts must be treated as distinct versions.

---

# 4. Transport Protocol Requirements

Downloader must support:

1. HTTPS (mandatory)
2. TLS verification (mandatory)
3. Certificate validation
4. Optional authenticated endpoints

Security invariant:

\[
\text{TLS\_verified} = \text{True}
\]

Connections failing certificate validation must be rejected.

---

# 5. Byte-Level Integrity

Downloaded byte stream:

\[
B = (b_1, b_2, \dots, b_n)
\]

Checksum:

\[
H_B = H(B)
\]

Hash computed immediately upon completion.

No decoding or parsing occurs prior to checksum.

---

# 6. Chunked Download Model

For large artifacts:

\[
B = \bigcup_{i=1}^{k} C_i
\]

where each chunk:

\[
C_i = (b_{m_i}, \dots, b_{m_{i+1}})
\]

Incremental checksum:

\[
H_k = H(H_{k-1} \parallel C_k)
\]

Ensures streaming-safe integrity.

---

# 7. Resume Capability

If interruption occurs at byte index \( j \):

\[
B = (b_1, \dots, b_j) \cup (b_{j+1}, \dots, b_n)
\]

Resume allowed only if:

\[
H(b_1, \dots, b_j) = H_{\text{stored\_partial}}
\]

Otherwise restart download.

---

# 8. Metadata Capture

Each download must record:

\[
\mathcal{M}
=
(
u,
t,
\text{HTTP\_status},
\text{headers},
H_B,
\text{downloader\_version}
)
\]

Metadata immutable after storage.

---

# 9. Rate Limiting

Request frequency:

\[
f = \frac{n}{\Delta t}
\]

Constraint:

\[
f \le f_{\max}
\]

Defined per-domain policy.

Violation constitutes governance breach.

---

# 10. Timeout Policy

Timeout function:

\[
T = T_0 + \alpha \cdot \text{size\_estimate}
\]

If:

\[
\text{elapsed\_time} > T
\]

Download aborted and logged.

---

# 11. Retry Strategy

Retry count \( k \):

\[
k \le k_{\max}
\]

Backoff schedule:

\[
t_k = t_0 \cdot 2^k
\]

Retries logged explicitly.

No silent retry loops allowed.

---

# 12. Content-Length Verification

If header provides:

\[
L_{\text{expected}}
\]

Downloaded length:

\[
L_{\text{actual}}
\]

Integrity check:

\[
L_{\text{actual}} = L_{\text{expected}}
\]

Mismatch triggers rejection.

---

# 13. Deterministic Storage Path

Storage path defined as:

\[
\text{path} = \text{root}/H_B
\]

Content-addressable storage required.

Invariant:

\[
H_{B_1} = H_{B_2}
\Rightarrow
\text{same storage path}
\]

---

# 14. Immutability Rule

Stored artifact:

\[
B_{\text{stored}}
\]

Must satisfy:

\[
\text{write-once}
\]

No modification allowed post-checksum.

---

# 15. Duplicate Handling

If:

\[
H_{B_1} = H_{B_2}
\]

Second download must:

- Not duplicate storage.
- Increase reference counter only.

---

# 16. Environment Determinism

Downloader environment fingerprint:

\[
H_{\text{env}}
=
H(\text{OS}, \text{runtime}, \text{dependency versions})
\]

Stored with metadata.

Cross-node invariant:

\[
H_{\text{env}_1} \neq H_{\text{env}_2}
\Rightarrow
\text{possible non-determinism warning}
\]

---

# 17. Security Constraints

Downloader must prevent:

1. Directory traversal attacks.
2. Executable auto-run.
3. Embedded script execution.
4. Unauthorized file overwrite.

All downloads sandboxed.

---

# 18. Large File Policy

For file size \( S \):

\[
S > S_{\text{threshold}}
\]

Downloader must:

- Use streaming mode.
- Avoid memory overflow.
- Log chunk integrity.

---

# 19. Failure Classification

Let failure type be:

\[
F \in \{
\text{NetworkError},
\text{Timeout},
\text{ChecksumMismatch},
\text{CertificateFailure},
\text{ContentLengthMismatch}
\}
\]

Each failure must produce structured log entry.

---

# 20. Compliance Requirement

Downloaded artifact \( B \) must satisfy:

\[
B \models \text{SPEC-ACQ-CHECKSUM}
\land
B \models \text{SPEC-CONTRACT-VERSIONING}
\]

Non-compliance blocks ingestion.

---

# 21. Strategic Interpretation

The downloader is the foundation of acquisition integrity.

It guarantees:

- Byte-level determinism,
- Cryptographic identity,
- Secure transport,
- Immutable storage,
- Reproducible artifact retrieval.

Without strict downloader governance,  
higher-level crawler guarantees collapse.

---

# 22. Concluding Statement

In the Thermognosis Engine, every downloaded artifact is:

- Cryptographically verified,
- Deterministically stored,
- Immutably preserved,
- Fully auditable.

Scientific credibility begins at the transport layer.

The downloader is the first line of epistemic defense.
