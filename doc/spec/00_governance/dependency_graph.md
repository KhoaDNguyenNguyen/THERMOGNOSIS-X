# Dependency Graph Specification  
**Document ID:** SPEC-GOV-DEPENDENCY-GRAPH  
**Layer:** spec/00_governance  
**Status:** Normative — Architectural Dependency Control Framework  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the formal **Dependency Graph (DG)** of the Thermognosis Engine.

Its objectives are:

1. To specify allowed dependencies between architectural layers.
2. To prevent circular and unstable module interactions.
3. To ensure long-term maintainability and theoretical integrity.
4. To provide an auditable structural blueprint of the system.
5. To align implementation with mathematical hierarchy.

The Dependency Graph governs the structural integrity of the entire system.

---

# 2. Formal Definition

Let:

\[
\mathcal{M} = (\mathcal{V}_M, \mathcal{E}_M)
\]

where:

- \( \mathcal{V}_M \) = set of modules,
- \( \mathcal{E}_M \subseteq \mathcal{V}_M \times \mathcal{V}_M \) = directed dependency edges.

Edge:

\[
(M_i \rightarrow M_j)
\]

means module \( M_i \) depends on module \( M_j \).

---

# 3. Acyclicity Requirement

The module graph must satisfy:

\[
\mathcal{M} \text{ is a Directed Acyclic Graph (DAG)}
\]

Formally:

\[
\nexists \; (M_1, \dots, M_k)
\quad \text{such that} \quad
M_1 \rightarrow M_2 \rightarrow \dots \rightarrow M_k \rightarrow M_1
\]

Circular dependencies are strictly prohibited.

---

# 4. Layered Architecture

The system is partitioned into ordered layers:

1. **Foundations (L0)**
2. **Physics (L1)**
3. **Statistical Model (L2)**
4. **Graph Model (L3)**
5. **Closed Loop (L4)**
6. **Governance (L5)**

Define layer index function:

\[
\ell : \mathcal{V}_M \to \{0,1,2,3,4,5\}
\]

---

# 5. Layer Dependency Constraint

Allowed dependency:

\[
M_i \rightarrow M_j
\quad \Rightarrow \quad
\ell(M_i) \ge \ell(M_j)
\]

Higher layers may depend on lower layers.

Forbidden:

\[
\ell(M_i) < \ell(M_j)
\]

No upward dependency allowed.

---

# 6. Foundational Independence

Layer L0 (Foundations) must satisfy:

\[
\forall M \in L0, \quad \deg^{in}(M) = 0
\]

Foundations cannot depend on any other module.

They define:

- Mathematical formalism
- Data structures
- Uncertainty theory

---

# 7. Physics Layer Constraint

Physics modules (L1):

\[
M_{phys} \rightarrow L0
\]

Allowed:

- Data formalism
- Measurement definitions

Forbidden:

- Statistical model dependency
- Closed-loop logic

Physics must remain model-agnostic.

---

# 8. Statistical Model Constraint

Statistical modules (L2) may depend on:

- Foundations (L0)
- Physics definitions (L1)

Constraint:

\[
L2 \nrightarrow L3, L4, L5
\]

Statistical inference must not depend on graph or acquisition policy.

---

# 9. Graph Model Constraint

Graph modules (L3) may depend on:

- Foundations (L0)
- Physics (L1)
- Statistical abstractions (interfaces only)

Graph layer must not depend on closed-loop logic.

---

# 10. Closed-Loop Constraint

Closed-loop modules (L4) may depend on:

- Statistical model
- Graph model
- Physics constraints

Closed-loop layer orchestrates but does not define primitives.

---

# 11. Governance Layer Constraint

Governance modules (L5) may inspect all layers but must not modify:

\[
L5 \nrightarrow \text{Core Computational Logic}
\]

Governance enforces compliance; it does not alter algorithms.

---

# 12. Interface-Based Dependency

Direct module-to-module coupling discouraged.

Preferred:

\[
M_i \rightarrow \text{Abstract Interface} \rightarrow M_j
\]

Reduces structural rigidity.

---

# 13. Dependency Matrix Representation

Define adjacency matrix:

\[
A_{ij}
=
\begin{cases}
1 & \text{if } M_i \rightarrow M_j \\
0 & \text{otherwise}
\end{cases}
\]

Constraint:

\[
A \text{ is upper-triangular under topological ordering}
\]

---

# 14. Topological Ordering

Exist ordering:

\[
(M_{i_1}, M_{i_2}, \dots, M_{i_n})
\]

such that:

\[
i_k < i_l \quad \text{if} \quad M_{i_k} \rightarrow M_{i_l}
\]

Ensures build determinism.

---

# 15. Dependency Complexity Bound

Define maximum in-degree:

\[
\deg^{in}(M_i) \le K
\]

to prevent high coupling.

Target:

\[
K \le 5
\]

Encourages modularity.

---

# 16. External Dependency Policy

External libraries must satisfy:

1. Active maintenance.
2. License compatibility.
3. Numerical reliability.
4. Reproducibility.

Each external dependency recorded:

\[
\text{Dependency} = (name, version, hash)
\]

---

# 17. Change Impact Analysis

When modifying module \( M \):

Affected set:

\[
\mathcal{A}(M)
=
\{ M' \mid M' \rightarrow^* M \}
\]

All affected modules must be retested.

---

# 18. Graph Stability Condition

Structural stability metric:

\[
S_t
=
\|A_t - A_{t-1}\|_F
\]

Excessive growth triggers architectural review.

---

# 19. Dependency Audit Procedure

Periodic audit must verify:

1. No cycles.
2. Layer compliance.
3. Controlled in-degree.
4. Valid external versions.
5. Updated dependency documentation.

Automated DAG validation required.

---

# 20. Refactoring Rule

If module violates layering constraint:

Refactor by:

1. Extracting interface.
2. Moving shared logic downward.
3. Reducing cross-layer references.

Direct deletion prohibited without migration plan.

---

# 21. Dependency as Mathematical Hierarchy

The dependency graph mirrors mathematical abstraction:

\[
\text{Foundations} \rightarrow \text{Physics} \rightarrow \text{Inference} \rightarrow \text{Decision}
\]

Architecture reflects epistemic structure.

---

# 22. Failure Modes

Violations may lead to:

- Circular inference logic.
- Physics-statistics entanglement.
- Instability in closed-loop dynamics.
- Governance override of core algorithms.
- Loss of reproducibility.

---

# 23. Compliance Requirement

System must satisfy:

\[
\mathcal{M} \models \text{SPEC-GOV-DEPENDENCY-GRAPH}
\]

All merges require automated DAG verification.

---

# 24. Strategic Interpretation

The Dependency Graph ensures:

- Architectural clarity,
- Theoretical separation,
- Implementation scalability,
- Long-term maintainability.

Without structural discipline, rapid AI-assisted development leads to architectural entropy.

---

# 25. Concluding Statement

The Thermognosis Engine is not a collection of scripts.  
It is a layered scientific system.

The Dependency Graph preserves its structural integrity.

Mathematical hierarchy must be mirrored in software hierarchy.  
Only then can the system remain stable under long-term evolution.
