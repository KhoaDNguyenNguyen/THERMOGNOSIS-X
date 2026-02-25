# -*- coding: utf-8 -*-
r"""
Thermognosis Engine: Bayesian Epistemic Confidence Tests
Document ID: SPEC-GOV-CODE-GENERATION-PROTOCOL

Rigorous statistical testing of the Bayesian propagation modules. Validates 
heteroscedastic noise responses and verifies identical mathematical execution 
across the C-contiguous FFI boundary.
"""

import numpy as np
import pytest
import synthetic_data
from thermognosis.wrappers.rust_wrapper import RustCore, RustCoreError
from synthetic_data import generate_physics_consistency_groups, generate_ranking_mock, generate_gap_data




def test_bayesian_uncertainty_propagation() -> None:
    """
    STRESS TEST: Heteroscedastic Noise Response & FFI Mathematical Adherence.
    
    Ensures that the first-order analytical error propagation correctly maps 
    uncertainty into the posterior variance of zT. Case A must yield a sharply
    peaked Dirac-like posterior, whereas Case B must yield a highly dispersed, 
    flat posterior reflecting low epistemic confidence.
    """
    engine = RustCore(deterministic=True)
    cases = synthetic_data.generate_sigma_sensitivity_cases()
    
    df_prec = cases['high_precision']
    df_noise = cases['high_noise']
    
    # ---------------------------------------------------------
    # BOUNDARY EXECUTION: CASE A (Extreme Precision)
    # ---------------------------------------------------------
    zt_prec, unc_prec = engine.propagate_error(
        df_prec['S'].values, df_prec['sigma'].values, df_prec['kappa'].values, df_prec['T'].values,
        df_prec['err_S'].values, df_prec['err_sigma'].values, df_prec['err_kappa'].values, df_prec['err_T'].values
    )
    
    # MATHEMATICAL AUDIT: Verify FFI did not introduce floating point truncation
    # zT = (S^2 * sigma * T) / kappa
    zt_expected = (df_prec['S'].values**2 * df_prec['sigma'].values * df_prec['T'].values) / df_prec['kappa'].values
    np.testing.assert_allclose(
        zt_prec, 
        zt_expected, 
        rtol=1e-8, 
        err_msg="CRITICAL: FFI Memory boundary caused silent precision loss in deterministic zT computation."
    )
    
    # ---------------------------------------------------------
    # BOUNDARY EXECUTION: CASE B (High Heteroscedastic Noise)
    # ---------------------------------------------------------
    zt_noise, unc_noise = engine.propagate_error(
        df_noise['S'].values, df_noise['sigma'].values, df_noise['kappa'].values, df_noise['T'].values,
        df_noise['err_S'].values, df_noise['err_sigma'].values, df_noise['err_kappa'].values, df_noise['err_T'].values
    )
    
    # Compute the variance of the resulting posterior distributions
    var_prec = np.mean(unc_prec ** 2)
    var_noise = np.mean(unc_noise ** 2)
    
    # ---------------------------------------------------------
    # RIGOROUS EPISTEMIC ASSERTIONS
    # ---------------------------------------------------------
    # 1. Variance Inversion Constraint
    assert var_prec < var_noise, (
        f"EPISTEMIC FAILURE: Noise variance ({var_noise}) is not strictly "
        f"greater than precision variance ({var_prec})."
    )
    
    # 2. Sharp Peak Validation (High-precision measurements must map to tight bounds)
    assert var_prec < 1e-4, f"EPISTEMIC BOUNDARY VIOLATION: Precision posterior is overly dispersed. Var: {var_prec}"
    
    # 3. Flat Peak Validation (Massive measurement noise must strictly explode the variance)
    assert var_noise > 0.1, f"EPISTEMIC BOUNDARY VIOLATION: Engine is overconfident in noisy data. Var: {var_noise}"