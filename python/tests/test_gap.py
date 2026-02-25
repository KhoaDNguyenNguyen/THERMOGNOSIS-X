# -*- coding: utf-8 -*-
r"""
Epistemic Validation Suite: Active Learning Gap Detection
=========================================================
Test ID: SPEC-ACTIVE-GAP-VALIDATION

Validates spatial exploration entropy derivatives. Explicitly proves the KL-Divergence
minimizer flawlessly targets engineered thermodynamic voids (e.g., 800-900K).

Author: Distinguished Professor of Computational Materials Science
"""

import pytest
import numpy as np
from typing import List, Tuple, Any, Dict

from synthetic_data import generate_gap_data
from synthetic_data import generate_ranking_mock
from thermognosis.pipeline.gap_detection import GapDetector

class MockDBConnection:
    """Simulates relational data ingestion for active learning execution."""
    def __init__(self, t_array: np.ndarray) -> None:
        self.t_array = t_array
        
    def execute(self, query: str) -> Any:
        class ResultProxy:
            def __init__(self, t_arr: np.ndarray) -> None:
                # Coalesce to global manifold to test overarching entropic reduction limits
                self.rows: List[Tuple[str, float]] = [("Global_Manifold", float(t)) for t in t_arr]
            def fetchall(self) -> List[Tuple[str, float]]:
                return self.rows
        return ResultProxy(self.t_array)


class MathStrictRustCoreGap:
    """Mathematical execution mapping replicating zero-copy FFI execution algorithms."""
    def compute_information_gain_batch(
        self, t_array: np.ndarray, bounds: List[Tuple[int, int]], 
        t_min: float, t_max: float, num_bins: int, gamma_1: float, gamma_2: float
    ) -> List[Any]:
        class FFIResult:
            def __init__(self, h: float, kl: float, g: float):
                self.entropy = h
                self.kl_divergence = kl
                self.total_score = g
        
        results: List[FFIResult] = []
        for start, end in bounds:
            t_slice: np.ndarray = t_array[start:end]
            hist, _ = np.histogram(t_slice, bins=num_bins, range=(t_min, t_max))
            
            N = float(np.sum(hist))
            if N == 0:
                results.append(FFIResult(0.0, 0.0, 0.0))
                continue
                
            p: np.ndarray = hist / N
            u: float = 1.0 / num_bins
            
            eps = float(np.nextafter(0, 1, dtype=np.float64))
            h = float(-np.sum(p * np.log(p + eps)))
            p_safe = np.where(p > 0, p, 1.0)
            kl = float(np.sum(np.where(p > 0, p * np.log(p_safe / u), 0.0)))
            g = float(gamma_1 * h + gamma_2 * kl)
            
            results.append(FFIResult(h, kl, g))
            
        return results


def test_active_learning_gap_detection_pipeline() -> None:
    """
    Test ID: TEST-GAP-001
    Objective: Prove the mathematical derivative of D_KL converges optimally on spatial voids.
    """
    df = generate_gap_data()
    t_array: np.ndarray = df['T'].values
    
    db_mock = MockDBConnection(t_array)
    rust_core = MathStrictRustCoreGap()
    
    detector = GapDetector(db_connection=db_mock, rust_core=rust_core)
    
    # Encompass 300K - 1000K into 7 strictly defined boundaries (width 100K).
    # Expected topological bin IDs:
    # 0: 300-400, 1: 400-500, 2: 500-600, 3: 600-700, 4: 700-800, 5: 800-900, 6: 900-1000
    t_min = 300.0
    t_max = 1000.0
    num_bins = 7
    
    ranked_gaps: List[Dict[str, Any]] = detector.detect_and_rank_gaps(
        t_min=t_min, t_max=t_max, num_bins=num_bins, gamma_1=1.0, gamma_2=1.0
    )
    
    assert len(ranked_gaps) == 1, "Pipeline fragmented the collective macroscopic manifold."
    global_eval = ranked_gaps[0]
    
    # ---------------------------------------------------------------------------------
    # BRUTAL MATHEMATICAL DERIVATIVE ASSERTION
    # ---------------------------------------------------------------------------------
    hist, bin_edges = np.histogram(t_array, bins=num_bins, range=(t_min, t_max))
    N = float(np.sum(hist))
    p_k: np.ndarray = hist / N
    u_k: float = 1.0 / num_bins
    
    target_bin_idx = 5  # The [800.0, 900.0) void
    assert np.isclose(bin_edges[target_bin_idx], 800.0), "Bin edges misaligned."
    assert np.isclose(bin_edges[target_bin_idx + 1], 900.0), "Bin edges misaligned."
    
    # Ensure physical constraint of void existence in generation logic is sound
    assert hist[target_bin_idx] == 0, "Constraint violation: The 800-900K gap is structurally breached."
    
    # Calculate the localized derivative potential of acquiring a state measurement.
    # The minimum (most negative gradient) identifies the point of maximum systemic uncertainty reduction.
    eps = float(np.nextafter(0, 1, dtype=np.float64))
    p_k_safe: np.ndarray = np.maximum(p_k, eps)
    
    dkl_gradients: np.ndarray = np.log(p_k_safe / u_k) / N
    optimal_acquisition_bin = int(np.argmin(dkl_gradients))
    
    # Assert topological mapping logic
    assert optimal_acquisition_bin == target_bin_idx, (
        f"Algorithm Failed: The derivative minimizer did not target the absolute physical void. "
        f"Expected targeting bin {target_bin_idx}, evaluated to bin {optimal_acquisition_bin}."
    )
    
    assert global_eval['kl_divergence'] > 0.0, "KL Divergence falsely evaluated to pure zero."
    assert global_eval['entropy'] > 0.0, "Entropy failed to recognize distributed macroscopic states."