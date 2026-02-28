"""
Thermognosis Engine - Rust Core FFI Boundary Wrapper
Document ID: SPEC-GOV-CODE-GENERATION-PROTOCOL
Status: Normative — Strict Mathematical Execution Environment

This module serves as the rigorously governed interface between the high-level
Python orchestrator and the natively compiled Rust backend (`rust_core`).
It strictly enforces memory layout constraints, explicit data typing, and
zero-copy tensor transmission across the Foreign Function Interface (FFI).

Layer: API / FFI Boundary
Implements:
    - SPEC-GOV-NAMING-RULES (Strict mathematical symbol mapping)
    - SPEC-GOV-ERROR-HIERARCHY (No silent failures; strict exception promotion)
"""

import logging
from pathlib import Path
from typing import Dict, Tuple, Union, Any, Optional, List

import numpy as np

# Configure module-level logger
logger = logging.getLogger(__name__)


class RustCoreError(Exception):
    """
    Exception raised for failures originating in or crossing the Rust FFI boundary.
    
    Document ID: SPEC-GOV-ERROR-HIERARCHY
    
    Traps PyO3 `ValueError` and `RuntimeError` exceptions thrown by the Rust core,
    ensuring they are wrapped in a domain-specific, structured exception type to
    prevent silent failures or segmentation faults in the Python runtime.
    """
    pass


class RustCore:
    """
    Highly optimized Python <-> Rust interface layer for the Thermognosis Engine.
    
    This class wraps the compiled `rust_core` PyO3 module, providing memory-safe
    and statically typed tensor abstractions. It handles C-contiguous memory
    assertions before crossing the FFI boundary to guarantee $\mathcal{O}(1)$ 
    bridging overhead.
    """

    def __init__(self, deterministic: bool = False):
        """
        Initializes the Rust Core backend connection.
        
        Args:
            deterministic (bool): If True, forces the Rust backend to use 
                strictly ordered sequential iterators instead of work-stealing 
                thread pools (Rayon), guaranteeing reproducible executions.
                
        Raises:
            RustCoreError: If the compiled `rust_core` shared library cannot be 
                located or imported in the current environment.
        """
        self.deterministic = deterministic
        try:
            # We import the compiled PyO3 cdylib module directly.
            # import rust_core
            import rust_core as backend
            self._backend = backend
            
            # Expose absolute physical bounds from the Rust core
            self.L0_SOMMERFELD = self._backend.L0_SOMMERFELD
            self.L_MIN = self._backend.L_MIN
            self.L_MAX = self._backend.L_MAX
            
            logger.info(f"Successfully loaded rust_core FFI backend. Deterministic mode: {self.deterministic}")
            
        except ImportError as e:
            logger.critical("Failed to load the compiled Rust FFI module 'rust_core'. "
                            "Ensure the crate is compiled and in the PYTHONPATH.")
            raise RustCoreError(f"Rust backend initialization failed: {e}") from e

    def _prepare_f64_array(self, tensor: Any, name: str) -> np.ndarray:
        """
        Memory Safety Guarantee: Forces the input into a 1D, C-contiguous, 
        float64 NumPy array to satisfy Rust's zero-copy slice extraction macro.
        
        Args:
            tensor (Any): The input scalar, list, or array.
            name (str): Identifier for logging/error context.
            
        Returns:
            np.ndarray: A contiguous np.float64 array.
            
        Raises:
            RustCoreError: If the tensor cannot be safely cast to float64.
        """
        try:
            arr_1d = np.atleast_1d(tensor)
            return np.ascontiguousarray(arr_1d, dtype=np.float64)
        except Exception as e:
            raise RustCoreError(f"FFI Memory Preparation Violation for '{name}': {e}") from e

    def _prepare_bool_array(self, tensor: Any, name: str) -> np.ndarray:
        """
        Memory Safety Guarantee: Forces the input into a 1D, C-contiguous, 
        boolean NumPy array for logical masking in Rust.
        """
        try:
            arr_1d = np.atleast_1d(tensor)
            return np.ascontiguousarray(arr_1d, dtype=np.bool_)
        except Exception as e:
            raise RustCoreError(f"FFI Memory Preparation Violation for '{name}': {e}") from e

    def compute_zt_from_csv(self, path: Union[str, Path]) -> Dict[str, Any]:
        """
        Streams a CSV file directly through the Rust core to compute zT and aggregate benchmarks.
        Bypasses Python-side array preparation and the GIL for maximum performance.
        
        Args:
            path (Union[str, Path]): Filepath to the CSV dataset containing thermoelectric properties.
            
        Returns:
            Dict[str, Any]: Parsed statistics, aggregates, and benchmarks calculated natively in Rust.
            
        Raises:
            RustCoreError: If I/O operations, structural parsing, or numerical execution fails
                within the Rust backend.
        """
        try:
            # Assuming the PyO3 module is loaded as self._backend
            # Convert path to strictly a string, guaranteeing compatibility across environments.
            return self._backend.compute_zt_from_csv_py(str(path), self.deterministic)
        except Exception as e:
            raise RustCoreError(f"Rust core failed during CSV processing: {str(e)}") from e

    def validate_dimensions(self, 
                            values: Union[float, np.ndarray], 
                            uncertainties: Union[float, np.ndarray], 
                            source_unit: str, 
                            target_unit: str) -> Tuple[np.ndarray, np.ndarray]:
        """
        Dimensionally validates and converts macroscopic property arrays.
        
        Document IDs: SPEC-UNIT-CONVERTER, SPEC-UNIT-DIM-VALIDATION
        
        Args:
            values (Union[float, np.ndarray]): Measured values to convert.
            uncertainties (Union[float, np.ndarray]): Corresponding 1-sigma uncertainties.
            source_unit (str): Unit string identifier (e.g., "m", "K", "degC").
            target_unit (str): Target unit string identifier.
            
        Returns:
            Tuple[np.ndarray, np.ndarray]: 
                - Converted values
                - Linearly propagated converted uncertainties
                
        Raises:
            RustCoreError: On dimension mismatch, unknown units, or FFI errors.
        """
        v_arr = self._prepare_f64_array(values, "values")
        u_arr = self._prepare_f64_array(uncertainties, "uncertainties")
        
        try:
            out_vals, out_uncs = self._backend.validate_dimensions_py(
                v_arr, u_arr, source_unit, target_unit, self.deterministic
            )
            return out_vals, out_uncs
        except (ValueError, RuntimeError) as e:
            raise RustCoreError(f"Dimensionality validation failed: {e}") from e

    def check_physics_consistency(self, 
                                  s: Union[float, np.ndarray], 
                                  sigma: Union[float, np.ndarray], 
                                  kappa: Union[float, np.ndarray], 
                                  t: Union[float, np.ndarray]) -> np.ndarray:
        """
        Validates thermodynamic constraints across state parameters.
        
        Document IDs: SPEC-PHYS-CONSISTENCY, SPEC-PHYS-CONSTRAINTS
        
        Args:
            s (Union[float, np.ndarray]): Seebeck coefficient (S) in V/K.
            sigma (Union[float, np.ndarray]): Electrical conductivity in S/m.
            kappa (Union[float, np.ndarray]): Thermal conductivity in W/(m·K).
            t (Union[float, np.ndarray]): Absolute Temperature (T) in K.
            
        Returns:
            np.ndarray: The computed dimensionless figure of merit (zT).
            
        Raises:
            RustCoreError: On thermodynamic violation (e.g., negative T or kappa).
        """
        s_arr = self._prepare_f64_array(s, "s")
        sigma_arr = self._prepare_f64_array(sigma, "sigma")
        kappa_arr = self._prepare_f64_array(kappa, "kappa")
        t_arr = self._prepare_f64_array(t, "t")
        
        try:
            zt_out = self._backend.check_physics_consistency_py(
                s_arr, sigma_arr, kappa_arr, t_arr, self.deterministic
            )
            return zt_out
        except (ValueError, RuntimeError) as e:
            raise RustCoreError(f"Physical consistency violation detected: {e}") from e

    def propagate_error(self, 
                        s: Union[float, np.ndarray], 
                        sigma: Union[float, np.ndarray], 
                        kappa: Union[float, np.ndarray], 
                        t: Union[float, np.ndarray],
                        err_s: Union[float, np.ndarray], 
                        err_sigma: Union[float, np.ndarray], 
                        err_kappa: Union[float, np.ndarray], 
                        err_t: Union[float, np.ndarray]) -> Tuple[np.ndarray, np.ndarray]:
        """
        Computes zT and its first-order analytical propagation of standard measurement uncertainties.
        
        Document IDs: P02-ZT-ERROR-PROPAGATION, T03-UNCERTAINTY-PROPAGATION
        
        Args:
            s, sigma, kappa, t: Nominal physical parameters.
            err_s, err_sigma, err_kappa, err_t: Corresponding 1-sigma uncertainties.
            
        Returns:
            Tuple[np.ndarray, np.ndarray]: 
                - zT: Expected value of the Figure of Merit.
                - uncertainty: 1-sigma analytical uncertainty of zT.
                
        Raises:
            RustCoreError: If negative variance occurs or dimensions mismatch.
        """
        s_arr = self._prepare_f64_array(s, "s")
        sigma_arr = self._prepare_f64_array(sigma, "sigma")
        kappa_arr = self._prepare_f64_array(kappa, "kappa")
        t_arr = self._prepare_f64_array(t, "t")
        
        es_arr = self._prepare_f64_array(err_s, "err_s")
        esigma_arr = self._prepare_f64_array(err_sigma, "err_sigma")
        ekappa_arr = self._prepare_f64_array(err_kappa, "err_kappa")
        et_arr = self._prepare_f64_array(err_t, "err_t")
        
        try:
            out_zt, out_unc = self._backend.propagate_error_py(
                s_arr, sigma_arr, kappa_arr, t_arr,
                es_arr, esigma_arr, ekappa_arr, et_arr,
                self.deterministic
            )
            return out_zt, out_unc
        except (ValueError, RuntimeError) as e:
            raise RustCoreError(f"Uncertainty propagation failed: {e}") from e

    def compute_quality_score(self, 
                              metrics: Dict[str, Union[float, np.ndarray]], 
                              lambda_reg: float = 0.01) -> Tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
        """
        Evaluates epistemological bounds to assign an authoritative Quality Class.
        
        Document IDs: SPEC-QUAL-SCORING, SPEC-QUAL-CREDIBILITY, SPEC-QUAL-COMPLETENESS
        
        Args:
            metrics (Dict[str, Union[float, np.ndarray]]): Dictionary containing exactly:
                'completeness', 'credibility', 'physics_consistency', 'error_magnitude',
                'smoothness', 'metadata', 'hard_constraint_gate' (boolean array).
            lambda_reg (float): Regularization penalty mapping parameter. Default is 0.01.
            
        Returns:
            Tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
                - base_score: Pre-regularization raw score
                - regularized_score: Entropy-penalized score bounded [0, 1]
                - entropy: Information entropy vector
                - class_labels: Discrete integer vector mapping to QualityClass levels.
                
        Raises:
            RustCoreError: If required keys are missing or computational instability occurs.
        """
        required_keys = {
            'completeness', 'credibility', 'physics_consistency', 
            'error_magnitude', 'smoothness', 'metadata', 'hard_constraint_gate'
        }
        
        if not required_keys.issubset(metrics.keys()):
            missing = required_keys - set(metrics.keys())
            raise RustCoreError(f"Missing required metrics for quality scoring: {missing}")

        c_arr = self._prepare_f64_array(metrics['completeness'], "completeness")
        cr_arr = self._prepare_f64_array(metrics['credibility'], "credibility")
        ph_arr = self._prepare_f64_array(metrics['physics_consistency'], "physics_consistency")
        err_arr = self._prepare_f64_array(metrics['error_magnitude'], "error_magnitude")
        sm_arr = self._prepare_f64_array(metrics['smoothness'], "smoothness")
        meta_arr = self._prepare_f64_array(metrics['metadata'], "metadata")
        hg_arr = self._prepare_bool_array(metrics['hard_constraint_gate'], "hard_constraint_gate")
        
        try:
            base, reg, ent, cls_labels = self._backend.compute_quality_score_py(
                c_arr, cr_arr, ph_arr, err_arr, sm_arr, meta_arr, hg_arr,
                float(lambda_reg), self.deterministic
            )
            return base, reg, ent, cls_labels
        except (ValueError, RuntimeError) as e:
            raise RustCoreError(f"Quality evaluation failed: {e}") from e

    def audit_thermodynamic_states(
        self,
        s: Union[float, np.ndarray],
        sigma: Union[float, np.ndarray],
        kappa: Union[float, np.ndarray],
        t: Union[float, np.ndarray],
        zt_reported: Optional[Union[float, np.ndarray]] = None,
    ) -> Dict[str, np.ndarray]:
        """
        Executes the Triple-Gate Epistemic Physics Audit over a thermodynamic dataset.

        Assigns a ``ConfidenceTier`` (A, B, C, Reject) and an ``AnomalyFlags``
        bitmask to every state by passing through three strictly ordered physical
        gates without silent data dropping.

        **Gate 1 — Algebraic Bounds**: Enforces T > 0, σ > 0, κ > 0 (finite).
        **Gate 2 — Wiedemann–Franz Consistency**: Validates the Lorenz number
          L = κ/(σT) ∈ [L_MIN, L_MAX] and the lattice residual κ_lattice ≥ 0.
        **Gate 3 — zT Cross-Validation**: Flags |zT_computed − zT_reported| / |zT_reported| > 10%.

        Document ID: SPEC-AUDIT-01

        Parameters
        ----------
        s : array-like
            Seebeck coefficient (V/K).
        sigma : array-like
            Electrical conductivity (S/m).
        kappa : array-like
            Total thermal conductivity (W m⁻¹ K⁻¹).
        t : array-like
            Absolute temperature (K).
        zt_reported : array-like or None, optional
            Externally reported dimensionless figure of merit (zT).
            Pass ``None`` or omit to skip Gate 3 cross-validation (array filled with NaN).

        Returns
        -------
        dict
            A dictionary of six flat NumPy arrays (all length N), suitable for
            direct ingestion into a Pandas DataFrame or Parquet writer:

            - ``"tier"`` (uint8): Ordinal confidence tier.
              ``1`` = TierA, ``2`` = TierB, ``3`` = TierC, ``4`` = Reject.
            - ``"anomaly_flags"`` (uint32): Bitmask of detected anomaly flags.
            - ``"zT_computed"`` (float64): Engine-computed figure of merit.
            - ``"kappa_lattice"`` (float64): Residual lattice thermal conductivity.
            - ``"lorenz_number"`` (float64): Effective Lorenz number L = κ/(σT).
            - ``"zT_cross_check_error"`` (float64): Relative zT deviation (NaN where
              no reliable reported value was available).

        Raises
        ------
        RustCoreError
            On dimension mismatch, FFI failure, or invalid input types.

        Examples
        --------
        >>> rc = RustCore()
        >>> audit = rc.audit_thermodynamic_states(s, sigma, kappa, T, zt_reported=ZT_lit)
        >>> tier_a_mask = audit["tier"] == 1
        >>> df = pd.DataFrame(audit)
        """
        s_arr     = self._prepare_f64_array(s,     "s")
        sigma_arr = self._prepare_f64_array(sigma, "sigma")
        kappa_arr = self._prepare_f64_array(kappa, "kappa")
        t_arr     = self._prepare_f64_array(t,     "t")

        # When no reported values are provided, Gate 3 is skipped via NaN sentinel.
        if zt_reported is None:
            zt_arr = np.full(s_arr.shape[0], np.nan, dtype=np.float64)
        else:
            zt_arr = self._prepare_f64_array(zt_reported, "zt_reported")

        try:
            raw: Dict[str, np.ndarray] = self._backend.audit_thermodynamics_py(
                s_arr, sigma_arr, kappa_arr, t_arr, zt_arr,
                self.deterministic,
            )
        except (ValueError, RuntimeError) as e:
            raise RustCoreError(f"Triple-Gate physics audit failed: {e}") from e

        # Remap keys to the canonical Q1 schema column names and return.
        return {
            "tier":                raw["tiers"],
            "anomaly_flags":       raw["anomaly_flags"],
            "zT_computed":         raw["zT_computed"],
            "kappa_lattice":       raw["kappa_lattice"],
            "lorenz_number":       raw["lorenz_number"],
            "zT_cross_check_error": raw["cross_check_error"],
        }