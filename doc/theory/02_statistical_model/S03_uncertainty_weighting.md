# S03 — Uncertainty Weighting Framework  
**Document ID:** S03-UNCERTAINTY-WEIGHTING  
**Layer:** Statistical Modeling / Noise Geometry  
**Status:** Normative — Measurement-Aware Inference  
**Dependencies:**  
- T00-SYS-AXIOMS  
- T01-DATA-FORMALISM  
- T03-UNCERTAINTY-PROPAGATION  
- P02-ZT-ERROR-PROPAGATION  
- S02-WEIGHTED-LIKELIHOOD-FORMULATION  

---

# 1. Purpose

This document formalizes the **Uncertainty Weighting Framework (UWF)** governing how measurement uncertainty modulates statistical inference within the Thermognosis Engine.

Its objectives are:

1. To distinguish uncertainty weighting from credibility weighting.
2. To formalize heteroscedastic likelihood modeling.
3. To integrate propagated uncertainty into inference.
4. To define information-theoretic interpretation of uncertainty weights.
5. To ensure mathematically coherent noise-aware learning.

Uncertainty weighting is not optional.  
It is a structural requirement for probabilistic consistency.

---

# 2. Measurement Model

For each observation:

\[
y_i = f_\theta(x_i) + \epsilon_i
\]

where:

\[
\epsilon_i \sim \mathcal{N}(0, \sigma_i^2)
\]

Measurement uncertainty \( \sigma_i^2 \) may arise from:

- Instrument precision,
- Propagation from upstream variables,
- Derived quantity nonlinear transformation.

---

# 3. Heteroscedastic Likelihood

Unlike homoscedastic models:

\[
\sigma_i^2 \neq \sigma^2
\]

Likelihood:

\[
p(\mathcal{D} \mid \theta)
=
\prod_{i=1}^N
\mathcal{N}
\left(
y_i \mid f_\theta(x_i), \sigma_i^2
\right)
\]

Log form:

\[
\log p(\mathcal{D} \mid \theta)
=
-
\frac{1}{2}
\sum_i
\left[
\frac{(y_i - f_\theta(x_i))^2}{\sigma_i^2}
+
\log(2\pi \sigma_i^2)
\right]
\]

---

# 4. Information Geometry Interpretation

Fisher information contribution:

\[
\mathcal{I}_i(\theta)
=
\frac{1}{\sigma_i^2}
\left(
\frac{\partial f_\theta(x_i)}{\partial \theta}
\right)^T
\left(
\frac{\partial f_\theta(x_i)}{\partial \theta}
\right)
\]

Thus:

\[
\mathcal{I}(\theta)
=
\sum_i
\frac{1}{\sigma_i^2}
\mathcal{J}_i
\]

Low uncertainty implies high curvature and high information contribution.

---

# 5. Effective Weight Definition

Define uncertainty weight:

\[
w_i^{(u)} = \frac{1}{\sigma_i^2}
\]

Thus weighted residual:

\[
\mathcal{L}(\theta)
=
\sum_i
w_i^{(u)}
(y_i - f_\theta(x_i))^2
\]

This corresponds to weighted least squares under Gaussian assumption.

---

# 6. Propagated Uncertainty Integration

If \( y_i = g(\mathbf{x}_i) \), and upstream covariance is \( \Sigma_i \):

\[
\sigma_i^2
=
\nabla g(\boldsymbol{\mu}_i)^T
\Sigma_i
\nabla g(\boldsymbol{\mu}_i)
\]

Thus uncertainty weighting incorporates nonlinear propagation results.

No derived quantity may be treated as noiseless.

---

# 7. Log-Scale Interpretation for Multiplicative Quantities

For positive quantities (e.g., \( zT \)):

Define:

\[
z_i = \log y_i
\]

Variance transformation:

\[
\mathrm{Var}(z_i)
\approx
\left(
\frac{\sigma_i}{y_i}
\right)^2
\]

Weight in log-space:

\[
w_i^{(u)}
=
\frac{1}{(\sigma_i / y_i)^2}
\]

This prevents bias in multiplicative regimes.

---

# 8. Combined Credibility and Uncertainty Weight

Total weight:

\[
w_i =
\frac{C_i}{\sigma_i^2}
\]

Resulting log-likelihood:

\[
\log p(\mathcal{D} \mid \theta)
=
-
\frac{1}{2}
\sum_i
C_i
\left[
\frac{(y_i - f_\theta(x_i))^2}{\sigma_i^2}
+
\log(2\pi \sigma_i^2)
\right]
\]

This unifies S01 and S02.

---

# 9. Bayesian Hierarchical Noise Modeling

In some cases, \( \sigma_i^2 \) is uncertain.

Model:

\[
\sigma_i^2 \sim \mathrm{InverseGamma}(\alpha, \beta)
\]

Marginal likelihood becomes Student-t:

\[
y_i \sim \mathrm{StudentT}
\left(
\nu,
f_\theta(x_i),
s_i^2
\right)
\]

This increases robustness against underestimated uncertainties.

---

# 10. Regularization Floor

To avoid singularities:

\[
\sigma_i^2 \ge \sigma_{min}^2
\]

with:

\[
\sigma_{min}^2 > 0
\]

Prevents infinite weights.

---

# 11. Asymptotic Consistency

If measurement uncertainty is correctly specified:

\[
\hat{\theta}
\to
\theta^*
\quad
\text{as}
\quad
N \to \infty
\]

If uncertainty is underestimated:

Model becomes overconfident:

\[
\mathrm{Var}(\hat{\theta}) \downarrow \text{artificially}
\]

Thus correct uncertainty estimation is critical.

---

# 12. Uncertainty-Aware Optimization

Define objective:

\[
\mathcal{J}
=
\mathbb{E}[f_\theta(x)]
-
\lambda
\sqrt{\mathrm{Var}[f_\theta(x)]}
\]

Measurement uncertainty influences predictive variance via likelihood.

---

# 13. Stability Under Rescaling

If data is scaled:

\[
y'_i = a y_i
\]

Then:

\[
\sigma'_i = |a| \sigma_i
\]

Weight invariance:

\[
\frac{1}{\sigma'^2_i}
=
\frac{1}{a^2 \sigma_i^2}
\]

Scaling remains consistent with probabilistic structure.

---

# 14. Diagnostic Checks

System must verify:

1. \( \sigma_i^2 > 0 \)
2. No zero or negative variance
3. Distributional assumptions reasonable
4. Relative uncertainty within plausible range

If:

\[
\frac{\sigma_i}{|y_i|} > \tau
\]

Measurement may require credibility review.

---

# 15. Governance Requirements

For every dataset:

- Store reported uncertainty.
- Store propagated uncertainty.
- Log effective weights.
- Document transformation (linear or log-space).

Transparency is mandatory.

---

# 16. Strategic Interpretation

Uncertainty weighting:

- Encodes measurement precision into statistical geometry.
- Prevents noisy data from dominating inference.
- Converts physical measurement limits into Bayesian curvature.

It ensures that confidence emerges from precision, not volume.

---

# 17. Compliance Requirement

All inference modules must satisfy:

\[
\text{Module} \models \text{S03-UNCERTAINTY-WEIGHTING}
\]

Failure to respect uncertainty weighting results in:

- Overconfident predictions,
- Misleading optimization,
- Statistical incoherence.

---

# 18. Concluding Statement

The Uncertainty Weighting Framework establishes the probabilistic backbone of measurement-aware inference.

Scientific integrity requires:

- Explicit uncertainty,
- Correct weighting,
- Transparent propagation,
- Mathematically consistent modeling.

Precision is not an afterthought.  
It is the curvature of knowledge.
