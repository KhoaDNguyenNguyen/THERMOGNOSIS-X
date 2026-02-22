# G02 — Citation Graph Dynamics  
**Document ID:** G02-CITATION-GRAPH-DYNAMICS  
**Layer:** Graph Modeling / Knowledge Provenance  
**Status:** Normative — Provenance and Influence Modeling  
**Dependencies:**  
- T00-SYS-AXIOMS  
- T01-DATA-FORMALISM  
- S01-BAYESIAN-CREDIBILITY-MODEL  
- G01-MATERIAL-IDENTITY-GRAPH  

---

# 1. Purpose

This document formalizes the **Citation Graph Dynamics (CGD)** module of the Thermognosis Engine.

Its objectives are:

1. To model scientific publications as a dynamic directed graph.
2. To quantify influence, trust propagation, and knowledge flow.
3. To integrate citation structure into credibility modeling.
4. To enable detection of anomalous or low-impact data sources.
5. To provide temporal awareness of scientific evolution.

Scientific data must not be interpreted without provenance structure.

---

# 2. Graph Definition

Define citation graph:

\[
\mathcal{G}_c = (\mathcal{V}_c, \mathcal{E}_c)
\]

where:

- \( \mathcal{V}_c \) = set of publication nodes.
- \( \mathcal{E}_c \subseteq \mathcal{V}_c \times \mathcal{V}_c \) = directed citation edges.

Edge:

\[
(v_i \to v_j)
\]

means publication \( i \) cites publication \( j \).

Graph is directed and acyclic under temporal ordering.

---

# 3. Temporal Structure

Each node has timestamp:

\[
t_i \in \mathbb{R}
\]

Constraint:

\[
(v_i \to v_j)
\Rightarrow
t_i > t_j
\]

Thus citation graph forms a directed acyclic graph (DAG).

Dynamic graph at time \( T \):

\[
\mathcal{G}_c(T)
\]

---

# 4. Adjacency Matrix Representation

Adjacency matrix:

\[
A_{ij}
=
\begin{cases}
1 & \text{if } v_i \to v_j \\
0 & \text{otherwise}
\end{cases}
\]

In-degree:

\[
k_j^{in} = \sum_i A_{ij}
\]

Out-degree:

\[
k_i^{out} = \sum_j A_{ij}
\]

---

# 5. PageRank-Based Influence

Define PageRank vector:

\[
\mathbf{r}
\]

Update rule:

\[
\mathbf{r}
=
\alpha A^T D^{-1} \mathbf{r}
+
(1-\alpha)\mathbf{v}
\]

where:

- \( D \) is diagonal matrix of out-degrees,
- \( \alpha \in (0,1) \) damping factor,
- \( \mathbf{v} \) personalization vector.

Stationary solution:

\[
\mathbf{r} = \lim_{k \to \infty} \mathbf{r}^{(k)}
\]

Higher \( r_i \) indicates greater scientific influence.

---

# 6. Influence-Weighted Credibility

Let publication \( s(i) \) be source of measurement \( i \).

Define influence score:

\[
I_{s(i)} = r_{s(i)}
\]

Modify credibility prior:

\[
C_i \propto \phi(I_{s(i)})
\]

Example mapping:

\[
C_i^{prior} =
\frac{I_{s(i)}}{\max_j I_j}
\]

This integrates citation influence into trust model.

---

# 7. Temporal Decay Model

Scientific relevance decays over time.

Define time-weighted influence:

\[
I_i(t)
=
r_i \exp(-\lambda (t - t_i))
\]

Decay parameter \( \lambda > 0 \).

Recent influential papers receive higher dynamic weight.

---

# 8. Knowledge Diffusion Model

Define influence propagation:

\[
\mathbf{C}(t+1)
=
\beta A^T \mathbf{C}(t)
+
(1-\beta)\mathbf{C}_0
\]

This models credibility diffusion across citation network.

Fixed point:

\[
\mathbf{C}^*
=
(I - \beta A^T)^{-1}
\mathbf{C}_0
\]

---

# 9. Anomaly Detection in Citation Structure

Anomalous patterns include:

- Isolated nodes with extreme claims.
- Self-citation clusters.
- Citation cartels.

Define clustering coefficient:

\[
C_i
=
\frac{2e_i}{k_i(k_i - 1)}
\]

High clustering combined with low external connectivity may indicate insular citation behavior.

---

# 10. Community Detection

Modularity:

\[
Q
=
\frac{1}{2m}
\sum_{ij}
\left[
A_{ij}
-
\frac{k_i k_j}{2m}
\right]
\delta(c_i, c_j)
\]

Community detection identifies subfields or thematic clusters.

This supports domain-specific credibility adjustment.

---

# 11. Spectral Analysis

Graph Laplacian:

\[
L = D - A
\]

Eigen decomposition:

\[
L = U \Lambda U^T
\]

Spectral gap indicates connectivity strength.

Small spectral gap may imply fragmented knowledge domains.

---

# 12. Integration with Material Identity Graph

Each publication node connects to:

\[
v_{pub} \to v_{meas}
\]

Measurement credibility depends on:

- Intrinsic uncertainty,
- Physical consistency,
- Citation influence.

Combined credibility:

\[
C_i =
f(C_i^{intrinsic}, I_{s(i)})
\]

---

# 13. Dynamic Evolution Modeling

Citation growth approximated by preferential attachment:

\[
P(k)
\propto k^{-\gamma}
\]

with typical:

\[
2 < \gamma < 3
\]

This scale-free structure implies influence concentration.

Model must account for heavy-tailed distribution.

---

# 14. Information Centrality

Betweenness centrality:

\[
BC(v)
=
\sum_{s \neq v \neq t}
\frac{\sigma_{st}(v)}{\sigma_{st}}
\]

High betweenness nodes bridge knowledge clusters.

Such publications may have disproportionate epistemic impact.

---

# 15. Credibility Stability Constraint

To avoid overreliance on citation count:

Require blended credibility:

\[
C_i
=
\alpha C_i^{data}
+
(1-\alpha) C_i^{citation}
\]

with \( 0 < \alpha < 1 \).

Data evidence must dominate citation prestige.

---

# 16. Governance Requirements

System must:

1. Maintain up-to-date citation graph.
2. Version graph snapshots.
3. Log influence metrics.
4. Prevent circular credibility amplification.
5. Allow audit of citation-based weight adjustment.

---

# 17. Strategic Interpretation

Citation Graph Dynamics enables:

- Quantitative provenance modeling,
- Influence-aware credibility adjustment,
- Detection of epistemic concentration,
- Temporal awareness of scientific evolution.

Scientific authority emerges from structured network dynamics, not isolated metrics.

---

# 18. Compliance Requirement

All credibility modules must satisfy:

\[
\text{Module} \models \text{G02-CITATION-GRAPH-DYNAMICS}
\]

Ignoring citation structure results in:

- Context-free inference,
- Misaligned trust modeling,
- Loss of provenance awareness.

---

# 19. Concluding Statement

The Citation Graph Dynamics module embeds the Thermognosis Engine within the evolving structure of scientific knowledge.

Data does not exist in isolation.  
It resides within a dynamic, directed, influence-weighted epistemic network.

Robust scientific intelligence requires modeling not only measurements,  
but the structure of scientific discourse itself.
