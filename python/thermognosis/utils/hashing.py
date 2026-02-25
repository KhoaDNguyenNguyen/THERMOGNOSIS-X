"""
Thermognosis Engine: Canonical Hashing and Lineage Validation
=============================================================

This module provides the mathematically rigorous infrastructure for deterministic, 
cross-platform hashing of computational artifacts (datasets, parameter dictionaries, 
and experimental records). It guarantees bitwise reproducibility of lineage graphs
across heterogeneous hardware environments (Linux, Windows, GPU/CPU).

Mathematical Formulation:
    For any semantic object D, the canonical hash is defined as:
    H_D = Hash(CanonicalSerialize(D))
    
    Where CanonicalSerialize enforces:
    1. Lexicographical ordering of all keys.
    2. Fixed-point precision rounding of all floating-point representations: 
       x |-> round(x, 12)
    3. Structural determinism for collections and tensors.

Implements:
    - SPEC-ACQ-CHECKSUM
    - SPEC-CONTRACT-VERSIONING
"""

import hashlib
import json
import math
import datetime
from typing import Any, Dict, Union, List

import numpy as np
import pandas as pd


# -----------------------------------------------------------------------------
# ERROR HIERARCHY (Implements: SPEC-GOV-ERROR-HIERARCHY)
# -----------------------------------------------------------------------------

class ThermognosisError(Exception):
    """Base exception for all domain-specific errors in the Thermognosis Engine."""
    pass

class CanonicalSerializationError(ThermognosisError):
    """
    Raised when an object violates the deterministic serialization constraints,
    such as containing unsupported types or cyclic references.
    """
    pass

class HashComputationError(ThermognosisError):
    """
    Raised when the cryptographic hash function fails to process the 
    canonical byte representation.
    """
    pass


# -----------------------------------------------------------------------------
# CONSTANTS & GOVERNANCE
# -----------------------------------------------------------------------------

# Global precision standard to ensure bitwise reproducibility across OS/Arch
# Windows and Linux may differ in standard double-precision float representation 
# at the ~15th decimal place due to compiler-specific optimizations.
NUMERICAL_PRECISION = 12


# -----------------------------------------------------------------------------
# CORE IMPLEMENTATION
# -----------------------------------------------------------------------------

def _standardize_dataframe(df: pd.DataFrame, precision: int) -> Dict[str, Any]:
    """
    Prepares a pandas DataFrame for canonical serialization using vectorized 
    optimizations for numerical stability and performance.
    
    Implements invariant: D^(v1) == D^(v2) => H_D1 == H_D2
    
    Parameters
    ----------
    df : pd.DataFrame
        The dataset to serialize.
    precision : int
        Decimal places for floating-point stabilization.
        
    Returns
    -------
    Dict[str, Any]
        A standardized, strictly ordered dictionary representation of the DataFrame.
    """
    try:
        # Enforce deterministic column ordering
        df_sorted = df[sorted(df.columns)].copy(deep=True)
        
        # Identify and round floating-point columns using vectorized operations
        float_cols = df_sorted.select_dtypes(include=[np.floating, float]).columns
        if not float_cols.empty:
            df_sorted[float_cols] = df_sorted[float_cols].round(precision)
            
            # Neutralize negative zeros (-0.0 -> 0.0) across all float columns
            df_sorted[float_cols] = df_sorted[float_cols] + 0.0

        # Deterministic handling of non-finite values (NaN, Inf)
        df_sorted = df_sorted.replace({
            np.nan: "NaN", 
            np.inf: "Infinity", 
            -np.inf: "-Infinity"
        })
        
        # Extract components deterministically
        return {
            "columns": df_sorted.columns.tolist(),
            "data": df_sorted.values.tolist(),
            "index": df_sorted.index.tolist()
        }
    except Exception as e:
        raise CanonicalSerializationError(f"Failed to standardize DataFrame: {str(e)}") from e


def _standardize_value(val: Any, precision: int) -> Any:
    """
    Recursively normalizes Python, NumPy, and Pandas objects into universally 
    JSON-serializable primitives, enforcing the Versioning and Lineage Contract.

    Parameters
    ----------
    val : Any
        The computational object (scalar, matrix, subset) to standardize.
    precision : int
        The fixed precision for floating point rounding.

    Returns
    -------
    Any
        A deterministically serializable primitive representation.
        
    Raises
    ------
    CanonicalSerializationError
        If the object type cannot be formalized mathematically.
    """
    if val is None:
        return None
    elif isinstance(val, bool): # Must precede int check, as bool subclasses int
        return val
    elif isinstance(val, (int, np.integer)):
        return int(val)
    elif isinstance(val, (float, np.floating)):
        if math.isnan(val):
            return "NaN"
        elif math.isinf(val):
            return "Infinity" if val > 0 else "-Infinity"
        # Format explicitly to circumvent platform-specific JSON float drift
        val_rounded = round(float(val), precision)
        # Neutralize negative zero
        if val_rounded == -0.0:
            val_rounded = 0.0
        return f"{val_rounded:.{precision}f}"
    elif isinstance(val, str):
        return val
    elif isinstance(val, dict):
        # Lexicographical key sorting enforced natively via sorted()
        return {str(k): _standardize_value(v, precision) for k, v in sorted(val.items())}
    elif isinstance(val, (list, tuple)):
        return [_standardize_value(v, precision) for v in val]
    elif isinstance(val, set):
        # Sets must be converted to sorted lists to preserve mathematical equivalence
        return sorted([_standardize_value(v, precision) for v in val])
    elif isinstance(val, np.ndarray):
        # Fallback to lists for tensor geometries; handles nested structures
        return _standardize_value(val.tolist(), precision)
    elif isinstance(val, pd.DataFrame):
        return _standardize_dataframe(val, precision)
    elif isinstance(val, pd.Series):
        # Treat series as a standardized dataframe column mapping
        return _standardize_value(val.to_dict(), precision)
    elif isinstance(val, (datetime.datetime, datetime.date, pd.Timestamp)):
        return val.isoformat()
    else:
        raise CanonicalSerializationError(
            f"Object of type {type(val)} maps to an indeterminate semantic state "
            "and cannot be canonically serialized."
        )


def canonical_serialize(data: Union[Dict[str, Any], pd.DataFrame, Any], precision: int = NUMERICAL_PRECISION) -> bytes:
    """
    Computes the strictly deterministic, canonical byte representation of a mathematical 
    object, dataset, or model parameter state.

    Implements: SPEC-ACQ-CHECKSUM

    Parameters
    ----------
    data : Union[Dict[str, Any], pd.DataFrame, Any]
        The semantic object \\( \\mathcal{D} \\) or parameter subset \\( \\theta \\) to serialize.
    precision : int, optional
        Floating point precision constraint. Defaults to NUMERICAL_PRECISION (12).

    Returns
    -------
    bytes
        UTF-8 encoded bytes of the canonical JSON string.
        
    Raises
    ------
    CanonicalSerializationError
        If structural or precision constraints are violated during evaluation.
    """
    try:
        standardized_tree = _standardize_value(data, precision)
        
        # separators=(',', ':') removes whitespace, sort_keys=True ensures final ordering
        json_string = json.dumps(
            standardized_tree,
            ensure_ascii=True,
            allow_nan=False, 
            sort_keys=True,
            separators=(',', ':')
        )
        return json_string.encode('utf-8')
    except Exception as e:
        if isinstance(e, CanonicalSerializationError):
            raise
        raise CanonicalSerializationError(f"Canonical serialization pipeline failed: {str(e)}") from e


def compute_sha256_hash(data: Union[Dict[str, Any], pd.DataFrame, Any], precision: int = NUMERICAL_PRECISION) -> str:
    """
    Calculates the cryptographic SHA-256 dataset/model hash required by the 
    Versioning and Lineage Contract.

    Formula:
    \\[ H_{\\mathcal{D}} = \\mathrm{Hash}(\\text{canonical serialization}) \\]
    
    Implements: SPEC-CONTRACT-VERSIONING

    Parameters
    ----------
    data : Union[Dict[str, Any], pd.DataFrame, Any]
        The entity to hash (e.g., dataset \\( \\mathcal{D}^{(v)} \\), parameters \\( \\theta \\)).
    precision : int, optional
        Floating point precision constraint. Defaults to NUMERICAL_PRECISION (12).

    Returns
    -------
    str
        The strictly deterministic 64-character lowercase hex digest.
        
    Raises
    ------
    HashComputationError
        If the hash cannot be computed from the canonical serialization.
    """
    try:
        canonical_bytes = canonical_serialize(data, precision=precision)
        return hashlib.sha256(canonical_bytes).hexdigest()
    except CanonicalSerializationError as e:
        raise HashComputationError(f"Cannot compute hash due to serialization violation: {str(e)}") from e
    except Exception as e:
        raise HashComputationError(f"Unexpected failure during SHA-256 hash evaluation: {str(e)}") from e