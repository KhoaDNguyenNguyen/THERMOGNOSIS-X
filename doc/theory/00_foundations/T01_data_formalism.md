# T01 — Formal Data Model and Measurement Space  
**Document ID:** T01-DATA-FORMALISM  
**Layer:** Theory / Foundations  
**Status:** Normative — Foundational  
**Dependency:** T00-SYS-AXIOMS  

---

# 1. Purpose

This document establishes the formal mathematical structure of data within the Thermognosis Engine.

Its objectives are:

1. To define the measurement space of thermoelectric properties.
2. To formalize the representation of experimental data.
3. To establish uncertainty modeling principles.
4. To define dataset evolution in a versioned closed-loop framework.
5. To guarantee consistency between physics, statistics, and storage layers.

This document is normative.  
All database schemas, API contracts, and modeling procedures must conform to the structures defined herein.

---

# 2. Ontology of Scientific Data

We define scientific data in this system as structured elements belonging to a measurable space:

\[
(\Omega, \mathcal{F}, \mathbb{P})
\]

where:

- \( \Omega \) is the sample space of all possible measurement outcomes,
- \( \mathcal{F} \) is the sigma-algebra of measurable events,
- \( \mathbb{P} \) is the probability measure encoding uncertainty.

A single thermoelectric observation is not treated as a scalar fact, but as a random vector:

\[
\mathbf{X} = (S, \sigma, \kappa, T)
\]

Each component is a random variable:

\[
X_i \sim \mathcal{D}_i(\mu_i, \theta_i)
\]

where:
- \( \mu_i \) is the central estimate,
- \( \theta_i \) encodes dispersion parameters (e.g., variance).

---

# 3. Measurement Space Definition

## 3.1 Thermoelectric Property Vector

Define the thermoelectric measurement space:

\[
\mathcal{M} \subset \mathbb{R}^4
\]

with coordinates:

\[
\mathcal{M} = \{ (S, \sigma, \kappa, T) \mid S \in \mathbb{R}, \sigma > 0, \kappa > 0, T > 0 \}
\]

Derived quantity:

\[
zT = \frac{S^2 \sigma T}{\kappa}
\]

Thus the extended space becomes:

\[
\mathcal{M}^* = \{ (S, \sigma, \kappa, T, zT) \}
\]

with constraint:

\[
zT = f(S, \sigma, \kappa, T)
\]

---

## 3.2 Temperature-Dependent Curves

A thermoelectric dataset is generally a function over temperature:

\[
\mathbf{X}(T) = (S(T), \sigma(T), \kappa(T))
\]

Define:

\[
T \in [T_{\min}, T_{\max}] \subset \mathbb{R}^+
\]

Thus a full measurement is an element of functional space:

\[
\mathbf{X} \in \mathcal{F}([T_{\min}, T_{\max}], \mathbb{R}^3)
\]

---

# 4. Canonical Representation of a Measurement

Each validated measurement record is formally defined as a tuple:

\[
m = (\mathcal{I}, \mathbf{X}(T), \mathbf{U}, w, \pi, v)
\]

where:

- \( \mathcal{I} \) : canonical material identity  
- \( \mathbf{X}(T) \) : property curves  
- \( \mathbf{U} \) : uncertainty structure  
- \( w \in [0,1] \) : credibility weight  
- \( \pi \) : provenance metadata  
- \( v \) : dataset version index  

---

# 5. Uncertainty Formalism

## 5.1 First-Order Error Propagation

Given:

\[
zT = f(S, \sigma, \kappa, T)
\]

Variance is approximated by:

\[
\mathrm{Var}(zT) \approx 
\sum_i 
\left( \frac{\partial f}{\partial x_i} \right)^2 
\mathrm{Var}(x_i)
\]

where:

\[
x_i \in \{S, \sigma, \kappa, T\}
\]

---

## 5.2 Monte Carlo Representation

Alternatively, define:

\[
x_i^{(k)} \sim \mathcal{D}_i
\]

\[
zT^{(k)} = f(x_1^{(k)}, x_2^{(k)}, x_3^{(k)}, x_4^{(k)})
\]

Estimate:

\[
\mathbb{E}[zT], \quad \mathrm{Var}(zT)
\]

---

# 6. Dataset Structure

Define dataset at version \( v \):

\[
\mathcal{D}_v = \{ m_1, m_2, \dots, m_n \}
\]

Evolution rule:

\[
\mathcal{D}_{v+1} = \mathcal{D}_v \cup \Delta \mathcal{D}_v
\]

Monotonicity condition:

\[
|\mathcal{D}_{v+1}| \ge |\mathcal{D}_v|
\]

No deletion without explicit archival record.

---

# 7. Weighted Statistical Learning

Let observations be:

\[
\{ (\mathbf{x}_i, y_i, w_i) \}_{i=1}^N
\]

Weighted likelihood:

\[
\mathcal{L}(\theta) = \prod_{i=1}^N p(y_i \mid \mathbf{x}_i, \theta)^{w_i}
\]

Posterior:

\[
p(\theta \mid \mathcal{D}) \propto \mathcal{L}(\theta) p(\theta)
\]

This formalism integrates credibility weighting into Bayesian modeling.

---

# 8. Graph-Structured Data Formalism

Define a directed graph:

\[
\mathcal{G} = (V, E)
\]

where:

- \( V \) includes material nodes and publication nodes,
- \( E \) includes:
  - citation edges,
  - material-derivation edges,
  - composition transformation edges.

Canonical material identity is defined as:

\[
\mathcal{C} : \text{RawFormula} \rightarrow V_{\text{material}}
\]

Graph invariance condition:

\[
\mathcal{C}(a) = \mathcal{C}(b) \Rightarrow a, b \text{ represent same material manifold}
\]

---

# 9. Functional View of the Entire Data Layer

The data layer is defined as:

\[
\mathcal{D} : \text{Literature} \rightarrow \mathcal{M}^*
\]

subject to constraints:

1. Deterministic ingestion  
2. Unit consistency  
3. Physical validity  
4. Explicit uncertainty  
5. Version traceability  

---

# 10. Information-Theoretic Characterization

Define entropy of dataset under model \( \mathcal{M}_v \):

\[
H(\mathcal{M}_v)
\]

Information gain from ingesting new measurement \( m \):

\[
IG(m) = H(\mathcal{M}_v) - H(\mathcal{M}_{v+1})
\]

This provides the formal basis for active acquisition.

---

# 11. Reproducibility Constraint

Each measurement must satisfy:

\[
\text{hash}(m_v) = h(\mathcal{I}, \mathbf{X}, \mathbf{U}, w, \pi)
\]

Thus:

\[
m_v \neq m_{v+1} \Rightarrow h_v \neq h_{v+1}
\]

Ensuring strict lineage traceability.

---

# 12. Philosophical Interpretation

Scientific data in the Thermognosis Engine is:

- Not a table of numbers,
- Not a static archive,
- Not a binary truth set.

It is a structured probabilistic manifold evolving under physical law and epistemic refinement.

---

# 13. Concluding Statement

This document formalizes the data ontology of the Thermognosis Engine.

All subsequent database schemas, API contracts, model architectures, and optimization procedures must preserve:

- measurement space integrity,
- uncertainty awareness,
- physical consistency,
- versioned evolution,
- weighted credibility.

The Thermognosis Engine is defined not merely by its code,  
but by the rigor of its data formalism.
