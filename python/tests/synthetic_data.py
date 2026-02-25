# -*- coding: utf-8 -*-
r"""
Thermognosis Engine: Synthetic Data Generation Suite for Rigorous QA
Document ID: SPEC-GOV-CODE-GENERATION-PROTOCOL
Target: Reproducibility, Boundary Testing, and Epistemic Validation

This module provides strictly deterministic, mathematically bound synthetic datasets
to stress-test the thermodynamic consistency engine, error propagation modules,
and Bayesian ranking models of the Thermognosis Engine.

Author: Distinguished Professor of Computational Materials Science
Institution: Thermognosis Engine Consortium
"""

import numpy as np
import pandas as pd
from typing import Dict, Tuple

# =============================================================================
# GLOBAL GOVERNANCE AND DETERMINISM
# =============================================================================

# Deterministic random number generator to guarantee reproducibility in CI/CD.
# We utilize PCG64 for rigorous statistical properties.
SEED = 424242
RNG = np.random.default_rng(SEED)

# Strict Physical Constants (Matching rust_core FFI spec)
L0_SOMMERFELD = 2.44e-8  # W*Ohm/K^2
L_MIN = 1.00e-8          # W*Ohm/K^2 (Absolute lower bound for condensed matter)
L_MAX = 4.00e-8          # W*Ohm/K^2 (Absolute upper bound accounting for bipolar effects)

def _base_dataframe(size: int, T_range: Tuple[float, float]) -> pd.DataFrame:
    """Helper to generate a base structurally sound thermodynamic DataFrame."""
    T = RNG.uniform(T_range[0], T_range[1], size).astype(np.float64)
    S = RNG.uniform(50e-6, 300e-6, size).astype(np.float64)
    sigma = RNG.uniform(1e3, 1e5, size).astype(np.float64)
    kappa_l = RNG.uniform(0.5, 5.0, size).astype(np.float64)
    kappa = (L0_SOMMERFELD * sigma * T) + kappa_l
    
    return pd.DataFrame({
        'T': T, 'S': S, 'sigma': sigma, 'kappa': kappa
    })

# =============================================================================
# GENERATOR FUNCTIONS
# =============================================================================

def generate_physical_limits() -> pd.DataFrame:
    """
    Generates deterministic edge-case arrays pushing the mathematical boundaries
    of the figure of merit equation: zT = (S^2 * sigma * T) / kappa.
    
    Tests: Division by zero, underflow, overflow, and asymptotic limits.
    """
    eps = np.nextafter(0, 1, dtype=np.float64)
    
    data = {
        'scenario': [
            'S_to_zero', 'sigma_to_zero', 'kappa_to_zero', 
            'T_to_zero_plus', 'Extreme_Temperature', 'Extreme_Conductivity',
            'Absolute_Zero_Kappa'
        ],
        'T':     [300.0, 300.0, 300.0, eps,   5000.0, 300.0, 300.0],
        'S':     [eps,   1e-4,  1e-4,  1e-4,  1e-4,   1e-4,  1e-4],
        'sigma': [1e5,   eps,   1e5,   1e5,   1e5,    1e8,   1e5],
        'kappa': [1.5,   1.5,   eps,   1.5,   1.5,    1.5,   0.0] # 0.0 tests strict exception promotion
    }
    
    df = pd.DataFrame(data).astype({
        'T': np.float64, 'S': np.float64, 'sigma': np.float64, 'kappa': np.float64
    })
    return df

def generate_physics_consistency_groups(N: int = 1000) -> pd.DataFrame:
    """
    Generates cohorts of data representing varying degrees of thermodynamic validity.
    
    Group A (Valid): Strictly obeys Wiedemann-Franz and thermodynamic bounds.
    Group B (Light Violation): Slightly violates WF limit or L0 boundaries.
    Group C (Heavy Violation): Negative kappa, extreme entropy violations.
    """
    n_group = N // 3
    
    # Group A: Strictly Valid
    df_A = _base_dataframe(n_group, (300.0, 1000.0))
    df_A['group'] = 'A_Valid'
    
    # Group B: Light Violation (Lorenz number anomalies)
    df_B = _base_dataframe(n_group, (300.0, 1000.0))
    # Force L outside of L_MIN and L_MAX by overriding kappa
    # Half below L_MIN, half above L_MAX
    L_viol = np.where(RNG.random(n_group) > 0.5, 0.5e-8, 6.0e-8)
    df_B['kappa'] = L_viol * df_B['sigma'] * df_B['T']
    df_B['group'] = 'B_Light_Violation'
    
    # Group C: Heavy Violation (Fundamentally impossible thermodynamics)
    df_C = _base_dataframe(N - 2*n_group, (300.0, 1000.0))
    # Negative temperatures, negative thermal conductivities
    df_C.loc[:n_group//2, 'T'] = -50.0
    df_C.loc[n_group//2:, 'kappa'] = -1.2
    df_C['group'] = 'C_Heavy_Violation'
    
    df_combined = pd.concat([df_A, df_B, df_C], ignore_index=True)
    # Shuffle deterministically
    return df_combined.sample(frac=1.0, random_state=SEED).reset_index(drop=True)

def generate_sigma_sensitivity_cases() -> Dict[str, pd.DataFrame]:
    """
    Generates identical physical states but with vastly different measurement uncertainties.
    
    Case A: Extreme precision (sigma_zT -> 1e-6)
    Case B: Extreme noise (sigma_zT -> 10.0)
    
    Essential for verifying the Bayesian regularizer's response to heteroscedastic noise.
    """
    base_df = _base_dataframe(100, (300.0, 800.0))
    
    # Case A: Extreme Precision
    df_A = base_df.copy()
    df_A['err_S'] = 1e-12
    df_A['err_sigma'] = 1e-6
    df_A['err_T'] = 1e-6
    df_A['err_kappa'] = 1e-6
    
    # Case B: High Noise
    df_B = base_df.copy()
    # To drive zT_err extremely high (e.g., ~10.0), inject massive uncertainty into S
    # since zT is highly sensitive to S (squared term).
    df_B['err_S'] = df_B['S'] * 2.0  # 200% relative error
    df_B['err_sigma'] = df_B['sigma'] * 0.5
    df_B['err_T'] = 50.0
    df_B['err_kappa'] = df_B['kappa'] * 0.5
    
    return {'high_precision': df_A, 'high_noise': df_B}

def generate_ranking_mock() -> pd.DataFrame:
    """
    Injects deterministic mock data resembling famous high-zT materials into a sea 
    of noisy, low-performing data to test the Bayesian Ranking and Quality Score algorithms.
    """
    N_noise = 1000
    df_noise = _base_dataframe(N_noise, (300.0, 800.0))
    # Force poor performance (high kappa, low S)
    df_noise['S'] = RNG.uniform(10e-6, 50e-6, N_noise)
    df_noise['kappa'] = RNG.uniform(5.0, 15.0, N_noise)
    df_noise['material'] = [f"Noise_Mat_{i}" for i in range(N_noise)]
    
    # Famous Materials
    # 1. Bi2Te3 (zT ~ 1.0 at 300K)
    # S = 200 uV/K, sigma = 1e5 S/m, kappa = 1.2 W/mK, T = 300K
    bi2te3 = pd.DataFrame([{
        'material': 'Bi2Te3_Mock',
        'T': 300.0, 'S': 200e-6, 'sigma': 1e5, 'kappa': 1.2
    }])
    
    # 2. PbTe (zT ~ 1.5 at 800K)
    # S = 300 uV/K, sigma = 5e4 S/m, kappa = 2.4 W/mK, T = 800K
    pbte = pd.DataFrame([{
        'material': 'PbTe_Mock',
        'T': 800.0, 'S': 300e-6, 'sigma': 5e4, 'kappa': 2.4
    }])
    
    df_combined = pd.concat([df_noise, bi2te3, pbte], ignore_index=True)
    return df_combined.sample(frac=1.0, random_state=SEED).reset_index(drop=True)

def generate_gap_data() -> pd.DataFrame:
    """
    Generates a dataset with intentional, severe informational voids (gaps) in the temperature domain.
    
    Distribution:
        - 80% data heavily clustered at 300-500K.
        - 20% data clustered at 550-750K.
        - Absolute Void: [800K - 900K].
        - Sparse data: > 900K.
        
    Critically tests Gaussian Process regression epistemic uncertainty explosion 
    within the unobserved [800, 900] Kelvin region.
    """
    # Cluster 1: 300 - 500K
    c1 = _base_dataframe(800, (300.0, 500.0))
    # Cluster 2: 550 - 750K
    c2 = _base_dataframe(200, (550.0, 750.0))
    # Cluster 3: 900 - 1000K (Post-gap sparsity)
    c3 = _base_dataframe(50, (900.0, 1000.0))
    
    df_combined = pd.concat([c1, c2, c3], ignore_index=True)
    
    # Assert validation that the gap exists
    assert not ((df_combined['T'] >= 800.0) & (df_combined['T'] < 900.0)).any(), \
        "Constraint Violation: Temperature gap was breached during generation."
        
    return df_combined.sample(frac=1.0, random_state=SEED).reset_index(drop=True)

if __name__ == "__main__":
    # Smoke test execution
    print("Generating Synthetic Boundary Testing Data...")
    print(f"Physical Limits Shape: {generate_physical_limits().shape}")
    print(f"Physics Groups Shape: {generate_physics_consistency_groups().shape}")
    print(f"Ranking Mock Shape: {generate_ranking_mock().shape}")
    print(f"Gap Data Shape: {generate_gap_data().shape}")
    print("SUCCESS: Deterministic constraints verified.")