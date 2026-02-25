# -*- coding: utf-8 -*-
r"""
Thermognosis Engine: Thermodynamic Stability & Boundary Tests
Document ID: SPEC-GOV-CODE-GENERATION-PROTOCOL

Executes stress tests against structural bounds, the Wiedemann-Franz limits, 
and Bayesian state regulation. Prevents zero-division segfaults and strictly
ranks materials based on physical validity.
"""

import numpy as np
import pytest
from sklearn.metrics import roc_auc_score
from typing import Dict, Any
import synthetic_data
from thermognosis.wrappers.rust_wrapper import RustCore, RustCoreError




def build_rigorous_metrics(df) -> Dict[str, Any]:
    """
    Translates raw physical state parameters into rigorous epistemic constraints 
    required by the Rust `compute_quality_score` layer.
    """
    T = df['T'].values.astype(np.float64)
    kappa = df['kappa'].values.astype(np.float64)
    sigma = df['sigma'].values.astype(np.float64)
    
    # 1. Hard Constraints: Rejects mathematically impossible macroscopic states
    hard_gate = (T > 0.0) & (kappa > 0.0) & (sigma > 0.0)
    
    # 2. Wiedemann-Franz Epistemic Bound (L_MIN)
    # The lattice thermal conductivity must mathematically be positive, 
    # meaning total kappa cannot drop below the electronic minimum.
    L_MIN = 1.00e-8
    kappa_e_min = L_MIN * sigma * T
    
    # Calculate degree of violation (Exponential penalty for violating WF limit)
    deviation = np.maximum(0.0, kappa_e_min - kappa)
    phys_score = np.exp(-deviation * 1e8) # 1e8 scale normalizes the Sommerfeld dimensionality
    
    # Wipe out scores for hard thermodynamic violations
    phys_score = np.where(hard_gate, phys_score, 0.0)
    
    N = len(df)
    return {
        'completeness': np.ones(N, dtype=np.float64),
        'credibility': np.ones(N, dtype=np.float64),
        'physics_consistency': np.ascontiguousarray(phys_score, dtype=np.float64),
        'error_magnitude': np.full(N, 0.05, dtype=np.float64),
        'smoothness': np.ones(N, dtype=np.float64),
        'metadata': np.ones(N, dtype=np.float64),
        'hard_constraint_gate': np.ascontiguousarray(hard_gate, dtype=np.bool_)
    }


def test_bayesian_quality_ranking_stability() -> None:
    """
    STRESS TEST: Epistemic Validation and Quality Scoring.
    
    Proves that the Regularized Bayesian Posterior accurately ranks physical states
    while strictly penalizing thermodynamic limit violations.
    """
    engine = RustCore(deterministic=True)
    df = synthetic_data.generate_physics_consistency_groups(N=1500)
    
    # Build strict zero-copy memory layouts
    metrics = build_rigorous_metrics(df)
    
    # Push through the Rust FFI Regularizer
    _, reg_score, _, _ = engine.compute_quality_score(metrics, lambda_reg=0.01)
    df['posterior'] = reg_score
    
    # Group extraction
    mean_valid = df[df['group'] == 'A_Valid']['posterior'].mean()
    mean_light = df[df['group'] == 'B_Light_Violation']['posterior'].mean()
    mean_heavy = df[df['group'] == 'C_Heavy_Violation']['posterior'].mean()
    
    # ---------------------------------------------------------
    # ASSERT MONOTONIC POSTERIOR DECAY
    # ---------------------------------------------------------
    assert mean_valid > mean_light, f"Ranking Violation: Valid ({mean_valid}) <= Light Viol ({mean_light})"
    assert mean_light > mean_heavy, f"Ranking Violation: Light Viol ({mean_light}) <= Heavy Viol ({mean_heavy})"
    
    # ---------------------------------------------------------
    # ASSERT CLASSIFICATION DISCRIMINATIVE POWER
    # ---------------------------------------------------------
    mask_AC = df['group'].isin(['A_Valid', 'C_Heavy_Violation'])
    df_AC = df[mask_AC]
    
    y_true = (df_AC['group'] == 'A_Valid').astype(int).values
    y_scores = df_AC['posterior'].values
    
    auc = roc_auc_score(y_true, y_scores)
    assert auc > 0.95, f"CRITICAL: Bayesian regularizer failed to segregate true physical models. ROC AUC: {auc}"


def test_ffi_boundary_exception_promotion() -> None:
    """
    STRESS TEST: Null Space & Mathematical Singularities.
    
    Forces zero-division (kappa=0) and sub-zero temperatures across the FFI.
    It MUST fail loudly, actively preventing the C-runtime from segfaulting.
    Document ID: SPEC-GOV-ERROR-HIERARCHY
    """
    engine = RustCore(deterministic=True)
    df_limits = synthetic_data.generate_physical_limits()
    
    with pytest.raises(RustCoreError) as excinfo:
        engine.check_physics_consistency(
            df_limits['S'].values,
            df_limits['sigma'].values,
            df_limits['kappa'].values,
            df_limits['T'].values
        )
    
    # Validate structured exception promotion
    assert "violation" in str(excinfo.value).lower() or "failed" in str(excinfo.value).lower(), \
        "FFI Exception lacked strict, structured domain context."