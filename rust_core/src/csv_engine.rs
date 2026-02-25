// rust_core/src/csv_engine.rs

use std::fs::File;
use std::io::BufReader;
use thiserror::Error;
use std::collections::HashMap;

#[derive(Error, Debug)]
pub enum CsvEngineError {
    #[error("I/O Failure: Could not access or read the target dataset. {0}")]
    IoError(#[from] std::io::Error),
    #[error("CSV Parsing Violation: Malformed matrix or boundary format. {0}")]
    CsvError(#[from] csv::Error),
    #[error("Epistemological Data Violation: Required columns not found.")]
    MissingRequiredColumns,
}

#[derive(Clone, Debug, Default)]
pub struct BenchmarkReport {
    pub total_rows: usize,
    pub total_states: usize,       // NEW: Đếm số trạng thái vật lý được gộp
    pub valid_states: usize,       // Đổi tên cho rõ nghĩa (trước là valid_rows)
    pub skipped_states: usize,     // Vi phạm vật lý
    pub incomplete_states: usize,  // NEW: Thiếu S, Sigma, hoặc Kappa
    pub mean_zt: f64,
    pub min_zt: f64,
    pub max_zt: f64,
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct StateKey {
    composition: String,
    sampleid: String,
    temperature: i64,
}

#[derive(Default, Debug)]
struct ThermoState {
    s: Option<f64>,
    sigma: Option<f64>,
    kappa: Option<f64>,
}

pub fn compute_zt_from_csv(path: &str, _deterministic: bool) -> Result<BenchmarkReport, CsvEngineError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut csv_reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(reader);

    let headers = csv_reader.headers()?;

    // Hỗ trợ quét ĐA CỘT (Multi-column scanning) để chống ghi đè
    let mut t_cols = Vec::new();
    let mut s_cols = Vec::new();
    let mut sigma_cols = Vec::new();
    let mut rho_cols = Vec::new();
    let mut kappa_cols = Vec::new();
    let mut comp_col = None;
    let mut sample_col = None;

    for (i, header) in headers.iter().enumerate() {
        let normalized = header.trim().to_lowercase();

        if normalized == "temperature" || normalized == "t" { t_cols.push(i); }
        else if normalized.contains("seebeck") || normalized == "s" || normalized == "tep" || normalized == "thermopower" { s_cols.push(i); }
        else if normalized == "electrical conductivity" || normalized == "conductance" { sigma_cols.push(i); }
        else if normalized.contains("resistivity") || normalized == "ρ" || normalized == "resistyvity" { rho_cols.push(i); }
        else if normalized == "thermal conductivity" || normalized == "total thermal conductivity" || normalized == "kappa" { kappa_cols.push(i); }
        else if normalized == "composition" { comp_col = Some(i); }
        else if normalized.contains("sampleid") { sample_col = Some(i); }
    }

    if t_cols.is_empty() { return Err(CsvEngineError::MissingRequiredColumns); }

    let mut record = csv::StringRecord::new();
    let mut state_map: HashMap<StateKey, ThermoState> = HashMap::new();

    let mut report = BenchmarkReport {
        min_zt: f64::MAX,
        max_zt: f64::MIN,
        ..Default::default()
    };

    while csv_reader.read_record(&mut record)? {
        report.total_rows += 1;

        let t_raw = match extract_first_valid(&record, &t_cols) {
            Some(v) => v,
            None => continue, // Bỏ qua row không có nhiệt độ
        };

        let t_key = t_raw.round() as i64;

        let composition = match comp_col {
            Some(idx) => record.get(idx).unwrap_or("").to_string(),
            None => "".to_string(),
        };
        
        let sampleid = match sample_col {
            Some(idx) => {
                let s = record.get(idx).unwrap_or("unknown").trim();
                if s.is_empty() { "unknown".to_string() } else { s.to_string() }
            },
            None => "unknown".to_string(),
        };

        let key = StateKey { composition, sampleid, temperature: t_key };
        let state = state_map.entry(key).or_insert_with(ThermoState::default);

        // Quét Seebeck
// Quét Seebeck (Đã là V/K trong interpolated data)
        if let Some(v) = extract_first_valid(&record, &s_cols) {
            state.s = Some(v);
        }

        // Quét Electrical Conductivity (S/m) & Resistivity (Ohm.m)
        if let Some(v) = extract_first_valid(&record, &sigma_cols) {
            state.sigma = Some(v);
        } else if let Some(v) = extract_first_valid(&record, &rho_cols) {
            if v > 1e-12 { // Tránh singularity (chia cho 0)
                state.sigma = Some(1.0 / v);
            }
        }

        // Quét Thermal Conductivity (W/mK)
        if let Some(v) = extract_first_valid(&record, &kappa_cols) {
            state.kappa = Some(v);
        }
    }

    report.total_states = state_map.len();

    // ---- Đánh giá ZT sau khi đã gộp và tái tạo trạng thái vật lý ----
// ---- Compute zT after full reconstruction ----
    for (_key, state) in state_map.iter() {
        if let (Some(s), Some(sigma), Some(kappa)) = (state.s, state.sigma, state.kappa) {
            
            let t_si = _key.temperature as f64;

            // 1. Hard Positivity Constraints (P03)
            if t_si <= 0.0 || kappa <= 0.0 || sigma <= 0.0 {
                report.skipped_states += 1;
                continue;
            }

            // 2. Empirical Physical Bounds (P03 - Lọc rác nội suy từ DB)
            // S không thể vượt qua 0.05 V/K (50,000 uV/K)
            // Sigma hiếm khi vượt 10^8 S/m (ngay cả kim loại)
            // Kappa bulk material < 5000 W/mK (kim cương)
            if s.abs() > 0.05 || sigma > 1e8 || kappa > 5000.0 {
                report.skipped_states += 1;
                continue;
            }

            let zt = (s * s * sigma * t_si) / kappa;

            // 3. Thermoelectric Bounds
            if !zt.is_finite() || zt < 0.0 || zt > 5.0 {
                report.skipped_states += 1;
                continue;
            }

            report.valid_states += 1;

            let delta = zt - report.mean_zt;
            report.mean_zt += delta / (report.valid_states as f64);

            if zt < report.min_zt {
                report.min_zt = zt;
            }
            if zt > report.max_zt {
                report.max_zt = zt;
            }
        } else {
            report.incomplete_states += 1;
        }
    }

    if report.valid_states == 0 {
        report.mean_zt = f64::NAN;
        report.min_zt = f64::NAN;
        report.max_zt = f64::NAN;
    }

    Ok(report)
}

#[inline(always)]
fn extract_first_valid(record: &csv::StringRecord, indices: &[usize]) -> Option<f64> {
    for &idx in indices {
        if let Some(raw) = record.get(idx) {
            if let Some(val) = extract_f64_robust(raw) {
                return Some(val);
            }
        }
    }
    None
}

#[inline(always)]
fn extract_f64_robust(raw: &str) -> Option<f64> {
    let trimmed = raw.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let interior = &trimmed[1..trimmed.len() - 1].trim();
        if interior.is_empty() { return None; }
        let terminal_segment = match interior.rfind(',') {
            Some(idx) => &interior[idx + 1..],
            None => interior,
        };
        terminal_segment.trim().parse::<f64>().ok()
    } else {
        trimmed.parse::<f64>().ok()
    }
}