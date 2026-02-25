#!/usr/bin/env python3
import time
import json
import logging
from pathlib import Path
import numpy as np
import pandas as pd
import starrydata as sd
import rust_core

def parse_json_array(val):
    if pd.isna(val) or not isinstance(val, str): return []
    try: return json.loads(val)
    except Exception: return []

def main():
    logging.basicConfig(level=logging.INFO, format="%(asctime)s - [PIPELINE] - %(levelname)s - %(message)s")
    logger = logging.getLogger("RealData")

    data_dir = Path("data")
    data_dir.mkdir(exist_ok=True)
    cache_file = data_dir / "starrydata_curves_cached.csv"

    if not cache_file.exists():
        logger.info("1. Downloading from Starrydata API...")
        dataset = sd.load_dataset()
        df = pd.read_csv(dataset.curves_csv, low_memory=False)
        df.to_csv(cache_file, index=False)
    else:
        logger.info(f"1. Loading from local cache: {cache_file}...")
        df = pd.read_csv(cache_file, low_memory=False)

    prop_col = 'prop_y' if 'prop_y' in df.columns else 'property_name'
    unit_col = 'unit_y' if 'unit_y' in df.columns else 'unit'

    # NEW: Thêm ZT vào target properties để kiểm chứng chéo
    target_props = {
        'Seebeck coefficient': 'S',
        'Electrical conductivity': 'Sigma',
        'Electrical resistivity': 'Rho',
        'Thermal conductivity': 'Kappa',
        'ZT': 'ZT_reported'
    }

    df = df[df[prop_col].isin(target_props.keys()) & (df['prop_x'] == 'Temperature')].copy()

    logger.info("2. Exploding arrays and aligning coordinates...")
    df['data_x'] = df['x'].apply(parse_json_array)
    df['data_y'] = df['y'].apply(parse_json_array)
    df = df.explode(['data_x', 'data_y'])
    df['data_x'] = pd.to_numeric(df['data_x'], errors='coerce')
    df['data_y'] = pd.to_numeric(df['data_y'], errors='coerce')
    df = df.dropna(subset=['data_x', 'data_y'])
    df['prop_mapped'] = df[prop_col].map(target_props)

    # =========================================================================
    # SPEC-UNIT-CONVERTER (FIXED REGEX with r'')
    # =========================================================================
    logger.info("3. Applying strict SI Unit Normalization...")
    
    mask_s = df['prop_mapped'] == 'S'
    df.loc[mask_s & df[unit_col].str.contains(r'uV|muV|μV', na=False, regex=True), 'data_y'] *= 1e-6
    df.loc[mask_s & df[unit_col].str.contains(r'mV', na=False), 'data_y'] *= 1e-3

    mask_sig = df['prop_mapped'] == 'Sigma'
    df.loc[mask_sig & df[unit_col].str.contains(r'cm', na=False), 'data_y'] *= 100.0
    df.loc[mask_sig & df[unit_col].str.contains(r'10\^3', na=False), 'data_y'] *= 1e3
    df.loc[mask_sig & df[unit_col].str.contains(r'10\^4', na=False), 'data_y'] *= 1e4

    mask_rho = df['prop_mapped'] == 'Rho'
    df.loc[mask_rho & df[unit_col].str.contains(r'mOhm.*cm', na=False, regex=True), 'data_y'] *= 1e-5
    df.loc[mask_rho & df[unit_col].str.contains(r'uOhm.*cm|μOhm.*cm', na=False, regex=True), 'data_y'] *= 1e-8
    df.loc[mask_rho & df[unit_col].str.contains(r'Ohm.*cm', na=False, regex=True) & ~df[unit_col].str.contains(r'mOhm|uOhm|μOhm', regex=True, na=False), 'data_y'] *= 1e-2
    df.loc[mask_rho & df[unit_col].str.contains(r'uOhm.*m|μOhm.*m', na=False, regex=True) & ~df[unit_col].str.contains(r'cm', na=False), 'data_y'] *= 1e-6

    mask_kap = df['prop_mapped'] == 'Kappa'
    df.loc[mask_kap & df[unit_col].str.contains(r'mW', na=False) & df[unit_col].str.contains(r'cm', na=False), 'data_y'] *= 0.1
    df.loc[mask_kap & df[unit_col].str.contains(r'mW', na=False) & df[unit_col].str.contains(r'm\^', na=False), 'data_y'] *= 1e-3

    logger.info("4. Binning Temperature and Pivoting to state matrix...")
    df['T_rounded'] = (df['data_x'] / 10).round() * 10
    df_grouped = df.groupby(['sample_id', 'T_rounded', 'prop_mapped'])['data_y'].mean().reset_index()
    pivot_df = df_grouped.pivot(index=['sample_id', 'T_rounded'], columns='prop_mapped', values='data_y').reset_index()

    if 'Rho' in pivot_df.columns and 'Sigma' in pivot_df.columns:
        mask_rho_valid = pivot_df['Sigma'].isna() & pivot_df['Rho'].notna() & (pivot_df['Rho'] > 1e-12)
        pivot_df.loc[mask_rho_valid, 'Sigma'] = 1.0 / pivot_df.loc[mask_rho_valid, 'Rho']

    # Lấy các sample có đủ 3 thông số vật lý (để tính zT)
    required_cols = ['S', 'Sigma', 'Kappa']
    for col in required_cols:
        if col not in pivot_df.columns: pivot_df[col] = np.nan
    final_df = pivot_df.dropna(subset=required_cols).copy()
    
    # Check nếu dataset có cột ZT_reported
    if 'ZT_reported' not in final_df.columns:
        final_df['ZT_reported'] = np.nan

    logger.info("5. Pushing arrays to Rust Physics Engine...")
    T_arr = np.ascontiguousarray(final_df['T_rounded'].values, dtype=np.float64)
    S_arr = np.ascontiguousarray(final_df['S'].values, dtype=np.float64)
    Sigma_arr = np.ascontiguousarray(final_df['Sigma'].values, dtype=np.float64)
    Kappa_arr = np.ascontiguousarray(final_df['Kappa'].values, dtype=np.float64)

    start_rust = time.perf_counter()
    zt_results = rust_core.py_compute_zt_batch(S_arr, Sigma_arr, Kappa_arr, T_arr)
    rust_elapsed = time.perf_counter() - start_rust

    valid_mask = ~np.isnan(zt_results)
    final_df['zT_computed'] = zt_results
    valid_df = final_df[valid_mask].copy()

    # =========================================================================
    # SPEC-PHYS-CONSISTENCY: INTERNAL CONFLICT DETECTION
    # =========================================================================
    # So sánh ZT tính toán và ZT tác giả báo cáo (nếu có)
    cross_check_df = valid_df.dropna(subset=['ZT_reported']).copy()
    if not cross_check_df.empty:
        cross_check_df['zT_Error'] = np.abs(cross_check_df['zT_computed'] - cross_check_df['ZT_reported'])
        # Flag những điểm có sai số > 10%
        inconsistent_states = cross_check_df[cross_check_df['zT_Error'] > 0.1 * cross_check_df['zT_computed']]
    else:
        inconsistent_states = pd.DataFrame()

    logger.info("=" * 60)
    logger.info(" THERMOGNOSIS ENGINE - RIGOROUS PHYSICS REPORT ")
    logger.info("=" * 60)
    logger.info(f" Total Mapped States   : {len(final_df):,}")
    logger.info(f" Valid Physical States : {len(valid_df):,}")
    logger.info(f" Rejected by Physics   : {len(final_df) - len(valid_df):,}")
    
    if len(valid_df) > 0:
        logger.info("-" * 60)
        logger.info(f" Median zT             : {valid_df['zT_computed'].median():.4f}")
        logger.info(f" 90th Percentile zT    : {valid_df['zT_computed'].quantile(0.90):.4f}")
        logger.info(f" Max zT                : {valid_df['zT_computed'].max():.4f}")
        logger.info("-" * 60)
        
        logger.info(" CROSS-VALIDATION (SPEC-PHYS-CONSISTENCY):")
        logger.info(f" States w/ Reported ZT : {len(cross_check_df):,}")
        logger.info(f" Internally Inconsistent: {len(inconsistent_states):,} (Deviation > 10%)")
        
        if not inconsistent_states.empty:
            top_bad = inconsistent_states.sort_values('zT_Error', ascending=False).head(3)
            logger.info(" TOP 3 WORST INTERNAL CONFLICTS (Computed vs Claimed):")
            for _, row in top_bad.iterrows():
                logger.info(f"   Sample ID: {int(row['sample_id']):<6} | T: {row['T_rounded']}K | Computed: {row['zT_computed']:.3f} | Claimed: {row['ZT_reported']:.3f}")
            
    logger.info("-" * 60)
    logger.info(f" Rust Kernel Time      : {rust_elapsed:.6f} seconds")
    logger.info("=" * 60)

if __name__ == "__main__":
    main()