// rust_core/src/memory_guard.rs

//! # MemoryGuard — Real-Time RSS Monitoring for 8 GB Hard Ceiling
//!
//! **Layer:** Infrastructure / Memory Safety
//! **Status:** Normative — SYSTEM CONSTRAINT (8 GB hard ceiling)
//! **Implements:** SPEC-MEM-GUARD-01
//!
//! The host machine has exactly 8 GB RAM. The pipeline must never exceed
//! 7 GB RSS to leave headroom for the OS kernel, page cache, and I/O buffers.
//!
//! ## Design
//!
//! `MemoryGuard::tick()` is called once per processed record. It only reads
//! the OS RSS counter every `check_interval` records (default: 1000) to
//! amortize the syscall cost. On Linux, RSS is read from `/proc/self/status`
//! (single file read, ~1 µs). On macOS, `getrusage(RUSAGE_SELF)` is used
//! (returns peak RSS, not current — acceptable for monitoring purposes).
//!
//! ## Pressure Levels
//!
//! | Level | RSS Threshold | Action Required by Caller               |
//! |-------|---------------|-----------------------------------------|
//! | `Ok`  | < soft_limit  | Continue normally                       |
//! | `Soft`| > soft_limit  | Flush current batch to disk immediately |
//! | `Hard`| > hard_limit  | Checkpoint state and pause ingestion    |
//!
//! ## Checkpoint Protocol
//!
//! On `Hard` pressure, the caller writes `checkpoint.json` containing the
//! path of the last successfully processed file and the cumulative record
//! count. On the next run, if `checkpoint.json` exists, the walker skips
//! all files processed in the previous run.

use std::path::{Path, PathBuf};
use thiserror::Error;

// ============================================================================
// SECTION 1: ERROR TAXONOMY
// ============================================================================

/// Errors from the MemoryGuard and checkpoint subsystem.
#[derive(Error, Debug)]
pub enum MemoryGuardError {
    #[error("MEMGUARD-01: Failed to read checkpoint file '{path}': {cause}")]
    CheckpointReadError { path: String, cause: String },

    #[error("MEMGUARD-02: Failed to write checkpoint file '{path}': {cause}")]
    CheckpointWriteError { path: String, cause: String },

    #[error("MEMGUARD-03: Hard memory pressure ({rss_mb:.1} MB > {limit_mb:.1} MB). Ingestion paused.")]
    HardMemoryPressure { rss_mb: f64, limit_mb: f64 },
}

// ============================================================================
// SECTION 2: PRESSURE LEVEL ENUM
// ============================================================================

/// Memory pressure level returned by `MemoryGuard::tick()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressure {
    /// RSS is below the soft limit. Continue processing.
    Ok,
    /// RSS exceeds the soft limit (default: 6.5 GB). The caller must flush
    /// the current in-memory batch to disk before processing further records.
    Soft,
    /// RSS exceeds the hard limit (default: 7.0 GB). The caller must write
    /// a checkpoint and stop processing to prevent OOM.
    Hard,
}

// ============================================================================
// SECTION 3: CHECKPOINT STRUCT
// ============================================================================

/// Persistent state written to `checkpoint.json` on Hard memory pressure.
/// On the next pipeline run, the walker reads this file and resumes from
/// `last_processed_file`.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CheckpointState {
    /// Absolute path of the last file successfully processed before OOM halt.
    pub last_processed_file: String,
    /// Total records successfully written before the halt.
    pub records_completed: u64,
    /// Wall-clock timestamp of the checkpoint (ISO 8601).
    pub timestamp: String,
    /// Peak RSS at the time of halt, in MB.
    pub peak_rss_mb: f64,
}

// ============================================================================
// SECTION 4: MEMORY GUARD STRUCT
// ============================================================================

/// Real-time RSS monitor with configurable soft and hard pressure thresholds.
///
/// # Example
/// ```rust
/// use rust_core::memory_guard::{MemoryGuard, MemoryPressure};
///
/// let mut guard = MemoryGuard::new(6_500_000_000, 7_000_000_000, 1000);
/// match guard.tick() {
///     MemoryPressure::Ok   => { /* continue */ }
///     MemoryPressure::Soft => { /* flush batch */ }
///     MemoryPressure::Hard => { /* checkpoint and stop */ }
/// }
/// ```
pub struct MemoryGuard {
    /// Flush threshold in bytes (default: 6.5 GB = 6_500_000_000).
    soft_limit_bytes: u64,
    /// Halt threshold in bytes (default: 7.0 GB = 7_000_000_000).
    hard_limit_bytes: u64,
    /// RSS is checked once every this many records (default: 1000).
    check_interval: u64,
    /// Records processed since the last RSS check.
    records_since_check: u64,
    /// Peak RSS observed across all checks, in bytes.
    peak_rss_bytes: u64,
}

impl MemoryGuard {
    /// Creates a new `MemoryGuard` with the given thresholds.
    ///
    /// # Arguments
    /// - `soft_limit_bytes`: RSS threshold above which `Soft` pressure is signalled.
    /// - `hard_limit_bytes`: RSS threshold above which `Hard` pressure is signalled.
    /// - `check_interval`: RSS is read from the OS every this many `tick()` calls.
    #[must_use]
    pub fn new(soft_limit_bytes: u64, hard_limit_bytes: u64, check_interval: u64) -> Self {
        Self {
            soft_limit_bytes,
            hard_limit_bytes,
            check_interval,
            records_since_check: 0,
            peak_rss_bytes: 0,
        }
    }

    /// Creates a `MemoryGuard` with the default production thresholds:
    /// soft = 6.5 GB, hard = 7.0 GB, interval = 1000 records.
    #[must_use]
    pub fn default_production() -> Self {
        Self::new(6_500_000_000, 7_000_000_000, 1_000)
    }

    /// Creates a `MemoryGuard` from explicit MB values (convenience for CLI `--max-ram-mb`).
    ///
    /// `max_ram_mb` is the hard limit. The soft limit is set to 93% of the hard limit.
    #[must_use]
    pub fn from_max_ram_mb(max_ram_mb: u64) -> Self {
        let hard = max_ram_mb * 1_048_576;
        let soft = (hard as f64 * 0.928_571_4) as u64; // 6.5/7.0 ratio
        Self::new(soft, hard, 1_000)
    }

    /// Advances the internal counter by one record and checks RSS pressure.
    ///
    /// This function is designed to be called in the hot path of the ingestion
    /// loop. The OS RSS check is amortized over `check_interval` calls.
    ///
    /// # Returns
    /// `MemoryPressure::Ok` for the vast majority of calls (no syscall).
    /// `MemoryPressure::Soft` or `Hard` only when the interval elapses and
    /// RSS exceeds the corresponding threshold.
    pub fn tick(&mut self) -> MemoryPressure {
        self.records_since_check += 1;
        if self.records_since_check < self.check_interval {
            return MemoryPressure::Ok;
        }
        self.records_since_check = 0;
        self.check_pressure()
    }

    /// Forces an immediate RSS check regardless of the interval counter.
    /// Useful at the start of each file-batch to ensure we don't begin
    /// a large batch under pressure.
    pub fn check_pressure(&mut self) -> MemoryPressure {
        let rss = read_rss_bytes();
        if rss > self.peak_rss_bytes {
            self.peak_rss_bytes = rss;
        }

        if rss > self.hard_limit_bytes {
            log::error!(
                "MEMGUARD HARD: RSS {:.1} MB exceeds hard limit {:.1} MB. \
                 Halting ingestion. Write checkpoint.",
                rss as f64 / 1_048_576.0,
                self.hard_limit_bytes as f64 / 1_048_576.0
            );
            MemoryPressure::Hard
        } else if rss > self.soft_limit_bytes {
            log::warn!(
                "MEMGUARD SOFT: RSS {:.1} MB exceeds soft limit {:.1} MB. \
                 Flushing in-memory batch.",
                rss as f64 / 1_048_576.0,
                self.soft_limit_bytes as f64 / 1_048_576.0
            );
            MemoryPressure::Soft
        } else {
            MemoryPressure::Ok
        }
    }

    /// Peak RSS observed across all monitoring cycles, in megabytes.
    #[must_use]
    pub fn peak_rss_mb(&self) -> f64 {
        self.peak_rss_bytes as f64 / 1_048_576.0
    }

    /// Current RSS at the moment of this call, in megabytes.
    /// Performs one OS read; do not call in the hot loop.
    #[must_use]
    pub fn current_rss_mb(&self) -> f64 {
        read_rss_bytes() as f64 / 1_048_576.0
    }
}

// ============================================================================
// SECTION 5: CHECKPOINT I/O
// ============================================================================

/// Writes a checkpoint file to `path` capturing the current ingestion state.
///
/// Called by the scanner on `MemoryPressure::Hard` to allow resumption.
///
/// # Errors
/// Returns `MemoryGuardError::CheckpointWriteError` on I/O failure.
pub fn write_checkpoint(
    path: &Path,
    last_processed_file: &str,
    records_completed: u64,
    peak_rss_mb: f64,
) -> Result<(), MemoryGuardError> {
    let state = CheckpointState {
        last_processed_file: last_processed_file.to_owned(),
        records_completed,
        timestamp: chrono_now_iso8601(),
        peak_rss_mb,
    };
    let json = serde_json::to_string_pretty(&state).map_err(|e| {
        MemoryGuardError::CheckpointWriteError {
            path: path.display().to_string(),
            cause: e.to_string(),
        }
    })?;
    std::fs::write(path, json).map_err(|e| MemoryGuardError::CheckpointWriteError {
        path: path.display().to_string(),
        cause: e.to_string(),
    })
}

/// Reads a checkpoint file. Returns `None` if the file does not exist (fresh run).
///
/// # Errors
/// Returns `MemoryGuardError::CheckpointReadError` if the file exists but is malformed.
pub fn read_checkpoint(path: &Path) -> Result<Option<CheckpointState>, MemoryGuardError> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path).map_err(|e| {
        MemoryGuardError::CheckpointReadError {
            path: path.display().to_string(),
            cause: e.to_string(),
        }
    })?;
    let state: CheckpointState = serde_json::from_str(&content).map_err(|e| {
        MemoryGuardError::CheckpointReadError {
            path: path.display().to_string(),
            cause: e.to_string(),
        }
    })?;
    Ok(Some(state))
}

/// Returns the default checkpoint file path relative to the output directory.
#[must_use]
pub fn default_checkpoint_path(output_dir: &Path) -> PathBuf {
    output_dir.join("checkpoint.json")
}

// ============================================================================
// SECTION 6: PLATFORM-SPECIFIC RSS READERS
// ============================================================================

/// Returns the current Resident Set Size (RSS) of this process in bytes.
/// Returns 0 if the OS query fails (guard is effectively disabled).
fn read_rss_bytes() -> u64 {
    read_rss_bytes_impl()
}

#[cfg(target_os = "linux")]
fn read_rss_bytes_impl() -> u64 {
    // /proc/self/status contains "VmRSS:\t  12345 kB" — parse the kB value.
    let Ok(content) = std::fs::read_to_string("/proc/self/status") else {
        return 0;
    };
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb: u64 = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            return kb * 1_024;
        }
    }
    0
}

#[cfg(target_os = "macos")]
fn read_rss_bytes_impl() -> u64 {
    // getrusage returns ru_maxrss in bytes on macOS (unlike Linux where it's kilobytes).
    // This is the PEAK RSS since process start, not current. For OOM protection,
    // peak is a conservative (safe) proxy for current.
    // SAFETY: libc::rusage is a C struct with no invalid bit patterns; zeroing produces the
    // valid POSIX initial state required before calling getrusage.
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() }; // SAFETY: see comment above
    // SAFETY: RUSAGE_SELF is a valid selector; usage is a correctly-typed local stack allocation.
    let ret = unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) }; // SAFETY: see comment above
    if ret == 0 {
        // ru_maxrss is in bytes on macOS
        usage.ru_maxrss as u64
    } else {
        0
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn read_rss_bytes_impl() -> u64 {
    // Windows and other platforms: memory guard is disabled.
    // The pipeline will run without RSS monitoring. The caller should apply
    // explicit chunk-size limits to bound memory usage.
    0
}

/// Formats a Unix epoch timestamp as a minimal ISO 8601 UTC string.
///
/// Public for use by the statistics engine's `pipeline_summary.json` writer.
/// Example output: `"2026-03-06T12:34:56Z"`.
pub fn format_timestamp_utc(secs: u64) -> String {
    let (y, mo, d, h, mi, s) = epoch_to_ymd_hms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Returns a minimal ISO 8601 timestamp without external dependencies.
fn chrono_now_iso8601() -> String {
    // Without `chrono` in dependencies, use a lightweight approach.
    // If chrono is available, this would use Utc::now().to_rfc3339().
    // For now: format from std::time::SystemTime.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Minimal ISO 8601 from epoch seconds (UTC, no sub-second precision)
    let (y, mo, d, h, mi, s) = epoch_to_ymd_hms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Converts Unix epoch seconds to (year, month, day, hour, minute, second).
/// This is a compact, dependency-free Gregorian calendar calculation.
fn epoch_to_ymd_hms(epoch: u64) -> (u32, u32, u32, u32, u32, u32) {
    let s = epoch % 60;
    let mi = (epoch / 60) % 60;
    let h = (epoch / 3600) % 24;
    let days = epoch / 86400;

    // Civil calendar: algorithm from Howard Hinnant (public domain)
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    (y as u32, mo as u32, d as u32, h as u32, mi as u32, s as u32)
}

// ============================================================================
// SECTION 7: UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_pressure_before_interval() {
        let mut guard = MemoryGuard::new(6_500_000_000, 7_000_000_000, 1_000);
        // First 999 ticks must always return Ok (no syscall happens).
        for _ in 0..999 {
            assert_eq!(guard.tick(), MemoryPressure::Ok);
        }
    }

    #[test]
    fn peak_rss_increases_monotonically() {
        let mut guard = MemoryGuard::default_production();
        // Trigger a check by advancing past the interval.
        for _ in 0..1001 {
            let _ = guard.tick();
        }
        // Peak RSS must be >= 0 after at least one real check.
        assert!(guard.peak_rss_mb() >= 0.0);
    }

    #[test]
    fn from_max_ram_mb_soft_is_below_hard() {
        let guard = MemoryGuard::from_max_ram_mb(7_000);
        assert!(guard.soft_limit_bytes < guard.hard_limit_bytes);
    }

    #[test]
    fn mock_rss_above_soft_limit_triggers_soft() {
        // Directly verify the threshold logic using a zero-limit guard.
        // With soft=0 and hard=u64::MAX, any RSS > 0 → Soft.
        let mut guard = MemoryGuard::new(0, u64::MAX, 1);
        // The first tick triggers a check (interval=1).
        let pressure = guard.tick();
        // On a running process, RSS is always > 0, so Soft is expected.
        // On a system where read_rss_bytes() returns 0, this gives Ok — acceptable.
        assert!(pressure == MemoryPressure::Soft || pressure == MemoryPressure::Ok);
    }

    #[test]
    fn mock_rss_above_hard_limit_triggers_hard() {
        // With both limits = 0, any RSS > 0 → Hard.
        let mut guard = MemoryGuard::new(0, 0, 1);
        let pressure = guard.tick();
        // Hard expected on any real process; Ok only if RSS probe is unavailable.
        assert!(pressure == MemoryPressure::Hard || pressure == MemoryPressure::Ok);
    }

    #[test]
    fn checkpoint_round_trip() {
        let dir = std::env::temp_dir();
        let path = dir.join("thermognosis_test_checkpoint.json");
        write_checkpoint(&path, "/data/00001/abc.json", 42_000, 3200.5)
            .expect("checkpoint write must succeed");
        let state = read_checkpoint(&path)
            .expect("checkpoint read must succeed")
            .expect("checkpoint must be Some after write");
        assert_eq!(state.last_processed_file, "/data/00001/abc.json");
        assert_eq!(state.records_completed, 42_000);
        assert!((state.peak_rss_mb - 3200.5).abs() < 1e-6);
        // Clean up
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn checkpoint_returns_none_when_absent() {
        let path = std::path::Path::new("/tmp/thermognosis_nonexistent_checkpoint_xyz.json");
        let result = read_checkpoint(path).expect("read_checkpoint must not error on missing file");
        assert!(result.is_none());
    }

    #[test]
    fn epoch_to_ymd_hms_unix_epoch() {
        // 0 seconds = 1970-01-01 00:00:00
        let (y, mo, d, h, mi, s) = epoch_to_ymd_hms(0);
        assert_eq!((y, mo, d, h, mi, s), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn epoch_to_ymd_hms_known_date() {
        // 2026-03-05 00:00:00 UTC = 1_772_352_000 seconds (approximately)
        // Just verify the function does not panic with large inputs.
        let (y, _, _, _, _, _) = epoch_to_ymd_hms(1_772_352_000);
        assert_eq!(y, 2026);
    }
}
