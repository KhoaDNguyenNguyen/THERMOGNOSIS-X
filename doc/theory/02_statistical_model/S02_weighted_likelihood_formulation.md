# S02 — Weighted Likelihood Formulation  
**Document ID:** S02-WEIGHTED-LIKELIHOOD-FORMULATION  
**Layer:** Statistical Modeling / Inference Engine  
**Status:** Normative — Core Inference Mechanism  
**Dependencies:**  
- T00-SYS-AXIOMS  
- T01-DATA-FORMALISM  
- T03-UNCERTAINTY-PROPAGATION  
- S01-BAYESIAN-CREDIBILITY-MODEL  
- P03-PHYSICAL-CONSTRAINTS  

---

# 1. Purpose

This document formalizes the **Weighted Likelihood Formulation (WLF)** used throughout the Thermognosis Engine.

Its objectives are:

1. To rigorously define credibility-weighted inference.
2. To integrate uncertainty-aware and credibility-aware modeling.
3. To provide a mathematically coherent alternative to binary data filtering.
4. To establish a scalable inference mechanism robust to heterogeneous data quality.
5. To serve as the default statistical backbone for all learning modules.

This formulation is normative.  
All statistical learning components must comply with this weighted inference framework.

---

# 2. Classical Likelihood Formulation

Given dataset:

\[
\mathcal{D} = \{(x_i, y_i, \sigma_i)\}_{i=1}^N
\]

Standard likelihood under Gaussian noise:

\[
p(\mathcal{D} \mid \theta)
=
\prod_{i=1}^N
\mathcal{N}
\left(
y_i \mid f_\theta(x_i), \sigma_i^2
\right)
\]

Log-likelihood:

\[
\log p(\mathcal{D} \mid \theta)
=
\sum_{i=1}^N
\log
\mathcal{N}
\left(
y_i \mid f_\theta(x_i), \sigma_i^2
\right)
\]

This formulation implicitly assumes equal credibility across data.

---

# 3. Introduction of Credibility Weights

From S01, define credibility:

\[
C_i \in [0,1]
\]

Weighted likelihood is defined as:

\[
p_w(\mathcal{D} \mid \theta)
=
\prod_{i=1}^N
p(y_i \mid \theta)^{C_i}
\]

Log form:

\[
\log p_w(\mathcal{D} \mid \theta)
=
\sum_{i=1}^N
C_i
\log p(y_i \mid \theta)
\]

This preserves probabilistic structure while soft-modulating influence.

---

# 4. Weighted Gaussian Likelihood

For Gaussian noise:

\[
\log p(y_i \mid \theta)
=
-
\frac{1}{2}
\left[
\frac{(y_i - f_\theta(x_i))^2}{\sigma_i^2}
+
\log(2\pi \sigma_i^2)
\right]
\]

Weighted version:

\[
\log p_w(\mathcal{D} \mid \theta)
=
-
\frac{1}{2}
\sum_{i=1}^N
C_i
\left[
\frac{(y_i - f_\theta(x_i))^2}{\sigma_i^2}
+
\log(2\pi \sigma_i^2)
\right]
\]

Interpretation:

- Residual penalty scaled by credibility.
- Low-trust data contribute less curvature to objective.

---

# 5. Effective Variance Interpretation

Weighted likelihood is equivalent to redefining variance:

\[
\tilde{\sigma}_i^2
=
\frac{\sigma_i^2}{C_i}
\]

Thus:

\[
\frac{(y_i - f_\theta(x_i))^2}{\tilde{\sigma}_i^2}
=
C_i
\frac{(y_i - f_\theta(x_i))^2}{\sigma_i^2}
\]

This provides intuitive interpretation:

Low credibility inflates effective uncertainty.

---

# 6. Bayesian Posterior Formulation

Posterior distribution:

\[
p(\theta \mid \mathcal{D})
\propto
p_w(\mathcal{D} \mid \theta)
p(\theta)
\]

Log posterior:

\[
\log p(\theta \mid \mathcal{D})
=
\sum_{i=1}^N
C_i
\log p(y_i \mid \theta)
+
\log p(\theta)
\]

This defines the weighted Bayesian inference engine.

---

# 7. Convexity and Stability

For linear models:

\[
f_\theta(x) = \theta^T x
\]

Weighted least squares objective:

\[
\mathcal{L}(\theta)
=
\sum_i
C_i
\frac{(y_i - \theta^T x_i)^2}{\sigma_i^2}
\]

Hessian:

\[
H =
X^T W X
\]

where:

\[
W = \mathrm{diag}\left(\frac{C_i}{\sigma_i^2}\right)
\]

If:

\[
C_i \ge 0
\]

then Hessian remains positive semi-definite.

Thus convexity is preserved.

---

# 8. Relation to Robust Statistics

Weighted likelihood generalizes:

- Huber loss,
- Tukey biweight,
- M-estimators.

If credibility is defined as:

\[
C_i = \psi(r_i)
\]

where \( r_i \) is residual, then model becomes robust estimator.

However, in Thermognosis:

Credibility is externally inferred, not residual-derived.

---

# 9. Hierarchical Weighted Likelihood

If credibility is uncertain:

\[
C_i \sim p(C_i \mid \mathcal{E}_i)
\]

Then marginal likelihood:

\[
p(\mathcal{D} \mid \theta)
=
\prod_i
\int
p(y_i \mid \theta)^{C_i}
p(C_i \mid \mathcal{E}_i)
dC_i
\]

This produces smoother trust integration.

---

# 10. Weighted Information Contribution

Fisher information:

\[
\mathcal{I}(\theta)
=
\sum_i
C_i
\mathcal{I}_i(\theta)
\]

Thus credibility scales information geometry.

High-credibility data sharpen posterior curvature.

---

# 11. Weighted Evidence Lower Bound (ELBO)

For variational inference:

\[
\mathcal{L}_{ELBO}
=
\sum_i
C_i
\mathbb{E}_{q(\theta)}
[\log p(y_i \mid \theta)]
-
\mathrm{KL}(q(\theta) \| p(\theta))
\]

Ensures credibility-aware variational training.

---

# 12. Interaction with Physical Constraints

If measurement violates constraint set \( \mathcal{C} \):

Credibility decreases:

\[
C_i \downarrow
\]

Thus weighted likelihood automatically reduces influence of non-physical data.

Physics indirectly regularizes inference.

---

# 13. Asymptotic Behavior

If:

\[
C_i \to 1
\quad \forall i
\]

Model reduces to classical Bayesian inference.

If:

\[
C_i \to 0
\]

Data point becomes asymptotically ignored.

Thus WLF generalizes classical likelihood without contradiction.

---

# 14. Numerical Stability

To avoid degeneracy:

Require:

\[
\sum_i C_i > 0
\]

If all credibility collapses:

Inference must halt with diagnostic alert.

Regularization floor:

\[
C_i \ge \epsilon
\]

with small \( \epsilon > 0 \).

---

# 15. Governance and Traceability

For each inference run, system must log:

- Credibility vector \( \mathbf{C} \),
- Effective variances \( \tilde{\sigma}_i^2 \),
- Total weighted information,
- Posterior shift relative to unweighted baseline.

This ensures transparency and auditability.

---

# 16. Strategic Interpretation

Weighted likelihood transforms:

- Data quality heterogeneity into structured inference,
- Trust into curvature modulation,
- Governance into statistical geometry.

It eliminates binary exclusion and replaces it with principled attenuation.

---

# 17. Compliance Requirement

All statistical modules must satisfy:

\[
\text{Module} \models \text{S02-WEIGHTED-LIKELIHOOD-FORMULATION}
\]

Violation results in:

- Inconsistent trust integration,
- Overfitting to unreliable data,
- Loss of probabilistic coherence.

---

# 18. Concluding Statement

The Weighted Likelihood Formulation is the mathematical core of credibility-aware inference in the Thermognosis Engine.

It ensures that:

- Uncertainty is propagated,
- Credibility is respected,
- Physics remains integrated,
- Bayesian coherence is preserved.

Scientific reliability emerges not from more data,  
but from correctly weighted evidence.
