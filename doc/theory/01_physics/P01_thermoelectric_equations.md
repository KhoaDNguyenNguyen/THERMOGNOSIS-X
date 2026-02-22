# P01 — Fundamental Thermoelectric Equations  
**Document ID:** P01-THERMOELECTRIC-EQUATIONS  
**Layer:** Theory / Physics  
**Status:** Normative — Governing Physical Law  
**Dependencies:** T00-SYS-AXIOMS, T01-DATA-FORMALISM, T02-MEASUREMENT-SPACE, T03-UNCERTAINTY-PROPAGATION  

---

# 1. Purpose

This document formalizes the governing thermoelectric equations underlying the Thermognosis Engine.

Its objectives are:

1. To define the macroscopic transport equations for thermoelectric phenomena.
2. To establish the constitutive relations connecting measurable quantities.
3. To formalize the definition of the thermoelectric figure of merit.
4. To define constraints that must be enforced during validation and modeling.
5. To serve as the physical foundation for all data ingestion, modeling, and optimization.

This document is normative.  
All derived quantities, validation routines, and optimization targets must remain consistent with the equations herein.

---

# 2. Coupled Transport Phenomena

Thermoelectric effects arise from the coupling of:

- Charge transport
- Heat transport

Under linear response theory, the constitutive relations are:

\[
\mathbf{J} = \sigma \left( -\nabla V + S \nabla T \right)
\]

\[
\mathbf{Q} = \Pi \mathbf{J} - \kappa \nabla T
\]

where:

- \( \mathbf{J} \) : electrical current density  
- \( \mathbf{Q} \) : heat flux density  
- \( \sigma \) : electrical conductivity  
- \( S \) : Seebeck coefficient  
- \( \Pi \) : Peltier coefficient  
- \( \kappa \) : thermal conductivity  
- \( V \) : electric potential  
- \( T \) : temperature  

---

# 3. Kelvin Relations

Thermodynamic consistency requires:

\[
\Pi = S T
\]

This is the Kelvin relation, ensuring reciprocity between Seebeck and Peltier effects.

Violation of this relation implies thermodynamic inconsistency.

---

# 4. Electrical Conductivity

Electrical conductivity is defined as:

\[
\sigma = n q \mu
\]

where:

- \( n \) : carrier concentration  
- \( q \) : elementary charge  
- \( \mu \) : carrier mobility  

Resistivity:

\[
\rho = \frac{1}{\sigma}
\]

All datasets must satisfy:

\[
\sigma > 0
\]

---

# 5. Thermal Conductivity Decomposition

Total thermal conductivity:

\[
\kappa = \kappa_e + \kappa_l
\]

where:

- \( \kappa_e \) : electronic contribution  
- \( \kappa_l \) : lattice contribution  

Electronic contribution approximated by Wiedemann–Franz law:

\[
\kappa_e = L \sigma T
\]

where:

- \( L \) : Lorenz number  

Typical metallic limit:

\[
L_0 = 2.44 \times 10^{-8} \, \mathrm{W \Omega K^{-2}}
\]

Deviation from expected Lorenz values must be flagged.

---

# 6. Seebeck Coefficient

The Seebeck coefficient is defined as:

\[
S = - \left( \frac{\partial V}{\partial T} \right)_{\mathbf{J}=0}
\]

Sign convention:

- \( S > 0 \) : p-type conduction  
- \( S < 0 \) : n-type conduction  

The magnitude of \( S \) typically satisfies:

\[
|S| < 1000 \, \mu \mathrm{V/K}
\]

Values outside physically realistic bounds require validation.

---

# 7. Power Factor

Define power factor:

\[
PF = S^2 \sigma
\]

Units:

\[
\mathrm{W \, m^{-1} K^{-2}}
\]

Power factor represents electrical performance independent of thermal conductivity.

---

# 8. Figure of Merit

The thermoelectric figure of merit is:

\[
zT = \frac{S^2 \sigma T}{\kappa}
\]

Equivalent representation:

\[
zT = \frac{PF \, T}{\kappa}
\]

This dimensionless parameter governs material performance.

All validated datasets must satisfy:

\[
zT \ge 0
\]

---

# 9. Efficiency of Thermoelectric Generator

Maximum conversion efficiency:

\[
\eta_{\max}
=
\frac{T_h - T_c}{T_h}
\cdot
\frac{\sqrt{1 + \bar{zT}} - 1}
{\sqrt{1 + \bar{zT}} + \frac{T_c}{T_h}}
\]

where:

- \( T_h \) : hot-side temperature  
- \( T_c \) : cold-side temperature  
- \( \bar{zT} \) : average figure of merit  

This defines the upper thermodynamic bound for module performance.

---

# 10. Temperature Dependence

Thermoelectric properties are temperature-dependent:

\[
S = S(T)
\]

\[
\sigma = \sigma(T)
\]

\[
\kappa = \kappa(T)
\]

Thus:

\[
zT = zT(T)
\]

Optimization must consider temperature window, not single-point maxima.

---

# 11. Physical Constraints for Validation

The Thermognosis Engine must enforce:

1. \( \sigma > 0 \)
2. \( \kappa > 0 \)
3. \( T > 0 \)
4. \( zT = \frac{S^2 \sigma T}{\kappa} \)
5. Reasonable bounds on Lorenz number:

\[
L = \frac{\kappa_e}{\sigma T}
\]

If:

\[
L \gg L_0
\quad \text{or} \quad
L \ll 10^{-9}
\]

then the record must be flagged.

---

# 12. Differential Sensitivity of zT

Gradient:

\[
\nabla zT =
\left(
\frac{\partial zT}{\partial S},
\frac{\partial zT}{\partial \sigma},
\frac{\partial zT}{\partial \kappa},
\frac{\partial zT}{\partial T}
\right)
\]

This gradient is essential for:

- Uncertainty propagation,
- Sensitivity analysis,
- Bayesian optimization.

---

# 13. Material Design Implications

Maximizing \( zT \) requires:

\[
\text{High } S
\quad
\text{High } \sigma
\quad
\text{Low } \kappa
\]

However, these quantities are interdependent via carrier concentration.

This defines an intrinsic trade-off manifold.

---

# 14. Constraint Geometry

The admissible region for optimization is:

\[
\mathcal{R}_{phys}
=
\{
(S, \sigma, \kappa, T)
\in \mathbb{R}^4
\mid
\sigma > 0,
\kappa > 0,
T > 0
\}
\]

with derived constraint:

\[
zT = f(S, \sigma, \kappa, T)
\]

All predicted values must lie within this region.

---

# 15. Strategic Interpretation

The Thermognosis Engine is physically constrained.

It is not permitted to:

- Predict negative conductivity,
- Produce inconsistent zT,
- Violate Kelvin relation,
- Ignore Wiedemann–Franz consistency.

All modeling layers are subordinate to physical law.

---

# 16. Concluding Statement

This document defines the governing thermoelectric equations that anchor the Thermognosis Engine to physical reality.

All ingestion, validation, modeling, optimization, and experimental feedback processes must satisfy:

\[
\text{Module} \models \text{P01-THERMOELECTRIC-EQUATIONS}
\]

The Thermognosis Engine is defined not merely by computational capability,  
but by unwavering adherence to thermodynamic and transport law.
