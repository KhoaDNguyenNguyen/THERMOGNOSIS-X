# Error Hierarchy Specification  
**Document ID:** SPEC-GOV-ERROR-HIERARCHY  
**Layer:** spec/00_governance  
**Status:** Normative — Uncertainty and Error Governance Framework  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Error Hierarchy Framework (EHF)** governing uncertainty modeling, propagation, aggregation, and decision integration within the Thermognosis Engine.

Its objectives are:

1. To formally classify all sources of uncertainty.
2. To prevent conflation of statistical and physical errors.
3. To enforce mathematically consistent propagation.
4. To ensure decision-making respects uncertainty structure.
5. To maintain publication-level rigor.

Error is not noise to suppress; it is structure to model.

---

# 2. Formal Definition of Total Error

Let observable:

\[
y = f(x; \theta)
\]

Total uncertainty:

\[
\Sigma_{\text{total}}
=
\Sigma_{\text{meas}}
+
\Sigma_{\text{model}}
+
\Sigma_{\text{struct}}
+
\Sigma_{\text{num}}
+
\Sigma_{\text{decision}}
\]

Each term corresponds to a distinct epistemic layer.

---

# 3. Error Taxonomy

## 3.1 Measurement Error

\[
\Sigma_{\text{meas}}
\]

Origin:

- Instrument precision
- Calibration drift
- Environmental fluctuation

Modeled as:

\[
\epsilon_{\text{meas}} \sim \mathcal{N}(0, \sigma_{\text{meas}}^2)
\]

May be heteroscedastic.

---

## 3.2 Model (Parameter) Uncertainty

\[
\Sigma_{\text{model}}
=
\text{Var}[\theta | \mathcal{D}]
\]

Bayesian posterior variance.

Reflects finite data and parameter identifiability.

---

## 3.3 Structural Uncertainty

\[
\Sigma_{\text{struct}}
\]

Arises from:

- Model class mismatch
- Missing physical mechanisms
- Incorrect functional form

Cannot be reduced purely by more data.

---

## 3.4 Numerical Error

\[
\Sigma_{\text{num}}
\]

Sources:

- Floating-point precision
- Matrix inversion instability
- Approximation truncation

For matrix inversion:

\[
K^{-1} \approx (K + \epsilon I)^{-1}
\]

Numerical stabilization must log \( \epsilon \).

---

## 3.5 Decision-Induced Uncertainty

\[
\Sigma_{\text{decision}}
\]

Emerges in acquisition policies and selection bias:

\[
\mathcal{D}_{t+1} \sim \pi(x | \mathcal{D}_t)
\]

Sequential decisions alter posterior geometry.

---

# 4. Hierarchical Structure

Define ordered hierarchy:

\[
E_0 \prec E_1 \prec E_2 \prec E_3 \prec E_4
\]

where:

- \( E_0 \): Measurement
- \( E_1 \): Parameter
- \( E_2 \): Structural
- \( E_3 \): Numerical
- \( E_4 \): Decision

Propagation must respect ordering.

---

# 5. Propagation Law

Given function:

\[
z = g(y_1, y_2, \dots, y_n)
\]

First-order propagation:

\[
\Sigma_z
=
J \Sigma_y J^T
\]

where:

\[
J = \frac{\partial g}{\partial y}
\]

Second-order correction (if required):

\[
\Sigma_z^{(2)}
=
\frac{1}{2}
\text{Tr}
\left(
H \Sigma_y H \Sigma_y
\right)
\]

where \( H \) is Hessian.

---

# 6. zT Example

For:

\[
zT = \frac{S^2 \sigma T}{\kappa}
\]

Uncertainty:

\[
\frac{\sigma_{zT}}{zT}
=
\sqrt{
4 \left(\frac{\sigma_S}{S}\right)^2
+
\left(\frac{\sigma_\sigma}{\sigma}\right)^2
+
\left(\frac{\sigma_\kappa}{\kappa}\right)^2
}
\]

Assumes independence unless covariance provided.

---

# 7. Covariance Handling

Full covariance matrix:

\[
\Sigma
=
\begin{bmatrix}
\sigma_1^2 & \rho_{12}\sigma_1\sigma_2 & \dots \\
\vdots & \ddots & \vdots \\
\dots & \dots & \sigma_n^2
\end{bmatrix}
\]

Correlation must not be ignored if known.

---

# 8. Bayesian Posterior Variance

For predictive distribution:

\[
p(y_* | x_*, \mathcal{D})
=
\mathcal{N}(\mu_*, \sigma_*^2)
\]

where:

\[
\sigma_*^2
=
k(x_*, x_*)
-
k_*^T K^{-1} k_*
\]

Posterior variance contributes to \( \Sigma_{\text{model}} \).

---

# 9. Structural Error Estimation

If multiple models \( \mathcal{M}_k \):

\[
\Sigma_{\text{struct}}
=
\text{Var}_{\mathcal{M}}[\mu_{\mathcal{M}}(x)]
\]

Model ensemble dispersion estimates structural uncertainty.

---

# 10. Numerical Stability Threshold

Define tolerance:

\[
\epsilon_{\text{num}} < 10^{-10}
\]

If:

\[
\kappa(K) > 10^{12}
\]

System must regularize.

---

# 11. Error Aggregation Principle

Errors must not be aggregated blindly.

Required:

\[
\Sigma_{\text{total}}
=
\sum_i \Sigma_i
+
\sum_{i \neq j} \text{Cov}(E_i, E_j)
\]

Cross-layer covariance must be evaluated.

---

# 12. Error Weighting in Likelihood

Weighted likelihood:

\[
\mathcal{L}
=
\prod_i
\mathcal{N}(y_i | \mu_i, \sigma_{i,\text{total}}^2)
\]

where:

\[
\sigma_{i,\text{total}}^2
=
\sigma_{\text{meas}}^2
+
\sigma_{\text{model}}^2
\]

---

# 13. Decision Sensitivity to Error

Acquisition must incorporate variance:

\[
\text{IG}(x)
=
H[p(\theta|\mathcal{D})]
-
\mathbb{E}_{y}
H[p(\theta|\mathcal{D} \cup y)]
\]

Entropy depends on uncertainty decomposition.

---

# 14. Error Logging Requirement

Every module must record:

- Measurement variance
- Posterior variance
- Numerical stabilization value
- Structural ensemble variance

No uncertainty term may be discarded silently.

---

# 15. Visualization Standard

Plots must distinguish:

1. Measurement error bars
2. Model credible intervals
3. Structural spread
4. Numerical warnings

Error layering must be visually explicit.

---

# 16. Sensitivity Analysis

Define sensitivity metric:

\[
S_i
=
\left|
\frac{\partial z}{\partial y_i}
\right|
\frac{\sigma_{y_i}}{\sigma_z}
\]

Large \( S_i \) identifies dominant uncertainty contributors.

---

# 17. Convergence and Error Reduction

Closed-loop must reduce:

\[
\Sigma_{\text{model}}^{(t+1)}
<
\Sigma_{\text{model}}^{(t)}
\]

but may not reduce structural error.

---

# 18. Error Hierarchy Invariant

Invariant condition:

\[
\Sigma_{\text{total}} \ge \Sigma_{\text{meas}}
\]

Total uncertainty cannot be less than measurement uncertainty.

---

# 19. Governance Audit

Audit must verify:

- All error sources identified.
- Proper propagation method used.
- Numerical stabilization logged.
- No silent truncation.
- Covariance considered when available.

---

# 20. Failure Modes

Violations may lead to:

- Overconfident predictions.
- Artificial zT inflation.
- Unstable acquisition policies.
- Non-reproducible results.
- Publication rejection.

---

# 21. Compliance Requirement

System must satisfy:

\[
\text{Module} \models \text{SPEC-GOV-ERROR-HIERARCHY}
\]

All inference must expose uncertainty structure.

---

# 22. Strategic Interpretation

Error is not a nuisance variable.

It is:

- A structural descriptor of knowledge,
- A driver of acquisition,
- A determinant of credibility.

A system that ignores hierarchy collapses epistemically.

---

# 23. Concluding Statement

The Thermognosis Engine treats uncertainty as a layered mathematical object.

Only by respecting error hierarchy can the system:

- Maintain physical credibility,
- Preserve statistical rigor,
- Support stable closed-loop discovery,
- Achieve Q1 publication standard.

In this system,  
confidence must always be earned mathematically.
