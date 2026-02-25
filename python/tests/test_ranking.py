# -*- coding: utf-8 -*-
r"""
Epistemic Validation Suite: Graph-Based Material Ranking
========================================================
Test ID: SPEC-GRAPH-RANK-VALIDATION

Strictly validates the mathematical bounds of the material ranking pipeline.
Ensures Lagrangian entropy regularization (G03-EMBEDDING-RANK-THEORY) correctly 
elevates true high-zT materials out of synthetic topological noise.

Author: Distinguished Professor of Computational Materials Science
"""

import pytest
import numpy as np
from typing import List, Tuple, Dict, Any

from synthetic_data import generate_gap_data
# và
from synthetic_data import generate_ranking_mock
from thermognosis.pipeline.ranking import MaterialRanker

class MockPGDatabase:
    """Simulates the authoritative PostgreSQL relational extraction constraint."""
    def __init__(self, data: List[Tuple[str, float, float, float]]) -> None:
        self._data = data

    def cursor(self) -> Any:
        class Cursor:
            def __init__(self, dataset: List[Tuple[str, float, float, float]]) -> None:
                self._dataset = dataset
            def __enter__(self) -> 'Cursor':
                return self
            def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
                pass
            def execute(self, query: str) -> None:
                pass
            def fetchall(self) -> List[Tuple[str, float, float, float]]:
                return self._dataset
        return Cursor(self._data)


class MockBulkWriter:
    """Captures transacted updates across cross-system boundaries."""
    def __init__(self) -> None:
        self.pg_data: List[Tuple[float, str]] = []
        self.graph_data: List[Dict[str, Any]] = []

    def update_material_ranks(
        self, pg_data: List[Tuple[float, str]], graph_data: List[Dict[str, Any]]
    ) -> None:
        self.pg_data = pg_data
        self.graph_data = graph_data


class MathStrictRustCoreRanking:
    """Exact native mathematical implementation of SPEC-GRAPH-RANK bypassing FFI."""
    def compute_material_rank_batch(
        self, p_arr: np.ndarray, zt_arr: np.ndarray, c_arr: np.ndarray, 
        bounds: List[Tuple[int, int]], alpha: float, beta: float
    ) -> List[float]:
        ranks: List[float] = []
        for start, end in bounds:
            p: np.ndarray = p_arr[start:end]
            zt: np.ndarray = zt_arr[start:end]
            c: np.ndarray = c_arr[start:end]

            # w_i = 1.0 + alpha * ln(1.0 + c_i)
            w: np.ndarray = 1.0 + alpha * np.log(1.0 + c)
            
            # R_m = sum(w_i * p_i * zT_i) / sum(w_i)
            r_m: float = float(np.sum(w * p * zt) / np.sum(w))
            
            # Lagrangian entropy regularization logic
            eps: float = float(np.nextafter(0, 1, dtype=np.float64))
            h_m: float = float(-np.sum(p * np.log(p + eps)))
            
            r_final: float = r_m - beta * h_m
            ranks.append(r_final)
            
        return ranks


def test_entropy_regularized_ranking_pipeline() -> None:
    """
    Test ID: TEST-RANK-001
    Objective: Prove mathematically rigorous extraction of signal from synthetic noise.
    """
    df = generate_ranking_mock()
    df['zT'] = (df['S']**2 * df['sigma'] * df['T']) / df['kappa']
    
    # Strictly bound deterministic generation for graph meta-features
    np.random.seed(424242)
    db_rows: List[Tuple[str, float, float, float]] = []
    
    for _, row in df.iterrows():
        mat_id = str(row['material'])
        zt = float(row['zT'])
        
        if mat_id in ['Bi2Te3_Mock', 'PbTe_Mock']:
            p_val, c_val = 0.95, 500.0  # High credibility, high citation
        else:
            p_val = float(np.random.uniform(0.1, 0.5))
            c_val = float(np.random.uniform(0.0, 10.0))
            
        db_rows.append((mat_id, p_val, zt, c_val))
        
    # Strictly sort by material_id to preserve O(1) FFI memory bounds mapping
    db_rows.sort(key=lambda x: x[0])
    
    db_mock = MockPGDatabase(db_rows)
    writer_mock = MockBulkWriter()
    rust_core = MathStrictRustCoreRanking()
    
    ranker = MaterialRanker(db_connection=db_mock, rust_core=rust_core, bulk_writer=writer_mock)
    
    # Pipeline Execution Parameterization
    num_ranked: int = ranker.update_all_ranks(alpha=1.0, beta=0.1)
    
    assert num_ranked == len(df['material'].unique()), "Pipeline fragmented sub-manifolds."
    assert len(writer_mock.pg_data) == num_ranked, "ACID write boundary failure for PG."
    assert len(writer_mock.graph_data) == num_ranked, "ACID write boundary failure for Graph."
    
    ranks_dict: Dict[str, float] = {mat_uuid: rank_val for rank_val, mat_uuid in writer_mock.pg_data}
    
    # Sort materials descending by regularized final rank score
    sorted_materials = sorted(ranks_dict.items(), key=lambda x: x[1], reverse=True)
    top_10: List[str] = [mat for mat, score in sorted_materials[:10]]
    
    # BRUTAL ASSERTION: True thermodynamic powerhouses must survive the regularizers
    assert 'Bi2Te3_Mock' in top_10, f"Bi2Te3_Mock buried. Mathematical calibration flawed: {top_10}"
    assert 'PbTe_Mock' in top_10, f"PbTe_Mock buried. Mathematical calibration flawed: {top_10}"
    
    for mat, score in ranks_dict.items():
        assert np.isfinite(score), f"Invalid non-finite boundary state for {mat}: {score}"