# P03 — Physical Constraints and Admissible State Space  
**Document ID:** P03-PHYSICAL-CONSTRAINTS  
**Layer:** Physics / Constraint Formalization  
**Status:** Normative — Hard Constraint Layer  
**Dependencies:**  
- T00-SYS-AXIOMS  
- T01-DATA-FORMALISM  
- T02-MEASUREMENT-SPACE  
- T03-UNCERTAINTY-PROPAGATION  
- P01-THERMOELECTRIC-EQUATIONS  
- P02-ZT-ERROR-PROPAGATION  

---

# 1. Purpose

This document formalizes the **physical constraints** governing all admissible thermoelectric states within the Thermognosis Engine.

Its objectives are:

1. To define the physically admissible region of the measurement space.
2. To enforce thermodynamic consistency.
3. To formalize transport-law constraints.
4. To prevent unphysical model predictions.
5. To provide a hard constraint layer for optimization and AI-driven modeling.

This document is normative and non-negotiable.  
All data, predictions, and optimizations must satisfy these constraints.

---

# 2. Admissible State Space

Let the thermoelectric state vector be:

\[
\mathbf{x} = (S, \sigma, \kappa, T)
\]

Define the admissible region:

\[
\mathcal{R}_{phys}
=
\left\{
\mathbf{x} \in \mathbb{R}^4
\;\middle|\;
\sigma > 0,\;
\kappa > 0,\;
T > 0
\right\}
\]

Additionally:

\[
zT = \frac{S^2 \sigma T}{\kappa} \ge 0
\]

Any state outside \( \mathcal{R}_{phys} \) is invalid.

---

# 3. Positivity Constraints

The following must hold:

\[
\sigma > 0
\]

\[
\kappa > 0
\]

\[
T > 0
\]

These arise from:

- Second law of thermodynamics,
- Positive entropy production,
- Physical definition of temperature.

Negative predictions must trigger automatic rejection.

---

# 4. Entropy Production Constraint

Under linear response:

\[
\dot{S}_{entropy}
=
\mathbf{J} \cdot (-\nabla V)
+
\frac{\mathbf{Q} \cdot (-\nabla T)}{T}
\]

Thermodynamic stability requires:

\[
\dot{S}_{entropy} \ge 0
\]

This implies:

\[
\sigma > 0
\quad \text{and} \quad
\kappa > 0
\]

Violation implies non-physical transport behavior.

---

# 5. Kelvin Reciprocity Constraint

From P01:

\[
\Pi = S T
\]

If experimental data reports \( \Pi \neq S T \), then:

- Measurement error,
- Unit inconsistency,
- Post-processing corruption

must be investigated.

This relation is mandatory.

---

# 6. Wiedemann–Franz Consistency

Electronic thermal conductivity:

\[
\kappa_e = L \sigma T
\]

Lorenz number:

\[
L = \frac{\kappa_e}{\sigma T}
\]

Physical constraint:

\[
10^{-9} < L < 10^{-7}
\]

Extreme deviations must be flagged for review.

---

# 7. Realistic Magnitude Constraints

Empirical physical ranges:

\[
|S| < 1000 \, \mu \mathrm{V/K}
\]

\[
0 < \sigma < 10^7 \, \mathrm{S/m}
\]

\[
0 < \kappa < 100 \, \mathrm{W/mK}
\]

\[
100 \, \mathrm{K} < T < 2000 \, \mathrm{K}
\]

Values outside plausible ranges require anomaly labeling.

These bounds are not rigid physics limits but validation filters.

---

# 8. Coupled Trade-Off Constraint

Carrier concentration \( n \) influences:

\[
S \sim n^{-2/3}
\]

\[
\sigma \sim n
\]

Thus:

\[
PF = S^2 \sigma
\]

exhibits a maximum at intermediate \( n \).

Optimization must respect intrinsic coupling manifold:

\[
\mathcal{M}_{tradeoff}
\subset \mathcal{R}_{phys}
\]

Models predicting simultaneous extreme \( S \) and extreme \( \sigma \) must be scrutinized.

---

# 9. Convex Feasible Region for Optimization

Define constraint set:

\[
\mathcal{C}
=
\{
\mathbf{x} \in \mathcal{R}_{phys}
\mid
\text{Kelvin},
\text{W-F},
\text{entropy}
\}
\]

Optimization objective must satisfy:

\[
\mathbf{x}_{opt} \in \mathcal{C}
\]

Hard constraints must override AI prediction.

---

# 10. Constraint Enforcement in Modeling

Let model prediction:

\[
\hat{\mathbf{x}} = f_\theta(\mathbf{u})
\]

If:

\[
\hat{\mathbf{x}} \notin \mathcal{C}
\]

then one of the following must occur:

1. Projection onto feasible set:

\[
\hat{\mathbf{x}} \leftarrow \Pi_{\mathcal{C}}(\hat{\mathbf{x}})
\]

2. Penalized loss:

\[
\mathcal{L}
=
\mathcal{L}_{data}
+
\lambda \, d(\hat{\mathbf{x}}, \mathcal{C})
\]

3. Rejection and retraining.

---

# 11. Constraint Geometry

Define signed constraint function:

\[
g_1(\mathbf{x}) = -\sigma
\]

\[
g_2(\mathbf{x}) = -\kappa
\]

\[
g_3(\mathbf{x}) = -T
\]

Feasibility condition:

\[
g_i(\mathbf{x}) \le 0
\]

The feasible region is a convex cone in \( (\sigma, \kappa, T) \)-subspace.

---

# 12. Stability Constraint

For small perturbations:

\[
\mathbf{x}' = \mathbf{x} + \delta \mathbf{x}
\]

System remains admissible if:

\[
\mathbf{x}' \in \mathcal{C}
\]

Thus constraints must be robust under uncertainty propagation.

---

# 13. Uncertainty-Aware Constraints

Given uncertainty covariance \( \Sigma \), require:

\[
P(\mathbf{X} \in \mathcal{C}) > 1 - \alpha
\]

Typical:

\[
\alpha = 0.05
\]

This ensures probabilistic physical validity.

---

# 14. Hierarchy of Constraints

Constraints are classified:

**Level 1 — Hard Physical Laws**
- Positivity
- Kelvin relation
- Entropy production

**Level 2 — Transport Consistency**
- Lorenz bounds
- Trade-off manifold

**Level 3 — Empirical Plausibility**
- Magnitude ranges

Hard constraints cannot be violated.  
Soft constraints may be flagged.

---

# 15. Strategic Interpretation

Physical constraints serve as:

- Guardrails for AI,
- Filters for noisy data,
- Stability anchors for optimization.

Without constraints, machine learning may:

- Exploit numerical artifacts,
- Generate non-physical extrema,
- Overfit to statistical noise.

Constraint integration ensures scientific validity.

---

# 16. Compliance Requirement

All modules must satisfy:

\[
\text{Module} \models \text{P03-PHYSICAL-CONSTRAINTS}
\]

Non-compliance constitutes:

- Physical invalidity,
- Scientific unreliability,
- Architectural failure.

---

# 17. Concluding Statement

The Thermognosis Engine is a physics-constrained intelligence system.

Its predictive power is subordinate to:

- Thermodynamics,
- Transport theory,
- Statistical consistency.

Physical law is not a suggestion.  
It is the boundary condition of scientific legitimacy.
