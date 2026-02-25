#!/usr/bin/env python3
import time
import logging
import numpy as np
import pandas as pd
import starrydata as sd
import rust_core

def main():
    logging.basicConfig(level=logging.INFO, format="%(asctime)s - [PIPELINE] - %(levelname)s - %(message)s")
    logger = logging.getLogger("Ingestion")

    logger.info("1. Downloading/Loading Starrydata via Official API...")
    # Tải dataset mới nhất (Nó sẽ cache lại trong máy bạn cho các lần chạy sau)
    dataset = sd.load_dataset() 
    
    # Load bảng curves (chứa giá trị vật lý)
    df_curves = pd.read_csv(dataset.curves_csv, low_memory=False)
    
    logger.info(f"Loaded {len(df_curves):,} raw property rows.")

    # Giả lập việc chúng ta có 4 mảng numpy (S, Sigma, Kappa, T) có cùng độ dài.
    # Trong thực tế, Starrydata lưu rời rạc từng Property (ví dụ: dòng 1 là Seebeck, dòng 2 là Kappa).
    # Để test kết nối NumPy -> Rust, ta tạo các mảng mock ngẫu nhiên dựa trên số liệu thực tế trước 
    # (Bước sau chúng ta sẽ code hàm Pivot dữ liệu thật).
    
    N = 10_000_000 # Test thử sức chịu đựng của Rust với 10 triệu trạng thái
    logger.info(f"2. Generating {N:,} states to stress-test Rust Physics Engine...")
    
    T_arr = np.random.uniform(300, 1000, N).astype(np.float64)       # 300K - 1000K
    S_arr = np.random.uniform(-0.0003, 0.0003, N).astype(np.float64) # V/K
    Sigma_arr = np.random.uniform(100, 100000, N).astype(np.float64) # S/m
    Kappa_arr = np.random.uniform(0.5, 5.0, N).astype(np.float64)    # W/mK

    # Giả lập một vài dữ liệu rác vi phạm định luật Wiedemann-Franz (Kappa cực nhỏ, Sigma cực lớn)
    Kappa_arr[10:100] = 0.00001
    Sigma_arr[10:100] = 9000000.0

    logger.info("3. Pushing Zero-Copy NumPy arrays across FFI to Rust Core...")
    start_time = time.perf_counter()

    # Gọi trực tiếp hàm Rust thông qua PyO3
    zt_results = rust_core.py_compute_zt_batch(S_arr, Sigma_arr, Kappa_arr, T_arr)

    elapsed = time.perf_counter() - start_time

    # Lọc ra các trạng thái vật lý hợp lệ (Rust trả về NaN cho các vi phạm)
    valid_mask = ~np.isnan(zt_results)
    valid_count = np.sum(valid_mask)
    invalid_count = N - valid_count

    logger.info("=" * 50)
    logger.info(" THERMOGNOSIS ENGINE - PHYSICS GATE REPORT ")
    logger.info("=" * 50)
    logger.info(f" Total States Processed : {N:,}")
    logger.info(f" Valid Physical States  : {valid_count:,}")
    logger.info(f" Rejected by Physics    : {invalid_count:,} (P03/P04 Violations)")
    if valid_count > 0:
        logger.info(f" Mean zT                : {np.mean(zt_results[valid_mask]):.4f}")
        logger.info(f" Max zT                 : {np.max(zt_results[valid_mask]):.4f}")
    logger.info("-" * 50)
    logger.info(f" Rust Execution Time    : {elapsed:.6f} seconds")
    logger.info(f" Physics Throughput     : {N / elapsed:,.0f} states/sec")
    logger.info("=" * 50)

if __name__ == "__main__":
    main()