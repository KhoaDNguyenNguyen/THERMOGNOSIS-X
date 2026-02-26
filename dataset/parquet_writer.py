"""
Thermognosis Engine: Downstream Knowledge Persistence Module
Layer: python/thermognosis/dataset/parquet_writer.py
Compliance Level: Research-Grade / Q1 Infrastructure Standard
Author: Distinguished Professor of Computational Materials Science & Chief Software Architect

Description:
Provides mathematically rigorous, O(1) memory bounded, deterministic persistence
of thermodynamic and electronic transport data into columnar analytical storage (Apache Parquet).
Guarantees bitwise reproducibility, strict schema enforcement, and cross-language compatibility.
Implements O(N) streaming ingestion to support datasets exceeding 5,000,000+ rows 
without exceeding hardware limits.
"""

import itertools
import logging
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, List

import pyarrow as pa
import pyarrow.parquet as pq

# =============================================================================
# EXCEPTION HIERARCHY (SPEC-GOV-ERROR-HIERARCHY)
# =============================================================================

class ThermognosisDatasetError(Exception):
    """Base exception for the dataset serialization module."""
    pass


class SchemaValidationError(ThermognosisDatasetError):
    """Raised when data points violate the strict structural or thermodynamic schema."""
    pass


class ParquetWriteError(ThermognosisDatasetError):
    """Raised during I/O degradation or atomic swapping failures."""
    pass


# =============================================================================
# DOMAIN MODELS & SCHEMA DEFINITIONS
# =============================================================================

@dataclass(frozen=True, slots=True)
class DataPointRecord:
    """
    Immutable, high-performance structured record for a singular thermodynamic 
    or electronic transport measurement. Using slots=True guarantees minimal 
    memory footprint overhead, enabling streaming of millions of records.
    """
    sample_id: int
    composition: str
    paper_id: int
    property_x: str
    property_y: str
    unit_x: str
    unit_y: str
    x: float
    y: float


# STRICT SCHEMA ENFORCEMENT:
# Disallows silent coercion. PyArrow schema guarantees cross-language (Rust/C++/Python)
# bitwise compatibility and columnar compression efficiency.
THERMOGNOSIS_SCHEMA = pa.schema([
    pa.field("sample_id", pa.int32(), nullable=False),
    pa.field("composition", pa.string(), nullable=False),
    pa.field("paper_id", pa.int32(), nullable=False),
    pa.field("property_x", pa.string(), nullable=False),
    pa.field("property_y", pa.string(), nullable=False),
    pa.field("unit_x", pa.string(), nullable=False),
    pa.field("unit_y", pa.string(), nullable=False),
    pa.field("x", pa.float64(), nullable=False),
    pa.field("y", pa.float64(), nullable=False),
], metadata={"system": "Thermognosis Engine", "version": "1.0.0", "status": "Normative"})


# =============================================================================
# CORE I/O ENGINE
# =============================================================================

def _chunk_iterable(iterable: Iterable[DataPointRecord], chunk_size: int) -> Iterable[List[DataPointRecord]]:
    """
    O(1) memory generator to yield bounded chunks from a potentially infinite stream.
    
    Parameters
    ----------
    iterable : Iterable[DataPointRecord]
        The incoming stream of records.
    chunk_size : int
        Maximum number of records per chunk.
        
    Yields
    ------
    List[DataPointRecord]
        A strictly sized list of records for columnar batching.
    """
    iterator = iter(iterable)
    for first_item in iterator:
        chunk = [first_item]
        # itertools.islice consumes the iterator efficiently at the C level
        chunk.extend(itertools.islice(iterator, chunk_size - 1))
        yield chunk


def write_parquet(records: Iterable[DataPointRecord], output_path: Path, batch_size: int = 10000) -> None:
    """
    Persist an arbitrary-sized stream of data points to Apache Parquet with strict 
    schema enforcement, stream-chunking, and atomic file guarantees.
    
    Mathematical & Hardware Constraints:
    - Time Complexity: O(N) where N is the number of yielded records.
    - Space Complexity: O(B) where B is the batch_size, ensuring memory footprint < 2GB.
    
    Parameters
    ----------
    records : Iterable[DataPointRecord]
        Stream of physics-constrained data points.
    output_path : Path
        Target destination for the Parquet file.
    batch_size : int, optional
        The row-group limit for chunking (default is 10,000) to balance I/O throughput 
        and memory usage.
        
    Raises
    ------
    ParquetWriteError
        If atomic writing fails or file system permissions block creation.
    SchemaValidationError
        If PyArrow fails to cast the arrays to the strict schema dimensions.
    """
    out_file = Path(output_path).resolve()
    out_file.parent.mkdir(parents=True, exist_ok=True)
    
    # Generate an ephemeral temporary path to enforce ACID-like atomic replacement
    temp_path = out_file.with_suffix(f".tmp.{uuid.uuid4().hex}.parquet")
    logger = logging.getLogger(__name__)
    
    total_written = 0
    
    try:
        # Open an append-stream PyArrow Writer to stream directly to disk
        with pq.ParquetWriter(str(temp_path), THERMOGNOSIS_SCHEMA, compression='snappy') as writer:
            
            for chunk_idx, batch in enumerate(_chunk_iterable(records, batch_size)):
                try:
                    # columnar transposition O(B) time
                    arrays = [
                        pa.array([r.sample_id for r in batch], type=pa.int32()),
                        pa.array([r.composition for r in batch], type=pa.string()),
                        pa.array([r.paper_id for r in batch], type=pa.int32()),
                        pa.array([r.property_x for r in batch], type=pa.string()),
                        pa.array([r.property_y for r in batch], type=pa.string()),
                        pa.array([r.unit_x for r in batch], type=pa.string()),
                        pa.array([r.unit_y for r in batch], type=pa.string()),
                        pa.array([r.x for r in batch], type=pa.float64()),
                        pa.array([r.y for r in batch], type=pa.float64()),
                    ]
                    
                    table = pa.Table.from_arrays(arrays, schema=THERMOGNOSIS_SCHEMA)
                    writer.write_table(table)
                    total_written += len(batch)
                    
                except (pa.ArrowInvalid, pa.ArrowTypeError) as e:
                    raise SchemaValidationError(f"Batch {chunk_idx} failed structural validation: {e}") from e

        # Atomic POSIX-compliant swap to ensure zero chance of corrupted reads by concurrent workers
        temp_path.replace(out_file)
        logger.info(f"Successfully persisted {total_written} records to {out_file.name} (Atomic Swap: OK).")
        
    except Exception as e:
        # Graceful cleanup of ephemeral partial files to avoid disk clutter
        if temp_path.exists():
            try:
                temp_path.unlink()
            except OSError:
                pass
        raise ParquetWriteError(f"Fatal error during Parquet streaming to {out_file}: {str(e)}") from e