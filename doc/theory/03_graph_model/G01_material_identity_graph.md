# G01 — Material Identity Graph  
**Document ID:** G01-MATERIAL-IDENTITY-GRAPH  
**Layer:** Graph Modeling / Knowledge Representation  
**Status:** Normative — Structural Knowledge Layer  
**Dependencies:**  
- T00-SYS-AXIOMS  
- T01-DATA-FORMALISM  
- P01-THERMOELECTRIC-EQUATIONS  
- S01-BAYESIAN-CREDIBILITY-MODEL  

---

# 1. Purpose

This document formalizes the **Material Identity Graph (MIG)**, the canonical graph representation of material entities within the Thermognosis Engine.

Its objectives are:

1. To define material identity as a structured graph object.
2. To distinguish identity from measurement instances.
3. To encode composition, structure, and processing lineage.
4. To enable graph-based reasoning and inference.
5. To provide a persistent and version-aware material ontology.

The Material Identity Graph is the structural backbone of material knowledge representation.

---

# 2. Conceptual Definition

A material is not a scalar record.

It is a structured entity defined by:

- Chemical composition,
- Crystallographic structure,
- Processing history,
- Doping configuration,
- Measurement context.

Formally, define a graph:

\[
\mathcal{G} = (\mathcal{V}, \mathcal{E})
\]

where:

- \( \mathcal{V} \) = set of nodes,
- \( \mathcal{E} \subseteq \mathcal{V} \times \mathcal{V} \) = set of edges.

---

# 3. Node Types

Nodes are typed:

\[
\mathcal{V}
=
\mathcal{V}_{material}
\cup
\mathcal{V}_{composition}
\cup
\mathcal{V}_{structure}
\cup
\mathcal{V}_{process}
\cup
\mathcal{V}_{measurement}
\]

Each material identity node \( v_m \in \mathcal{V}_{material} \) represents a unique material instance.

---

# 4. Composition Representation

Composition vector:

\[
\mathbf{c}
=
(c_1, c_2, \dots, c_k)
\]

with constraint:

\[
\sum_{i=1}^k c_i = 1
\]

Edges:

\[
(v_m, v_{element_i}) \in \mathcal{E}
\]

Weight:

\[
w_{mi} = c_i
\]

This forms a weighted bipartite subgraph between materials and elements.

---

# 5. Structural Identity

Structural node encodes:

- Space group,
- Lattice parameters,
- Phase classification.

Define structural descriptor:

\[
\mathbf{s}
=
(a, b, c, \alpha, \beta, \gamma, SG)
\]

Edge:

\[
(v_m, v_s) \in \mathcal{E}
\]

Structural nodes enable crystallographic equivalence detection.

---

# 6. Processing Lineage

Processing history is modeled as a directed acyclic graph (DAG):

\[
\mathcal{G}_{proc} = (\mathcal{V}_{proc}, \mathcal{E}_{proc})
\]

Edges:

\[
(v_{proc_i} \to v_{proc_j})
\]

represent transformation steps.

Material node connects to final process node:

\[
(v_{proc_{final}}, v_m)
\]

This encodes synthesis pathway.

---

# 7. Measurement Separation

Measurements are not identity.

Define measurement nodes:

\[
v_{meas}
\]

Edge:

\[
(v_m, v_{meas})
\]

Measurement nodes contain:

- Thermoelectric properties,
- Uncertainty,
- Credibility score.

This separation ensures identity invariance across experiments.

---

# 8. Identity Function

Define identity function:

\[
\mathrm{ID}(v_m)
=
\left(
\mathbf{c},
\mathbf{s},
\text{process fingerprint}
\right)
\]

Two material nodes are identical if:

\[
\mathrm{ID}(v_{m1}) = \mathrm{ID}(v_{m2})
\]

Within tolerance thresholds.

---

# 9. Graph Invariants

Material Identity Graph must satisfy:

1. No duplicate identity nodes.
2. Composition normalization constraint.
3. Acyclic processing subgraph.
4. Typed edge consistency.
5. Immutable identity after creation.

---

# 10. Graph Distance Metric

Define similarity between materials:

\[
d(m_i, m_j)
=
\lambda_c \| \mathbf{c}_i - \mathbf{c}_j \|_2
+
\lambda_s d_s(\mathbf{s}_i, \mathbf{s}_j)
+
\lambda_p d_p(proc_i, proc_j)
\]

This metric supports:

- Clustering,
- Nearest-neighbor retrieval,
- Graph embedding.

---

# 11. Spectral Representation

Adjacency matrix:

\[
A \in \mathbb{R}^{|\mathcal{V}| \times |\mathcal{V}|}
\]

Graph Laplacian:

\[
L = D - A
\]

Spectral decomposition:

\[
L = U \Lambda U^T
\]

Graph embeddings derived from eigenvectors enable machine learning integration.

---

# 12. Knowledge Consistency Constraint

If two measurement nodes connected to same material node report conflicting properties beyond uncertainty bounds:

\[
|y_1 - y_2| > k \sqrt{\sigma_1^2 + \sigma_2^2}
\]

Flag inconsistency.

Graph structure enables conflict detection at identity level.

---

# 13. Versioning and Temporal Evolution

Material identity is immutable.

However, graph evolves:

\[
\mathcal{G}_t
\]

Versioned snapshots:

\[
\{\mathcal{G}_1, \mathcal{G}_2, \dots\}
\]

Ensure reproducibility and auditability.

---

# 14. Graph Constraints and Physics

Graph-level constraint:

\[
v_m \rightarrow v_{meas}
\]

must satisfy physical constraints (P03).

Thus graph enforces physics structurally.

---

# 15. Integration with Bayesian Modeling

Define feature extraction function:

\[
\phi(v_m)
\in
\mathbb{R}^d
\]

Model:

\[
y = f_\theta(\phi(v_m))
\]

Graph embeddings become inputs to statistical models.

Identity graph ensures consistent feature mapping.

---

# 16. Information Flow

Graph enables:

- Propagation of credibility across measurements,
- Sharing of information between similar materials,
- Active learning prioritization.

Graph centrality measures:

\[
C_{deg}(v)
=
\frac{\deg(v)}{|\mathcal{V}|-1}
\]

identify influential materials.

---

# 17. Governance Requirements

The system must enforce:

1. Unique material identity per node.
2. No measurement stored without material linkage.
3. Explicit typing of all nodes and edges.
4. Full traceability of processing lineage.
5. Audit logs for graph mutation.

---

# 18. Strategic Interpretation

The Material Identity Graph transforms material data from flat tables into relational knowledge.

It enables:

- Structural reasoning,
- Similarity-aware modeling,
- Lineage-aware validation,
- Physics-aware graph analytics.

Without structured identity, inference collapses into fragmented records.

---

# 19. Compliance Requirement

All modules must satisfy:

\[
\text{Module} \models \text{G01-MATERIAL-IDENTITY-GRAPH}
\]

Non-compliance results in:

- Identity ambiguity,
- Duplicate representation,
- Knowledge fragmentation.

---

# 20. Concluding Statement

The Material Identity Graph defines the ontological foundation of the Thermognosis Engine.

Materials are not rows in a table.  
They are structured, relational, versioned scientific entities.

Robust AI-driven discovery requires structured identity before statistical inference.
