# CL01 — Closed-Loop Operator Definition  
**Document ID:** CL01-CLOSED-LOOP-OPERATOR-DEFINITION  
**Layer:** Closed-Loop Intelligence / Decision Dynamics  
**Status:** Normative — Core Control-Theoretic Specification  
**Dependencies:**  
- T00-SYS-AXIOMS  
- T01-DATA-FORMALISM  
- T03-UNCERTAINTY-PROPAGATION-THEORY  
- S01-BAYESIAN-CREDIBILITY-MODEL  
- G01-MATERIAL-IDENTITY-GRAPH  
- G03-EMBEDDING-RANK-THEORY  

---

# 1. Purpose

This document defines the **Closed-Loop Operator (CLO)** governing the iterative scientific intelligence cycle of the Thermognosis Engine.

The Closed-Loop Operator formalizes:

1. Data assimilation,
2. Model update,
3. Decision selection,
4. Experiment recommendation,
5. Feedback incorporation.

The objective is to transform the system from a passive analytics engine into an **active scientific decision system**.

---

# 2. Conceptual Overview

Let the system state at iteration \( t \) be:

\[
\mathcal{S}_t
=
(\mathcal{G}_t, \mathcal{M}_t, \mathcal{D}_t)
\]

where:

- \( \mathcal{G}_t \): Graph state (materials + citations),
- \( \mathcal{M}_t \): Statistical model state,
- \( \mathcal{D}_t \): Dataset.

The Closed-Loop Operator is defined as:

\[
\mathcal{O}
:
\mathcal{S}_t
\rightarrow
\mathcal{S}_{t+1}
\]

---

# 3. Operator Decomposition

The operator decomposes into five sub-operators:

\[
\mathcal{O}
=
\mathcal{U}
\circ
\mathcal{A}
\circ
\mathcal{R}
\circ
\mathcal{E}
\circ
\mathcal{F}
\]

where:

- \( \mathcal{F} \): Data Fusion,
- \( \mathcal{E} \): Estimation,
- \( \mathcal{R} \): Ranking,
- \( \mathcal{A} \): Acquisition,
- \( \mathcal{U} \): Update.

---

# 4. Data Fusion Operator \( \mathcal{F} \)

Input:

\[
\mathcal{D}_t, \mathcal{G}_t
\]

Output:

\[
\tilde{\mathcal{D}}_t
\]

Fusion rule incorporates credibility:

\[
y_i^{fused}
=
\sum_{j \in \mathcal{N}(i)}
w_j y_j
\]

with:

\[
w_j
=
\frac{C_j}{\sum_k C_k}
\]

where \( C_j \) is credibility score.

---

# 5. Estimation Operator \( \mathcal{E} \)

Given fused dataset:

\[
\tilde{\mathcal{D}}_t
\]

Posterior update:

\[
p(\theta | \tilde{\mathcal{D}}_t)
\propto
p(\tilde{\mathcal{D}}_t | \theta)
p(\theta)
\]

Model predictive distribution:

\[
p(y^* | x^*, \tilde{\mathcal{D}}_t)
\]

This defines current belief state.

---

# 6. Ranking Operator \( \mathcal{R} \)

Define expected utility:

\[
U(v)
=
\mathbb{E}[zT(v)]
\]

Posterior variance:

\[
\sigma^2(v)
\]

Rank score:

\[
R(v)
=
\alpha U(v)
+
\beta \sigma(v)
+
\gamma C(v)
\]

Constraint:

\[
\alpha + \beta + \gamma = 1
\]

---

# 7. Acquisition Operator \( \mathcal{A} \)

Define acquisition function:

\[
A(v)
=
\mathbb{E}[U(v)]
+
\kappa \sigma(v)
\]

or Expected Improvement:

\[
EI(v)
=
\mathbb{E}[\max(0, U(v) - U_{best})]
\]

Select next experiment:

\[
v_{t+1}
=
\arg\max_{v} A(v)
\]

---

# 8. Update Operator \( \mathcal{U} \)

After experiment result \( y_{t+1} \):

\[
\mathcal{D}_{t+1}
=
\mathcal{D}_t
\cup
\{(v_{t+1}, y_{t+1})\}
\]

Graph update:

\[
\mathcal{G}_{t+1}
=
\mathcal{G}_t
\cup
\text{new nodes/edges}
\]

Model retraining:

\[
\theta_{t+1}
=
\arg\max
p(\theta | \mathcal{D}_{t+1})
\]

---

# 9. Fixed Point Condition

Closed-loop convergence condition:

\[
\mathcal{S}_{t+1}
\approx
\mathcal{S}_t
\]

or:

\[
\|\theta_{t+1} - \theta_t\| < \epsilon
\]

Indicates knowledge stabilization.

---

# 10. Stability Criterion

Define Lyapunov-like function:

\[
V_t
=
\mathbb{E}[\text{Prediction Error}_t]
\]

Closed-loop stable if:

\[
V_{t+1} \le V_t
\]

Monotonic decrease ensures learning consistency.

---

# 11. Information Gain Objective

Information gain:

\[
IG(v)
=
H[p(\theta | \mathcal{D}_t)]
-
\mathbb{E}_{y_v}
H[p(\theta | \mathcal{D}_t \cup y_v)]
\]

Acquisition may maximize:

\[
v_{t+1}
=
\arg\max IG(v)
\]

---

# 12. Physical Constraint Enforcement

All updates must satisfy:

\[
\mathcal{C}_{phys}(v) = 1
\]

Where:

\[
\mathcal{C}_{phys}
\]

represents physical consistency operator defined in P03.

If violated:

- Reject measurement,
- Down-weight credibility,
- Flag anomaly.

---

# 13. Uncertainty Propagation Across Loop

Total uncertainty:

\[
\sigma_{total}^2
=
\sigma_{measurement}^2
+
\sigma_{model}^2
+
\sigma_{structural}^2
\]

Uncertainty must be recalculated each iteration.

No reuse of outdated posterior.

---

# 14. Exploration–Exploitation Balance

Define exploration coefficient \( \kappa \).

Dynamic schedule:

\[
\kappa_t
=
\kappa_0 \exp(-\eta t)
\]

Ensures:

- Early exploration,
- Late exploitation.

---

# 15. Operator Composition Formalism

Closed-loop iteration:

\[
\mathcal{S}_{t+1}
=
\mathcal{O}(\mathcal{S}_t)
\]

Recursive expansion:

\[
\mathcal{S}_{t+n}
=
\mathcal{O}^n(\mathcal{S}_t)
\]

This defines discrete-time scientific control system.

---

# 16. Convergence Guarantees (Theoretical)

Under assumptions:

1. Bounded parameter space,
2. Positive information gain,
3. Stable likelihood,

We expect:

\[
\lim_{t \to \infty}
\mathbb{E}[\text{Prediction Error}_t]
\rightarrow
\epsilon
\]

---

# 17. Governance Requirements

The Closed-Loop Operator must:

1. Log every acquisition decision.
2. Version all model states.
3. Store acquisition rationale.
4. Maintain reproducible random seeds.
5. Provide rollback capability.

---

# 18. Strategic Interpretation

The Closed-Loop Operator transforms the Thermognosis Engine into:

- A self-improving system,
- A decision-theoretic controller,
- A physics-aware learning agent.

It operationalizes scientific reasoning into an iterative computational process.

---

# 19. Compliance Requirement

All deployment pipelines must satisfy:

\[
\text{Pipeline} \models \text{CL01-CLOSED-LOOP-OPERATOR-DEFINITION}
\]

Failure results in:

- Static inference,
- No learning dynamics,
- No strategic experimentation.

---

# 20. Concluding Statement

The Closed-Loop Operator defines the formal mechanism by which the Thermognosis Engine learns, decides, and evolves.

It is not merely automation.  
It is a mathematically defined scientific control system.

True intelligence emerges only when estimation, ranking, acquisition, and update are rigorously composed into a stable closed loop.
