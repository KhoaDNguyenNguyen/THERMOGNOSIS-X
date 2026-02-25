# scripts/run_starrydata_fast.py

import time
import logging
import rust_core

def main():
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s - [THERMOGNOSIS] - %(levelname)s - %(message)s"
    )
    logger = logging.getLogger("Benchmark")
    
    csv_path = "data/20191119_interpolated_data_1k.csv"
    
    logger.info("Initiating High-Performance Rust Core...")
    logger.info(f"Target Dataset: {csv_path}")
    
    start_time = time.perf_counter()
    
    try:
        report = rust_core.compute_zt_from_csv_py(csv_path, deterministic=False)
    except ValueError as e:
        logger.error(f"Rust Core raised an error: {e}")
        return

    elapsed = time.perf_counter() - start_time
    
    logger.info("=" * 45)
    logger.info(" THERMOGNOSIS ENGINE BENCHMARK REPORT ")
    logger.info("=" * 45)
    logger.info(f" Total CSV Rows Read   : {report['total_rows']:,}")
    logger.info(f" Unique Thermo States  : {report['total_states']:,}")
    logger.info("-" * 45)
    logger.info(f" Valid Physical States : {report['valid_states']:,}")
    logger.info(f" Incomplete States     : {report['incomplete_states']:,}")
    logger.info(f" Skipped (Constraint)  : {report['skipped_states']:,}")
    logger.info("-" * 45)
    logger.info(f" Mean zT               : {report['mean_zt']:.6f}")
    logger.info(f" Max zT                : {report['max_zt']:.6f}")
    logger.info(f" Min zT                : {report['min_zt']:.6f}")
    logger.info("-" * 45)
    logger.info(f" Kernel Execution Time : {elapsed:.6f} seconds")
    
    if elapsed > 0:
        throughput = report['total_rows'] / elapsed
        logger.info(f" System Throughput     : {throughput:,.0f} rows/sec")
    logger.info("=" * 45)

if __name__ == "__main__":
    main()