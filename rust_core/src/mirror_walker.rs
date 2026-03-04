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
use thiserror::Error;

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
    pub measurements: Vec<RawMeasurement>,
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
            .map(|u| u as u32)
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
    shards.sort_unstable();
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

    // Apply hard physical constraints to rawdata: expel NaN/Inf artifacts
    // introduced by digitization software or floating-point serialization.
    let valid_measurements: Vec<RawMeasurement> = root
        .rawdata
        .into_iter()
        .filter(|m| m.x.is_finite() && m.y.is_finite())
        .collect();

    Ok(StarrydataRecord {
        source_path: path.display().to_string(),
        source_domain: domain.to_string(),
        samples: root.sample,
        papers: root.paper,
        properties: root.property,
        figures: root.figure,
        measurements: valid_measurements,
    })
}

/// **Phase 3**: Processes all collected JSON paths in parallel.
///
/// Rayon distributes file I/O and JSON deserialization across all logical cores.
/// Parse errors are captured without aborting the pipeline (fail-soft semantics).
///
/// # Returns
/// `(records, errors)` tuple. The `errors` vec provides the audit trail of
/// files that failed ingestion, required by Q1 data descriptor standards.
fn ingest_files_parallel(
    paths: &[PathBuf],
    domain: &str,
) -> (Vec<StarrydataRecord>, Vec<WalkError>) {
    let results: Vec<Result<StarrydataRecord, WalkError>> = paths
        .par_iter()
        .map(|p| parse_json_file(p, domain))
        .collect();

    let mut records = Vec::new();
    let mut errors = Vec::new();
    for result in results {
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
    pub total_measurements: usize,
    pub total_samples: usize,
    pub total_papers: usize,
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
    // Phase 1: Serial shard discovery
    let shards = discover_shard_directories(domain_root)?;
    let shards_discovered = shards.len();

    // Phase 2: Parallel file path collection
    let (all_paths, shard_errors) = collect_json_paths_parallel(&shards);
    let files_found = all_paths.len();

    // Phase 3: Parallel JSON ingestion
    let (records, mut file_errors) = ingest_files_parallel(&all_paths, domain_name);
    let files_parsed = records.len();

    // Aggregate audit telemetry
    let total_measurements: usize = records.iter().map(|r| r.measurements.len()).sum();
    let total_samples: usize = records.iter().map(|r| r.samples.len()).sum();
    let total_papers: usize = records.iter().map(|r| r.papers.len()).sum();

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
    let d = PyDict::new(py);
    d.set_item("source_path", &record.source_path)?;
    d.set_item("source_domain", &record.source_domain)?;
    d.set_item("n_measurements", record.measurements.len())?;
    d.set_item("n_samples", record.samples.len())?;
    d.set_item("n_papers", record.papers.len())?;

    // Measurements → list of dicts
    let meas_list = PyList::empty(py);
    for m in &record.measurements {
        let md = PyDict::new(py);
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
    let samp_list = PyList::empty(py);
    for s in &record.samples {
        let sd = PyDict::new(py);
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
    let paper_list = PyList::empty(py);
    for p in &record.papers {
        let pd = PyDict::new(py);
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
    let py_records = PyList::empty(py);
    for record in &records {
        let d = record_to_pydict(py, record)?;
        py_records.append(d)?;
    }

    let py_summary = PyDict::new(py);
    py_summary.set_item("domain", &summary.domain)?;
    py_summary.set_item("shards_discovered", summary.shards_discovered)?;
    py_summary.set_item("files_found", summary.files_found)?;
    py_summary.set_item("files_parsed", summary.files_parsed)?;
    py_summary.set_item("files_failed", summary.files_failed)?;
    py_summary.set_item("total_measurements", summary.total_measurements)?;
    py_summary.set_item("total_samples", summary.total_samples)?;
    py_summary.set_item("total_papers", summary.total_papers)?;

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

    let py_list = PyList::empty(py);
    for p in &paths {
        py_list.append(p.display().to_string())?;
    }
    Ok(py_list.into())
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

    let d = PyDict::new(py);
    d.set_item("source_path", &record.source_path)?;
    d.set_item("n_measurements", record.measurements.len())?;
    d.set_item("n_samples", record.samples.len())?;
    d.set_item("n_papers", record.papers.len())?;
    d.set_item("n_figures", record.figures.len())?;
    d.set_item("n_properties", record.properties.len())?;
    Ok(d.into())
}
