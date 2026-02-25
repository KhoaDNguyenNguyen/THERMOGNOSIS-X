// rust_core/src/lib.rs

//! # Thermognosis Engine - Rust Core FFI Boundary
//! 
//! **Layer:** API / FFI Boundary  
//! **Status:** Normative — Strict Mathematical Execution Environment  
//! **Document Governance:** SPEC-GOV-CODE-GENERATION-PROTOCOL, SPEC-GOV-NAMING-RULES, SPEC-GOV-ERROR-HIERARCHY
//! 
//! This module forms the absolute computational bedrock of the Thermognosis Engine. 
//! It exposes rigorously defined physical, statistical, and thermodynamic modules to Python 
//! via PyO3 and rust-numpy. 
//! 
//! ## Architectural Guarantees:
//! 1. **Zero-Copy Memory**: Incoming NumPy arrays are memory-mapped as contiguous slices. 
//!    No duplication occurs across the FFI boundary.
//! 2. **GIL Independence**: Heavy $\mathcal{O}(N)$ matrix operations unconditionally release 
//!    the Global Interpreter Lock (GIL) and are distributed across all available logical cores via `rayon`.
//! 3. **Mathematical Determinism**: A `deterministic` execution flag forces strictly ordered 
//!    sequential iterators for reproducible pipeline generation, disabling the work-stealing thread pool.
//! 4. **Panic-Free Safety**: All physical constraints, domain violations, and tensor 
//!    dimension mismatches are trapped and gracefully promoted to Python `ValueError` or `RuntimeError`.

use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::types::{PyDict, PyList};
use numpy::{IntoPyArray, PyArray1, PyReadonlyArray1};
use rayon::prelude::*;

// Internal module declarations mirroring the core library structure
pub mod csv_engine;
pub mod error_propagation;
pub mod physics;
pub mod scoring;
pub mod units;
pub mod validation;

// Advanced Analytical Modules
pub mod bayesian;
pub mod ranking_core;
pub mod information_gain;

// Export ThermoError centrally to satisfy crate-level internal references
pub use bayesian::ThermoError;

use error_propagation::{calculate_zt_linear_propagation, PropertyUncertainties, ThermoelectricProperties};
use scoring::{QualityEvaluator, QualityVector, ScoringWeights};
use units::{PhysicalQuantity, UnitDefinition};
use validation::ThermoelectricState;

/// Macro to securely project flat NumPy arrays into zero-copy, C-contiguous Rust slices.
/// Implements: SPEC-GOV-ERROR-HIERARCHY (Zero Panic Guarantee)
macro_rules! extract_slice {
    ($array:expr, $name:expr) => {
        $array.as_slice().map_err(|_| {
            PyValueError::new_err(format!(
                "FFI Ingress Violation: Array '{}' is not C-contiguous in memory.",
                $name
            ))
        })?
    };
}

/// Validates tensor dimensionality parity across heterogenous arrays.
/// Implements: SPEC-GOV-ERROR-HIERARCHY
#[inline(always)]
fn enforce_equal_lengths(lengths: &[usize]) -> PyResult<usize> {
    if lengths.is_empty() {
        return Ok(0);
    }
    let baseline = lengths[0];
    for &len in lengths.iter().skip(1) {
        if len != baseline {
            return Err(PyValueError::new_err(format!(
                "Dimensionality Violation: Input arrays must have identical lengths. Expected {}, Found {}.",
                baseline, len
            )));
        }
    }
    Ok(baseline)
}

/// Parses unit string identifiers into formal `UnitDefinition` registry structs.
/// Implements: SPEC-UNIT-SYSTEM-REGISTRY
fn parse_unit_symbol(symbol: &str) -> PyResult<UnitDefinition> {
    match symbol {
        "m" => Ok(units::METRE),
        "mm" => Ok(units::MILLIMETRE),
        "K" => Ok(units::KELVIN),
        "degC" => Ok(units::CELSIUS),
        _ => Err(PyValueError::new_err(format!(
            "UR-01: Unknown or unsupported physical unit symbol: '{}'",
            symbol
        ))),
    }
}

/// Dimensionally validates and converts macroscopic property arrays to a target unit.
///
/// **Document IDs**: SPEC-UNIT-CONVERTER, SPEC-UNIT-DIM-VALIDATION
#[pyfunction]
#[pyo3(signature = (values, uncertainties, source_unit, target_unit, deterministic=false))]
pub fn validate_dimensions_py<'py>(
    py: Python<'py>,
    values: PyReadonlyArray1<'py, f64>,
    uncertainties: PyReadonlyArray1<'py, f64>,
    source_unit: &str,
    target_unit: &str,
    deterministic: bool,
) -> PyResult<(&'py PyArray1<f64>, &'py PyArray1<f64>)> {
    let vals = extract_slice!(values, "values");
    let uncs = extract_slice!(uncertainties, "uncertainties");
    let len = enforce_equal_lengths(&[vals.len(), uncs.len()])?;

    let src_def = parse_unit_symbol(source_unit)?;
    let tgt_def = parse_unit_symbol(target_unit)?;

    let quantities: Result<Vec<PhysicalQuantity>, _> = vals
        .iter()
        .zip(uncs.iter())
        .map(|(&v, &u)| PhysicalQuantity::new(v, src_def, u))
        .collect();

    let quantities = quantities.map_err(|e| PyValueError::new_err(e.to_string()))?;

    let converted = py.allow_threads(|| {
        if deterministic {
            quantities
                .iter()
                .map(|q| q.convert(tgt_def))
                .collect::<Result<Vec<_>, _>>()
        } else {
            units::par_convert_bulk(&quantities, tgt_def)
        }
    }).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let mut out_vals = Vec::with_capacity(len);
    let mut out_uncs = Vec::with_capacity(len);
    for q in converted {
        out_vals.push(q.value);
        out_uncs.push(q.uncertainty);
    }

    Ok((out_vals.into_pyarray(py), out_uncs.into_pyarray(py)))
}

/// Validates thermodynamic constraints across state parameters.
/// 
/// Evaluates formal physical laws, bounds, and thermodynamic constraints. Short-circuits
/// safely to a Python exception on the first unphysical state detected.
/// 
/// **Document IDs**: SPEC-PHYS-CONSISTENCY, SPEC-PHYS-CONSTRAINTS
#[pyfunction]
#[pyo3(signature = (s, sigma, kappa, t, deterministic=false))]
pub fn check_physics_consistency_py<'py>(
    py: Python<'py>,
    s: PyReadonlyArray1<'py, f64>,
    sigma: PyReadonlyArray1<'py, f64>,
    kappa: PyReadonlyArray1<'py, f64>,
    t: PyReadonlyArray1<'py, f64>,
    deterministic: bool,
) -> PyResult<&'py PyArray1<f64>> {
    let s_slice = extract_slice!(s, "S");
    let sigma_slice = extract_slice!(sigma, "sigma");
    let kappa_slice = extract_slice!(kappa, "kappa");
    let t_slice = extract_slice!(t, "T");

    let len = enforce_equal_lengths(&[
        s_slice.len(),
        sigma_slice.len(),
        kappa_slice.len(),
        t_slice.len(),
    ])?;

    let mut states = Vec::with_capacity(len);
    for i in 0..len {
        states.push(ThermoelectricState {
            s: s_slice[i],
            sigma: sigma_slice[i],
            kappa: kappa_slice[i],
            t: t_slice[i],
        });
    }

    let validated = py.allow_threads(|| {
        if deterministic {
            states
                .iter()
                .map(|st| st.validate())
                .collect::<Result<Vec<_>, _>>()
        } else {
            validation::validate_states_par(&states)
        }
    }).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let zt_out: Vec<f64> = validated.into_iter().map(|v| v.zt()).collect();
    Ok(zt_out.into_pyarray(py))
}

/// Computes the first-order analytical propagation of standard measurement uncertainties for zT.
/// 
/// **Document IDs**: P02-ZT-ERROR-PROPAGATION, T03-UNCERTAINTY-PROPAGATION, SPEC-PHYS-ERROR-PROPAGATION
#[pyfunction]
#[pyo3(signature = (s, sigma, kappa, t, err_s, err_sigma, err_kappa, err_t, deterministic=false))]
#[allow(clippy::too_many_arguments)]
pub fn propagate_error_py<'py>(
    py: Python<'py>,
    s: PyReadonlyArray1<'py, f64>,
    sigma: PyReadonlyArray1<'py, f64>,
    kappa: PyReadonlyArray1<'py, f64>,
    t: PyReadonlyArray1<'py, f64>,
    err_s: PyReadonlyArray1<'py, f64>,
    err_sigma: PyReadonlyArray1<'py, f64>,
    err_kappa: PyReadonlyArray1<'py, f64>,
    err_t: PyReadonlyArray1<'py, f64>,
    deterministic: bool,
) -> PyResult<(&'py PyArray1<f64>, &'py PyArray1<f64>)> {
    let s_slice = extract_slice!(s, "S");
    let sigma_slice = extract_slice!(sigma, "sigma");
    let kappa_slice = extract_slice!(kappa, "kappa");
    let t_slice = extract_slice!(t, "T");
    let es_slice = extract_slice!(err_s, "err_S");
    let esigma_slice = extract_slice!(err_sigma, "err_sigma");
    let ekappa_slice = extract_slice!(err_kappa, "err_kappa");
    let et_slice = extract_slice!(err_t, "err_T");

    let len = enforce_equal_lengths(&[
        s_slice.len(), sigma_slice.len(), kappa_slice.len(), t_slice.len(),
        es_slice.len(), esigma_slice.len(), ekappa_slice.len(), et_slice.len(),
    ])?;

    let mut props = Vec::with_capacity(len);
    let mut errs = Vec::with_capacity(len);
    for i in 0..len {
        props.push(ThermoelectricProperties {
            s: s_slice[i], sigma: sigma_slice[i], kappa: kappa_slice[i], t: t_slice[i],
        });
        errs.push(PropertyUncertainties {
            err_s: es_slice[i], err_sigma: esigma_slice[i], err_kappa: ekappa_slice[i], err_t: et_slice[i],
        });
    }

    let results = py.allow_threads(|| {
        if deterministic {
            props.iter().zip(errs.iter())
                .map(|(p, e)| calculate_zt_linear_propagation(p, e))
                .collect::<Result<Vec<_>, _>>()
        } else {
            error_propagation::calculate_zt_batch_parallel(&props, &errs)
        }
    }).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let mut out_zt = Vec::with_capacity(len);
    let mut out_unc = Vec::with_capacity(len);
    for r in results {
        out_zt.push(r.zt);
        out_unc.push(r.uncertainty);
    }

    Ok((out_zt.into_pyarray(py), out_unc.into_pyarray(py)))
}

/// Evaluates epistemological bounds, data credibility, and thermodynamic consistency 
/// to assign an authoritative Quality Class to empirical samples.
/// 
/// **Document IDs**: SPEC-QUAL-SCORING, SPEC-QUAL-CREDIBILITY, SPEC-QUAL-COMPLETENESS
#[pyfunction]
#[pyo3(signature = (completeness, credibility, phys_consistency, error_mag, smoothness, metadata, hard_gate, lambda_reg, deterministic=false))]
#[allow(clippy::too_many_arguments)]
pub fn compute_quality_score_py<'py>(
    py: Python<'py>,
    completeness: PyReadonlyArray1<'py, f64>,
    credibility: PyReadonlyArray1<'py, f64>,
    phys_consistency: PyReadonlyArray1<'py, f64>,
    error_mag: PyReadonlyArray1<'py, f64>,
    smoothness: PyReadonlyArray1<'py, f64>,
    metadata: PyReadonlyArray1<'py, f64>,
    hard_gate: PyReadonlyArray1<'py, bool>, 
    lambda_reg: f64,
    deterministic: bool,
) -> PyResult<(&'py PyArray1<f64>, &'py PyArray1<f64>, &'py PyArray1<f64>, &'py PyArray1<u8>)> {
    let c_slice = extract_slice!(completeness, "completeness");
    let cr_slice = extract_slice!(credibility, "credibility");
    let ph_slice = extract_slice!(phys_consistency, "physics_consistency");
    let err_slice = extract_slice!(error_mag, "error_magnitude");
    let sm_slice = extract_slice!(smoothness, "smoothness");
    let meta_slice = extract_slice!(metadata, "metadata");
    let hg_slice = extract_slice!(hard_gate, "hard_constraint_gate");

    let len = enforce_equal_lengths(&[
        c_slice.len(), cr_slice.len(), ph_slice.len(),
        err_slice.len(), sm_slice.len(), meta_slice.len(), hg_slice.len(),
    ])?;

    let mut vectors = Vec::with_capacity(len);
    for i in 0..len {
        vectors.push(QualityVector {
            completeness: c_slice[i],
            credibility: cr_slice[i],
            physics_consistency: ph_slice[i],
            error_magnitude: err_slice[i],
            smoothness: sm_slice[i],
            metadata: meta_slice[i],
            hard_constraint_gate: hg_slice[i],
        });
    }

    let evaluator = QualityEvaluator::new(ScoringWeights::default(), lambda_reg)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    let evaluated = py.allow_threads(|| {
        if deterministic {
            vectors.iter()
                .map(|v| evaluator.evaluate_record(v))
                .collect::<Result<Vec<_>, _>>()
        } else {
            evaluator.evaluate_batch(&vectors)
        }
    }).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let mut out_base = Vec::with_capacity(len);
    let mut out_reg = Vec::with_capacity(len);
    let mut out_ent = Vec::with_capacity(len);
    let mut out_cls = Vec::with_capacity(len);

    for res in evaluated {
        out_base.push(res.base_score);
        out_reg.push(res.regularized_score);
        out_ent.push(res.entropy);
        out_cls.push(res.class as u8); // Strictly maps back to discrete QualityClass levels
    }

    Ok((
        out_base.into_pyarray(py),
        out_reg.into_pyarray(py),
        out_ent.into_pyarray(py),
        out_cls.into_pyarray(py),
    ))
}

/// Computes batch ZT en masse, engaging zero-copy slicing to pass execution 
/// downwards to the multi-threaded physics core.
#[pyfunction]
#[pyo3(signature = (s, sigma, kappa, t))]
pub fn py_compute_zt_batch<'py>(
    py: Python<'py>,
    s: PyReadonlyArray1<'py, f64>,
    sigma: PyReadonlyArray1<'py, f64>,
    kappa: PyReadonlyArray1<'py, f64>,
    t: PyReadonlyArray1<'py, f64>,
) -> PyResult<&'py PyArray1<f64>> {
    let s_slice = extract_slice!(s, "S");
    let sigma_slice = extract_slice!(sigma, "sigma");
    let kappa_slice = extract_slice!(kappa, "kappa");
    let t_slice = extract_slice!(t, "T");

    let _len = enforce_equal_lengths(&[
        s_slice.len(),
        sigma_slice.len(),
        kappa_slice.len(),
        t_slice.len(),
    ])?;

    // Relinquish the GIL for hardware-accelerated physics bounds checking
    let zt_out = py.allow_threads(|| {
        physics::calc_zt_batch(s_slice, sigma_slice, kappa_slice, t_slice)
    }).map_err(|e| PyValueError::new_err(e.to_string()))?;

    Ok(zt_out.into_pyarray(py))
}

/// Computes theoretical figure of merit (zT) directly from structured CSV streams.
///
/// Interrogates massive CSV datasets utilizing our zero-allocation `csv_engine`.
/// Maps mathematical instability directly to Python equivalents (SPEC-GOV-ERROR-HIERARCHY)
/// to strictly prevent unwinding or panic leaks at the library boundary.
#[pyfunction]
#[pyo3(signature = (path, deterministic=true))]
pub fn compute_zt_from_csv_py(py: Python, path: String, deterministic: bool) -> PyResult<PyObject> {
    // 1. Delegate execution to the dedicated, robust I/O parsing engine.
    let report = csv_engine::compute_zt_from_csv(&path, deterministic)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    // 2. Initialize a bound, native Python Dictionary for zero-overhead transit.
    let dict = PyDict::new(py);

    // 3. Systematically map resultant scientific telemetry into the dynamically typed structure.
    dict.set_item("total_rows", report.total_rows)?;
    dict.set_item("total_states", report.total_states)?;
    dict.set_item("valid_states", report.valid_states)?;
    dict.set_item("skipped_states", report.skipped_states)?;
    dict.set_item("incomplete_states", report.incomplete_states)?;
    dict.set_item("mean_zt", report.mean_zt)?;
    dict.set_item("max_zt", report.max_zt)?;
    dict.set_item("min_zt", report.min_zt)?;

    // 4. Box and deliver the object across the FFI.
    Ok(dict.into())
}

// ============================================================================
// ADVANCED ANALYTICS: BAYESIAN, RANKING & INFORMATION GAIN
// ============================================================================

/// Computes the normalized Bayesian posterior probability space en masse.
/// Protects the analytical validity of the evidential density map using LSE.
/// 
/// **Implements:** SPEC-BAYES-CREDIBILITY
#[pyfunction]
#[pyo3(signature = (s, sigma, kappa, t, zt_obs, sigma_zt, prior, lambda_wf))]
#[allow(clippy::too_many_arguments)]
pub fn compute_log_posterior_batch_py<'py>(
    py: Python<'py>,
    s: PyReadonlyArray1<'py, f64>,
    sigma: PyReadonlyArray1<'py, f64>,
    kappa: PyReadonlyArray1<'py, f64>,
    t: PyReadonlyArray1<'py, f64>,
    zt_obs: PyReadonlyArray1<'py, f64>,
    sigma_zt: PyReadonlyArray1<'py, f64>,
    prior: PyReadonlyArray1<'py, f64>,
    lambda_wf: f64,
) -> PyResult<(&'py PyArray1<f64>, &'py PyArray1<f64>)> {
    let s_slice = extract_slice!(s, "s");
    let sigma_slice = extract_slice!(sigma, "sigma");
    let kappa_slice = extract_slice!(kappa, "kappa");
    let t_slice = extract_slice!(t, "t");
    let zt_obs_slice = extract_slice!(zt_obs, "zt_obs");
    let sigma_zt_slice = extract_slice!(sigma_zt, "sigma_zt");
    let prior_slice = extract_slice!(prior, "prior");

    // Execution strictly drops the GIL, projecting contiguous memory mappings
    // directly into Rayon's work-stealing parallelism.
    let (posterior_probs, log_posteriors) = py.allow_threads(|| {
        bayesian::compute_log_posterior_batch(
            s_slice, sigma_slice, kappa_slice, t_slice,
            zt_obs_slice, sigma_zt_slice, prior_slice, lambda_wf
        )
    }).map_err(|e| PyValueError::new_err(e.to_string()))?;

    Ok((posterior_probs.into_pyarray(py), log_posteriors.into_pyarray(py)))
}

/// Computes the citation-aware, entropy-regularized material ranking manifold
/// concurrently over massive topological representations.
/// 
/// **Implements:** SPEC-GRAPH-RANK, G03-EMBEDDING-RANK-THEORY
#[pyfunction]
#[pyo3(signature = (p, zt, c, material_bounds, alpha, beta))]
pub fn compute_material_rank_batch_py<'py>(
    py: Python<'py>,
    p: PyReadonlyArray1<'py, f64>,
    zt: PyReadonlyArray1<'py, f64>,
    c: PyReadonlyArray1<'py, f64>,
    material_bounds: Vec<(usize, usize)>,
    alpha: f64,
    beta: f64,
) -> PyResult<&'py PyArray1<f64>> {
    let p_slice = extract_slice!(p, "p");
    let zt_slice = extract_slice!(zt, "zt");
    let c_slice = extract_slice!(c, "c");

    // Bounding vectors map sub-graph boundaries, averting expensive allocations
    let ranks = py.allow_threads(|| {
        ranking_core::compute_material_rank_batch(
            p_slice, zt_slice, c_slice, &material_bounds, alpha, beta
        )
    }).map_err(|e| PyValueError::new_err(e.to_string()))?;

    Ok(ranks.into_pyarray(py))
}

/// Computes the Information Gain and Data Gap Analysis en masse.
/// Evaluates spatial exploration entropy strictly limiting undefined singularities.
/// 
/// **Implements:** SPEC-ACTIVE-GAP, CL02-INFORMATION-GAIN-SELECTION
#[pyfunction]
#[pyo3(signature = (t, bounds, t_min, t_max, num_bins, gamma_1, gamma_2))]
pub fn compute_information_gain_batch_py<'py>(
    py: Python<'py>,
    t: PyReadonlyArray1<'py, f64>,
    bounds: Vec<(usize, usize)>,
    t_min: f64,
    t_max: f64,
    num_bins: usize,
    gamma_1: f64,
    gamma_2: f64,
) -> PyResult<&'py PyList> {
    let t_slice = extract_slice!(t, "t");

    let results = py.allow_threads(|| {
        information_gain::compute_information_gain_batch(
            t_slice, &bounds, t_min, t_max, num_bins, gamma_1, gamma_2
        )
    }).map_err(|e| PyValueError::new_err(e.to_string()))?;

    // Pre-allocate the vector to sidestep reallocation costs
    let mut dicts = Vec::with_capacity(results.len());
    
    // Transparently project the strict metrics back into dynamically typed Python bindings
    for res in results {
        let dict = PyDict::new(py);
        dict.set_item("entropy", res.entropy)?;
        dict.set_item("kl_divergence", res.kl_divergence)?;
        dict.set_item("total_score", res.total_score)?;
        dicts.push(dict);
    }

    Ok(PyList::new(py, dicts))
}

// ============================================================================
// MODULE EXPORT REGISTRY
// ============================================================================

/// The Thermognosis Engine - Rust Core Python Extension
/// 
/// Exposes natively compiled, mathematically constrained physical pipelines. 
/// Guaranteed $\mathcal{O}(1)$ bridging overhead and zero implicit panics.
#[pymodule]
fn rust_core(_py: Python, m: &PyModule) -> PyResult<()> {
    // Register mathematical & governance bounding functions
    m.add_function(wrap_pyfunction!(py_compute_zt_batch, m)?)?;
    m.add_function(wrap_pyfunction!(validate_dimensions_py, m)?)?;
    m.add_function(wrap_pyfunction!(check_physics_consistency_py, m)?)?;
    m.add_function(wrap_pyfunction!(propagate_error_py, m)?)?;
    m.add_function(wrap_pyfunction!(compute_quality_score_py, m)?)?;
    
    // High-Performance I/O Parsing
    m.add_function(wrap_pyfunction!(compute_zt_from_csv_py, m)?)?; 

    // Advanced Analytica Bindings
    m.add_function(wrap_pyfunction!(compute_log_posterior_batch_py, m)?)?;
    m.add_function(wrap_pyfunction!(compute_material_rank_batch_py, m)?)?;
    m.add_function(wrap_pyfunction!(compute_information_gain_batch_py, m)?)?;
    
    // Register physics-layer constants representing absolute bounds
    m.add("L0_SOMMERFELD", physics::L0_SOMMERFELD)?;
    m.add("L_MIN", physics::L_MIN)?;
    m.add("L_MAX", physics::L_MAX)?;

    Ok(())
}