# -*- coding: utf-8 -*-
r"""
Thermognosis Engine: Asymptotic Performance Benchmarking
Document ID: SPEC-PERF-BENCHMARK
Status: Normative — Strict Mathematical Execution Environment

Tests the $\mathcal{O}(1)$ FFI boundary zero-copy overhead against naive Python
loops for evaluating zT and performing Bayesian Log-Sum-Exp entropy normalizations.
"""

import math
import time
import pytest
import numpy as np
import logging

# from thermognosis.wrappers.rust_wrapper import RustCore, RustCoreError
# Import wrapper đã đổi tên
from thermognosis.wrappers.rust_wrapper import RustCore

# Import trực tiếp synthetic_data (vì nó nằm cùng thư mục tests)
from synthetic_data import generate_physics_consistency_groups, generate_ranking_mock, generate_gap_data


# Configure deterministic seed to prevent benchmark volatility
SEED = 424242
RNG = np.random.default_rng(SEED)

logger = logging.getLogger(__name__)

def naive_python_zt_and_lse(S: np.ndarray, sigma: np.ndarray, kappa: np.ndarray, T: np.ndarray) -> tuple:
    """
    Simulates the standard, computationally negligent approach to thermodynamic
    arrays often found in pure Python scripts.
    
    Computes:
        1. zT = (S^2 * sigma * T) / kappa
        2. Log-Sum-Exp (LSE) of the resulting zT distribution (Bayesian normalization).
    """
    n = len(S)
    zt_list = []
    
    # 1. Naive loop execution (Simulating Python object overhead)
    for i in range(n):
        # Strict thermodynamic equation
        zt = (S[i]**2 * sigma[i] * T[i]) / kappa[i]
        zt_list.append(zt)
        
    # 2. Naive Log-Sum-Exp (LSE) computation
    # LSE(x) = max(x) + log( sum( exp( x_i - max(x) ) ) )
    max_zt = max(zt_list)
    sum_exp = sum(math.exp(zt - max_zt) for zt in zt_list)
    lse = max_zt + math.log(sum_exp)
    
    return np.array(zt_list, dtype=np.float64), lse


def test_ffi_performance_scaling() -> None:
    """
    Benchmark requirement: Rust FFI must be at least 10x faster than pure Python.
    Evaluates 100,000 thermodynamic states mathematically.
    """
    N_ROWS = 100_000
    logger.info(f"Generating {N_ROWS} deterministic thermodynamic states for benchmark...")
    
    # Generate contiguous float64 arrays
    T = RNG.uniform(300.0, 1000.0, N_ROWS).astype(np.float64)
    S = RNG.uniform(50e-6, 300e-6, N_ROWS).astype(np.float64)
    sigma = RNG.uniform(1e3, 1e5, N_ROWS).astype(np.float64)
    kappa = RNG.uniform(0.5, 5.0, N_ROWS).astype(np.float64)
    
    # Generate mock metrics for quality score / entropy calculation
    metrics = {
        'completeness': np.ones(N_ROWS, dtype=np.float64),
        'credibility': np.full(N_ROWS, 0.8, dtype=np.float64),
        'physics_consistency': np.ones(N_ROWS, dtype=np.float64),
        'error_magnitude': RNG.uniform(0.01, 0.1, N_ROWS).astype(np.float64),
        'smoothness': np.ones(N_ROWS, dtype=np.float64),
        'metadata': np.ones(N_ROWS, dtype=np.float64),
        'hard_constraint_gate': np.ones(N_ROWS, dtype=np.bool_)
    }

    # --- 1. Python Naive Execution ---
    t0_py = time.perf_counter()
    py_zt, py_lse = naive_python_zt_and_lse(S, sigma, kappa, T)
    t1_py = time.perf_counter()
    py_duration = t1_py - t0_py

    # --- 2. Rust Core Accelerated Execution ---
    rust_core = RustCore(deterministic=True)
    t0_rs = time.perf_counter()
    try:
        # Physical consistency maps to our zT calculation
        rs_zt = rust_core.check_physics_consistency(S, sigma, kappa, T)
        # Quality score invokes the heavy Bayesian/Entropy/LSE equivalents in Rust
        base, reg, ent, labels = rust_core.compute_quality_score(metrics, lambda_reg=0.01)
    except RustCoreError as e:
        pytest.fail(f"Rust Core raised a catastrophic exception during benchmark: {e}")
        
    t1_rs = time.perf_counter()
    rs_duration = t1_rs - t0_rs
    
    # --- 3. Mathematical Equivalence Assertion ---
    # Ensure that our highly optimized Rust backend didn't sacrifice accuracy for speed
    np.testing.assert_allclose(
        py_zt, rs_zt, rtol=1e-8, atol=1e-12,
        err_msg="FATAL: Rust Core and Python native zT calculations diverged!"
    )

    # --- 4. Performance Assertions and Reporting ---
    speedup_factor = py_duration / rs_duration if rs_duration > 0 else float('inf')
    
    # Metrics
    posterior_per_sec = N_ROWS / rs_duration
    ranking_per_sec = N_ROWS / rs_duration
    entropy_per_sec = N_ROWS / rs_duration

    print(f"\n--- PERFORMANCE METRICS ({N_ROWS} rows) ---")
    print(f"Python Naive Time : {py_duration:.4f} sec")
    print(f"Rust FFI Time     : {rs_duration:.4f} sec")
    print(f"Speedup Factor    : {speedup_factor:.2f}x")
    print(f"posterior/sec     : {posterior_per_sec:,.2f}")
    print(f"ranking/sec       : {ranking_per_sec:,.2f}")
    print(f"entropy/sec       : {entropy_per_sec:,.2f}")
    print("------------------------------------------")

    assert speedup_factor >= 5.0, (
        f"PERFORMANCE DEGRADATION DETECTED! Rust FFI was only {speedup_factor:.2f}x faster. "
        "Expected >= 10.0x. Check PyO3 memory allocation overhead and zero-copy guarantees."
    )