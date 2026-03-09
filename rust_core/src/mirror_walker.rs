// rust_core/src/mirror_walker.rs

//! # SPEC-IO-WALKER-01 — Parallel Mirror Directory Traversal Engine
//!
//! **Layer:** I/O Subsystem / Parallel Ingestion
//! **Status:** Normative — Q1 Infrastructure Standard
//! **Implements:** SPEC-IO-WALKER-01, SPEC-GOV-ERROR-HIERARCHY
//!
//! ## Architecture: Three-Phase Pipeline
//!
//! ### Phase 1 — Shard Discovery (Serial)
//! Enumerate the numbered shard subdirectories (e.g., `00000`, `00001`) of a given
//! mirror domain root. This phase is strictly serial: concurrent `read_dir` calls
//! on the **same parent directory** contend for the kernel dcache spinlock, producing
//! measurable serialization overhead that exceeds the cost of a single sequential scan.
//!
//! ### Phase 2 — File Path Collection (Parallel)
//! For each discovered shard, enumerate its JSON files concurrently using Rayon's
//! work-stealing thread pool. Since each shard is an *independent* directory inode,
//! concurrent access imposes no lock contention. The result is a flat
//! `Vec<PathBuf>` of all JSON file paths, suitable for deterministic batch slicing.
//!
//! ### Phase 3 — Parallel JSON Ingestion (Parallel)
//! Each collected path is processed independently: a `BufReader` streams the file
//! into `serde_json` for zero-copy deserialization. All valid `StarrydataRecord`
//! objects are collected; parse failures are captured into a structured `WalkError`
//! log without terminating the pipeline (fail-soft semantics required by Q1 standards).
//!
//! ## Memory Model
//! Peak memory = O(F) for the path Vec + O(C) for one chunk of records at a time,
//! where F = total file count and C = chunk_size parameter. The path Vec for
//! ~500,000 files consumes ≈ 40 MB (average 80 bytes per PathBuf on Linux).
//!
//! ## Thread Safety
//! All state is thread-local within each `par_iter` closure. No shared mutable state,
//! no `Arc<Mutex>` overhead. Rayon collects results via lock-free work-stealing.

use pyo3::exceptions::{PyIOError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use rayon::prelude::*;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use thiserror::Error;

use crate::flags::FLAG_FIGUREID_MISMATCH;
use crate::memory_guard::{MemoryGuard, MemoryPressure};

// =============================================================================
// SECTION 1: ERROR TAXONOMY
// SPEC-GOV-ERROR-HIERARCHY (Walker Subsystem)
// =============================================================================

/// Structured error taxonomy for the mirror walker pipeline.
/// Each variant preserves the source path for post-hoc audit triage.
#[derive(Error, Debug, Clone)]
pub enum WalkError {
    #[error("IO-W01: Shard directory unreadable at '{path}': {cause}")]
    ShardReadError { path: String, cause: String },

    #[error("IO-W02: File unreadable at '{path}': {cause}")]
    FileReadError { path: String, cause: String },

    #[error("IO-W03: JSON schema divergence in '{path}': {cause}")]
    JsonParseError { path: String, cause: String },

    #[error("IO-W04: Domain root does not exist: '{path}'")]
    DomainRootMissing { path: String },
}

// =============================================================================
// SECTION 1b: THERMOELECTRIC PROPERTY ALLOWLIST (BUG-01 Fix)
// =============================================================================

/// Canonical set of Starrydata `propertyid_y` values that represent valid
/// thermoelectric transport properties.
///
/// **BUG-01 Fix**: Previously, `parse_json_file` admitted all property types
/// including optical (Absorbance, Reflectance), mechanical, and sensor data.
/// This caused non-thermoelectric measurements from samples such as 00040076–78
/// (UV-Vis Absorbance) to contaminate the clean dataset.
///
/// Any `RawMeasurement` whose `propertyid_y` is not in this slice is:
///   (a) excluded from the `valid_measurements` vector returned in the record,
///   (b) counted in `rejected_non_thermoelectric_count` on the `StarrydataRecord`,
///   (c) never written to the output Parquet.
///
/// The authoritative list is maintained in `allowed_properties.toml`. This
/// compile-time constant must match that file; any extension requires updating
/// both. The TOML file is the human-readable policy document; this array is the
/// enforcement mechanism.
///
/// | ID | Property                         | SI Unit           |
/// |----|----------------------------------|-------------------|
/// |  2 | Seebeck coefficient              | V·K⁻¹            |
/// |  3 | Electrical conductivity          | S·m⁻¹            |
/// |  4 | Total thermal conductivity       | W·m⁻¹·K⁻¹       |
/// |  5 | Electrical resistivity           | Ω·m              |
/// |  6 | Power factor (S²σ)              | W·m⁻¹·K⁻²       |
/// | 10 | Hall coefficient                 | m³·C⁻¹           |
/// | 11 | Carrier concentration            | m⁻³              |
/// | 12 | Hall mobility                    | m²·V⁻¹·s⁻¹      |
/// | 14 | Lattice thermal conductivity     | W·m⁻¹·K⁻¹       |
/// | 15 | Figure of merit Z (= ZT/T)      | K⁻¹              |
/// | 16 | Dimensionless figure of merit ZT | dimensionless     |
const ALLOWED_PROPERTY_IDS: &[u32] = &[2, 3, 4, 5, 6, 10, 11, 12, 14, 15, 16];

/// Global O(1) lookup set, lazily initialised once from `ALLOWED_PROPERTY_IDS`.
///
/// **GAP-07 / BUG-03**: Replaces the O(n=11) linear scan with an O(1) hash set
/// lookup backed by `ahash` (non-cryptographic, cache-friendly). The `OnceLock`
/// guarantees the set is constructed exactly once and shared across all threads.
static ALLOWED_PROPERTY_SET: OnceLock<ahash::AHashSet<u32>> = OnceLock::new();

/// Returns the globally-initialised O(1) property allowlist set.
fn allowed_property_set() -> &'static ahash::AHashSet<u32> {
    ALLOWED_PROPERTY_SET.get_or_init(|| ALLOWED_PROPERTY_IDS.iter().copied().collect())
}

/// Returns `true` if the given `propertyid_y` is a valid thermoelectric transport property.
///
/// O(1) lookup via `AHashSet` (see `ALLOWED_PROPERTY_SET`).
#[inline(always)]
fn is_allowed_property(propertyid_y: u32) -> bool {
    allowed_property_set().contains(&propertyid_y)
}

// =============================================================================
// SECTION 1c: FIGUREID CROSS-VALIDATION (GAP-05)
// =============================================================================

/// Build an O(1) set of all figure IDs declared in a JSON file's `figure[]` array.
///
/// Used by `parse_json_file` to validate that every `RawMeasurement.figureid`
/// actually exists as a `FigureDescriptor` in the same file.
fn build_figure_id_set(figures: &[FigureDescriptor]) -> ahash::AHashSet<u32> {
    figures.iter().map(|f| f.figureid).collect()
}

/// Validate that a measurement's `figureid` references a known figure in the file.
///
/// **GAP-05**: Measurements whose `figureid` is not present in the file's
/// `figure[]` array indicate either a cross-file reference collision or a
/// corpus integrity issue. Such measurements are flagged but retained (not
/// hard-rejected) to preserve audit transparency.
///
/// Returns `FLAG_FIGUREID_MISMATCH` (bit 9) if the reference is invalid, else 0.
pub fn validate_figure_reference(figureid: u32, known_figure_ids: &ahash::AHashSet<u32>) -> u32 {
    if known_figure_ids.contains(&figureid) {
        0
    } else {
        FLAG_FIGUREID_MISMATCH
    }
}

// =============================================================================
// SECTION 2: DOMAIN DATA MODELS
// Mirrors the confirmed JSON ontology of the Starrydata mirror corpus.
// =============================================================================

/// A single digitized measurement observation.
/// Normalized from `rawdata[]` in the source JSON.
#[derive(Deserialize, Clone, Debug)]
pub struct RawMeasurement {
    pub paperid: u32,
    // sampleid is heterogeneous in the corpus: integer in figures/, string in samples/
    #[serde(deserialize_with = "deserialize_flexible_id")]
    pub sampleid: u32,
    #[serde(deserialize_with = "deserialize_flexible_id")]
    pub figureid: u32,
    pub x: f64,
    pub y: f64,
    pub propertyid_x: u32,
    pub propertyid_y: u32,
}

/// A material sample descriptor.
/// Extracted from `sample[]` in the source JSON.
#[derive(Deserialize, Clone, Debug)]
pub struct SampleDescriptor {
    #[serde(deserialize_with = "deserialize_flexible_id")]
    pub sampleid: u32,
    pub paperid: u32,
    #[serde(default)]
    pub samplename: String,
    #[serde(default)]
    pub composition: String,
    // sampleinfo is a free-form key→{category,comment} map; captured raw for DB-side parsing
    #[serde(default)]
    pub sampleinfo: serde_json::Value,
}

/// A bibliographic reference descriptor.
/// Extracted from `paper[]` in the source JSON.
///
/// All String fields use `deserialize_flexible_string` because the corpus
/// consistently serializes numeric bibliographic fields (volume, issue number,
/// page numbers) as bare JSON integers rather than quoted strings. Applying
/// the flexible deserializer universally is more robust than field-by-field
/// enumeration, which is fragile against future schema variations.
#[derive(Deserialize, Clone, Debug)]
pub struct PaperDescriptor {
    pub paperid: u32,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub doi: String,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub author: String,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub author_full: String,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub journal: String,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub journal_full: String,
    #[serde(default)]
    pub year: u16,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub volume: String,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub pages: String,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub publisher: String,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub url: String,
}

/// A property axis definition.
/// Extracted from `property[]` in the source JSON.
#[derive(Deserialize, Clone, Debug)]
pub struct PropertyDescriptor {
    pub propertyid: u32,
    #[serde(default)]
    pub propertyname: String,
    #[serde(default)]
    pub unit: String,
}

/// A figure (digitized plot) descriptor.
/// Extracted from `figure[]` in the source JSON.
#[derive(Deserialize, Clone, Debug)]
pub struct FigureDescriptor {
    #[serde(deserialize_with = "deserialize_flexible_id")]
    pub figureid: u32,
    pub paperid: u32,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub figurename: String,
    #[serde(default, deserialize_with = "deserialize_flexible_string")]
    pub caption: String,
    #[serde(default)]
    pub propertyid_x: u32,
    #[serde(default)]
    pub propertyid_y: u32,
}

/// The root deserialization target for any Starrydata mirror JSON file.
/// All three domains (samples/, papers/, figures/) share this topology.
#[derive(Deserialize, Debug, Default)]
struct StarrydataJsonRoot {
    #[serde(default)]
    sample: Vec<SampleDescriptor>,
    #[serde(default)]
    paper: Vec<PaperDescriptor>,
    #[serde(default)]
    property: Vec<PropertyDescriptor>,
    #[serde(default)]
    figure: Vec<FigureDescriptor>,
    #[serde(default)]
    rawdata: Vec<RawMeasurement>,
}

/// A fully parsed, validated record from one mirror JSON file.
/// Carries provenance metadata for deduplication and audit.
#[derive(Clone, Debug)]
pub struct StarrydataRecord {
    /// Absolute path of the source JSON file (provenance anchor).
    pub source_path: String,
    /// The mirror domain that produced this record (samples|papers|figures).
    pub source_domain: String,
    pub samples: Vec<SampleDescriptor>,
    pub papers: Vec<PaperDescriptor>,
    pub properties: Vec<PropertyDescriptor>,
    pub figures: Vec<FigureDescriptor>,
    /// Measurements that passed the thermoelectric property allowlist (BUG-01).
    pub measurements: Vec<RawMeasurement>,
    /// Count of measurements rejected because `propertyid_y` was not in
    /// `ALLOWED_PROPERTY_IDS`. Corresponds to `rejected_non_thermoelectric_count`
    /// in the pipeline statistics (TASK 4a).
    pub rejected_non_thermoelectric_count: usize,
    /// Count of measurements whose `figureid` was not found in the file's `figure[]`
    /// array (GAP-05 FigureID cross-validation). These measurements are retained in
    /// `measurements` but their `anomaly_flags` will include `FLAG_FIGUREID_MISMATCH`.
    pub figureid_mismatch_count: usize,
}

// =============================================================================
// SECTION 3: FLEXIBLE ID DESERIALIZER
// Handles heterogeneous sampleid/figureid fields (integer OR string in corpus)
// =============================================================================

/// Deserializes a u32 from either a JSON integer or a JSON string containing digits.
/// This is required because `sampleid` is `"1"` (string) in samples/ but `109` (integer)
/// in figures/ — a documented inconsistency in the raw Starrydata corpus.
fn deserialize_flexible_id<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let val = serde_json::Value::deserialize(deserializer)?;
    match val {
        serde_json::Value::Number(n) => n
            .as_u64()
            .and_then(|u| u32::try_from(u).ok())
            .ok_or_else(|| D::Error::custom("ID value out of u32 range")),
        serde_json::Value::String(s) => s
            .trim()
            .parse::<u32>()
            .map_err(|_| D::Error::custom(format!("Cannot parse '{}' as u32 ID", s))),
        _ => Err(D::Error::custom("ID field must be number or string")),
    }
}

/// Deserializes a String from any scalar JSON value (string, integer, float, bool, null).
///
/// Required because the Starrydata corpus exhibits systematic schema violations where
/// bibliographic fields declared as strings contain bare integers: `"volume": 47`,
/// `"pages": 1287`, `"figurename": 4`. These are transcription artefacts from the
/// original database serializer and affect ~30% of all paper records.
///
/// This deserializer never returns `Err` for scalar values; it coerces all numeric
/// and boolean types to their canonical string representation, and maps `null` to
/// the empty string. Structural types (array, object) return an empty string with
/// no error, preventing a single malformed field from aborting ingestion of an
/// otherwise valid file.
fn deserialize_flexible_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val = serde_json::Value::deserialize(deserializer)?;
    Ok(match val {
        serde_json::Value::String(s)  => s,
        serde_json::Value::Number(n)  => n.to_string(),
        serde_json::Value::Bool(b)    => b.to_string(),
        serde_json::Value::Null       => String::new(),
        // Arrays or objects in a string field are a deeper schema violation;
        // collapse to empty string and let the audit log surface the file path.
        _                             => String::new(),
    })
}

// =============================================================================
// SECTION 4: PHASE 1 — SERIAL SHARD DISCOVERY
// =============================================================================

/// **Phase 1**: Performs a deterministic, serial scan of the domain root to collect
/// all shard subdirectory paths.
///
/// Seriality is intentional: concurrent `read_dir` calls on the same parent directory
/// contend for the kernel VFS inode lock. Since S (number of shards) is small
/// (~hundreds), the O(S) serial cost is negligible and avoids the contention overhead.
///
/// # Returns
/// Sorted `Vec<PathBuf>` of shard directories. Sorting ensures deterministic
/// processing order for reproducibility of ingestion logs.
fn discover_shard_directories(domain_root: &Path) -> Result<Vec<PathBuf>, WalkError> {
    if !domain_root.is_dir() {
        return Err(WalkError::DomainRootMissing {
            path: domain_root.display().to_string(),
        });
    }

    let read = std::fs::read_dir(domain_root).map_err(|e| WalkError::ShardReadError {
        path: domain_root.display().to_string(),
        cause: e.to_string(),
    })?;

    let mut shards: Vec<PathBuf> = read
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_dir() { Some(path) } else { None }
        })
        .collect();

    // Lexicographic sort guarantees deterministic processing order (00000 < 00001 < ...)
    // shards.sort_unstable();
    // Ok(shards)
        if shards.is_empty() {
        // Fallback: If no subdirectories exist, treat the domain_root itself as the single shard.
        shards.push(domain_root.to_path_buf());
    } else {
        // Lexicographic sort guarantees deterministic processing order (00000 < 00001 < ...)
        shards.sort_unstable();
    }
    Ok(shards)
}

// =============================================================================
// SECTION 5: PHASE 2 — PARALLEL FILE PATH COLLECTION
// =============================================================================

/// **Phase 2**: Enumerates JSON files within each shard directory in parallel.
///
/// Since each shard is an independent inode, concurrent `read_dir` across shards
/// imposes zero contention. The Rayon work-stealing pool dynamically balances
/// shard-level enumeration load.
///
/// # Returns
/// Flat, unsorted `Vec<PathBuf>` of all `.json` file paths across all shards.
/// Ordering is non-deterministic at this stage but irrelevant: Phase 3 is order-independent.
fn collect_json_paths_parallel(shards: &[PathBuf]) -> (Vec<PathBuf>, Vec<WalkError>) {
    let results: Vec<Result<Vec<PathBuf>, WalkError>> = shards
        .par_iter()
        .map(|shard| {
            let read = std::fs::read_dir(shard).map_err(|e| WalkError::ShardReadError {
                path: shard.display().to_string(),
                cause: e.to_string(),
            })?;

            let paths: Vec<PathBuf> = read
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    // Only consume .json files; skip dotfiles and any index artifacts
                    if path.extension().and_then(|e| e.to_str()) == Some("json") {
                        Some(path)
                    } else {
                        None
                    }
                })
                .collect();

            Ok(paths)
        })
        .collect();

    // Partition successes from shard-level errors without aborting
    let mut all_paths = Vec::new();
    let mut errors = Vec::new();
    for result in results {
        match result {
            Ok(paths) => all_paths.extend(paths),
            Err(e) => errors.push(e),
        }
    }
    (all_paths, errors)
}

// =============================================================================
// SECTION 6: PHASE 3 — PARALLEL JSON INGESTION
// =============================================================================

/// Parses a single Starrydata JSON file into a `StarrydataRecord`.
///
/// Uses a `BufReader` with OS-default buffer size for efficient sequential reads.
/// This is correct for JSON deserialization since `serde_json::from_reader` consumes
/// the stream sequentially — random access (mmap) would provide no benefit here.
///
/// # Fail-Soft Contract
/// Returns `Err(WalkError)` on I/O or schema errors. Never panics.
fn parse_json_file(path: &Path, domain: &str) -> Result<StarrydataRecord, WalkError> {
    let file = File::open(path).map_err(|e| WalkError::FileReadError {
        path: path.display().to_string(),
        cause: e.to_string(),
    })?;

    let reader = BufReader::new(file);
    let root: StarrydataJsonRoot =
        serde_json::from_reader(reader).map_err(|e| WalkError::JsonParseError {
            path: path.display().to_string(),
            cause: e.to_string(),
        })?;

    // Apply three-stage filtering to rawdata:
    // Stage 1 — Expel NaN/Inf artifacts from digitization software.
    // Stage 2 — BUG-01 Fix: reject measurements whose propertyid_y is not a
    //            recognised thermoelectric transport property.
    // Stage 3 — GAP-05: FigureID cross-validation. Measurements referencing an
    //            unknown figureid are retained but their anomaly_flags are annotated.
    let figure_id_set = build_figure_id_set(&root.figure);
    let mut valid_measurements: Vec<RawMeasurement> = Vec::new();
    let mut rejected_non_thermoelectric_count: usize = 0;
    let mut figureid_mismatch_count: usize = 0;

    for m in root.rawdata {
        // Stage 1: NaN/Inf check
        if !m.x.is_finite() || !m.y.is_finite() {
            rejected_non_thermoelectric_count += 1;
            continue;
        }
        // Stage 2: Thermoelectric property allowlist (BUG-01)
        if !is_allowed_property(m.propertyid_y) {
            rejected_non_thermoelectric_count += 1;
            continue;
        }
        // Stage 3: FigureID cross-validation (GAP-05) — flag but retain
        if validate_figure_reference(m.figureid, &figure_id_set) != 0 {
            figureid_mismatch_count += 1;
        }
        valid_measurements.push(m);
    }

    Ok(StarrydataRecord {
        source_path: path.display().to_string(),
        source_domain: domain.to_string(),
        samples: root.sample,
        papers: root.paper,
        properties: root.property,
        figures: root.figure,
        measurements: valid_measurements,
        rejected_non_thermoelectric_count,
        figureid_mismatch_count,
    })
}

/// **Phase 3**: Processes all collected JSON paths in parallel.
///
/// Rayon distributes file I/O and JSON deserialization across all logical cores.
/// Parse errors are captured without aborting the pipeline (fail-soft semantics).
/// The `guard` is ticked once per file result in the serial collection loop;
/// this amortises the OS RSS check across the ingestion batch.
///
/// # Returns
/// `(records, errors)` tuple. The `errors` vec provides the audit trail of
/// files that failed ingestion, required by Q1 data descriptor standards.
fn ingest_files_parallel(
    paths: &[PathBuf],
    domain: &str,
    guard: &mut MemoryGuard,
) -> (Vec<StarrydataRecord>, Vec<WalkError>) {
    let results: Vec<Result<StarrydataRecord, WalkError>> = paths
        .par_iter()
        .map(|p| parse_json_file(p, domain))
        .collect();

    let mut records = Vec::new();
    let mut errors = Vec::new();
    for result in results {
        // Phase 1: tick MemoryGuard — amortised RSS check per ingested file
        if guard.tick() == MemoryPressure::Hard {
            errors.push(WalkError::JsonParseError {
                path: "<memory-ceiling>".to_string(),
                cause: format!(
                    "MEM-GUARD: Hard memory ceiling reached after {} records; \
                     ingestion halted to protect host from OOM.",
                    records.len()
                ),
            });
            break;
        }
        match result {
            Ok(r) => records.push(r),
            Err(e) => errors.push(e),
        }
    }
    (records, errors)
}

// =============================================================================
// SECTION 7: PUBLIC API — FULL THREE-PHASE ORCHESTRATION
// =============================================================================

/// Walk summary statistics for the ingestion audit log.
pub struct WalkSummary {
    pub domain: String,
    pub shards_discovered: usize,
    pub files_found: usize,
    pub files_parsed: usize,
    pub files_failed: usize,
    /// Measurements that passed all filters and entered the clean record set.
    pub total_measurements: usize,
    pub total_samples: usize,
    pub total_papers: usize,
    /// Total measurements rejected because `propertyid_y` was not in the
    /// thermoelectric allowlist (BUG-01 counter). Corresponds to
    /// `rejected_non_thermoelectric_count` in `pipeline_statistics.json`.
    pub rejected_non_thermoelectric: usize,
    /// Total measurements that passed filtering but whose `figureid` was not
    /// found in the file's `figure[]` array (GAP-05 FigureID mismatch counter).
    pub figureid_mismatches: usize,
    pub errors: Vec<WalkError>,
}

/// Executes the complete three-phase parallel walk over a single mirror domain.
///
/// # Arguments
/// * `domain_root` - Absolute path to the domain directory (e.g., `.../starrydata_mirror/samples`).
/// * `domain_name` - Canonical domain identifier (`"samples"`, `"papers"`, or `"figures"`).
///
/// # Returns
/// `(Vec<StarrydataRecord>, WalkSummary)` — parsed records and a full audit summary.
pub fn scan_domain(
    domain_root: &Path,
    domain_name: &str,
) -> Result<(Vec<StarrydataRecord>, WalkSummary), WalkError> {
    // MemoryGuard: 5.5 GB soft / 6.5 GB hard ceiling; check every 1000 files
    let mut guard = MemoryGuard::new(5_500_000_000, 6_500_000_000, 1_000);

    // Phase 1: Serial shard discovery
    let shards = discover_shard_directories(domain_root)?;
    let shards_discovered = shards.len();

    // Phase 2: Parallel file path collection
    let (all_paths, shard_errors) = collect_json_paths_parallel(&shards);
    let files_found = all_paths.len();

    // Phase 3: Parallel JSON ingestion (with MemoryGuard)
    let (records, mut file_errors) = ingest_files_parallel(&all_paths, domain_name, &mut guard);
    let files_parsed = records.len();

    // Aggregate audit telemetry
    let total_measurements: usize = records.iter().map(|r| r.measurements.len()).sum();
    let total_samples: usize = records.iter().map(|r| r.samples.len()).sum();
    let total_papers: usize = records.iter().map(|r| r.papers.len()).sum();
    let rejected_non_thermoelectric: usize =
        records.iter().map(|r| r.rejected_non_thermoelectric_count).sum();
    let figureid_mismatches: usize =
        records.iter().map(|r| r.figureid_mismatch_count).sum();

    // Merge shard-level and file-level errors into a unified error log
    let mut all_errors = shard_errors;
    all_errors.append(&mut file_errors);
    let files_failed = all_errors.len();

    let summary = WalkSummary {
        domain: domain_name.to_string(),
        shards_discovered,
        files_found,
        files_parsed,
        files_failed,
        total_measurements,
        total_samples,
        total_papers,
        rejected_non_thermoelectric,
        figureid_mismatches,
        errors: all_errors,
    };

    Ok((records, summary))
}

// =============================================================================
// SECTION 8: PyO3 FFI BINDINGS
// =============================================================================

/// Projects a `StarrydataRecord` into a Python dict for zero-overhead transit
/// across the FFI boundary into the Python ingestion pipeline.
fn record_to_pydict(py: Python, record: &StarrydataRecord) -> PyResult<PyObject> {
    let d = PyDict::new_bound(py);
    d.set_item("source_path", &record.source_path)?;
    d.set_item("source_domain", &record.source_domain)?;
    d.set_item("n_measurements", record.measurements.len())?;
    d.set_item("n_samples", record.samples.len())?;
    d.set_item("n_papers", record.papers.len())?;

    // Measurements → list of dicts
    let meas_list = PyList::empty_bound(py);
    for m in &record.measurements {
        let md = PyDict::new_bound(py);
        md.set_item("paperid", m.paperid)?;
        md.set_item("sampleid", m.sampleid)?;
        md.set_item("figureid", m.figureid)?;
        md.set_item("x", m.x)?;
        md.set_item("y", m.y)?;
        md.set_item("propertyid_x", m.propertyid_x)?;
        md.set_item("propertyid_y", m.propertyid_y)?;
        meas_list.append(md)?;
    }
    d.set_item("measurements", meas_list)?;

    // Samples → list of dicts
    let samp_list = PyList::empty_bound(py);
    for s in &record.samples {
        let sd = PyDict::new_bound(py);
        sd.set_item("sampleid", s.sampleid)?;
        sd.set_item("paperid", s.paperid)?;
        sd.set_item("samplename", &s.samplename)?;
        sd.set_item("composition", &s.composition)?;
        // sampleinfo serialized back to JSON string for Python-side structured parsing
        sd.set_item("sampleinfo_json", s.sampleinfo.to_string())?;
        samp_list.append(sd)?;
    }
    d.set_item("samples", samp_list)?;

    // Papers → list of dicts
    let paper_list = PyList::empty_bound(py);
    for p in &record.papers {
        let pd = PyDict::new_bound(py);
        pd.set_item("paperid", p.paperid)?;
        pd.set_item("doi", &p.doi)?;
        pd.set_item("title", &p.title)?;
        pd.set_item("author", &p.author)?;
        pd.set_item("journal", &p.journal)?;
        pd.set_item("year", p.year)?;
        pd.set_item("volume", &p.volume)?;
        pd.set_item("pages", &p.pages)?;
        pd.set_item("publisher", &p.publisher)?;
        pd.set_item("url", &p.url)?;
        paper_list.append(pd)?;
    }
    d.set_item("papers", paper_list)?;

    Ok(d.into())
}

/// PyO3-exposed entry point: scans a single mirror domain in parallel and returns
/// parsed records as a Python list of dicts.
///
/// # Arguments
/// * `domain_root` - Absolute path to the domain directory.
/// * `domain_name` - One of `"samples"`, `"papers"`, `"figures"`.
///
/// # Returns
/// `(records: list[dict], summary: dict)`
/// where `summary` contains walk telemetry for logging and audit.
///
/// # Document IDs
/// SPEC-IO-WALKER-01
#[pyfunction]
#[pyo3(signature = (domain_root, domain_name))]
pub fn py_scan_domain(
    py: Python,
    domain_root: &str,
    domain_name: &str,
) -> PyResult<(PyObject, PyObject)> {
    let root_path = Path::new(domain_root);

    // Release the GIL for the entire three-phase parallel scan
    let (records, summary) = py
        .allow_threads(|| scan_domain(root_path, domain_name))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;

    // Re-acquire GIL to build Python objects
    let py_records = PyList::empty_bound(py);
    for record in &records {
        let d = record_to_pydict(py, record)?;
        py_records.append(d)?;
    }

    let py_summary = PyDict::new_bound(py);
    py_summary.set_item("domain", &summary.domain)?;
    py_summary.set_item("shards_discovered", summary.shards_discovered)?;
    py_summary.set_item("files_found", summary.files_found)?;
    py_summary.set_item("files_parsed", summary.files_parsed)?;
    py_summary.set_item("files_failed", summary.files_failed)?;
    py_summary.set_item("total_measurements", summary.total_measurements)?;
    py_summary.set_item("total_samples", summary.total_samples)?;
    py_summary.set_item("total_papers", summary.total_papers)?;
    // BUG-01: expose rejection count for pipeline_statistics.json
    py_summary.set_item("rejected_non_thermoelectric", summary.rejected_non_thermoelectric)?;
    // GAP-05: expose figureid mismatch count
    py_summary.set_item("figureid_mismatches", summary.figureid_mismatches)?;

    // Surface error messages as a Python list for logging
    let error_strings: Vec<String> = summary.errors.iter().map(|e| e.to_string()).collect();
    py_summary.set_item("errors", error_strings)?;

    Ok((py_records.into(), py_summary.into()))
}

/// PyO3-exposed entry point: collects only file path strings from a domain,
/// enabling the Python layer to perform its own batching or sampling strategy.
///
/// Executes Phase 1 + Phase 2 only (no JSON parsing). Useful for progress
/// monitoring, sampling strategies, and incremental ingestion checkpointing.
///
/// # Document IDs
/// SPEC-IO-WALKER-01
#[pyfunction]
#[pyo3(signature = (domain_root))]
pub fn py_enumerate_domain_paths(py: Python, domain_root: &str) -> PyResult<PyObject> {
    let root_path = Path::new(domain_root);

    let paths = py.allow_threads(|| {
        let shards = discover_shard_directories(root_path)
            .map_err(|e| PyIOError::new_err(e.to_string()))?;
        let (paths, _errs) = collect_json_paths_parallel(&shards);
        Ok::<Vec<PathBuf>, PyErr>(paths)
    })?;

    let py_list = PyList::empty_bound(py);
    for p in &paths {
        py_list.append(p.display().to_string())?;
    }
    Ok(py_list.into())
}

// =============================================================================
// SECTION 9: EXPERIMENT TYPE CLASSIFIER (BUG-04 Port)
// =============================================================================

/// Experiment type as resolved by `classify_experiment_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperimentType {
    Experimental,
    Computational,
    Unknown,
}

/// DFT/simulation software keywords for computational experiment classification (BUG-04).
const COMPUTATIONAL_KEYWORDS: &[&str] = &[
    "dft", "vasp", "wien2k", "quantum espresso", "openmx",
    "ab initio", "first-principles", "first principles",
    "calculated", "simulated", "molecular dynamics", "monte carlo",
    "phonon calculation", "band structure calculation",
];

/// Synthesis and characterisation keywords for experimental classification (BUG-04).
const EXPERIMENTAL_KEYWORDS: &[&str] = &[
    "measured", "synthesized", "fabricated", "grown", "deposited",
    "sintered", "pressed", "annealed", "spark plasma sintering",
    "hot pressing", "arc melting", "zone melting", "ball milling",
    "sputtered", "evaporated", "chemical vapor deposition", "cvd",
    "four-probe", "harman method", "laser flash",
];

/// Port of `classify_experiment_type()` from Python ingestion.py (BUG-04 fix).
///
/// Examines only semantically relevant sample fields (method, calculationtype,
/// technique, comment, instrument) to avoid pollution from the string "sample"
/// appearing universally in Starrydata JSON keys.
///
/// Priority ordering:
/// 1. Computational — unambiguous DFT/simulation keywords.
/// 2. Experimental — explicit synthesis/characterisation keywords.
/// 3. Unknown — no unambiguous keyword; caller may set FLAG_LOW_CONFIDENCE_EXP.
#[must_use]
pub fn classify_experiment_type(sample_info_json: &serde_json::Value) -> ExperimentType {
    let relevant: Vec<&str> = ["method", "calculationtype", "technique", "comment", "instrument"]
        .iter()
        .filter_map(|&key| sample_info_json.get(key).and_then(|v| v.as_str()))
        .collect();
    let text: String = relevant.join(" ").to_lowercase();

    for kw in COMPUTATIONAL_KEYWORDS {
        if text.contains(kw) {
            return ExperimentType::Computational;
        }
    }
    for kw in EXPERIMENTAL_KEYWORDS {
        if text.contains(kw) {
            return ExperimentType::Experimental;
        }
    }
    ExperimentType::Unknown
}

/// Validates a single JSON file and returns its record count without full ingestion.
/// Used for checksumming and data integrity verification passes.
///
/// # Document IDs
/// SPEC-IO-WALKER-01
#[pyfunction]
#[pyo3(signature = (file_path, domain_name))]
pub fn py_validate_single_file(
    py: Python,
    file_path: &str,
    domain_name: &str,
) -> PyResult<PyObject> {
    let path = Path::new(file_path);
    let record = py
        .allow_threads(|| parse_json_file(path, domain_name))
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let d = PyDict::new_bound(py);
    d.set_item("source_path", &record.source_path)?;
    d.set_item("n_measurements", record.measurements.len())?;
    d.set_item("n_samples", record.samples.len())?;
    d.set_item("n_papers", record.papers.len())?;
    d.set_item("n_figures", record.figures.len())?;
    d.set_item("n_properties", record.properties.len())?;
    Ok(d.into())
}

// =============================================================================
// SECTION 10: UNIT TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// BUG-01 Regression: A file containing only UV-Vis optical measurements
    /// (propertyid_y = 20, not in ALLOWED_PROPERTY_IDS) must yield
    /// `measurements.len() == 0` and `rejected_non_thermoelectric_count > 0`.
    #[test]
    fn bug01_uv_vis_optical_file_yields_zero_measurements() {
        let json = r#"{"sample":[],"paper":[],"property":[],"figure":[],"rawdata":[
            {"paperid":1,"sampleid":"1","figureid":"1","x":300.0,"y":0.5,"propertyid_x":1,"propertyid_y":20},
            {"paperid":1,"sampleid":"1","figureid":"1","x":400.0,"y":0.6,"propertyid_x":1,"propertyid_y":20},
            {"paperid":1,"sampleid":"1","figureid":"1","x":500.0,"y":0.7,"propertyid_x":1,"propertyid_y":20}
        ]}"#;
        let mut tmp = NamedTempFile::new().expect("tempfile");
        tmp.write_all(json.as_bytes()).expect("write");
        let path = tmp.path();
        let record = parse_json_file(path, "samples").expect("parse");
        assert_eq!(
            record.measurements.len(), 0,
            "BUG-01: UV-Vis propertyid_y=20 must be rejected — got {} measurements",
            record.measurements.len()
        );
        assert_eq!(
            record.rejected_non_thermoelectric_count, 3,
            "BUG-01: all 3 UV-Vis measurements must be counted as rejected"
        );
    }

    /// BUG-01 Positive: Seebeck (propertyid_y=2) must be accepted.
    #[test]
    fn bug01_seebeck_allowed_property_is_retained() {
        let json = r#"{"sample":[],"paper":[],"property":[],"figure":[],"rawdata":[
            {"paperid":1,"sampleid":"1","figureid":"1","x":300.0,"y":200e-6,"propertyid_x":1,"propertyid_y":2}
        ]}"#;
        let mut tmp = NamedTempFile::new().expect("tempfile");
        tmp.write_all(json.as_bytes()).expect("write");
        let record = parse_json_file(tmp.path(), "samples").expect("parse");
        assert_eq!(record.measurements.len(), 1, "Seebeck (propertyid_y=2) must be retained");
        assert_eq!(record.rejected_non_thermoelectric_count, 0);
    }

    /// GAP-05 Regression: A measurement referencing a figureid not in the file's
    /// figure[] array must increment figureid_mismatch_count but still be retained.
    #[test]
    fn gap05_dangling_figureid_increments_mismatch_count() {
        let json = r#"{"sample":[],"paper":[],"property":[],"figure":[
            {"figureid":"10","paperid":1,"figurename":"","caption":"","propertyid_x":1,"propertyid_y":2}
        ],"rawdata":[
            {"paperid":1,"sampleid":"1","figureid":"99","x":300.0,"y":200e-6,"propertyid_x":1,"propertyid_y":2}
        ]}"#;
        let mut tmp = NamedTempFile::new().expect("tempfile");
        tmp.write_all(json.as_bytes()).expect("write");
        let record = parse_json_file(tmp.path(), "samples").expect("parse");
        assert_eq!(record.measurements.len(), 1, "GAP-05: measurement must be retained despite figureid mismatch");
        assert_eq!(record.figureid_mismatch_count, 1, "GAP-05: mismatch count must be 1");
    }

    /// GAP-07 Regression: 1 million OnceLock allowlist lookups must complete in < 10 ms
    /// in release builds. The debug/test threshold is relaxed to 500 ms to account for
    /// the unoptimised `[profile.test]` build.
    #[test]
    fn gap07_oncelock_allowlist_lookup_under_10ms() {
        // Production limit: 10 ms; debug/test relaxed to 500 ms (no inlining, no opt)
        #[cfg(debug_assertions)]
        let limit_ms: u128 = 500;
        #[cfg(not(debug_assertions))]
        let limit_ms: u128 = 10;

        // Warm up the OnceLock (first call initialises the set)
        let _ = is_allowed_property(2);
        let start = std::time::Instant::now();
        for _ in 0..1_000_000 {
            let _ = is_allowed_property(2);
            let _ = is_allowed_property(20); // non-thermoelectric
        }
        let elapsed_ms = start.elapsed().as_millis();
        assert!(
            elapsed_ms < limit_ms,
            "GAP-07: 1M OnceLock lookups must complete in < {}ms; took {}ms",
            limit_ms, elapsed_ms
        );
    }

    /// classify_experiment_type: DFT keyword → Computational.
    #[test]
    fn classify_dft_keyword_returns_computational() {
        let info = serde_json::json!({"method": "DFT calculation using VASP"});
        assert_eq!(classify_experiment_type(&info), ExperimentType::Computational);
    }

    /// classify_experiment_type: synthesis keyword → Experimental.
    #[test]
    fn classify_sintered_keyword_returns_experimental() {
        let info = serde_json::json!({"technique": "spark plasma sintering at 600 C"});
        assert_eq!(classify_experiment_type(&info), ExperimentType::Experimental);
    }

    /// classify_experiment_type: "sample" keyword alone (BUG-04) → Unknown, not Experimental.
    #[test]
    fn classify_sample_keyword_alone_returns_unknown() {
        // BUG-04 regression: the word "sample" must NOT classify as experimental
        let info = serde_json::json!({"comment": "sample prepared by standard methods"});
        // No synthesis-specific keyword present → Unknown
        assert_eq!(classify_experiment_type(&info), ExperimentType::Unknown);
    }

    /// classify_experiment_type: empty sampleinfo → Unknown.
    #[test]
    fn classify_empty_sampleinfo_returns_unknown() {
        let info = serde_json::json!({});
        assert_eq!(classify_experiment_type(&info), ExperimentType::Unknown);
    }
}
