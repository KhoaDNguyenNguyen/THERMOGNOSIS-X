# PHYS — Error Propagation Specification  
**Document ID:** SPEC-PHYS-ERROR-PROPAGATION  
**Layer:** spec/05_physics  
**Status:** Normative — Uncertainty Quantification and Propagation Framework  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Physical Error Propagation Specification (PEPS)** governing the formal treatment, propagation, and validation of measurement and model uncertainties within the Thermognosis Engine.

Scientific computation without uncertainty quantification is incomplete.  
Every physical quantity must carry both magnitude and epistemic uncertainty.

This specification ensures:

- Consistent mathematical propagation of uncertainty.
- Dimensional coherence of error terms.
- Deterministic and reproducible uncertainty evaluation.
- Compatibility with statistical and physical constraint layers.

---

# 2. Representation of Uncertain Quantity

A physical quantity with uncertainty is defined as:

\[
Q = (v, U, \sigma, \mathcal{M})
\]

where:

- \( v \in \mathbb{R} \) — central estimate  
- \( \sigma \ge 0 \) — standard uncertainty  
- \( U \in \mathcal{R}_U \) — unit  
- \( \mathcal{M} \) — metadata  

Dimensional constraint:

\[
\mathbf{d}(v) = \mathbf{d}(\sigma)
\]

---

# 3. First-Order (Linear) Error Propagation

Let:

\[
z = f(x_1, x_2, \dots, x_n)
\]

Assuming independent variables, first-order Taylor expansion yields:

\[
\sigma_z^2 =
\sum_{i=1}^n
\left(
\frac{\partial f}{\partial x_i}
\sigma_i
\right)^2
\]

Vector form:

\[
\sigma_z^2 =
\nabla f^\top
\Sigma
\nabla f
\]

where:

- \( \nabla f \) — gradient vector  
- \( \Sigma \) — covariance matrix  

---

# 4. Covariance-Aware Propagation

For correlated variables:

\[
\sigma_z^2 =
\sum_{i,j}
\frac{\partial f}{\partial x_i}
\frac{\partial f}{\partial x_j}
\mathrm{Cov}(x_i, x_j)
\]

Matrix representation:

\[
\sigma_z^2 =
J_f
\Sigma
J_f^\top
\]

where:

\[
J_f = \frac{\partial f}{\partial \mathbf{x}}
\]

Covariance tracking is mandatory when data source provides correlation.

---

# 5. Multiplicative and Power Rules

For product:

\[
z = xy
\]

Relative uncertainty:

\[
\left( \frac{\sigma_z}{z} \right)^2
=
\left( \frac{\sigma_x}{x} \right)^2
+
\left( \frac{\sigma_y}{y} \right)^2
\]

For power:

\[
z = x^n
\]

\[
\frac{\sigma_z}{z}
=
|n| \frac{\sigma_x}{x}
\]

---

# 6. Logarithmic and Exponential Functions

For:

\[
z = \ln x
\]

\[
\sigma_z =
\frac{\sigma_x}{x}
\]

For:

\[
z = e^x
\]

\[
\sigma_z =
e^x \sigma_x
\]

Domain constraint:

\[
x > 0 \quad \text{for logarithm}
\]

---

# 7. Ratio Example — Thermoelectric ZT

Given:

\[
ZT =
\frac{S^2 \sigma T}{\kappa}
\]

Relative uncertainty:

\[
\left( \frac{\sigma_{ZT}}{ZT} \right)^2
=
4 \left( \frac{\sigma_S}{S} \right)^2
+
\left( \frac{\sigma_\sigma}{\sigma} \right)^2
+
\left( \frac{\sigma_T}{T} \right)^2
+
\left( \frac{\sigma_\kappa}{\kappa} \right)^2
\]

Assuming independence.

---

# 8. Second-Order Error Propagation

For higher precision:

\[
z \approx
f(\mathbf{x})
+
\nabla f^\top \delta \mathbf{x}
+
\frac{1}{2}
\delta \mathbf{x}^\top
H_f
\delta \mathbf{x}
\]

Variance correction:

\[
\sigma_z^2
=
\nabla f^\top \Sigma \nabla f
+
\frac{1}{2}
\mathrm{Tr}(H_f \Sigma H_f \Sigma)
\]

where:

- \( H_f \) — Hessian matrix  

Second-order propagation required when:

\[
\frac{\sigma_x}{x} > 0.1
\]

---

# 9. Monte Carlo Propagation

When analytical propagation is intractable:

1. Sample:
   \[
   x_i^{(k)} \sim \mathcal{N}(v_i, \sigma_i^2)
   \]
2. Compute:
   \[
   z^{(k)} = f(x_1^{(k)}, \dots)
   \]
3. Estimate:
   \[
   \sigma_z^2 = \mathrm{Var}(z^{(k)})
   \]

Monte Carlo required for highly nonlinear transformations.

---

# 10. Dimensional Integrity of Uncertainty

Uncertainty must preserve dimension:

\[
\mathbf{d}(\sigma_z) = \mathbf{d}(z)
\]

Propagation must not alter dimensional class.

---

# 11. Conversion-Aware Propagation

Under unit scaling:

\[
z' = k z
\]

\[
\sigma_{z'} = |k| \sigma_z
\]

Affine offset does not affect uncertainty magnitude.

---

# 12. Aggregation of Measurements

For mean:

\[
\bar{x} =
\frac{1}{n}
\sum_{i=1}^n x_i
\]

Uncertainty:

\[
\sigma_{\bar{x}} =
\frac{\sigma_x}{\sqrt{n}}
\]

If independent.

For weighted mean:

\[
\bar{x} =
\frac{\sum w_i x_i}{\sum w_i}
\]

\[
\sigma_{\bar{x}}^2 =
\frac{1}{\sum w_i}
\]

with:

\[
w_i = \frac{1}{\sigma_i^2}
\]

---

# 13. Constraint-Aware Uncertainty

For inequality constraint:

\[
C(\mathcal{S}) \ge 0
\]

Evaluate probability:

\[
\mathbb{P}(C(\mathcal{S}) \ge 0)
\]

State accepted if:

\[
\mathbb{P} \ge 1 - \alpha
\]

Default:

\[
\alpha = 0.05
\]

---

# 14. Surrogate Model Uncertainty

For predictive model:

\[
\hat{y} = f_\theta(x)
\]

Total predictive variance:

\[
\sigma_{\text{total}}^2
=
\sigma_{\text{model}}^2
+
\sigma_{\text{measurement}}^2
\]

Bayesian models must propagate posterior variance.

---

# 15. Error Classification

Uncertainty errors classified as:

- EPROP-01: Missing uncertainty
- EPROP-02: Dimensional mismatch
- EPROP-03: Ignored covariance
- EPROP-04: Nonlinear underestimation
- EPROP-05: Inconsistent unit scaling

Each must log computational context.

---

# 16. Determinism Requirement

Given identical:

- Input values,
- Covariance matrix,
- Propagation method,
- Random seed (if Monte Carlo),

Uncertainty output must be reproducible.

---

# 17. Numerical Stability

Propagation must avoid catastrophic cancellation.

If condition number of \( \Sigma \) exceeds threshold:

\[
\kappa(\Sigma) > 10^{12}
\]

system must raise numerical stability warning.

---

# 18. Computational Complexity

First-order propagation:

\[
\mathcal{O}(n)
\]

Covariance-aware:

\[
\mathcal{O}(n^2)
\]

Monte Carlo:

\[
\mathcal{O}(nN)
\]

where \( N \) is sample count.

---

# 19. Formal Soundness Condition

Uncertainty propagation is sound if:

1. Dimensional consistency holds.
2. Variance remains non-negative.
3. Probability distributions remain normalized.
4. Numerical stability preserved.

---

# 20. Strategic Interpretation

The Error Propagation Specification ensures:

- Scientific claims include quantified confidence.
- Model selection considers uncertainty, not only mean.
- Closed-loop exploration accounts for epistemic risk.
- Publication-level metrics include defensible error bars.

Ignoring uncertainty inflates false precision.

---

# 21. Concluding Statement

All physical computations within the Thermognosis Engine must satisfy:

\[
Q \models \text{SPEC-PHYS-ERROR-PROPAGATION}
\]

Every reported quantity must carry a defensible uncertainty estimate.

Precision without quantified uncertainty is not scientific rigor.
