# P04 — Wiedemann–Franz Law and Lorenz Number Constraints  
**Document ID:** P04-WIEDEMANN-FRANZ-LIMIT  
**Layer:** Physics / Electronic Transport Consistency  
**Status:** Normative — Transport Constraint Layer  
**Dependencies:**  
- T00-SYS-AXIOMS  
- T01-DATA-FORMALISM  
- T02-MEASUREMENT-SPACE  
- T03-UNCERTAINTY-PROPAGATION  
- P01-THERMOELECTRIC-EQUATIONS  
- P03-PHYSICAL-CONSTRAINTS  

---

# 1. Purpose

This document formalizes the Wiedemann–Franz law as a **transport consistency constraint** within the Thermognosis Engine.

Its objectives are:

1. To define the theoretical basis of the Wiedemann–Franz relation.
2. To formalize the Lorenz number.
3. To define admissible bounds for electronic thermal conductivity.
4. To integrate Lorenz constraints into validation and modeling.
5. To prevent unphysical decomposition of thermal conductivity.

This document is normative.  
All decomposition of thermal conductivity into electronic and lattice components must comply with this framework.

---

# 2. Statement of the Wiedemann–Franz Law

In metals and degenerate semiconductors:

\[
\kappa_e = L \sigma T
\]

where:

- \( \kappa_e \) : electronic thermal conductivity  
- \( \sigma \) : electrical conductivity  
- \( T \) : temperature  
- \( L \) : Lorenz number  

The Lorenz number is defined as:

\[
L = \frac{\kappa_e}{\sigma T}
\]

---

# 3. Sommerfeld Value

Under free-electron Fermi-liquid approximation:

\[
L_0 = \frac{\pi^2}{3}
\left(
\frac{k_B}{q}
\right)^2
\]

Numerically:

\[
L_0 \approx 2.44 \times 10^{-8}
\; \mathrm{W \Omega K^{-2}}
\]

This value applies in the limit of:

- Elastic scattering,
- Degenerate statistics,
- Parabolic band approximation.

---

# 4. Physical Interpretation

The Wiedemann–Franz law arises because:

- Both electrical and electronic thermal transport are carried by the same charge carriers.
- Heat and charge currents share identical scattering mechanisms.

Thus:

\[
\frac{\kappa_e}{\sigma}
\propto T
\]

Deviation implies modification of carrier scattering or non-degenerate behavior.

---

# 5. Decomposition of Thermal Conductivity

Total thermal conductivity:

\[
\kappa = \kappa_e + \kappa_l
\]

Using Wiedemann–Franz:

\[
\kappa_l = \kappa - L \sigma T
\]

Constraint:

\[
\kappa_l \ge 0
\]

If:

\[
\kappa - L \sigma T < 0
\]

then either:

- Measurement error,
- Incorrect Lorenz estimation,
- Model inconsistency

is present.

---

# 6. Lorenz Number in Semiconductors

For non-degenerate semiconductors:

\[
L \neq L_0
\]

Lorenz number becomes function of reduced Fermi energy:

\[
L = 
\left(
\frac{k_B}{q}
\right)^2
\left[
\frac{(r + 5/2) F_{r+3/2}(\eta)}
{(r + 3/2) F_{r+1/2}(\eta)}
-
\left(
\frac{(r + 3/2) F_{r+1/2}(\eta)}
{(r + 1/2) F_{r-1/2}(\eta)}
\right)^2
\right]
\]

where:

- \( F_j(\eta) \) are Fermi–Dirac integrals,
- \( r \) is scattering parameter,
- \( \eta \) is reduced Fermi level.

Thus Lorenz number must be treated as model-dependent.

---

# 7. Admissible Lorenz Bounds

For validation purposes:

\[
10^{-9}
<
L
<
10^{-7}
\quad
\mathrm{W \Omega K^{-2}}
\]

Values outside this interval must be flagged.

This bound accommodates:

- Metallic regime,
- Degenerate semiconductors,
- Moderate non-degenerate behavior.

---

# 8. Temperature Scaling Constraint

From Wiedemann–Franz:

\[
\frac{\partial \kappa_e}{\partial T}
=
L \sigma
+
T \sigma \frac{\partial L}{\partial T}
+
T L \frac{\partial \sigma}{\partial T}
\]

Models predicting inconsistent temperature scaling must be audited.

---

# 9. Impact on zT

Recall:

\[
zT = \frac{S^2 \sigma T}{\kappa_e + \kappa_l}
\]

Substitute:

\[
zT =
\frac{S^2 \sigma T}
{L \sigma T + \kappa_l}
\]

Simplify:

\[
zT =
\frac{S^2}
{L + \frac{\kappa_l}{\sigma T}}
\]

This expression shows:

- Lower Lorenz number increases potential zT.
- However \( L \) cannot be arbitrarily small without violating transport physics.

Thus Lorenz constraint directly limits achievable performance.

---

# 10. Constraint in Optimization

During AI optimization:

\[
\hat{\kappa}_e = \hat{L} \hat{\sigma} T
\]

Require:

\[
\hat{L} \in [L_{min}, L_{max}]
\]

If unconstrained models produce:

\[
\hat{L} \to 0
\]

then artificially inflated zT may result.

Penalty function:

\[
\mathcal{L}_{WF}
=
\lambda
\max(0, L_{min} - \hat{L})^2
+
\lambda
\max(0, \hat{L} - L_{max})^2
\]

---

# 11. Uncertainty Propagation of Lorenz Number

Given uncertainties in:

\[
\kappa_e, \sigma, T
\]

Lorenz variance:

\[
\sigma_L^2
=
\left(
\frac{\partial L}{\partial \kappa_e}
\right)^2 \sigma_{\kappa_e}^2
+
\left(
\frac{\partial L}{\partial \sigma}
\right)^2 \sigma_\sigma^2
+
\left(
\frac{\partial L}{\partial T}
\right)^2 \sigma_T^2
\]

with:

\[
\frac{\partial L}{\partial \kappa_e}
=
\frac{1}{\sigma T}
\]

\[
\frac{\partial L}{\partial \sigma}
=
- \frac{\kappa_e}{\sigma^2 T}
\]

\[
\frac{\partial L}{\partial T}
=
- \frac{\kappa_e}{\sigma T^2}
\]

Uncertainty bounds must be considered before declaring physical violation.

---

# 12. Detection of Anomalies

Anomalies include:

1. Negative \( \kappa_l \)
2. Lorenz outside admissible bounds
3. Inconsistent temperature scaling
4. Non-monotonic behavior incompatible with carrier statistics

Such cases must be flagged for:

- Experimental review,
- Data correction,
- Model recalibration.

---

# 13. Integration into Validation Pipeline

Validation algorithm:

1. Compute:

\[
L = \frac{\kappa_e}{\sigma T}
\]

2. Check bounds:

\[
L_{min} \le L \le L_{max}
\]

3. Compute lattice conductivity:

\[
\kappa_l = \kappa - L \sigma T
\]

4. Enforce:

\[
\kappa_l \ge 0
\]

Failure at any step triggers data flag.

---

# 14. Strategic Interpretation

The Wiedemann–Franz constraint serves as:

- A physical regularizer,
- A decomposition validator,
- A safeguard against artificial zT inflation.

It embeds microscopic transport physics into macroscopic optimization.

Without this constraint, the system may generate:

- Unrealistic electronic suppression,
- Artificial lattice conductivity collapse,
- Non-physical performance projections.

---

# 15. Compliance Requirement

All modules must satisfy:

\[
\text{Module} \models \text{P04-WIEDEMANN-FRANZ-LIMIT}
\]

Transport consistency is mandatory.

---

# 16. Concluding Statement

The Wiedemann–Franz law defines a fundamental coupling between heat and charge transport.

In the Thermognosis Engine:

- It is not optional,
- It is not heuristic,
- It is a structural constraint.

All thermoelectric modeling must remain anchored to transport physics,  
ensuring predictive power without sacrificing physical legitimacy.
