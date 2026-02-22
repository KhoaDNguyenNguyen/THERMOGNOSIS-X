# Code Generation Protocol  
**Document ID:** SPEC-GOV-CODE-GENERATION-PROTOCOL  
**Layer:** spec/00_governance  
**Status:** Normative — AI-Assisted Implementation Governance  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Code Generation Protocol (CGP)** governing AI-assisted implementation within the Thermognosis Engine.

Its objectives are:

1. To formalize the interaction between specification and code.
2. To guarantee mathematical fidelity between theory and implementation.
3. To prevent architectural drift during rapid AI-assisted development.
4. To ensure reproducibility, traceability, and auditability.
5. To align software production with research-grade standards.

This protocol is mandatory for all AI-generated or AI-assisted code.

---

# 2. Foundational Principle

Implementation must satisfy:

\[
\text{Code} \models \text{Specification}
\]

where:

- Specification = formal Markdown + LaTeX documents.
- Code = executable implementation.

No code may precede formal specification.

---

# 3. Specification-First Requirement

Before generating any code module \( M \), the following must exist:

1. Formal mathematical definition.
2. Defined input/output structure.
3. Assumptions and constraints.
4. Dependency mapping.

Formally:

\[
\exists \; \text{Spec}_M \quad \text{such that} \quad M \rightarrow \text{Spec}_M
\]

No speculative implementation allowed.

---

# 4. Prompt Structure Standard

Every AI prompt must include:

1. The relevant Markdown specification.
2. Explicit target module name.
3. Dependency list.
4. Required compliance tests.

Template:

### Input:
* **Spec file(s)**
* **Interface definition**
* **Constraints**
* **Expected behavior**

### Output:
* **Deterministic Python module**
* **Unit tests**
* **Docstrings aligned with spec**

---

# 5. Mathematical Fidelity Rule

All mathematical definitions must map to code constructs.

Example:

Specification:

\[
IG(v)
=
H[p(\theta|\mathcal{D}_t)]
-
\mathbb{E}_{y_v} H[p(\theta|\mathcal{D}_t \cup y_v)]
\]

Implementation must:

- Compute entropy explicitly.
- Document approximation method.
- Log approximation error.

If approximation used:

\[
IG_{approx} = IG + \epsilon
\]

Error bound \( \epsilon \) must be recorded.

---

# 6. Determinism Requirement

All stochastic components must include:

\[
\texttt{seed} = s
\]

and logged:

\[
\text{Seed Registry} = \{s_1, s_2, \dots\}
\]

Ensures reproducibility.

---

# 7. Dependency Isolation

Module dependency graph:

\[
\mathcal{M} = (\mathcal{V}_M, \mathcal{E}_M)
\]

where:

- \( \mathcal{V}_M \) = modules,
- \( \mathcal{E}_M \) = import relations.

Constraints:

1. No circular dependency.
2. Physics layer independent from statistical layer.
3. Closed-loop layer depends only on abstract interfaces.

---

# 8. Layer Separation

Mandatory architectural layers:

1. Foundations
2. Physics
3. Statistical Model
4. Graph Model
5. Closed Loop
6. Governance

Formal constraint:

\[
L_i \rightarrow L_{i+1}
\]

No backward dependency.

---

# 9. Unit Testing Requirement

Each module must include:

- Deterministic unit tests.
- Boundary condition tests.
- Constraint violation tests.

Example:

If:

\[
zT = \frac{S^2 \sigma T}{\kappa}
\]

Test:

- \( \kappa > 0 \)
- Unit consistency
- Numerical stability

---

# 10. Validation Against Theorem Registry

Every module implementing theoretical claim must reference theorem ID:

\[
\text{Implements: T3}
\]

Verification:

- Simulation consistent with theorem.
- Edge-case analysis documented.

---

# 11. Numerical Stability Protocol

For matrix inversion:

\[
K^{-1}
\]

must use:

\[
K \leftarrow K + \epsilon I
\]

with logged \( \epsilon \).

For log-determinant:

Use numerically stable methods (e.g., Cholesky).

---

# 12. Code Documentation Standard

Each function must include:

1. Mathematical definition.
2. Variable meaning.
3. Units (if physical).
4. Complexity estimate.
5. Reference to specification section.

---

# 13. Error Handling Policy

If physical constraint violated:

\[
\mathcal{C}_{phys}(v) = 0
\]

System must:

- Raise structured exception.
- Log anomaly.
- Down-weight credibility.

Silent failure prohibited.

---

# 14. Versioning Requirement

Every code artifact must record:

- Spec version.
- Dataset hash.
- Dependency versions.
- Git commit hash.

Formally:

\[
\text{Artifact} = f(\text{Spec}_v, \text{Data}_h, \text{Commit}_g)
\]

---

# 15. AI-Generated Code Review Protocol

All AI-generated code must undergo:

1. Structural compliance review.
2. Mathematical consistency verification.
3. Unit test coverage ≥ 90%.
4. Cross-reference to spec.

---

# 16. Prohibited Practices

The following are strictly forbidden:

1. Hard-coded constants without documentation.
2. Silent random initialization.
3. Hidden state mutation.
4. Undocumented approximations.
5. Direct modification of foundational equations.

---

# 17. Performance Benchmark Requirement

All core modules must include:

- Runtime complexity estimate:
  \[
  \mathcal{O}(n^3) \quad \text{or similar}
  \]
- Memory complexity analysis.
- Scalability note.

---

# 18. Continuous Integration Policy

Automated pipeline must:

1. Run all tests.
2. Validate numerical stability.
3. Check deterministic output.
4. Verify no spec-code mismatch.

Failure blocks merge.

---

# 19. Spec-to-Code Traceability Matrix

Maintain mapping:

| Spec Section | Code Module | Test File |
|--------------|------------|----------|

Formal requirement:

\[
\forall \text{Spec Section} \; \exists \text{Code Mapping}
\]

---

# 20. Code as Formal Realization

Implementation must be viewed as:

\[
\text{Executable Mathematics}
\]

Code is not procedural scripting.  
It is a computational instantiation of formal definitions.

---

# 21. Governance Audit Checklist

Before approval:

- ✔ Mathematical fidelity verified  
- ✔ Determinism confirmed  
- ✔ Unit tests passed  
- ✔ Dependency structure valid  
- ✔ Theorem alignment checked  
- ✔ Documentation complete  

---

# 22. Strategic Interpretation

The Code Generation Protocol ensures that:

- AI accelerates development without degrading rigor.
- Architecture remains stable under rapid iteration.
- Implementation remains mathematically defensible.
- The system remains publication-ready.

AI is a productivity multiplier,  
but governance ensures scientific integrity.

---

# 23. Compliance Requirement

All generated modules must satisfy:

\[
\text{Module} \models \text{SPEC-GOV-CODE-GENERATION-PROTOCOL}
\]

Non-compliance results in:

- Code rejection,
- Architectural rollback,
- Governance review.

---

# 24. Concluding Statement

The Thermognosis Engine is defined first in mathematics,  
then realized in code.

This protocol guarantees that implementation speed never compromises theoretical integrity.

In a research-grade system,  
code must be as rigorous as the equations it implements.

