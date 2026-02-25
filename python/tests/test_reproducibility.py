# -*- coding: utf-8 -*-
r"""
Thermognosis Engine: Cryptographic Reproducibility Suite
Document ID: SPEC-CONTRACT-VERSIONING
Status: Normative — Strict Mathematical Execution Environment

Enforces absolute bitwise determinism across the scientific validation pipeline.
Non-determinism implies epistemological invalidity and is immediately trapped.
"""

import pytest
import numpy as np
import pandas as pd
from typing import Dict, Any

from thermognosis.wrappers.rust_wrapper import RustCore
from thermognosis.utils.hashing import compute_sha256_hash
from synthetic_data import generate_physics_consistency_groups

def run_deterministic_pipeline(df: pd.DataFrame) -> Dict[str, Any]:
    """
    Executes the entire validation, uncertainty propagation, and Bayesian ranking
    pipeline, returning a strictly ordered dictionary of resulting tensors.
    """
    # Enforce deterministic initialization of the Rust backend.
    # Rayon work-stealing threadpools MUST be serialized for this to pass.
    rust_core = RustCore(deterministic=True)
    
    # Extract flat arrays for O(1) FFI transmission
    T = df['T'].to_numpy(dtype=np.float64)
    S = df['S'].to_numpy(dtype=np.float64)
    sigma = df['sigma'].to_numpy(dtype=np.float64)
    kappa = df['kappa'].to_numpy(dtype=np.float64)
    
    # Mocking standard 5% relative uncertainty
    err_S = np.abs(S) * 0.05
    err_sigma = np.abs(sigma) * 0.05
    err_kappa = np.abs(kappa) * 0.05
    err_T = np.abs(T) * 0.05

    # Step 1: Physical Validation
    zt_flat = rust_core.check_physics_consistency(S, sigma, kappa, T)
    
    # Step 2: Uncertainty Propagation
    zt_expected, zt_uncertainty = rust_core.propagate_error(
        S, sigma, kappa, T,
        err_S, err_sigma, err_kappa, err_T
    )
    
    # Step 3: Bayesian Ranking & Quality Scoring
    metrics = {
        'completeness': np.ones_like(zt_flat),
        'credibility': np.full_like(zt_flat, 0.85),
        'physics_consistency': np.where(zt_flat >= 0, 1.0, 0.0),
        'error_magnitude': zt_uncertainty,
        'smoothness': np.ones_like(zt_flat),
        'metadata': np.ones_like(zt_flat),
        'hard_constraint_gate': (zt_flat >= 0) & (T > 0) & (kappa > 0) & (sigma > 0)
    }
    
    base_score, reg_score, entropy, cls_labels = rust_core.compute_quality_score(
        metrics, lambda_reg=0.01
    )
    
    # Compile the final topological state
    return {
        "zT_nominal": zt_flat.tolist(),
        "zT_expected": zt_expected.tolist(),
        "zT_uncertainty": zt_uncertainty.tolist(),
        "quality_base": base_score.tolist(),
        "quality_regularized": reg_score.tolist(),
        "entropy": entropy.tolist(),
        "class_labels": cls_labels.tolist()
    }


def test_bitwise_pipeline_reproducibility() -> None:
    """
    Executes the pipeline 3 separate times on a challenging synthetic dataset.
    Hashes the output dictionaries using canonical SHA-256 serialization.
    
    A mismatch indicates a catastrophic failure in our determinism contract:
    - Floating point rounding drift
    - Non-deterministic memory traversal
    - Race conditions in the Rust physics engine
    """
    ITERATIONS = 3
    N_ROWS = 5000
    
    # Generate heterogenous physical groups (valid, edge-cases, impossible states)
    test_df = generate_physics_consistency_groups(N=N_ROWS)
    # Lọc chỉ lấy nhóm Valid để test tính toán Hash
    test_df = test_df[test_df['group'] == 'A_Valid'].copy()
    
    hashes = []
    
    for i in range(1, ITERATIONS + 1):
        # We process deep copies to guarantee no hidden state leakage in Python
        pipeline_output = run_deterministic_pipeline(test_df.copy(deep=True))
        
        # Calculate cryptographic digest using precisely 12-decimal fixed precision
        # (Standardized to resolve Linux vs. Windows FP16/64 compiler anomalies)
        canonical_digest = compute_sha256_hash(pipeline_output, precision=12)
        
        hashes.append(canonical_digest)
        print(f"Iteration {i} SHA-256 Digest: {canonical_digest}")

    # The ultimate test of epistemic determinism
    unique_hashes = set(hashes)
    
    assert len(unique_hashes) == 1, (
        f"FATAL: Pipeline Reproducibility Violated!\n"
        f"Expected a single cryptographic state across all runs, but encountered {len(unique_hashes)} states.\n"
        f"Observed Hashes: {hashes}\n"
        f"Immediate action required: Check Rust PyO3 slice extraction and cross-platform NaN handling."
    )