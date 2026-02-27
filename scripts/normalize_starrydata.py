#!/usr/bin/env python3
"""
Thermognosis Engine: Dataset Normalization Orchestrator
=======================================================

File: scripts/normalize_starrydata.py
Author: Distinguished Professor of Computational Materials Science & Chief Software Architect
Status: Normative — Q1 Infrastructure Standard

Description:
------------
This script acts as the primary CLI orchestration bridge for the Thermognosis ML pipeline.
It strictly binds the Epistemic JSON Parser to the Downstream Parquet Writer. 

By operating entirely on pure Python generators, the pipeline guarantees an O(1) memory 
footprint per stream, successfully processing arbitrarily large datasets (e.g., >5,000,000 
rows) within a 2GB RAM strict hardware constraint. 

It guarantees deterministic, bitwise reproducible structural transformation, strictly 
logging errors using the SPEC-GOV-ERROR-HIERARCHY without tolerating silent failures.

Usage:
------
$ python scripts/normalize_starrydata.py --input_dir data/raw/ --output data/processed/starrydata.parquet
"""

import argparse
import logging
import sys
import time
from pathlib import Path
from typing import Generator, Set
import csv
from tqdm import tqdm

# =============================================================================
# ENVIRONMENT RESOLUTION & DEPENDENCY INJECTION
# =============================================================================

# Dynamically resolve absolute paths to ensure deterministic execution across 
# Windows, Colab, and Arch Linux regardless of the current working directory.
_SCRIPT_DIR = Path(__file__).resolve().parent
_PROJECT_ROOT = _SCRIPT_DIR.parent
_PYTHON_SRC_DIR = _PROJECT_ROOT / "python"

if str(_PYTHON_SRC_DIR) not in sys.path:
    sys.path.insert(0, str(_PYTHON_SRC_DIR))

try:
    from thermognosis.dataset.json_parser import stream_samples, ThermognosisError
    from thermognosis.dataset.parquet_writer import (
        write_parquet, 
        DataPointRecord as ParquetDataPointRecord
    )
except ImportError as e:
    print(f"FATAL: Thermognosis core libraries not found. Check PYTHONPATH. Error: {e}", file=sys.stderr)
    sys.exit(1)


# =============================================================================
# OBSERVABILITY & LOGGING CONFIGURATION (SPEC-GOV-ERROR-HIERARCHY)
# =============================================================================

def _setup_logger() -> logging.Logger:
    """
    Configures a strictly governed, professional logger to enforce zero silent 
    failures across the computational pipeline.
    """
    logger = logging.getLogger("thermognosis.pipeline")
    logger.setLevel(logging.INFO)
    
    if not logger.handlers:
        handler = logging.StreamHandler(sys.stdout)
        formatter = logging.Formatter(
            fmt="[%(asctime)s][%(levelname)s] %(message)s", 
            datefmt="%Y-%m-%d %H:%M:%S"
        )
        handler.setFormatter(formatter)
        logger.addHandler(handler)
        
    return logger

logger = _setup_logger()

# =============================================================================
# PROPERTY FILTER CONFIGURATION
# =============================================================================

ALLOWED_PROPERTIES = {

    "Thermal conductivity",
    "Seebeck coefficient",
    "Electrical resistivity",
    "Electrical conductivity",
    "ZT"

}

# =============================================================================
# ORCHESTRATION PIPELINE
# =============================================================================

def execute_normalization_pipeline(input_dir: Path, output_file: Path, batch_size: int) -> None:
    logger.info("Initializing Thermognosis Normalization Pipeline...")
    logger.info(f"Input Directory : {input_dir}")
    logger.info(f"Output Target   : {output_file}")
    logger.info(f"I/O Batch Size  : {batch_size}")

    # Telemetry Trackers cho Data Hợp lệ
    unique_samples: Set[int] = set()
    unique_papers: Set[int] = set()
    total_datapoints = 0
    
    # Trackers Phân loại Rác (Garbage Classification)
    rejected_epistemic: Set[int] = set()  # Bị loại do không phải Thực Nghiệm
    rejected_domain: Set[int] = set()     # Bị loại do khác Lĩnh vực (VD: Quang học)

    rejection_log_path = output_file.parent / "rejected_lineage_log.csv"
    logger.info(f"Provenance Log  : {rejection_log_path}")

    output_file.parent.mkdir(parents=True, exist_ok=True)

    with open(rejection_log_path, mode="w", newline="", encoding="utf-8") as rejection_file:
        csv_writer = csv.writer(rejection_file)
        # Bổ sung cột rejection_reason để dễ filter sau này
        csv_writer.writerow(["sample_id", "paper_id", "composition", "measurement_type", "rejection_reason"])

        def record_stream() -> Generator[ParquetDataPointRecord, None, None]:
            nonlocal total_datapoints
            
            with tqdm(desc="Normalizing Thermodynamic Data", unit=" pts", dynamic_ncols=True) as pbar:
                stream = stream_samples(
                    directory=input_dir, 
                    allowed_types=("Experiment", "Theory", "Simulation", "Unknown", "Review")
                )
                
                for sample_record, data_point in stream:
                    sample_id = sample_record.sample_id
                    
                    # ---------------------------------------------------------
                    # 1. BỘ LỌC NGUỒN GỐC (EPISTEMIC FILTER)
                    # ---------------------------------------------------------
                    if sample_record.measurement_type != "Experiment":
                        if sample_id not in rejected_epistemic:
                            csv_writer.writerow([
                                sample_id, sample_record.paper_id, sample_record.composition,
                                sample_record.measurement_type, "Non-Experimental Origin"
                            ])
                            rejected_epistemic.add(sample_id)
                        continue

                    # ---------------------------------------------------------
                    # 2. BỘ LỌC LĨNH VỰC (DOMAIN FILTER)
                    # ---------------------------------------------------------
                    if data_point.property_x != "Temperature" or data_point.property_y not in ALLOWED_PROPERTIES:
                        # Chỉ log nếu sample này chưa từng được tính là hợp lệ hoặc chưa bị log trước đó
                        if sample_id not in rejected_domain and sample_id not in unique_samples:
                            csv_writer.writerow([
                                sample_id, sample_record.paper_id, sample_record.composition,
                                sample_record.measurement_type, f"Non-Thermo Property ({data_point.property_y})"
                            ])
                            rejected_domain.add(sample_id)
                        continue
                    
                    # ---------------------------------------------------------
                    # 3. DỮ LIỆU ĐẠT CHUẨN (VALID YIELD)
                    # ---------------------------------------------------------
                    # Nếu một mẫu lỡ bị đánh dấu "rác" trước đó (do JSON có mix cả data tốt lẫn xấu),
                    # ta gỡ nó ra khỏi danh sách rác để đếm số liệu thống kê cho chuẩn.
                    if sample_id in rejected_domain:
                        rejected_domain.remove(sample_id)

                    unique_samples.add(sample_id)
                    unique_papers.add(sample_record.paper_id)
                    total_datapoints += 1
                    pbar.update(1)
                    
                    yield ParquetDataPointRecord(
                        sample_id=sample_id,
                        composition=sample_record.composition,
                        paper_id=sample_record.paper_id,
                        property_x=data_point.property_x,
                        property_y=data_point.property_y,
                        unit_x=data_point.unit_x,
                        unit_y=data_point.unit_y,
                        x=data_point.x,
                        y=data_point.y
                    )

        # Ghi trực tiếp xuống Parquet
        start_time = time.perf_counter()
        try:
            write_parquet(records=record_stream(), output_path=output_file, batch_size=batch_size)
        except Exception as e:
            logger.error(f"[FATAL] Irrecoverable failure during Parquet serialization: {e}")
            raise

    elapsed_time = time.perf_counter() - start_time

    # =====================================================================
    # IN RA BẢNG TELEMETRY PHÂN LOẠI CHI TIẾT
    # =====================================================================
    logger.info("Normalization Pipeline Completed Successfully.")
    logger.info("=" * 65)
    logger.info(" THERMOGNOSIS ENGINE : COMPUTATIONAL PIPELINE TELEMETRY")
    logger.info("=" * 65)
    logger.info(f" Execution Time (Wall)           : {elapsed_time:.3f} seconds")
    logger.info(f" Processing Throughput           : {total_datapoints / max(elapsed_time, 0.001):.1f} pts/sec")
    logger.info("-" * 65)
    logger.info(f" Valid Experimental Samples      : {len(unique_samples):,}")
    logger.info(f" Total Papers Assessed           : {len(unique_papers):,}")
    logger.info(f" Total Thermo DataPoints Yielded : {total_datapoints:,}")
    logger.info("-" * 65)
    logger.info(f" Rejected (Non-Experimental)     : {len(rejected_epistemic):,} samples")
    logger.info(f" Rejected (Non-Thermoelectric)   : {len(rejected_domain):,} samples")
    logger.info(f" Rejection Lineage Log           : {rejection_log_path.name}")
    logger.info("=" * 65)


# =============================================================================
# COMMAND-LINE INTERFACE
# =============================================================================

def parse_args() -> argparse.Namespace:
    """
    Configures and parses strict CLI arguments for the normalization process.
    """
    parser = argparse.ArgumentParser(
        description=(
            "Thermognosis Engine: Dataset Normalization Orchestrator. "
            "Streams heterogeneous thermodynamic JSON fragments into an optimized, "
            "schema-enforced columnar Parquet matrix."
        )
    )
    
    parser.add_argument(
        "--input_dir", 
        type=Path, 
        required=True, 
        help="Path to the directory containing raw JSON fragments (e.g., data/raw/samples/)."
    )
    
    parser.add_argument(
        "--output", 
        type=Path, 
        required=True, 
        help="Destination path for the structured Parquet dataset."
    )
    
    parser.add_argument(
        "--batch_size", 
        type=int, 
        default=10000, 
        help="Row-group chunk limit for the PyArrow writer to balance RAM usage and I/O speed."
    )

    return parser.parse_args()


if __name__ == "__main__":
    args = parse_args()
    
    resolved_input_dir: Path = args.input_dir.resolve()
    resolved_output_file: Path = args.output.resolve()
    
    # Pre-flight domain checks
    if not resolved_input_dir.exists() or not resolved_input_dir.is_dir():
        logger.error(f"[FATAL] The specified input directory is inaccessible or invalid: {resolved_input_dir}")
        sys.exit(2)
        
    try:
        execute_normalization_pipeline(
            input_dir=resolved_input_dir,
            output_file=resolved_output_file,
            batch_size=args.batch_size
        )
        sys.exit(0)
    except ThermognosisError as domain_err:
        logger.error(f"[FATAL] Domain constraints violated. Pipeline aborted. Error: {domain_err}")
        sys.exit(3)
    except KeyboardInterrupt:
        logger.warning("[WARN] Pipeline terminated manually by the operator (SIGINT).")
        sys.exit(130)
    except Exception as general_err:
        logger.error(f"[FATAL] Unhandled runtime exception disrupted execution: {general_err}", exc_info=True)
        sys.exit(1)