#!/usr/bin/env python3
# -*- coding: utf-8 -*-
r"""
Thermognosis Engine: Journal Q1 Reproducibility and Validation Suite
Document ID: SPEC-GOV-CODE-GENERATION-PROTOCOL
Target: Reproducibility, Boundary Testing, and Epistemic Validation

This script executes the normative boundaries, mathematical limits, and 
statistical edge-cases of the Thermognosis Engine. It is dual-purposed:
1. Fully compatible with `pytest` for CI/CD pipelines.
2. Executable as a standalone script to generate the Markdown/LaTeX
   Supplementary Material tables for academic submission.

Author: Distinguished Professor of Computational Materials Science
Institution: Thermognosis Engine Consortium
"""

import sys
import numpy as np
import pytest
from typing import Callable, List, Tuple

# =============================================================================
# RIGOROUS MATHEMATICAL REFERENCE ENGINE
# =============================================================================

class ReferenceThermognosisEngine:
    """
    Pure Python Reference Implementation of the Thermognosis Rust Core.
    Guarantees strict floating point determinism and mathematical bounds 
    matching the FFI layer for standalone reproducibility verification.
    """
    L0_SOMMERFELD = 2.44e-8  # W*Ohm/K^2
    L_MIN = 1.00e-8
    L_MAX = 4.00e-8
    SEED = 424242

    @classmethod
    def compute_zt(cls, S: np.ndarray, sigma: np.ndarray, kappa: np.ndarray, T: np.ndarray) -> np.ndarray:
        """Computes the dimensionless figure of merit with strict exception promotion."""
        if np.any(kappa <= 0.0):
            raise ValueError("Strict exception promotion: Thermodynamic violation (kappa <= 0.0)")
        if np.any(T < 0.0):
            raise ValueError("Strict exception promotion: Thermodynamic violation (T < 0.0)")
        return (S**2 * sigma * T) / kappa

    @classmethod
    def compute_quality_score(cls, S: np.ndarray, sigma: np.ndarray, kappa: np.ndarray, T: np.ndarray) -> np.ndarray:
        """Evaluates epistemological bounds, actively penalizing unphysical states."""
        zT = cls.compute_zt(S, sigma, kappa, T)
        L = kappa / (sigma * T)
        # Wiedemann-Franz Gate Penalty
        wf_penalty = np.where((L >= cls.L_MIN) & (L <= cls.L_MAX), 1.0, 1e-6)
        return zT * wf_penalty

    @classmethod
    def compute_lse_probabilities(cls, x: np.ndarray) -> np.ndarray:
        """LogSumExp stabilization to prevent catastrophic overflow during Bayesian ranking."""
        x_max = np.max(x)
        exp_x = np.exp(x - x_max)
        return exp_x / np.sum(exp_x)


# =============================================================================
# VALIDATION ASSERTIONS (PYTEST COMPATIBLE)
# =============================================================================

def test_limit_t_to_zero_plus():
    """
    | Limit T->0+ | T = 1e-12 | No NaN/Inf, zT ~ 0 |
    """
    eps = np.nextafter(0, 1, dtype=np.float64)
    zT = ReferenceThermognosisEngine.compute_zt(
        np.array([1e-4]), np.array([1e5]), np.array([1.5]), np.array([eps])
    )
    val = zT[0]
    
    assert not np.isnan(val), "Result is NaN"
    assert not np.isinf(val), "Result is Inf"
    assert val < 1e-10, "Mathematical Bound Failed: Result not asymptotically zero"
    return f"{val:.4e}"

def test_kappa_zero_exception():
    """
    | Division by Zero | kappa = 0.0 | Raises ValueError |
    """
    raised = False
    try:
        ReferenceThermognosisEngine.compute_zt(
            np.array([1e-4]), np.array([1e5]), np.array([0.0]), np.array([300.0])
        )
    except ValueError:
        raised = True
        
    assert raised, "Strict exception promotion failed: zero-division allowed."
    return "Raised ValueError"

def test_negative_temperature_rejection():
    """
    | Thermodynamics | T = -50.0 K | Exception Promoted |
    """
    raised = False
    try:
        ReferenceThermognosisEngine.compute_zt(
            np.array([1e-4]), np.array([1e5]), np.array([1.5]), np.array([-50.0])
        )
    except ValueError:
        raised = True
        
    assert raised, "Negative absolute temperature did not trigger strict exception."
    return "Raised ValueError"

def test_lse_stability():
    """
    | LSE Stability | N=10^6 large values | Sum(P) == 1.0 |
    """
    rng = np.random.default_rng(ReferenceThermognosisEngine.SEED)
    # 1 million massive exponential arguments that would typically cause float64 overflow
    x = rng.normal(loc=1e4, scale=1e2, size=1_000_000)
    P = ReferenceThermognosisEngine.compute_lse_probabilities(x)
    total_p = np.sum(P)
    
    assert np.isclose(total_p, 1.0, rtol=1e-9), "Bayesian Marginalization Overflow: Probabilities do not sum to 1.0"
    return f"Sum(P) = {total_p:.12f}"

def test_wf_penalty():
    """
    | WF Penalty | kappa < L_MIN*sigma*T | P(invalid) < P(valid) |
    """
    L0 = ReferenceThermognosisEngine.L0_SOMMERFELD
    # Material A: Valid thermodynamics
    kappa_valid = L0 * 1e5 * 300.0
    score_valid = ReferenceThermognosisEngine.compute_quality_score(
        np.array([1e-4]), np.array([1e5]), np.array([kappa_valid]), np.array([300.0])
    )[0]
    
    # Material B: Physically impossible conductivity ratio
    kappa_invalid = 0.1 * ReferenceThermognosisEngine.L_MIN * 1e5 * 300.0
    score_invalid = ReferenceThermognosisEngine.compute_quality_score(
        np.array([1e-4]), np.array([1e5]), np.array([kappa_invalid]), np.array([300.0])
    )[0]
    
    assert score_invalid < score_valid, "WF Penalty failed: Unphysical state was not deprioritized."
    return f"Inv: {score_invalid:.3f} < Val: {score_valid:.3f}"

def test_ranking_consensus():
    """
    | Ranking Consensus | Bi2Te3 in Top 10 | True |
    """
    rng = np.random.default_rng(ReferenceThermognosisEngine.SEED)
    N_noise = 1000
    
    # Generate background noise (zT < 0.1)
    S_noise = rng.uniform(10e-6, 50e-6, N_noise)
    sigma_noise = rng.uniform(1e3, 1e4, N_noise)
    kappa_noise = rng.uniform(5.0, 15.0, N_noise)
    T_noise = rng.uniform(300, 800, N_noise)
    
    # Famous Reference Materials
    # Bi2Te3: zT ~ 1.0, PbTe: zT ~ 1.5
    S_famous = np.array([200e-6, 300e-6])
    sigma_famous = np.array([1e5, 5e4])
    kappa_famous = np.array([1.2, 2.4])
    T_famous = np.array([300.0, 800.0])
    
    S_all = np.concatenate([S_noise, S_famous])
    sigma_all = np.concatenate([sigma_noise, sigma_famous])
    kappa_all = np.concatenate([kappa_noise, kappa_famous])
    T_all = np.concatenate([T_noise, T_famous])
    
    scores = ReferenceThermognosisEngine.compute_quality_score(S_all, sigma_all, kappa_all, T_all)
    top_indices = np.argsort(scores)[::-1]
    
    rank_1 = np.where(top_indices == N_noise)[0][0]
    rank_2 = np.where(top_indices == N_noise + 1)[0][0]
    
    assert rank_1 < 10 and rank_2 < 10, "Bayesian Ranking Failure: Famous materials lost in noise."
    return f"Bi2Te3: Rank {rank_1+1}, PbTe: Rank {rank_2+1}"


# =============================================================================
# Q1 SUPPLEMENTARY MATERIAL TABLE GENERATOR
# =============================================================================

def _extract_table_metadata(docstring: str) -> List[str]:
    """Parses the docstring metadata injected into test blocks."""
    if not docstring:
        return ["Unknown", "Unknown", "Unknown"]
    lines = [line.strip() for line in docstring.split('\n') if '|' in line]
    if lines:
        return [p.strip() for p in lines[0].split('|') if p.strip()]
    return ["Unknown", "Unknown", "Unknown"]

def generate_validation_table():
    """Executes the test suite and directly formats outputs to markdown layout."""
    tests = [
        test_limit_t_to_zero_plus,
        test_kappa_zero_exception,
        test_negative_temperature_rejection,
        test_lse_stability,
        test_wf_penalty,
        test_ranking_consensus
    ]
    
    results_table = []
    
    for t in tests:
        meta = _extract_table_metadata(t.__doc__)
        try:
            obs = t()
            results_table.append(meta + [obs, "Pass"])
        except AssertionError as e:
            results_table.append(meta + [f"FAIL: {e}", "Fail"])
        except Exception as e:
            results_table.append(meta + [f"CRASH: {e}", "Fail"])
            
    # Markdown Table Output
    print("\n### Table S1: Epistemic Validation and Mathematical Boundary Stress-Test\n")
    print("| Test Name | Condition | Expected Mathematical Behavior | Observed Result | Pass/Fail |")
    print("|:---|:---|:---|:---|:---|")
    for row in results_table:
        print(f"| {row[0]} | {row[1]} | {row[2]} | {row[3]} | {row[4]} |")
    print("\n")

if __name__ == "__main__":
    generate_validation_table()