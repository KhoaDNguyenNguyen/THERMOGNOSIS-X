# ID — Material Canonicalization Specification  
**Document ID:** SPEC-ID-MATERIAL-CANONICALIZATION  
**Layer:** spec/07_identity_graph  
**Status:** Normative — Material Identity Normalization and Canonical Representation Framework  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Material Canonicalization Specification (MCS)** governing:

- Standardized material identity representation,
- Resolution of naming ambiguity and synonymy,
- Composition normalization,
- Structural and phase disambiguation,
- Deterministic identity mapping within the Identity Graph.

Scientific literature frequently contains inconsistent naming conventions, abbreviated formulas, non-standard stoichiometry notation, and incomplete phase descriptors. Canonicalization ensures that semantically identical materials map to a single graph identity.

Without canonicalization, relational reasoning, embedding stability, and ranking validity are compromised.

---

# 2. Material Identity Formalization

A material record is defined as:

\[
\mathcal{M} =
(\mathbf{c}, \mathbf{s}, \mathbf{p}, \mathbf{m})
\]

where:

- \( \mathbf{c} \) — composition vector  
- \( \mathbf{s} \) — structural descriptor  
- \( \mathbf{p} \) — phase/state descriptor  
- \( \mathbf{m} \) — metadata  

Canonical material identity:

\[
\mathcal{M}_{\text{canon}} = \Phi(\mathcal{M})
\]

where:

\[
\Phi : \mathcal{M} \to \mathcal{I}
\]

is the canonicalization mapping.

---

# 3. Composition Normalization

Let raw formula:

\[
\text{A}_{a} \text{B}_{b} \text{C}_{c}
\]

Composition vector:

\[
\mathbf{c} =
\left(
\frac{a}{a+b+c},
\frac{b}{a+b+c},
\frac{c}{a+b+c}
\right)
\]

Canonical stoichiometric reduction:

\[
(a, b, c)
\rightarrow
\left(
\frac{a}{g},
\frac{b}{g},
\frac{c}{g}
\right)
\]

where:

\[
g = \gcd(a,b,c)
\]

Ensures minimal integer representation.

---

# 4. Ordering Convention

Elements must be ordered lexicographically or by predefined chemical priority:

\[
\text{Formula}_{\text{canon}}
=
\text{Sort}(\text{Elements})
\]

Example rule:

- Metallic elements first,
- Followed by non-metals,
- Then dopants.

Ordering rule must be globally deterministic.

---

# 5. Doping Representation

Doped material:

\[
\text{A}_{1-x} \text{B}_x \text{C}
\]

Canonical doping descriptor:

\[
(\text{Host} = \text{AC}, \text{Dopant} = \text{B}, x)
\]

Parameter domain:

\[
x \in [0,1]
\]

Discrete vs continuous doping must be distinguished.

---

# 6. Fractional Composition Handling

Floating representation normalized to tolerance \( \epsilon \):

\[
|c_i - \hat{c}_i| < \epsilon
\]

Default:

\[
\epsilon = 10^{-6}
\]

Prevents false identity splits.

---

# 7. Structural Canonicalization

Structure descriptor includes:

- Space group \( G \),
- Lattice parameters \( (a,b,c,\alpha,\beta,\gamma) \),
- Prototype label.

Canonical structure:

\[
\mathbf{s}_{\text{canon}} =
(G, \text{Prototype})
\]

If space group unknown, structure flagged as incomplete.

---

# 8. Phase Disambiguation

Phase descriptor:

\[
\mathbf{p} =
(\text{Phase}, T, P)
\]

Two materials identical only if:

\[
\mathbf{c}_{\text{canon}} \land \mathbf{s}_{\text{canon}} \land \mathbf{p}_{\text{canon}}
\]

all match within tolerance.

---

# 9. Polymorphism Handling

If identical composition but different structure:

\[
\mathcal{M}_1 \ne \mathcal{M}_2
\]

Distinct node identities must be created.

---

# 10. Synonym Resolution

Let raw name:

\[
n_{\text{raw}}
\]

Canonical name:

\[
n_{\text{canon}} = \Psi(n_{\text{raw}})
\]

Mapping \( \Psi \) must resolve:

- Abbreviations,
- Alternative naming conventions,
- Trade names,
- Historical aliases.

All synonyms must map to same canonical identifier.

---

# 11. Identity Hashing

Canonical identity hash:

\[
\text{ID}_{\text{material}} =
\text{hash}
(
\mathbf{c}_{\text{canon}},
\mathbf{s}_{\text{canon}},
\mathbf{p}_{\text{canon}}
)
\]

Hash must be deterministic and collision-resistant.

---

# 12. Equivalence Relation

Define equivalence:

\[
\mathcal{M}_1 \sim \mathcal{M}_2
\]

if:

\[
\Phi(\mathcal{M}_1) =
\Phi(\mathcal{M}_2)
\]

This defines partition of material space into equivalence classes.

---

# 13. Graph Node Uniqueness

For all canonical materials:

\[
\forall i,j :
\mathcal{M}_{\text{canon},i}
=
\mathcal{M}_{\text{canon},j}
\Rightarrow
v_i = v_j
\]

Duplicate canonical nodes prohibited.

---

# 14. Canonicalization Determinism

Given identical raw material record:

\[
\mathcal{M}
\]

Mapping must satisfy:

\[
\Phi(\mathcal{M}) = \text{constant}
\]

No stochastic behavior permitted.

---

# 15. Versioning of Canonical Rules

Canonicalization rule set:

\[
\mathcal{R}_{\text{canon}}^{(t)}
\]

Material identity must log rule version used.

Changes in rules must trigger re-indexing.

---

# 16. Uncertainty in Composition

If composition has uncertainty:

\[
c_i \sim \mathcal{N}(\mu_i, \sigma_i^2)
\]

Canonical identity must use:

\[
\mu_i
\]

Uncertainty stored but does not affect identity mapping.

---

# 17. Edge Case Handling

Special categories:

- Amorphous materials,
- Composite materials,
- Alloys without precise stoichiometry.

Such cases must include descriptive metadata and flagged as non-stoichiometric.

---

# 18. Error Classification

- ID-CAN-01: Non-reduced stoichiometry
- ID-CAN-02: Ambiguous phase
- ID-CAN-03: Conflicting structural descriptor
- ID-CAN-04: Synonym collision
- ID-CAN-05: Hash inconsistency

All canonicalization errors must halt node creation.

---

# 19. Formal Soundness Condition

Canonicalization framework is sound if:

1. Equivalence relation is reflexive, symmetric, transitive.
2. Hash mapping deterministic.
3. No two distinct materials share canonical identity.
4. Identical materials never produce distinct identities.

---

# 20. Strategic Interpretation

Material canonicalization ensures:

- Identity graph coherence,
- Accurate aggregation of experimental evidence,
- Stable embedding representation,
- Valid ranking and prioritization.

Canonical identity transforms fragmented literature terminology into structured scientific objects.

---

# 21. Concluding Statement

All material nodes must satisfy:

\[
\mathcal{M} \models \text{SPEC-ID-MATERIAL-CANONICALIZATION}
\]

Material identity is the atomic unit of relational scientific intelligence.  
Its canonical representation must be mathematically precise, deterministic, and globally consistent.
