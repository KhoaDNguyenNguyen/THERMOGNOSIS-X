# CL02 — Information Gain Selection  
**Document ID:** CL02-INFORMATION-GAIN-SELECTION  
**Layer:** Closed-Loop Intelligence / Experimental Design  
**Status:** Normative — Information-Theoretic Acquisition Policy  
**Dependencies:**  
- T00-SYS-AXIOMS  
- T03-UNCERTAINTY-PROPAGATION-THEORY  
- S01-BAYESIAN-CREDIBILITY-MODEL  
- G03-EMBEDDING-RANK-THEORY  
- CL01-CLOSED-LOOP-OPERATOR-DEFINITION  

---

# 1. Purpose

This document formalizes the **Information Gain Selection (IGS)** mechanism used to determine the next optimal experiment or data acquisition step within the closed-loop system.

The objective is to ensure that every selected experiment:

1. Maximizes expected knowledge gain,
2. Reduces epistemic uncertainty,
3. Respects physical constraints,
4. Maintains long-term convergence stability.

This module defines the acquisition policy using rigorous information theory.

---

# 2. Bayesian State Representation

At iteration \( t \), define posterior:

\[
p(\theta | \mathcal{D}_t)
\]

where:

- \( \theta \) = model parameters,
- \( \mathcal{D}_t \) = current dataset.

Predictive distribution for candidate material \( v \):

\[
p(y_v | \mathcal{D}_t)
=
\int
p(y_v | \theta)
p(\theta | \mathcal{D}_t)
d\theta
\]

---

# 3. Entropy Definition

Shannon entropy of parameter posterior:

\[
H[p(\theta | \mathcal{D}_t)]
=
- \int
p(\theta | \mathcal{D}_t)
\log p(\theta | \mathcal{D}_t)
d\theta
\]

Entropy quantifies epistemic uncertainty.

---

# 4. Expected Information Gain

For candidate experiment \( v \), define:

\[
IG(v)
=
H[p(\theta | \mathcal{D}_t)]
-
\mathbb{E}_{y_v}
H[p(\theta | \mathcal{D}_t \cup \{y_v\})]
\]

where expectation is taken over predictive distribution:

\[
\mathbb{E}_{y_v}
=
\int
p(y_v | \mathcal{D}_t)
(\cdot)
dy_v
\]

Interpretation:

- High \( IG(v) \) implies strong expected reduction in uncertainty.

---

# 5. Mutual Information Interpretation

Information gain equivalently expressed as mutual information:

\[
IG(v)
=
I(y_v ; \theta | \mathcal{D}_t)
\]

where:

\[
I(X;Y)
=
H(X)
-
H(X|Y)
\]

Thus selection maximizes information shared between observation and parameters.

---

# 6. Gaussian Process Closed-Form Case

For Gaussian Process model:

Predictive variance:

\[
\sigma^2(v)
\]

Posterior entropy proportional to:

\[
\log \det K_t
\]

Information gain simplifies to:

\[
IG(v)
=
\frac{1}{2}
\log
\left(
1 + \frac{\sigma^2(v)}{\sigma_n^2}
\right)
\]

where:

- \( \sigma_n^2 \) = noise variance.

Thus:

\[
\arg\max IG(v)
=
\arg\max \sigma^2(v)
\]

for homoscedastic noise.

---

# 7. Cost-Aware Information Gain

Define experiment cost:

\[
C(v)
\]

Define normalized objective:

\[
A(v)
=
\frac{IG(v)}{C(v)}
\]

Selection rule:

\[
v_{t+1}
=
\arg\max_v A(v)
\]

Ensures economic rationality.

---

# 8. Constraint-Aware Selection

Let physical constraint operator:

\[
\mathcal{C}_{phys}(v)
\in
\{0,1\}
\]

Define feasible set:

\[
\mathcal{V}_{feasible}
=
\{ v \mid \mathcal{C}_{phys}(v)=1 \}
\]

Selection restricted to:

\[
v_{t+1}
=
\arg\max_{v \in \mathcal{V}_{feasible}} IG(v)
\]

---

# 9. Risk-Averse Information Gain

Introduce risk penalty:

\[
R(v)
=
\lambda \mathbb{E}[\text{Constraint Violation}]
\]

Adjusted acquisition:

\[
A(v)
=
IG(v) - R(v)
\]

Ensures safe exploration.

---

# 10. Multi-Objective Information Gain

When optimizing multiple properties \( y^{(k)} \):

\[
IG(v)
=
\sum_k
w_k
I(y_v^{(k)} ; \theta_k | \mathcal{D}_t)
\]

with:

\[
\sum_k w_k = 1
\]

Supports joint thermoelectric optimization.

---

# 11. Exploration–Exploitation Trade-Off

Define utility function:

\[
U(v)
=
\mathbb{E}[zT(v)]
\]

Hybrid objective:

\[
A(v)
=
\alpha IG(v)
+
(1-\alpha) U(v)
\]

Parameter \( \alpha \in [0,1] \).

---

# 12. Submodularity Property

Information gain is submodular under Gaussian assumptions:

\[
IG(A \cup \{v\}) - IG(A)
\ge
IG(B \cup \{v\}) - IG(B)
\]

for \( A \subseteq B \).

This property enables greedy near-optimal selection.

---

# 13. Convergence Property

Under regularity conditions:

\[
\lim_{t \to \infty}
H[p(\theta | \mathcal{D}_t)]
\rightarrow
0
\]

if sufficient informative experiments selected.

Ensures asymptotic learning.

---

# 14. Approximation Strategies

Exact computation may be intractable.

Approximation options:

1. Monte Carlo estimation:
\[
IG(v)
\approx
\frac{1}{N}
\sum_{i=1}^N
\left[
\log p(\theta_i | \mathcal{D}_t \cup y_v^{(i)})
-
\log p(\theta_i | \mathcal{D}_t)
\right]
\]

2. Variational approximation.
3. Laplace approximation.

Approximation error must be logged.

---

# 15. Numerical Stability Requirement

Ensure:

\[
\det(K_t) > 0
\]

and avoid singular covariance.

Use jitter:

\[
K_t \leftarrow K_t + \epsilon I
\]

---

# 16. Governance Requirements

System must:

1. Log computed \( IG(v) \) values.
2. Record cost assumptions.
3. Version acquisition hyperparameters.
4. Provide reproducible sampling seeds.
5. Allow post-hoc audit of selection decision.

---

# 17. Strategic Interpretation

Information Gain Selection transforms experiment planning into:

- Quantitative uncertainty reduction,
- Economically rational decision-making,
- Physically constrained exploration.

It formalizes scientific curiosity as entropy minimization.

---

# 18. Compliance Requirement

All acquisition modules must satisfy:

\[
\text{Acquisition Engine}
\models
\text{CL02-INFORMATION-GAIN-SELECTION}
\]

Non-compliance results in:

- Heuristic experimentation,
- Uncontrolled exploration,
- Inefficient resource allocation.

---

# 19. Concluding Statement

Information Gain Selection defines the rational core of the closed-loop system.

Experiments are not chosen by intuition.  
They are selected by maximizing expected reduction in epistemic uncertainty.

In this framework, scientific progress is quantified as entropy minimization under physical and economic constraints.
