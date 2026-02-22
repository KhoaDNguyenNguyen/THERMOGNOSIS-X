# QUAL — Unified Scoring Specification  
**Document ID:** SPEC-QUAL-SCORING  
**Layer:** spec/06_quality  
**Status:** Normative — Integrated Quality Scoring and Decision Framework  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Unified Quality Scoring Specification (UQSS)** for aggregating multiple quality dimensions into a single, interpretable, mathematically sound decision metric.

While individual specifications define:

- Completeness,
- Credibility,
- Physical consistency,
- Uncertainty rigor,
- Smoothness validity,

this document establishes a formal scoring system that integrates these dimensions into a reproducible governance mechanism.

The unified score governs:

- Model training eligibility,
- Optimization inclusion,
- Publication readiness,
- Closed-loop experimentation.

---

# 2. Quality Vector Representation

For each record \( \mathcal{R} \), define a quality vector:

\[
\mathbf{q} =
\left(
q_{\text{comp}},
q_{\text{cred}},
q_{\text{phys}},
q_{\text{err}},
q_{\text{smooth}},
q_{\text{meta}}
\right)
\]

where each component lies in:

\[
q_i \in [0,1]
\]

All components must be formally defined by their respective specifications.

---

# 3. Hard Constraint Gate

Before scoring, enforce binary admissibility:

\[
\mathcal{G}(\mathcal{R}) =
\begin{cases}
1 & \text{if mandatory constraints satisfied} \\
0 & \text{otherwise}
\end{cases}
\]

If:

\[
\mathcal{G}(\mathcal{R}) = 0
\]

then final score:

\[
S(\mathcal{R}) = 0
\]

No partial credit for fundamental violations.

---

# 4. Weighted Aggregation Model

Primary scoring model:

\[
S(\mathcal{R})
=
\sum_{i=1}^n
w_i q_i
\]

subject to:

\[
\sum_{i=1}^n w_i = 1
\]

Weights reflect strategic importance.

Default weight configuration:

\[
\begin{aligned}
w_{\text{comp}} &= 0.25 \\
w_{\text{cred}} &= 0.25 \\
w_{\text{phys}} &= 0.20 \\
w_{\text{err}} &= 0.15 \\
w_{\text{smooth}} &= 0.10 \\
w_{\text{meta}} &= 0.05
\end{aligned}
\]

Weights must be version-controlled.

---

# 5. Multiplicative Risk-Sensitive Model

Alternative conservative model:

\[
S_{\text{mult}}(\mathcal{R})
=
\prod_{i=1}^n q_i^{w_i}
\]

Properties:

- Sensitive to weakest dimension,
- Penalizes imbalance,
- Suitable for high-risk scientific claims.

---

# 6. Entropy-Regularized Score

Define quality entropy:

\[
H(\mathbf{q}) =
- \sum_i q_i \log q_i
\]

Low entropy indicates dominance by few dimensions.

Entropy penalty:

\[
S_{\text{reg}}
=
S - \lambda H(\mathbf{q})
\]

Encourages balanced quality profile.

---

# 7. Uncertainty-Weighted Adjustment

If uncertainty in quality components exists:

\[
q_i \sim \mathcal{N}(\mu_i, \sigma_i^2)
\]

Expected score:

\[
\mathbb{E}[S]
=
\sum_i w_i \mu_i
\]

Risk-adjusted score:

\[
S_{\text{risk}}
=
\mathbb{E}[S] - \gamma \sqrt{\mathrm{Var}(S)}
\]

---

# 8. Threshold Classification

Define score classes:

- Class A: \( S \ge 0.90 \)
- Class B: \( 0.80 \le S < 0.90 \)
- Class C: \( 0.65 \le S < 0.80 \)
- Class D: \( 0.50 \le S < 0.65 \)
- Reject: \( S < 0.50 \)

Default modeling eligibility:

\[
S \ge 0.80
\]

---

# 9. Pareto-Dominance Analysis

For records \( \mathcal{R}_1, \mathcal{R}_2 \):

\[
\mathcal{R}_1 \succ \mathcal{R}_2
\]

if:

\[
q_i^{(1)} \ge q_i^{(2)} \ \forall i
\quad
\text{and}
\quad
\exists j : q_j^{(1)} > q_j^{(2)}
\]

Pareto frontier records receive priority weighting.

---

# 10. Sensitivity Analysis

Sensitivity of total score to component \( q_i \):

\[
\frac{\partial S}{\partial q_i} = w_i
\]

For multiplicative model:

\[
\frac{\partial S_{\text{mult}}}{\partial q_i}
=
w_i \frac{S_{\text{mult}}}{q_i}
\]

Large sensitivity indicates vulnerability.

---

# 11. Stability Under Weight Perturbation

Let perturbed weights:

\[
w_i' = w_i + \delta w_i
\]

Stability condition:

\[
|S' - S| \le \epsilon
\]

for small \( \|\delta w\| \).

Instability suggests over-weighting of single dimension.

---

# 12. Time-Evolution of Score

If record updated:

\[
S_t(\mathcal{R})
\]

Score must evolve monotonically under quality improvement:

\[
\Delta q_i > 0
\Rightarrow
\Delta S \ge 0
\]

---

# 13. Cross-Dataset Normalization

To compare across datasets:

\[
\tilde{S} =
\frac{S - S_{\min}}{S_{\max} - S_{\min}}
\]

Ensures comparability across project phases.

---

# 14. Auditability Requirement

Every score must log:

- Component values,
- Weight configuration,
- Model version,
- Timestamp.

Recomputation must reproduce identical result.

---

# 15. Error Classification

- QUAL-SCORE-01: Missing component score
- QUAL-SCORE-02: Weight misconfiguration
- QUAL-SCORE-03: Non-normalized weights
- QUAL-SCORE-04: Entropy instability
- QUAL-SCORE-05: Risk-adjusted miscalculation

All scoring anomalies must be traceable.

---

# 16. Governance Rule

A record is eligible for:

- Model training if \( S \ge 0.80 \),
- Optimization input if \( S \ge 0.85 \),
- Publication-grade reporting if \( S \ge 0.90 \).

Override requires documented justification.

---

# 17. Formal Soundness Condition

Scoring framework is sound if:

1. Bounded in \( [0,1] \),
2. Monotonic in each component,
3. Deterministic under fixed weights,
4. Robust to small perturbations,
5. Transparent and auditable.

---

# 18. Strategic Interpretation

Unified scoring transforms quality evaluation into:

- Quantitative governance,
- Automated decision control,
- Transparent ranking,
- Reproducible prioritization.

It ensures scientific claims are not merely valid — but optimally supported.

---

# 19. Concluding Statement

All records must satisfy:

\[
\mathcal{R} \models \text{SPEC-QUAL-SCORING}
\]

Only records meeting required score thresholds may influence modeling, optimization, or strategic reporting.

Scientific excellence requires measurable quality.
