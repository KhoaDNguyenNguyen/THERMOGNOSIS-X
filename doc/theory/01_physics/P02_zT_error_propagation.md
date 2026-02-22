# P02 — Uncertainty Propagation for Thermoelectric Figure of Merit (zT)  
**Document ID:** P02-ZT-ERROR-PROPAGATION  
**Layer:** Physics / Statistical Consistency  
**Status:** Normative — Quantitative Integrity Requirement  
**Dependencies:**  
- T00-SYS-AXIOMS  
- T01-DATA-FORMALISM  
- T02-MEASUREMENT-SPACE  
- T03-UNCERTAINTY-PROPAGATION  
- P01-THERMOELECTRIC-EQUATIONS  

---

# 1. Purpose

This document establishes the rigorous mathematical framework for uncertainty propagation of the thermoelectric figure of merit:

\[
zT = \frac{S^2 \sigma T}{\kappa}
\]

The objectives are:

1. To derive analytical error propagation expressions.
2. To define correlated uncertainty handling.
3. To formalize second-order corrections.
4. To define Monte Carlo propagation standards.
5. To enforce physically consistent uncertainty reporting.

This document is normative.  
All computed or reported values of \( zT \) must include quantified uncertainty consistent with this framework.

---

# 2. Mathematical Definition

Let:

\[
\mathbf{X} = (S, \sigma, \kappa, T)^T
\]

be a random vector with mean:

\[
\boldsymbol{\mu} = (\mu_S, \mu_\sigma, \mu_\kappa, \mu_T)^T
\]

and covariance matrix:

\[
\Sigma = \mathrm{Cov}(\mathbf{X})
\]

Define:

\[
Z = f(\mathbf{X}) = \frac{S^2 \sigma T}{\kappa}
\]

Then \( Z \) is a nonlinear transformation of \( \mathbf{X} \).

---

# 3. First-Order (Linear) Error Propagation

Using first-order Taylor expansion:

\[
f(\mathbf{X}) \approx f(\boldsymbol{\mu}) + \nabla f(\boldsymbol{\mu})^T (\mathbf{X} - \boldsymbol{\mu})
\]

Variance approximation:

\[
\mathrm{Var}(Z)
\approx
\nabla f(\boldsymbol{\mu})^T
\Sigma
\nabla f(\boldsymbol{\mu})
\]

---

## 3.1 Gradient of zT

Partial derivatives:

\[
\frac{\partial zT}{\partial S}
=
\frac{2 S \sigma T}{\kappa}
\]

\[
\frac{\partial zT}{\partial \sigma}
=
\frac{S^2 T}{\kappa}
\]

\[
\frac{\partial zT}{\partial \kappa}
=
- \frac{S^2 \sigma T}{\kappa^2}
\]

\[
\frac{\partial zT}{\partial T}
=
\frac{S^2 \sigma}{\kappa}
\]

Gradient vector:

\[
\nabla zT =
\begin{pmatrix}
\frac{2 S \sigma T}{\kappa} \\
\frac{S^2 T}{\kappa} \\
- \frac{S^2 \sigma T}{\kappa^2} \\
\frac{S^2 \sigma}{\kappa}
\end{pmatrix}
\]

---

# 4. Independent Variable Approximation

If variables are assumed independent:

\[
\mathrm{Var}(zT)
\approx
\sum_i
\left(
\frac{\partial zT}{\partial x_i}
\right)^2
\sigma_{x_i}^2
\]

Explicitly:

\[
\sigma_{zT}^2
=
\left(
\frac{2 S \sigma T}{\kappa}
\right)^2 \sigma_S^2
+
\left(
\frac{S^2 T}{\kappa}
\right)^2 \sigma_\sigma^2
+
\left(
\frac{S^2 \sigma T}{\kappa^2}
\right)^2 \sigma_\kappa^2
+
\left(
\frac{S^2 \sigma}{\kappa}
\right)^2 \sigma_T^2
\]

This approximation is valid only when covariance terms are negligible.

---

# 5. Correlated Uncertainty

Full covariance propagation:

\[
\sigma_{zT}^2
=
\sum_i \sum_j
\frac{\partial zT}{\partial x_i}
\frac{\partial zT}{\partial x_j}
\mathrm{Cov}(x_i, x_j)
\]

Neglecting covariance when present introduces systematic bias in uncertainty estimation.

Correlations may arise from:

- Shared instrumentation,
- Derived quantities,
- Post-processing transformations.

Covariance matrix must be stored when available.

---

# 6. Relative Error Representation

Define relative uncertainties:

\[
\delta_S = \frac{\sigma_S}{S}
\quad
\delta_\sigma = \frac{\sigma_\sigma}{\sigma}
\quad
\delta_\kappa = \frac{\sigma_\kappa}{\kappa}
\quad
\delta_T = \frac{\sigma_T}{T}
\]

Under independence and small-error assumption:

\[
\frac{\sigma_{zT}}{zT}
\approx
\sqrt{
(2 \delta_S)^2
+
\delta_\sigma^2
+
\delta_\kappa^2
+
\delta_T^2
}
\]

This highlights sensitivity amplification:

- Seebeck uncertainty contributes quadratically (factor 2).
- Thermal conductivity uncertainty directly penalizes precision.

---

# 7. Second-Order Correction

For larger uncertainties:

\[
\mathbb{E}[Z]
\approx
f(\boldsymbol{\mu})
+
\frac{1}{2}
\mathrm{Tr}
\left(
H_f(\boldsymbol{\mu}) \Sigma
\right)
\]

where:

\[
H_f
=
\nabla^2 f
\]

Second-order corrections are mandatory when:

\[
\max(\delta_i) > 0.1
\]

---

# 8. Monte Carlo Propagation Standard

When:

- Non-Gaussian distributions exist,
- Strong correlations exist,
- Nonlinear effects dominate,

Monte Carlo propagation must be used.

Procedure:

1. Sample:

\[
\mathbf{X}^{(k)} \sim p(\mathbf{x})
\]

2. Compute:

\[
Z^{(k)} = \frac{S^{(k)2} \sigma^{(k)} T^{(k)}}{\kappa^{(k)}}
\]

3. Estimate:

\[
\bar{Z} = \frac{1}{N} \sum_k Z^{(k)}
\]

\[
\sigma_{zT}^2 =
\frac{1}{N-1}
\sum_k (Z^{(k)} - \bar{Z})^2
\]

Monte Carlo sample size must satisfy convergence diagnostics.

---

# 9. Sensitivity Ranking

Define sensitivity index:

\[
I_i =
\left|
\frac{\partial \ln zT}{\partial \ln x_i}
\right|
\]

Explicitly:

\[
I_S = 2
\quad
I_\sigma = 1
\quad
I_\kappa = 1
\quad
I_T = 1
\]

Thus:

Seebeck uncertainty dominates relative error propagation.

This insight guides:

- Instrument precision allocation,
- Experimental design,
- Bayesian acquisition strategy.

---

# 10. Uncertainty-Weighted Optimization

Define effective objective:

\[
\mathcal{J}
=
\mathbb{E}[zT]
-
\lambda \sigma_{zT}
\]

where \( \lambda > 0 \) controls risk aversion.

This ensures optimization favors both:

- High expected performance,
- Low uncertainty.

---

# 11. Physical Consistency Constraints

Uncertainty propagation must preserve:

\[
\sigma_{zT} \ge 0
\]

If propagated variance yields:

\[
\sigma_{zT}^2 < 0
\]

then numerical instability or covariance mis-specification exists.

Such cases must trigger automatic validation failure.

---

# 12. Reporting Standard

Every reported zT must be expressed as:

\[
zT = \hat{zT} \pm \sigma_{zT}
\]

or

\[
zT = \hat{zT}
\pm
\Delta zT_{95\%}
\]

Confidence intervals must specify:

- Propagation method,
- Independence assumption,
- Sample size (if Monte Carlo).

---

# 13. Architectural Enforcement

The Thermognosis Engine must enforce:

1. No zT without propagated uncertainty.
2. No uncertainty computed without covariance awareness.
3. Automatic flagging when relative error exceeds threshold.
4. Storage of propagation method metadata.
5. Separation of measurement uncertainty and model uncertainty.

---

# 14. Strategic Interpretation

zT is a nonlinear amplification of measurement noise.

Ignoring uncertainty propagation:

- Inflates performance claims,
- Distorts ranking,
- Corrupts optimization.

Rigorous propagation transforms performance reporting from optimistic estimation to statistically defensible inference.

---

# 15. Concluding Statement

This document defines the formal uncertainty propagation framework for thermoelectric figure of merit evaluation.

All modules computing or consuming \( zT \) must satisfy:

\[
\text{Module} \models \text{P02-ZT-ERROR-PROPAGATION}
\]

Scientific credibility of the Thermognosis Engine depends not only on predictive power,  
but on mathematically rigorous uncertainty quantification.
