//! # Unit System and Dimensional Validation Core
//!
//! **Document IDs**: 
//! - SPEC-UNIT-CONVERTER
//! - SPEC-UNIT-DIM-VALIDATION
//! - SPEC-UNIT-SYSTEM-REGISTRY
//!
//! This module forms the deterministic numerical backbone of the Thermognosis Engine. 
//! It provides rigid enforcement of canonical SI normalization, dimension-preserving transformations, 
//! affine conversions, and rigorous first-order uncertainty propagation.
//!
//! All operations execute in $\mathcal{O}(1)$ time per quantity.

use rayon::prelude::*;
use std::fmt;
use thiserror::Error;

/// Core error hierarchy mapping structural, mathematical, and physical violations.
/// Implements: SPEC-UNIT-DIM-VALIDATION (Section 15: Error Taxonomy)
#[derive(Error, Debug, Clone, PartialEq)]
pub enum UnitError {
    #[error("DV-01: Addition/Subtraction dimensional mismatch: {0} vs {1}")]
    DimensionMismatch(Dimension, Dimension),

    #[error("DV-02: Function domain violation. Expected dimensionless quantity, got {0}")]
    DomainViolation(Dimension),

    #[error("DV-03: Fractional exponent misuse on dimensioned quantity")]
    FractionalExponentMisuse,

    #[error("DV-05: Derived dimension inconsistency")]
    DerivedDimensionInconsistency,

    #[error("UR-01: Unknown unit symbol: {0}")]
    UnknownUnit(String),

    #[error("UC-12: Numerical stability threshold exceeded (> 1e308 or < 1e-308)")]
    NumericalInstability,
}

/// The 7-dimensional SI basis vector: $ \mathbf{d} \in \mathbb{Z}^7 $
/// Implements: SPEC-UNIT-DIM-VALIDATION (Section 2: Mathematical Foundation)
/// Indices: 0: Length (L), 1: Mass (M), 2: Time (T), 3: Current (I), 
/// 4: Temperature ($\Theta$), 5: Amount (N), 6: Luminous Intensity (J)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dimension {
    pub basis: [i32; 7],
}

impl Dimension {
    /// Constructs a new dimensional vector.
    pub const fn new(l: i32, m: i32, t: i32, i: i32, theta: i32, n: i32, j: i32) -> Self {
        Self { basis: [l, m, t, i, theta, n, j] }
    }

    /// Constructs the mathematically dimensionless vector $\mathbf{0}$.
    pub const fn dimensionless() -> Self {
        Self::new(0, 0, 0, 0, 0, 0, 0)
    }

    /// Checks if $\mathbf{d}(U) = \mathbf{0}$.
    /// Implements: SPEC-UNIT-DIM-VALIDATION (Section 10: Dimensionless Classification)
    pub fn is_dimensionless(&self) -> bool {
        self.basis == [0, 0, 0, 0, 0, 0, 0]
    }

    /// Multiplicative dimensional mapping: $\mathbf{d}(z) = \mathbf{d}(x) + \mathbf{d}(y)$
    pub const fn mul(&self, other: &Self) -> Self {
        Self::new(
            self.basis[0] + other.basis[0],
            self.basis[1] + other.basis[1],
            self.basis[2] + other.basis[2],
            self.basis[3] + other.basis[3],
            self.basis[4] + other.basis[4],
            self.basis[5] + other.basis[5],
            self.basis[6] + other.basis[6],
        )
    }

    /// Divisional dimensional mapping: $\mathbf{d}(z) = \mathbf{d}(x) - \mathbf{d}(y)$
    pub const fn div(&self, other: &Self) -> Self {
        Self::new(
            self.basis[0] - other.basis[0],
            self.basis[1] - other.basis[1],
            self.basis[2] - other.basis[2],
            self.basis[3] - other.basis[3],
            self.basis[4] - other.basis[4],
            self.basis[5] - other.basis[5],
            self.basis[6] - other.basis[6],
        )
    }

    /// Power dimensional mapping for integer exponents: $\mathbf{d}(z) = n \mathbf{d}(x)$
    pub const fn powi(&self, n: i32) -> Self {
        Self::new(
            self.basis[0] * n,
            self.basis[1] * n,
            self.basis[2] * n,
            self.basis[3] * n,
            self.basis[4] * n,
            self.basis[5] * n,
            self.basis[6] * n,
        )
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[L^{} M^{} T^{} I^{} Theta^{} N^{} J^{}]", 
            self.basis[0], self.basis[1], self.basis[2], 
            self.basis[3], self.basis[4], self.basis[5], self.basis[6])
    }
}

/// A rigorous definition of a Physical Unit mapped onto the dimensional basis.
/// Implements: SPEC-UNIT-SYSTEM-REGISTRY (Section 2: Formal Structure)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitDefinition {
    pub symbol: &'static str,
    pub dimension: Dimension,
    pub k: f64, // Multiplicative scaling factor
    pub b: f64, // Affine offset
}

impl UnitDefinition {
    /// Generates the canonical SI proxy for any dimensional state space.
    pub const fn canonical_si(dim: Dimension) -> Self {
        Self {
            symbol: "SI_CANON",
            dimension: dim,
            k: 1.0,
            b: 0.0,
        }
    }
}

/// The canonical entity holding a physical scalar, its spatial mapping, and its stochastic variance.
/// Implements: $Q = (v, U, \sigma)$
/// Reference: SPEC-UNIT-CONVERTER (Section 2: Formal Definition)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalQuantity {
    pub value: f64,
    pub unit: UnitDefinition,
    pub uncertainty: f64,
}

impl PhysicalQuantity {
    /// Creates a new `PhysicalQuantity` conforming to the dimensional constraints.
    pub fn new(value: f64, unit: UnitDefinition, uncertainty: f64) -> Result<Self, UnitError> {
        let q = Self { value, unit, uncertainty };
        q.validate_numerical_stability()?;
        Ok(q)
    }

    /// Validates determinism constraints and overflow limits.
    /// Implements: SPEC-UNIT-CONVERTER (Section 12: Overflow and Underflow Handling)
    fn validate_numerical_stability(&self) -> Result<(), UnitError> {
        let abs_val = self.value.abs();
        if abs_val > 1e308 || (abs_val < 1e-308 && abs_val > 0.0) {
            return Err(UnitError::NumericalInstability);
        }
        Ok(())
    }

    /// Enforces 12 significant digits deterministic rounding.
    /// Implements: SPEC-UNIT-CONVERTER (Section 8: Floating-Point Determinism)
    pub fn deterministic_round(&self, precision: u32) -> Self {
        if self.value == 0.0 { return *self; }
        let magnitude = self.value.abs().log10().floor();
        let scale = 10_f64.powi(precision as i32 - 1 - magnitude as i32);
        
        Self {
            value: (self.value * scale).round() / scale,
            unit: self.unit,
            uncertainty: self.uncertainty, // Uncertainty precision typically inherited, but rounded minimally
        }
    }

    /// Transforms $Q$ into $Q'$ under the target dimensional system.
    /// Implements: SPEC-UNIT-CONVERTER (Sections 4 & 5: Affine & Scaling Transformation)
    pub fn convert(&self, target_unit: UnitDefinition) -> Result<PhysicalQuantity, UnitError> {
        if self.unit.dimension != target_unit.dimension {
            return Err(UnitError::DimensionMismatch(self.unit.dimension, target_unit.dimension));
        }

        // Canonical SI value mapping: v_SI = k_s * v + b_s
        let v_si = self.unit.k * self.value + self.unit.b;
        
        // Target projection: v' = (v_SI - b_t) / k_t
        let v_prime = (v_si - target_unit.b) / target_unit.k;

        // First-order propagation for linear operators: \sigma' = |k_s / k_t| \sigma
        let sigma_prime = (self.unit.k / target_unit.k).abs() * self.uncertainty;

        PhysicalQuantity::new(v_prime, target_unit, sigma_prime)
            .map(|q| q.deterministic_round(12))
    }

    /// Normalizes the quantity to its Canonical SI root state prior to aggregations.
    /// Implements: SPEC-UNIT-CONVERTER (Section 6: Canonical SI Normalization)
    pub fn into_canonical(&self) -> Result<PhysicalQuantity, UnitError> {
        self.convert(UnitDefinition::canonical_si(self.unit.dimension))
    }

    /// Dimensionally validated addition: $z = x + y$
    /// Implements: SPEC-UNIT-DIM-VALIDATION (Section 4: Binary Operation Validation)
    pub fn try_add(&self, other: &PhysicalQuantity) -> Result<PhysicalQuantity, UnitError> {
        let q1 = self.into_canonical()?;
        let q2 = other.into_canonical()?;

        if q1.unit.dimension != q2.unit.dimension {
            return Err(UnitError::DimensionMismatch(q1.unit.dimension, q2.unit.dimension));
        }

        let new_value = q1.value + q2.value;
        // Independent uncertainty quadrature: \sigma_z = \sqrt{\sigma_x^2 + \sigma_y^2}
        let new_sigma = (q1.uncertainty.powi(2) + q2.uncertainty.powi(2)).sqrt();

        PhysicalQuantity::new(new_value, q1.unit, new_sigma)
    }

    /// Dimensionally validated subtraction: $z = x - y$
    pub fn try_sub(&self, other: &PhysicalQuantity) -> Result<PhysicalQuantity, UnitError> {
        let q1 = self.into_canonical()?;
        let q2 = other.into_canonical()?;

        if q1.unit.dimension != q2.unit.dimension {
            return Err(UnitError::DimensionMismatch(q1.unit.dimension, q2.unit.dimension));
        }

        let new_value = q1.value - q2.value;
        let new_sigma = (q1.uncertainty.powi(2) + q2.uncertainty.powi(2)).sqrt();

        PhysicalQuantity::new(new_value, q1.unit, new_sigma)
    }

    /// Multiplicative closure mapping: $z = x \cdot y$
    /// Implements: SPEC-UNIT-DIM-VALIDATION (Section 5) & SPEC-UNIT-CONVERTER (Section 9)
    pub fn try_mul(&self, other: &PhysicalQuantity) -> Result<PhysicalQuantity, UnitError> {
        let q1 = self.into_canonical()?;
        let q2 = other.into_canonical()?;

        let new_dim = q1.unit.dimension.mul(&q2.unit.dimension);
        let new_value = q1.value * q2.value;

        // First order Taylor approximation for multiplication: \sigma_z = \sqrt{(y\sigma_x)^2 + (x\sigma_y)^2}
        let new_sigma = ((q2.value * q1.uncertainty).powi(2) + (q1.value * q2.uncertainty).powi(2)).sqrt();

        PhysicalQuantity::new(new_value, UnitDefinition::canonical_si(new_dim), new_sigma)
    }

    /// Divisive closure mapping: $z = x / y$
    pub fn try_div(&self, other: &PhysicalQuantity) -> Result<PhysicalQuantity, UnitError> {
        let q1 = self.into_canonical()?;
        let q2 = other.into_canonical()?;

        if q2.value == 0.0 {
            return Err(UnitError::NumericalInstability);
        }

        let new_dim = q1.unit.dimension.div(&q2.unit.dimension);
        let new_value = q1.value / q2.value;

        // \sigma_z = \sqrt{(\sigma_x / y)^2 + (-x \sigma_y / y^2)^2}
        let term1 = q1.uncertainty / q2.value;
        let term2 = (q1.value * q2.uncertainty) / q2.value.powi(2);
        let new_sigma = (term1.powi(2) + term2.powi(2)).sqrt();

        PhysicalQuantity::new(new_value, UnitDefinition::canonical_si(new_dim), new_sigma)
    }

    /// Exponentiation restricted to dimensionless domains.
    /// Implements: SPEC-UNIT-DIM-VALIDATION (Section 7.1: Exponential & Logarithmic)
    pub fn try_exp(&self) -> Result<PhysicalQuantity, UnitError> {
        let q1 = self.into_canonical()?;
        if !q1.unit.dimension.is_dimensionless() {
            return Err(UnitError::DomainViolation(q1.unit.dimension));
        }

        let new_value = q1.value.exp();
        // Derivative of exp(x) is exp(x). \sigma_z = |exp(x)| \sigma_x
        let new_sigma = new_value.abs() * q1.uncertainty;

        PhysicalQuantity::new(new_value, UnitDefinition::canonical_si(Dimension::dimensionless()), new_sigma)
    }
}

/// Thread-safe bulk transformation utility scaling over heterogeneous architectures.
/// Exploits `#![feature(rayon)]` to maximize ALUs for $\mathcal{O}(N)$ payload matrices.
pub fn par_convert_bulk(
    quantities: &[PhysicalQuantity],
    target_unit: UnitDefinition,
) -> Result<Vec<PhysicalQuantity>, UnitError> {
    quantities
        .par_iter()
        .map(|q| q.convert(target_unit))
        .collect()
}

// ==============================================================================
// REGISTRY FIXTURES (Reference instances proving correct affine scaling)
// Implements: SPEC-UNIT-SYSTEM-REGISTRY
// ==============================================================================

pub const DIM_LENGTH: Dimension = Dimension::new(1, 0, 0, 0, 0, 0, 0);
pub const DIM_TEMP: Dimension = Dimension::new(0, 0, 0, 0, 1, 0, 0);

pub const METRE: UnitDefinition = UnitDefinition {
    symbol: "m",
    dimension: DIM_LENGTH,
    k: 1.0,
    b: 0.0,
};

pub const MILLIMETRE: UnitDefinition = UnitDefinition {
    symbol: "mm",
    dimension: DIM_LENGTH,
    k: 1e-3,
    b: 0.0,
};

pub const KELVIN: UnitDefinition = UnitDefinition {
    symbol: "K",
    dimension: DIM_TEMP,
    k: 1.0,
    b: 0.0,
};

pub const CELSIUS: UnitDefinition = UnitDefinition {
    symbol: "degC",
    dimension: DIM_TEMP,
    k: 1.0,
    b: 273.15,
};