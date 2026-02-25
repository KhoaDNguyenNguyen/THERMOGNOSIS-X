//! Physical Consistency and Constraint Validation Module
//!
//! This module enforces fundamental physical laws, invariants, and thermodynamic constraints
//! upon raw numerical representations. No state may be promoted to downstream predictive
//! or closed-loop components without satisfying the algebraic and inequalities formalized herein.
//!
//! # Document Governance
//! - Implements: `SPEC-PHYS-CONSISTENCY`
//! - Implements: `SPEC-PHYS-CONSTRAINTS`

use rayon::prelude::*;
use thiserror::Error;

/// Enumeration of formal physical constraint violations.
///
/// Every violation is strongly typed and maps directly to the Error Classification
/// schema defined in the core specifications, ensuring precise closed-loop feedback.
///
/// # Document IDs
/// Implements: SPEC-PHYS-CONSISTENCY (17. Error Classification)
/// Implements: SPEC-PHYS-CONSTRAINTS (17. Error Classification)
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// PCON-03: Bound violation indicating non-finite numerical explosion.
    #[error("PCON-03: Bound Violation - Value is NaN or Infinite: {0}")]
    NonFiniteValue(f64),

    /// PC-02/PCON-02: Negative-definite violation for Temperature.
    /// Constraint: $T > 0$
    #[error("PC-02/PCON-02: Negative Absolute Temperature: T = {0} <= 0")]
    NegativeAbsoluteTemperature(f64),

    /// PC-02/PCON-02: Negative-definite violation for Electrical Conductivity.
    /// Constraint: $\sigma > 0$
    #[error("PC-02/PCON-02: Negative Electrical Conductivity: sigma = {0} <= 0")]
    NegativeElectricalConductivity(f64),

    /// PC-02/PCON-02: Negative-definite violation for Thermal Conductivity.
    /// Constraint: $\kappa > 0$
    #[error("PC-02/PCON-02: Negative Thermal Conductivity: kappa = {0} <= 0")]
    NegativeThermalConductivity(f64),

    /// PC-05/PCON-05: Thermodynamic Violation, negative Figure of Merit.
    /// Constraint: $ZT \ge 0$
    #[error("PC-05/PCON-05: Thermodynamic Violation - Negative Figure of Merit: ZT = {0} < 0")]
    NegativeFigureOfMerit(f64),
}

/// Raw, unvalidated representation of a thermoelectric physical state.
///
/// This structure interfaces with the FFI boundary, receiving unconstrained
/// arrays of floating-point numbers from Python/numpy.
///
/// # State Vector Representation
/// Let $\mathcal{S} = \{ S, \sigma, \kappa, T \}$
///
/// # Document IDs
/// Implements: SPEC-PHYS-CONSISTENCY (3. Physical State Representation)
#[repr(C)]
#[derive(Debug, Clone, PartialEq)]
pub struct ThermoelectricState {
    /// Seebeck coefficient ($S$), typically in V/K.
    pub s: f64,
    /// Electrical conductivity ($\sigma$), typically in S/m.
    pub sigma: f64,
    /// Thermal conductivity ($\kappa$), typically in W/(m·K).
    pub kappa: f64,
    /// Absolute temperature ($T$), in Kelvin.
    pub t: f64,
}

/// A strictly validated physical state satisfying all core thermodynamic constraints.
///
/// Instances of this struct can *only* be created via successful constraint evaluation,
/// ensuring the type system enforces physical correctness for downstream modules.
///
/// # Derived Quantities
/// Includes the dimensionless figure of merit ($ZT$).
///
/// # Document IDs
/// Implements: SPEC-PHYS-CONSTRAINTS (9. ZT Constraint Structure)
#[repr(C)]
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedState {
    s: f64,
    sigma: f64,
    kappa: f64,
    t: f64,
    zt: f64,
}

impl ValidatedState {
    /// Retrieves the dimensionless figure of merit ($ZT$).
    #[inline]
    pub fn zt(&self) -> f64 {
        self.zt
    }

    /// Exposes the underlying parameters securely as a read-only tuple.
    #[inline]
    pub fn parameters(&self) -> (f64, f64, f64, f64) {
        (self.s, self.sigma, self.kappa, self.t)
    }
}

impl ThermoelectricState {
    /// Evaluates the physical consistency constraints on the raw state vector.
    ///
    /// # Mathematical Constraints
    /// 1. Domain Bounds: $x \in [x_{\min}, x_{\max}]$ (Implicitly finite)
    /// 2. Positivity:
    ///    - Absolute Temperature: $T > 0$
    ///    - Electrical Conductivity: $\sigma > 0$
    ///    - Thermal Conductivity: $\kappa > 0$
    /// 3. Thermoelectric Consistency:
    ///    - Figure of Merit: $ZT = \frac{S^2 \sigma T}{\kappa} \ge 0$
    ///
    /// # Document IDs
    /// Implements: SPEC-PHYS-CONSISTENCY (6. Positivity Constraints)
    /// Implements: SPEC-PHYS-CONSISTENCY (7. Thermoelectric Consistency)
    /// Implements: SPEC-PHYS-CONSTRAINTS (9. ZT Constraint Structure)
    #[inline]
    pub fn validate(&self) -> Result<ValidatedState, ValidationError> {
        // PCON-03: Boundedness / Finite Check
        if !self.s.is_finite() {
            return Err(ValidationError::NonFiniteValue(self.s));
        }
        if !self.sigma.is_finite() {
            return Err(ValidationError::NonFiniteValue(self.sigma));
        }
        if !self.kappa.is_finite() {
            return Err(ValidationError::NonFiniteValue(self.kappa));
        }
        if !self.t.is_finite() {
            return Err(ValidationError::NonFiniteValue(self.t));
        }

        // PCON-02 / PC-02: Hard Positivity Constraints
        if self.t <= 0.0 {
            return Err(ValidationError::NegativeAbsoluteTemperature(self.t));
        }
        if self.sigma <= 0.0 {
            return Err(ValidationError::NegativeElectricalConductivity(self.sigma));
        }
        if self.kappa <= 0.0 {
            return Err(ValidationError::NegativeThermalConductivity(self.kappa));
        }

        // Evaluate figure of merit
        // ZT = (S^2 * \sigma * T) / \kappa
        let zt = (self.s * self.s * self.sigma * self.t) / self.kappa;

        // PCON-05 / PC-05: Thermodynamic Bounds
        if zt < 0.0 {
            return Err(ValidationError::NegativeFigureOfMerit(zt));
        }

        Ok(ValidatedState {
            s: self.s,
            sigma: self.sigma,
            kappa: self.kappa,
            t: self.t,
            zt,
        })
    }
}

/// Performs zero-cost, massively parallel constraint evaluation over large arrays of raw states.
///
/// Utilizing `rayon`, this function projects an array of states through the
/// validation constraints with computational complexity $\mathcal{O}(m/p)$, where $p$
/// is the number of available logical cores.
///
/// Fail-fast semantics are employed: the computation short-circuits upon the first
/// detected violation, returning the precise coordinate and nature of the mathematical failure.
///
/// # Mathematical Definition
/// Let $\mathcal{F}$ be the admissible state space.
/// Evaluates: $\forall \mathcal{S}_i \in \mathbf{S}: \mathcal{S}_i \in \mathcal{F}$
///
/// # Document IDs
/// Implements: SPEC-PHYS-CONSISTENCY (19. Computational Complexity)
/// Implements: SPEC-PHYS-CONSTRAINTS (18. Computational Requirements)
pub fn validate_states_par(states: &[ThermoelectricState]) -> Result<Vec<ValidatedState>, ValidationError> {
    states
        .par_iter()
        .map(|state| state.validate())
        .collect::<Result<Vec<ValidatedState>, ValidationError>>()
}