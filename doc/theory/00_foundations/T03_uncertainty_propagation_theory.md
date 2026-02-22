# T03 — Uncertainty Propagation Theory  
**Document ID:** T03-UNCERTAINTY-PROPAGATION  
**Layer:** Theory / Foundations  
**Status:** Normative — Foundational  
**Dependencies:** T00-SYS-AXIOMS, T01-DATA-FORMALISM, T02-MEASUREMENT-SPACE  

---

# 1. Purpose

This document establishes the formal theory of uncertainty propagation within the Thermognosis Engine.

Its objectives are:

1. To define uncertainty as a first-class mathematical object.
2. To formalize analytical and numerical propagation of measurement uncertainty.
3. To define weighted statistical inference under credibility constraints.
4. To provide a principled foundation for uncertainty-aware optimization.
5. To ensure long-term scientific integrity and reproducibility.

Uncertainty is not optional metadata.  
It is structurally embedded in all scientific reasoning within this system.

---

# 2. Measurement as Random Variables

Let a thermoelectric measurement be defined as:

\[
\mathbf{X} = (S, \sigma, \kappa, T)
\]

Each component is modeled as a random variable:

\[
X_i \sim \mathcal{D}_i(\mu_i, \theta_i)
\]

Commonly:

\[
X_i \sim \mathcal{N}(\mu_i, \sigma_i^2)
\]

Thus the joint distribution is:

\[
\mathbf{X} \sim p(\mathbf{x})
\]

with covariance matrix:

\[
\Sigma =
\begin{pmatrix}
\sigma_S^2 & \cdots \\
\vdots & \ddots
\end{pmatrix}
\]

---

# 3. Derived Quantity: Figure of Merit

The thermoelectric figure of merit is:

\[
zT = f(S, \sigma, \kappa, T)
= \frac{S^2 \sigma T}{\kappa}
\]

Since \( zT \) is a nonlinear function of random variables, it is itself a random variable:

\[
Z = f(\mathbf{X})
\]

---

# 4. First-Order (Linear) Error Propagation

Using first-order Taylor expansion around mean \( \boldsymbol{\mu} \):

\[
f(\mathbf{X}) \approx f(\boldsymbol{\mu})
+ \nabla f(\boldsymbol{\mu})^T (\mathbf{X} - \boldsymbol{\mu})
\]

Variance approximation:

\[
\mathrm{Var}(Z)
\approx
\nabla f(\boldsymbol{\mu})^T
\Sigma
\nabla f(\boldsymbol{\mu})
\]

For independent variables:

\[
\mathrm{Var}(Z)
\approx
\sum_i
\left(
\frac{\partial f}{\partial x_i}
\right)^2
\sigma_i^2
\]

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

---

# 5. Second-Order Propagation

When nonlinearity is significant, include second-order terms:

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
\]

is the Hessian matrix of \( f \).

This improves accuracy for large relative uncertainties.

---

# 6. Monte Carlo Propagation

For general distributions:

1. Sample:

\[
\mathbf{X}^{(k)} \sim p(\mathbf{x})
\]

2. Compute:

\[
Z^{(k)} = f(\mathbf{X}^{(k)})
\]

3. Estimate:

\[
\mathbb{E}[Z]
=
\frac{1}{N}
\sum_{k=1}^N Z^{(k)}
\]

\[
\mathrm{Var}(Z)
=
\frac{1}{N-1}
\sum_{k=1}^N
(Z^{(k)} - \bar{Z})^2
\]

Monte Carlo propagation is mandatory when:

- Correlations exist,
- Distributions are non-Gaussian,
- Nonlinearities dominate.

---

# 7. Correlated Uncertainty

Let covariance matrix:

\[
\Sigma =
\begin{pmatrix}
\sigma_S^2 & \rho_{S\sigma}\sigma_S\sigma_\sigma & \cdots \\
\cdots & \sigma_\sigma^2 & \cdots \\
\cdots & \cdots & \ddots
\end{pmatrix}
\]

Full propagation:

\[
\mathrm{Var}(Z)
=
\sum_i \sum_j
\frac{\partial f}{\partial x_i}
\frac{\partial f}{\partial x_j}
\mathrm{Cov}(x_i, x_j)
\]

Neglecting covariance when present leads to systematic bias.

---

# 8. Credibility-Weighted Uncertainty

Each measurement is assigned weight:

\[
w_i = P(\text{valid}_i \mid \phi_i)
\]

Effective variance scaling:

\[
\tilde{\sigma}_i^2
=
\frac{\sigma_i^2}{w_i}
\]

This penalizes low-credibility measurements by inflating uncertainty.

---

# 9. Propagation into Bayesian Modeling

Given dataset:

\[
\mathcal{D} = \{ (\mathbf{x}_i, y_i, \Sigma_i, w_i) \}
\]

Likelihood:

\[
p(y_i \mid \mathbf{x}_i, \theta)
=
\mathcal{N}
\left(
\mu_\theta(\mathbf{x}_i),
\Sigma_i
\right)
\]

Weighted posterior:

\[
p(\theta \mid \mathcal{D})
\propto
\prod_i
p(y_i \mid \mathbf{x}_i, \theta)^{w_i}
p(\theta)
\]

Thus uncertainty directly influences parameter posterior.

---

# 10. Predictive Uncertainty

For predictive model \( f_\theta \):

Total predictive variance:

\[
\mathrm{Var}(y_*)
=
\mathrm{Var}_{model}(y_*)
+
\mathrm{Var}_{data}(y_*)
\]

where:

- \( \mathrm{Var}_{model} \) = epistemic uncertainty,
- \( \mathrm{Var}_{data} \) = propagated measurement uncertainty.

---

# 11. Information-Theoretic Interpretation

Entropy of posterior:

\[
H(\theta)
=
- \int p(\theta \mid \mathcal{D})
\log p(\theta \mid \mathcal{D})
d\theta
\]

Expected reduction from new measurement:

\[
IG(m)
=
H(\theta)
-
\mathbb{E}
\left[
H(\theta \mid m)
\right]
\]

Uncertainty propagation is therefore foundational to active acquisition.

---

# 12. Asymptotic Convergence

Under bounded noise and consistent estimators:

\[
\mathrm{Var}(\hat{\theta})
\rightarrow 0
\quad \text{as} \quad
|\mathcal{D}| \rightarrow \infty
\]

However, if systematic bias exists:

\[
\lim_{|\mathcal{D}| \to \infty}
\mathbb{E}[\hat{\theta}]
\neq \theta^*
\]

Thus uncertainty modeling must include bias detection mechanisms.

---

# 13. Architectural Implications

The system must enforce:

1. No scalar storage without uncertainty representation.
2. Mandatory propagation for derived quantities.
3. Explicit covariance handling when available.
4. Credibility-weighted variance scaling.
5. Separation between epistemic and aleatoric uncertainty.

---

# 14. Philosophical Interpretation

Uncertainty is not weakness.  
It is the quantification of epistemic limitation.

The Thermognosis Engine is defined as:

\[
\text{A probabilistically coherent, uncertainty-aware scientific intelligence system.}
\]

Neglecting uncertainty transforms science into numerology.  
Proper propagation transforms data into knowledge.

---

# 15. Concluding Statement

This document establishes the mathematical foundation for uncertainty propagation across:

- Measurement validation,
- Data storage,
- Statistical modeling,
- Optimization,
- Active learning.

All future implementations must satisfy:

\[
\text{Module} \models \text{T03-UNCERTAINTY-PROPAGATION}
\]

Failure to propagate uncertainty consistently constitutes structural non-compliance with the system's scientific integrity.

The Thermognosis Engine is not merely data-driven.  
It is uncertainty-driven.
