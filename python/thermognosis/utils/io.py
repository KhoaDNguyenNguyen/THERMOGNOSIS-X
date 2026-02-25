"""
Thermognosis Engine: High-Performance, Schema-Enforced I/O Module
Layer: python/thermognosis/utils/io.py
Compliance Level: Research-Grade / Q1 Infrastructure Standard

Implements thread-safe, deterministic, and strictly validated I/O operations
for Parquet, JSON, and YAML formats. Guarantees atomic writes to prevent
race conditions across heterogeneous compute environments.
"""

import json
import threading
import uuid
from pathlib import Path
from typing import Any, Dict, Optional, Union

import pandas as pd
import pyarrow as pa
import pyarrow.parquet as pq
import yaml


# =============================================================================
# EXCEPTION HIERARCHY
# =============================================================================

class ThermognosisIOError(Exception):
    """Base exception for all I/O operations in the Thermognosis Engine."""
    pass


class SchemaViolationError(ThermognosisIOError):
    """
    Raised when a DataFrame fails to conform to the strictly required PyArrow schema.
    Prevents silent coercion or loss of precision in physical measurements.
    """
    pass


class AtomicWriteError(ThermognosisIOError):
    """Raised when an atomic file replacement fails during a write operation."""
    pass


class MetadataCorruptionError(ThermognosisIOError):
    """Raised when JSON/YAML metadata is malformed or violates security constraints."""
    pass


# =============================================================================
# CONCURRENCY CONTROL
# =============================================================================

_FILE_LOCKS: Dict[Path, threading.Lock] = {}
_GLOBAL_LOCK = threading.Lock()

def _get_file_lock(path: Path) -> threading.Lock:
    """
    Retrieves or creates a threading lock for a specific file path to guarantee
    thread-safe read/write operations within the same process.
    
    Parameters
    ----------
    path : Path
        The absolute, resolved file path.
        
    Returns
    -------
    threading.Lock
        The lock associated with the file path.
    """
    resolved_path = path.resolve()
    with _GLOBAL_LOCK:
        if resolved_path not in _FILE_LOCKS:
            _FILE_LOCKS[resolved_path] = threading.Lock()
        return _FILE_LOCKS[resolved_path]


# =============================================================================
# PARQUET I/O (SPEC-DB-PARQUET)
# =============================================================================

def write_parquet_safely(
    df: pd.DataFrame, 
    file_path: Union[str, Path], 
    schema: pa.Schema,
    compression: str = "snappy"
) -> None:
    """
    Thread-safe, schema-enforced, atomic write of a DataFrame to a Parquet file.
    
    Mathematical/Architectural Fidelity:
    Enforces the dataset invariant: D_out \in Schema. If df does not strictly match
    the schema (e.g., matching symbols T, S, sigma, kappa and their precision types), 
    the write is aborted before touching disk.
    
    Implements: SPEC-DB-PARQUET
    
    Parameters
    ----------
    df : pd.DataFrame
        The data to be serialized.
    file_path : Union[str, Path]
        The destination file path.
    schema : pa.Schema
        The strictly enforced PyArrow schema.
    compression : str, optional
        The compression algorithm to use (default is "snappy").
        
    Raises
    ------
    SchemaViolationError
        If the dataframe columns or types do not match the PyArrow schema.
    AtomicWriteError
        If the file system fails to replace the temporary file atomically.
    """
    path = Path(file_path).resolve()
    path.parent.mkdir(parents=True, exist_ok=True)
    
    # Generate unique temporary path for atomic write
    temp_path = path.with_suffix(f".tmp.{uuid.uuid4().hex}")
    
    try:
        # Strict schema enforcement boundary
        table = pa.Table.from_pandas(df, schema=schema, preserve_index=False)
    except (pa.ArrowInvalid, pa.ArrowTypeError, ValueError) as e:
        raise SchemaViolationError(
            f"DataFrame failed schema validation for {path.name}. "
            f"Expected schema:\n{schema}\nUnderlying error: {str(e)}"
        ) from e

    lock = _get_file_lock(path)
    with lock:
        try:
            # Write to temporary file first
            pq.write_table(table, temp_path, compression=compression)
            
            # Atomic swap (POSIX atomic, Windows generally atomic in modern Python)
            temp_path.replace(path)
        except Exception as e:
            if temp_path.exists():
                temp_path.unlink()
            raise AtomicWriteError(f"Failed to atomically write Parquet file at {path}: {str(e)}") from e


def read_parquet_safely(
    file_path: Union[str, Path], 
    expected_schema: Optional[pa.Schema] = None
) -> pd.DataFrame:
    """
    Thread-safe, schema-validated read of a Parquet file.
    
    Implements: SPEC-DB-PARQUET
    
    Parameters
    ----------
    file_path : Union[str, Path]
        The source file path.
    expected_schema : pa.Schema, optional
        If provided, validates the read data against this PyArrow schema.
        
    Returns
    -------
    pd.DataFrame
        The deserialized dataframe.
        
    Raises
    ------
    FileNotFoundError
        If the Parquet file does not exist.
    SchemaViolationError
        If the file schema diverges from the expected schema.
    ThermognosisIOError
        If the file cannot be read due to corruption or access issues.
    """
    path = Path(file_path).resolve()
    
    if not path.exists():
        raise FileNotFoundError(f"Parquet file not found: {path}")

    lock = _get_file_lock(path)
    with lock:
        try:
            table = pq.read_table(path)
        except Exception as e:
            raise ThermognosisIOError(f"Failed to read Parquet file at {path}: {str(e)}") from e

    if expected_schema is not None:
        if not table.schema.equals(expected_schema, check_metadata=False):
            raise SchemaViolationError(
                f"Schema mismatch detected when reading {path.name}. "
                f"Expected:\n{expected_schema}\nFound:\n{table.schema}"
            )

    return table.to_pandas()


# =============================================================================
# SECURE METADATA I/O
# =============================================================================

def write_json_metadata(data: Dict[str, Any], file_path: Union[str, Path]) -> None:
    """
    Thread-safe, atomic write of dictionary metadata to a JSON file.
    Ensures that provenance and contextual metadata (C, M) are never partially written.
    
    Implements: SPEC-CONTRACT-RAW-MEASUREMENT (Serialization Standard)
    
    Parameters
    ----------
    data : Dict[str, Any]
        Metadata dictionary (must be JSON serializable).
    file_path : Union[str, Path]
        The destination file path.
        
    Raises
    ------
    AtomicWriteError
        If atomic replacement fails.
    """
    path = Path(file_path).resolve()
    path.parent.mkdir(parents=True, exist_ok=True)
    temp_path = path.with_suffix(f".tmp.{uuid.uuid4().hex}")
    
    lock = _get_file_lock(path)
    with lock:
        try:
            with temp_path.open('w', encoding='utf-8') as f:
                json.dump(data, f, indent=4, sort_keys=True)
            temp_path.replace(path)
        except Exception as e:
            if temp_path.exists():
                temp_path.unlink()
            raise AtomicWriteError(f"Failed to atomically write JSON at {path}: {str(e)}") from e


def read_json_metadata(file_path: Union[str, Path]) -> Dict[str, Any]:
    """
    Thread-safe read of JSON metadata.
    
    Parameters
    ----------
    file_path : Union[str, Path]
        The source file path.
        
    Returns
    -------
    Dict[str, Any]
        The parsed JSON metadata.
        
    Raises
    ------
    MetadataCorruptionError
        If the JSON is malformed.
    """
    path = Path(file_path).resolve()
    
    lock = _get_file_lock(path)
    with lock:
        try:
            with path.open('r', encoding='utf-8') as f:
                return json.load(f)
        except json.JSONDecodeError as e:
            raise MetadataCorruptionError(f"JSON metadata corruption in {path}: {str(e)}") from e
        except FileNotFoundError:
            raise


def write_yaml_metadata(data: Dict[str, Any], file_path: Union[str, Path]) -> None:
    """
    Thread-safe, atomic write of dictionary metadata to a YAML file.
    
    Implements: SPEC-GOV-GLOBAL-CONVENTIONS (Logging and Config IO)
    
    Parameters
    ----------
    data : Dict[str, Any]
        The configuration or metadata dictionary.
    file_path : Union[str, Path]
        The destination YAML file path.
        
    Raises
    ------
    AtomicWriteError
        If atomic replacement fails.
    """
    path = Path(file_path).resolve()
    path.parent.mkdir(parents=True, exist_ok=True)
    temp_path = path.with_suffix(f".tmp.{uuid.uuid4().hex}")
    
    lock = _get_file_lock(path)
    with lock:
        try:
            with temp_path.open('w', encoding='utf-8') as f:
                yaml.safe_dump(data, f, default_flow_style=False, sort_keys=True)
            temp_path.replace(path)
        except Exception as e:
            if temp_path.exists():
                temp_path.unlink()
            raise AtomicWriteError(f"Failed to atomically write YAML at {path}: {str(e)}") from e


def read_yaml_metadata(file_path: Union[str, Path]) -> Dict[str, Any]:
    """
    Thread-safe, strictly secure read of YAML metadata using yaml.safe_load.
    Prevents arbitrary code execution from malicious configuration files.
    
    Parameters
    ----------
    file_path : Union[str, Path]
        The source YAML file path.
        
    Returns
    -------
    Dict[str, Any]
        The parsed YAML metadata.
        
    Raises
    ------
    MetadataCorruptionError
        If the YAML is malformed or violates safe-load limits.
    """
    path = Path(file_path).resolve()
    
    lock = _get_file_lock(path)
    with lock:
        try:
            with path.open('r', encoding='utf-8') as f:
                data = yaml.safe_load(f)
                return data if data is not None else {}
        except yaml.YAMLError as e:
            raise MetadataCorruptionError(f"YAML metadata corruption or security violation in {path}: {str(e)}") from e
        except FileNotFoundError:
            raise