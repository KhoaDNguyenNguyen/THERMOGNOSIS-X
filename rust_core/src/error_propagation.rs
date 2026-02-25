// rust_core/src/error_propagation.rs

//! # Thermoelectric Uncertainty Propagation Core
//! 
//! This module implements the rigorously defined first-order analytical error 
//! propagation framework for the thermoelectric figure of merit (zT).
//! 
//! **Layer:** Physics / Statistical Consistency  
//! **Status:** Normative — Quantitative Integrity Requirement  
//! **Implements:** P02-ZT-ERROR-PROPAGATION, T03-UNCERTAINTY-PROPAGATION, SPEC-PHYS-ERROR-PROPAGATION
//! 
//! It provides both scalar and zero-cost parallelized batch operations via `rayon` 
//! to guarantee computational supremacy across massive experimental datasets.

use rayon::prelude::*;
use thiserror::Error;

/// Mathematical and Physical Error Constraints for Uncertainty Propagation.
/// Implements constraints from Document ID: P02-ZT-ERROR-PROPAGATION (Section 11)
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ErrorPropagationError {
    #[error("Physical Consistency Violation: Thermal conductivity (kappa) must be strictly positive. Found: {0}")]
    InvalidThermalConductivity(f64),
    
    #[error("Physical Consistency Violation: Absolute temperature (T) must be strictly positive. Found: {0}")]
    InvalidTemperature(f64),
    
    #[error("Mathematical Violation: Propagated variance is negative ({0}), indicating severe numerical instability.")]
    NegativeVariance(f64),

    #[error("Dimensionality Violation: Input arrays for parallel iteration must have identical lengths. Props: {0}, Errs: {1}")]
    DimensionMismatch(usize, usize),
}

/// Core deterministic properties of a thermoelectric state.
/// Symbols strictly map to physical definitions: (S, sigma, kappa, T).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThermoelectricProperties {
    /// Seebeck coefficient (S) in V/K
    pub s: f64,
    /// Electrical conductivity (sigma) in S/m
    pub sigma: f64,
    /// Thermal conductivity (kappa) in W/(m·K)
    pub kappa: f64,
    /// Absolute Temperature (T) in K
    pub t: f64,
}

/// Independent standard measurement uncertainties (1-sigma).
/// Maps to Section 4: Independent Variable Approximation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyUncertainties {
    /// Standard error of Seebeck coefficient
    pub err_s: f64,
    /// Standard error of electrical conductivity
    pub err_sigma: f64,
    /// Standard error of thermal conductivity
    pub err_kappa: f64,
    /// Standard error of temperature measurement
    pub err_t: f64,
}

/// The statistically rigorous result encompassing both the estimated
/// Figure of Merit (zT) and its absolute propagated standard uncertainty.
/// Implements Section 12: Reporting Standard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZtResult {
    /// Expected value of the Figure of Merit
    pub zt: f64,
    /// 1-sigma analytical uncertainty
    pub uncertainty: f64,
}

/// Computes the thermoelectric figure of merit (zT) and its first-order propagated uncertainty.
/// 
/// **Document ID:** P02-ZT-ERROR-PROPAGATION (Sections 3 & 4)
/// 
/// # Mathematical Formulation
/// Base equation:
/// \[ zT = \frac{S^2 \sigma T}{\kappa} \]
/// 
/// Gradients:
/// \[ \frac{\partial zT}{\partial S} = \frac{2 S \sigma T}{\kappa} \]
/// \[ \frac{\partial zT}{\partial \sigma} = \frac{S^2 T}{\kappa} \]
/// \[ \frac{\partial zT}{\partial \kappa} = - \frac{S^2 \sigma T}{\kappa^2} \]
/// \[ \frac{\partial zT}{\partial T} = \frac{S^2 \sigma}{\kappa} \]
/// 
/// First-Order Variance (Independent Approximation):
/// \[ \sigma_{zT}^2 \approx \left(\frac{\partial zT}{\partial S}\right)^2 \sigma_S^2 + \left(\frac{\partial zT}{\partial \sigma}\right)^2 \sigma_\sigma^2 + \left(\frac{\partial zT}{\partial \kappa}\right)^2 \sigma_\kappa^2 + \left(\frac{\partial zT}{\partial T}\right)^2 \sigma_T^2 \]
#[inline(always)]
pub fn calculate_zt_linear_propagation(
    props: &ThermoelectricProperties,
    errs: &PropertyUncertainties,
) -> Result<ZtResult, ErrorPropagationError> {
    let s = props.s;
    let sigma = props.sigma;
    let kappa = props.kappa;
    let t = props.t;

    // Axiomatic Physical Constraints Enforcement
    if kappa <= 0.0 {
        return Err(ErrorPropagationError::InvalidThermalConductivity(kappa));
    }
    if t <= 0.0 {
        return Err(ErrorPropagationError::InvalidTemperature(t));
    }

    // Nominal zT Calculation
    let zt = (s * s * sigma * t) / kappa;

    // Analytical Gradients Computation
    let dz_ds = (2.0 * s * sigma * t) / kappa;
    let dz_dsigma = (s * s * t) / kappa;
    let dz_dkappa = -(s * s * sigma * t) / (kappa * kappa);
    let dz_dt = (s * s * sigma) / kappa;

    // Independent First-Order Variance Propagation
    let var_zt = (dz_ds * dz_ds) * (errs.err_s * errs.err_s)
        + (dz_dsigma * dz_dsigma) * (errs.err_sigma * errs.err_sigma)
        + (dz_dkappa * dz_dkappa) * (errs.err_kappa * errs.err_kappa)
        + (dz_dt * dz_dt) * (errs.err_t * errs.err_t);

    // Section 11 Constraint: Ensure Physical Consistency of Variance
    if var_zt < 0.0 {
        return Err(ErrorPropagationError::NegativeVariance(var_zt));
    }

    Ok(ZtResult {
        zt,
        uncertainty: var_zt.sqrt(),
    })
}

/// High-throughput parallel calculation of zT and its uncertainties.
/// 
/// Maximizes CPU utilization across heterogeneous host environments (Arch, Windows, Colab)
/// by distributing rigorous mathematical transformations over available threads.
/// 
/// **Document ID:** T03-UNCERTAINTY-PROPAGATION
pub fn calculate_zt_batch_parallel(
    props_list: &[ThermoelectricProperties],
    errs_list: &[PropertyUncertainties],
) -> Result<Vec<ZtResult>, ErrorPropagationError> {
    if props_list.len() != errs_list.len() {
        return Err(ErrorPropagationError::DimensionMismatch(
            props_list.len(),
            errs_list.len(),
        ));
    }

    // Zero-cost abstraction for parallel iteration via Rayon
    props_list
        .par_iter()
        .zip(errs_list.par_iter())
        .map(|(props, errs)| calculate_zt_linear_propagation(props, errs))
        .collect()
}