# QUAL — Credibility Specification  
**Document ID:** SPEC-QUAL-CREDIBILITY  
**Layer:** spec/06_quality  
**Status:** Normative — Scientific Credibility and Evidence Weighting Framework  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Credibility Specification (CRS)** governing the formal evaluation of scientific reliability, evidential strength, and trustworthiness of data, models, and derived results within the Thermognosis Engine.

Completeness ensures informational sufficiency.  
Credibility ensures epistemic reliability.

No dataset, extracted result, or model prediction shall influence decision-making, optimization, or publication without passing credibility assessment.

---

# 2. Formal Definition of Credibility

Let a record be:

\[
\mathcal{R}
\]

Define credibility function:

\[
\mathcal{K} : \mathcal{D} \rightarrow [0,1]
\]

where:

\[
\mathcal{K}(\mathcal{R}) = 1
\]

indicates maximum credibility and

\[
\mathcal{K}(\mathcal{R}) = 0
\]

indicates unusable evidence.

Minimum admissible threshold:

\[
\mathcal{K}(\mathcal{R}) \ge \tau_{\text{cred}}
\]

Default:

\[
\tau_{\text{cred}} = 0.75
\]

---

# 3. Source Credibility Component

Each record must have a source classification:

- Peer-reviewed journal
- Conference proceeding
- Preprint
- Internal measurement
- Simulated data

Assign base source weight:

\[
w_{\text{source}} \in [0,1]
\]

Higher-tier journals receive higher prior credibility weight.

---

# 4. Reproducibility Component

If independent confirmations exist:

\[
n_{\text{rep}} \ge 1
\]

Reproducibility factor:

\[
w_{\text{rep}} =
1 - e^{-n_{\text{rep}}}
\]

Lack of replication reduces credibility.

---

# 5. Uncertainty Transparency Component

Presence of uncertainty:

\[
\sigma \text{ defined}
\]

Transparency weight:

\[
w_{\text{unc}} =
\begin{cases}
1 & \text{if uncertainty reported} \\
0.5 & \text{if partially reported} \\
0 & \text{if missing}
\end{cases}
\]

---

# 6. Physical Consistency Component

If record satisfies:

\[
\mathcal{S} \models \text{PHYS-CONSTRAINTS}
\]

then:

\[
w_{\text{phys}} = 1
\]

Otherwise penalized proportionally to violation magnitude:

\[
w_{\text{phys}} =
e^{-\alpha \Delta_{\text{phys}}}
\]

---

# 7. Statistical Robustness Component

For sample size:

\[
n
\]

Robustness factor:

\[
w_{\text{stat}} =
\frac{n}{n + n_0}
\]

where \( n_0 \) is stabilization constant.

Insufficient sample size reduces credibility.

---

# 8. Model Validity Component

For surrogate model prediction:

\[
\hat{y} = f_\theta(x)
\]

Cross-validation error:

\[
E_{\text{CV}}
\]

Credibility weight:

\[
w_{\text{model}} =
e^{-\beta E_{\text{CV}}}
\]

Poor predictive performance lowers credibility.

---

# 9. Consistency Across Sources

If multiple independent sources provide value:

\[
x_i
\]

Compute weighted mean:

\[
\bar{x} =
\frac{\sum w_i x_i}{\sum w_i}
\]

Variance of disagreement:

\[
\sigma_{\text{between}}^2
=
\frac{\sum w_i (x_i - \bar{x})^2}{\sum w_i}
\]

High inter-source variance reduces credibility.

---

# 10. Temporal Credibility

Older measurements may become obsolete.

Time decay factor:

\[
w_{\text{time}} =
e^{-\lambda (t_{\text{current}} - t_{\text{pub}})}
\]

If physics unchanged, decay may be disabled.

---

# 11. Composite Credibility Score

Overall credibility:

\[
\mathcal{K}(\mathcal{R})
=
w_{\text{source}}
\cdot
w_{\text{rep}}
\cdot
w_{\text{unc}}
\cdot
w_{\text{phys}}
\cdot
w_{\text{stat}}
\cdot
w_{\text{model}}
\cdot
w_{\text{time}}
\]

Multiplicative structure ensures weakest component dominates.

---

# 12. Bayesian Credibility Interpretation

Credibility can be interpreted as posterior belief:

\[
\mathcal{K}(\mathcal{R})
=
\mathbb{P}(\text{Record is reliable} \mid \text{Evidence})
\]

Updating rule:

\[
P(H|E) =
\frac{P(E|H)P(H)}{P(E)}
\]

This enables dynamic credibility revision.

---

# 13. Conflict Resolution

If two high-credibility records conflict:

\[
|x_1 - x_2| > \gamma
\]

System must:

1. Flag inconsistency,
2. Trigger manual review,
3. Record discrepancy magnitude.

---

# 14. Credibility Classes

Classification:

- Class A: \( \mathcal{K} \ge 0.9 \)
- Class B: \( 0.75 \le \mathcal{K} < 0.9 \)
- Class C: \( 0.5 \le \mathcal{K} < 0.75 \)
- Class D: \( \mathcal{K} < 0.5 \)

Only Class A and B eligible for model training by default.

---

# 15. Determinism Requirement

Given identical:

- Input metadata,
- Validation metrics,
- Weight parameters,

Credibility score must be deterministic.

Version change must be logged.

---

# 16. Sensitivity Analysis

Sensitivity of credibility to component weights:

\[
\frac{\partial \mathcal{K}}{\partial w_i}
=
\frac{\mathcal{K}}{w_i}
\]

Large sensitivity implies fragile credibility.

System must monitor weight dominance.

---

# 17. Error Classification

- QUAL-CRED-01: Low source reliability
- QUAL-CRED-02: Missing replication
- QUAL-CRED-03: Missing uncertainty
- QUAL-CRED-04: Physical inconsistency
- QUAL-CRED-05: Statistical weakness
- QUAL-CRED-06: Model instability
- QUAL-CRED-07: Inter-source conflict

All credibility reductions must log reason.

---

# 18. Governance Rule

If:

\[
\mathcal{K}(\mathcal{R}) < \tau_{\text{cred}}
\]

record must:

- Be excluded from training,
- Be excluded from optimization,
- Be clearly labeled in reports.

---

# 19. Strategic Interpretation

Credibility enforcement ensures:

- Evidence weighting is principled.
- Weak or speculative results do not bias inference.
- Conflicting literature is formally resolved.
- Publication-level conclusions are defensible.

Credibility converts qualitative trust into quantitative governance.

---

# 20. Formal Soundness Condition

Credibility framework is sound if:

1. All components are measurable.
2. Score bounded in \([0,1]\).
3. Updates monotonic with added evidence.
4. Deterministic under fixed parameters.

---

# 21. Concluding Statement

All records must satisfy:

\[
\mathcal{R} \models \text{SPEC-QUAL-CREDIBILITY}
\]

Only sufficiently credible evidence may influence modeling, optimization, and scientific reporting.

Scientific rigor requires not only data — but justified trust in that data.
