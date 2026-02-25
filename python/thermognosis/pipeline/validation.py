# -*- coding: utf-8 -*-
"""
Thermognosis Engine: Physical Validation and Constraint Enforcement

This module implements the deterministic verification of thermodynamic and 
transport property constraints, alongside rigorous first-order uncertainty 
propagation for the thermoelectric figure of merit (zT).

Document IDs: 
    - SPEC-PHYS-CONSTRAINTS
    - P02-ZT-ERROR-PROPAGATION
    - SPEC-GOV-CODE-GENERATION-PROTOCOL
    - SPEC-GOV-ERROR-HIERARCHY

Author: Distinguished Professor of Computational Materials Science
Institution: Thermognosis Engine Consortium
"""

import numpy as np
import pandas as pd
from typing import Optional, Union, Tuple
from dataclasses import dataclass

# Fallback capability for heterogeneous hardware (GPU/CPU)
# NumPy is highly optimized for vectorization, but CuPy can drop-in if available.
try:
    import cupy as cp
    HAS_GPU = True
except ImportError:
    HAS_GPU = False


# =============================================================================
# EXCEPTION HIERARCHY (Implements: SPEC-GOV-ERROR-HIERARCHY)
# =============================================================================

class ThermognosisPhysicsError(Exception):
    """Base exception for all physical consistency violations."""
    pass

class PhysicalConstraintError(ThermognosisPhysicsError):
    """
    Raised when a numerical state violates a hard physical constraint.
    Implements: PCON-01 through PCON-06 (SPEC-PHYS-CONSTRAINTS).
    """
    def __init__(self, message: str, violations: int):
        super().__init__(f"{message} (Total violations: {violations})")
        self.violations = violations

class NumericalStabilityError(ThermognosisPhysicsError):
    """Raised when calculations risk overflow, underflow, or singularities."""
    pass


# =============================================================================
# DATA STRUCTURES
# =============================================================================

@dataclass(frozen=True)
class ValidatedThermoelectricState:
    """
    Immutable data object representing a physically validated thermoelectric state.
    
    Attributes
    ----------
    T : np.ndarray
        Absolute temperature (K).
    S : np.ndarray
        Seebeck coefficient (V/K).
    sigma : np.ndarray
        Electrical conductivity (S/m).
    kappa : np.ndarray
        Thermal conductivity (W/(m K)).
    zT : np.ndarray
        Thermoelectric figure of merit (dimensionless).
    zT_err : np.ndarray
        Propagated standard uncertainty for zT.
    is_valid : np.ndarray
        Boolean mask of physically admissible states.
    """
    T: np.ndarray
    S: np.ndarray
    sigma: np.ndarray
    kappa: np.ndarray
    zT: np.ndarray
    zT_err: np.ndarray
    is_valid: np.ndarray


# =============================================================================
# CORE VALIDATION AND COMPUTATION LOGIC
# =============================================================================

class ThermoelectricValidator:
    """
    Vectorized validation and calculation engine for thermoelectric properties.
    Ensures mathematical fidelity, deterministic execution, and physical coherence.
    """

    @staticmethod
    def _to_array(data: Union[pd.Series, np.ndarray, float, int]) -> np.ndarray:
        """Standardizes input to 64-bit float NumPy arrays for deterministic precision."""
        arr = np.atleast_1d(np.asarray(data, dtype=np.float64))
        # Optional hardware-specific optimization could route to cp.asarray(arr) here
        return arr

    @classmethod
    def compute_zt(
        cls,
        S: Union[np.ndarray, float],
        sigma: Union[np.ndarray, float],
        T: Union[np.ndarray, float],
        kappa: Union[np.ndarray, float],
        err_S: Optional[Union[np.ndarray, float]] = None,
        err_sigma: Optional[Union[np.ndarray, float]] = None,
        err_T: Optional[Union[np.ndarray, float]] = None,
        err_kappa: Optional[Union[np.ndarray, float]] = None
    ) -> Tuple[np.ndarray, np.ndarray]:
        """
        Computes the thermoelectric figure of merit (zT) and its standard uncertainty.

        Implements: P02-ZT-ERROR-PROPAGATION

        Mathematical Formulation:
            zT = (S^2 * sigma * T) / kappa

        First-order Taylor expansion variance (assuming independent variables):
            Var(zT) = (∂zT/∂S)^2 Var(S) + (∂zT/∂sigma)^2 Var(sigma) + 
                      (∂zT/∂T)^2 Var(T) + (∂zT/∂kappa)^2 Var(kappa)

            ∂zT/∂S     = (2 * S * sigma * T) / kappa
            ∂zT/∂sigma = (S^2 * T) / kappa
            ∂zT/∂T     = (S^2 * sigma) / kappa
            ∂zT/∂kappa = -(S^2 * sigma * T) / kappa^2

        Parameters
        ----------
        S : np.ndarray or float
            Seebeck coefficient.
        sigma : np.ndarray or float
            Electrical conductivity.
        T : np.ndarray or float
            Absolute temperature.
        kappa : np.ndarray or float
            Thermal conductivity.
        err_* : np.ndarray or float, optional
            Standard uncertainties for the respective parameters.

        Returns
        -------
        Tuple[np.ndarray, np.ndarray]
            (zT, zT_err) - The computed figure of merit and its absolute uncertainty.
        """
        S_arr = cls._to_array(S)
        sigma_arr = cls._to_array(sigma)
        T_arr = cls._to_array(T)
        kappa_arr = cls._to_array(kappa)

        # Broadcast check
        if not (S_arr.shape == sigma_arr.shape == T_arr.shape == kappa_arr.shape):
            raise ValueError("Input arrays for S, sigma, T, kappa must have identical shapes.")

        # Prevent ZeroDivisionError via strictly evaluated masks (handled in validation phase)
        # We use a safe denominator strategy. Non-physical kappas will be flagged invalid later.
        safe_kappa = np.where(kappa_arr == 0, np.nan, kappa_arr)
        
        # Core computation
        zT = (S_arr**2 * sigma_arr * T_arr) / safe_kappa

        # Default uncertainties to zero if not provided
        err_S_arr = cls._to_array(err_S) if err_S is not None else np.zeros_like(S_arr)
        err_sig_arr = cls._to_array(err_sigma) if err_sigma is not None else np.zeros_like(sigma_arr)
        err_T_arr = cls._to_array(err_T) if err_T is not None else np.zeros_like(T_arr)
        err_kap_arr = cls._to_array(err_kappa) if err_kappa is not None else np.zeros_like(kappa_arr)

        # Partial derivatives
        dzT_dS = (2.0 * S_arr * sigma_arr * T_arr) / safe_kappa
        dzT_dsigma = (S_arr**2 * T_arr) / safe_kappa
        dzT_dT = (S_arr**2 * sigma_arr) / safe_kappa
        dzT_dkappa = -(S_arr**2 * sigma_arr * T_arr) / (safe_kappa**2)

        # First-order error propagation
        zT_var = (
            (dzT_dS * err_S_arr)**2 +
            (dzT_dsigma * err_sig_arr)**2 +
            (dzT_dT * err_T_arr)**2 +
            (dzT_dkappa * err_kap_arr)**2
        )
        zT_err = np.sqrt(zT_var)

        return zT, zT_err

    @classmethod
    def validate(
        cls,
        S: Union[np.ndarray, pd.Series],
        sigma: Union[np.ndarray, pd.Series],
        T: Union[np.ndarray, pd.Series],
        kappa: Union[np.ndarray, pd.Series],
        err_S: Optional[Union[np.ndarray, pd.Series]] = None,
        err_sigma: Optional[Union[np.ndarray, pd.Series]] = None,
        err_T: Optional[Union[np.ndarray, pd.Series]] = None,
        err_kappa: Optional[Union[np.ndarray, pd.Series]] = None,
        strict: bool = False
    ) -> ValidatedThermoelectricState:
        """
        Evaluates hard physical constraints and returns a validated state object.

        Implements: SPEC-PHYS-CONSTRAINTS
        
        Constraints Enforced:
            1. Positivity Constraint: T > 0
            2. Positivity Constraint: sigma > 0
            3. Positivity Constraint: kappa > 0
            4. Thermodynamic Constraint: zT >= 0

        Parameters
        ----------
        S, sigma, T, kappa : np.ndarray or pd.Series
            Physical observables.
        err_* : np.ndarray or pd.Series, optional
            Corresponding standard uncertainties.
        strict : bool, default False
            If True, raises a PhysicalConstraintError if any data points fail validation.
            If False, returns the validated state with `is_valid` boolean flags.

        Returns
        -------
        ValidatedThermoelectricState
            Immutable dataclass containing arrays and valid state masks.

        Raises
        ------
        PhysicalConstraintError
            If `strict` is True and constraint violations exist.
        """
        # Convert all to deterministic numpy arrays
        T_arr = cls._to_array(T)
        S_arr = cls._to_array(S)
        sigma_arr = cls._to_array(sigma)
        kappa_arr = cls._to_array(kappa)

        # Compute Figure of Merit
        zT_arr, zT_err_arr = cls.compute_zt(
            S=S_arr, sigma=sigma_arr, T=T_arr, kappa=kappa_arr,
            err_S=err_S, err_sigma=err_sigma, err_T=err_T, err_kappa=err_kappa
        )

        # Vectorized Constraint Evaluation (SPEC-PHYS-CONSTRAINTS)
        # NaN propagation explicitly handled: if nan, condition is False
        valid_T = np.greater(T_arr, 0.0, where=~np.isnan(T_arr))
        valid_sigma = np.greater(sigma_arr, 0.0, where=~np.isnan(sigma_arr))
        valid_kappa = np.greater(kappa_arr, 0.0, where=~np.isnan(kappa_arr))
        valid_zT = np.greater_equal(zT_arr, 0.0, where=~np.isnan(zT_arr))

        # Intersection of all physically admissible spaces
        is_valid = valid_T & valid_sigma & valid_kappa & valid_zT & ~np.isnan(zT_arr)

        if strict and not np.all(is_valid):
            violations = int(np.sum(~is_valid))
            raise PhysicalConstraintError(
                "Numerical state violates thermodynamic positivity constraints (T>0, sigma>0, kappa>0, zT>=0).",
                violations=violations
            )

        return ValidatedThermoelectricState(
            T=T_arr,
            S=S_arr,
            sigma=sigma_arr,
            kappa=kappa_arr,
            zT=zT_arr,
            zT_err=zT_err_arr,
            is_valid=is_valid
        )