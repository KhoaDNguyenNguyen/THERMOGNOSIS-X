"""
Thermognosis Engine: Active Learning Gap Detection Pipeline
===========================================================
Document IDs: SPEC-ACTIVE-GAP, CL02-INFORMATION-GAIN-SELECTION, SPEC-GOV-CODE-GENERATION-PROTOCOL
Layer: spec/09_pipeline/active_learning
Status: Normative — Strict Mathematical Execution Environment

This module isolates spatial exploration entropy and topological data gaps 
within empirical thermoelectric measurement regimes. It prepares flat, 
C-contiguous memory tensors for zero-copy FFI execution in the Rust backend.
"""

import logging
import numpy as np
from typing import Any, List, Dict, Tuple
from sqlalchemy.exc import SQLAlchemyError

# Configure module-level logger
logger = logging.getLogger(__name__)


class GapDetectionError(Exception):
    """
    Base exception for failures within the Active Learning Gap Detection pipeline.
    
    Implements: SPEC-GOV-ERROR-HIERARCHY
    Ensures mathematical and structural failures do not pass silently.
    """
    pass


class DatabaseExtractionError(GapDetectionError):
    """Raised when the database yields malformed, empty, or inaccessible datasets."""
    pass


class FFISynchronizationError(GapDetectionError):
    """Raised when the Rust backend fails to execute the entropic evaluation."""
    pass


class GapDetector:
    """
    Executes the active learning gap detection pipeline by extracting thermodynamic
    data, structuring it into memory-contiguous sub-manifolds, and evaluating
    information gain via Rust-accelerated en masse statistical inference.
    
    Implements: SPEC-ACTIVE-GAP
    """

    def __init__(self, db_connection: Any, rust_core: Any):
        """
        Initializes the GapDetector with required external dependencies.
        
        Parameters
        ----------
        db_connection : Any
            Active SQLAlchemy database connection or session manager.
        rust_core : Any
            The Python <-> Rust FFI boundary wrapper.
        """
        self._db = db_connection
        self._rust_core = rust_core
        logger.info("GapDetector initialized. Ready for strict mathematical execution.")

    def _extract_and_structure_data(self) -> Tuple[np.ndarray, List[Tuple[int, int]], List[str]]:
        """
        Queries the relational store and formats the macroscopic temperature states
        into a flat, zero-copy-ready array.
        
        Returns
        -------
        Tuple[np.ndarray, List[Tuple[int, int]], List[str]]
            - t_array: 1D C-contiguous float64 array of Temperatures (T).
            - bounds: List of (start_idx, end_idx) mapping materials to array slices.
            - material_ids: List of unique material identifiers matching `bounds` order.
            
        Raises
        ------
        DatabaseExtractionError
            If querying the database fails or no data is found.
        """
        query = """
            SELECT material_id, temperature 
            FROM measurements 
            WHERE temperature IS NOT NULL 
            ORDER BY material_id ASC;
        """
        try:
            # Assuming an SQLAlchemy-like execute/fetchall interface
            result_proxy = self._db.execute(query)
            rows = result_proxy.fetchall()
        except SQLAlchemyError as e:
            raise DatabaseExtractionError(f"Failed to extract thermodynamic state data: {e}") from e

        if not rows:
            raise DatabaseExtractionError("Zero measurements found. Vacuous state space.")

        t_list = []
        bounds = []
        material_ids = []
        
        current_mat = None
        start_idx = 0
        
        # O(N) single-pass continuous memory allocation strategy
        for i, row in enumerate(rows):
            mat_id = row[0]
            t_val = row[1]
            
            t_list.append(t_val)
            
            if mat_id != current_mat:
                if current_mat is not None:
                    bounds.append((start_idx, i))
                    material_ids.append(current_mat)
                current_mat = mat_id
                start_idx = i
                
        # Terminal boundary resolution
        if current_mat is not None:
            bounds.append((start_idx, len(rows)))
            material_ids.append(current_mat)

        # Force strict C-contiguous 64-bit float memory layout for zero-copy FFI
        t_array = np.ascontiguousarray(t_list, dtype=np.float64)
        
        logger.debug(f"Structured {len(t_array)} states across {len(bounds)} sub-manifolds.")
        return t_array, bounds, material_ids

    def detect_and_rank_gaps(
        self, 
        t_min: float = 300.0, 
        t_max: float = 1200.0, 
        num_bins: int = 10, 
        gamma_1: float = 1.0, 
        gamma_2: float = 1.0
    ) -> List[Dict[str, Any]]:
        """
        Executes the spatial exploration entropy evaluation pipeline.
        
        Mathematical Formalization:
            1. $p_k = n_k / \sum n_j$
            2. $H = - \sum_k p_k \ln(p_k)$
            3. $D_{KL}(P \parallel U) = \sum_k p_k \ln(p_k / u_k)$ where $u_k = 1 / K$
            4. $G = \gamma_1 H + \gamma_2 D_{KL}$
            
        Parameters
        ----------
        t_min : float
            Lower physical bound of the temperature integration domain (K).
        t_max : float
            Upper physical bound of the temperature integration domain (K).
        num_bins : int
            Number of discrete microstates ($K$) for the uniform prior $U$.
        gamma_1 : float
            Regularization weight for Shannon Entropy ($H$).
        gamma_2 : float
            Regularization weight for the Kullback-Leibler Divergence ($D_{KL}$).
            
        Returns
        -------
        List[Dict[str, Any]]
            A rigorously ranked list of dictionaries mapping material identifiers
            to their specific information gain functional evaluations. 
            Sorted strictly by `gap_score` descending.
            
        Raises
        ------
        GapDetectionError
            For constraint violations and propagated FFI execution failures.
        """
        logger.info(
            f"Initiating Gap Detection over [{t_min}K, {t_max}K] "
            f"with K={num_bins}, γ1={gamma_1}, γ2={gamma_2}."
        )

        # 1. Structural DB Query and Contiguous Memory Preparation
        t_array, bounds, material_ids = self._extract_and_structure_data()

        # 2. Rust FFI Bridging (Zero-Copy)
        try:
            # Invokes core mathematical evaluations concurrently across sub-manifolds
            ffi_results = self._rust_core.compute_information_gain_batch(
                t_array, bounds, t_min, t_max, num_bins, gamma_1, gamma_2
            )
        except Exception as e:
            logger.error("Catastrophic failure at the Rust FFI boundary.")
            raise FFISynchronizationError(f"Rust core failed to compute information gain: {e}") from e

        if len(ffi_results) != len(material_ids):
            raise FFISynchronizationError("FFI boundary dimension mismatch: Bounds length != Results length.")

        # 3. Post-Processing & Strict Ranking
        ranked_gaps = []
        for mat_id, result in zip(material_ids, ffi_results):
            # Adaptively extract properties whether FFI returns dicts or PyO3 objects
            entropy = getattr(result, 'entropy', result.get('entropy', 0.0)) if isinstance(result, dict) else result.entropy
            kl_div = getattr(result, 'kl_divergence', result.get('kl_divergence', 0.0)) if isinstance(result, dict) else result.kl_divergence
            g_score = getattr(result, 'total_score', result.get('total_score', 0.0)) if isinstance(result, dict) else result.total_score
            
            ranked_gaps.append({
                "material_id": mat_id,
                "entropy": float(entropy),
                "kl_divergence": float(kl_div),
                "gap_score": float(g_score)
            })

        # Sort descending to prioritize maximal thermodynamic ignorance (highest gap score)
        ranked_gaps.sort(key=lambda x: x["gap_score"], reverse=True)
        
        top_mat = ranked_gaps[0]["material_id"] if ranked_gaps else "None"
        logger.info(f"Gap detection complete. Highest priority target: {top_mat}.")

        return ranked_gaps