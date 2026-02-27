import pandas as pd

# 1. KIỂM TRA DATA CHÍNH (PARQUET)
print("=== KIỂM TRA DỮ LIỆU ĐÃ LỌC (PARQUET) ===")
df_clean = pd.read_parquet("dataset/processed/starry_clean.parquet")
print(f"Tổng số điểm dữ liệu: {len(df_clean)}")

# Chứng minh 1: Chỉ có các properties về nhiệt điện lọt được vào đây
unique_props = df_clean['property_y'].unique()
print(f"Các loại đo đạc lọt qua phễu:\n{unique_props}\n")


# 2. KIỂM TRA FILE LOG DATA BỊ VỨT (CSV)
print("=== KIỂM TRA LOG DỮ LIỆU BỊ VỨT (CSV) ===")
df_rejected = pd.read_csv("dataset/processed/rejected_non_experimental_lineage.csv")
print(f"Tổng số mẫu bị loại: {len(df_rejected)}")
print(df_rejected[['sample_id', 'composition', 'measurement_type']])