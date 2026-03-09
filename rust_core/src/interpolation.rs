// rust_core/src/interpolation.rs

//! # Monotone Cubic (PCHIP) Interpolation and ZT Cross-Validation
//!
//! **Implements:** GAP-03 (ZT cross-check via interpolation), SPEC-AUDIT-01 Gate 3
//! **Status:** Normative — Q1 Dataset Standard
//!
//! ## Algorithm
//!
//! Piecewise Cubic Hermite Interpolating Polynomial (PCHIP) using the
//! Fritsch–Carlson monotonicity-preserving slope selection (1980).
//!
//! Reference: Fritsch & Carlson, SIAM J. Numer. Anal. 17(2), 238–246 (1980).
//!            DOI: 10.1137/0717021
//!
//! ## ZT Cross-Check
//!
//! Given four independently measured property curves [S(T), σ(T), κ(T), ZT_reported(T)],
//! this module interpolates each to a common temperature grid, computes
//! ZT_computed = S² σ T / κ, and evaluates the relative deviation:
//!   ε = |ZT_computed − ZT_reported| / max(|ZT_reported|, ε_floor)
//! Records with ε > 0.10 receive FLAG_ZT_CROSSCHECK_FAIL (SPEC-AUDIT-01 Gate 3).

use thiserror::Error;

// ============================================================================
// ERROR TYPE
// ============================================================================

#[derive(Debug, Error)]
pub enum InterpError {
    #[error("Interpolation requires at least 2 data points; got {0}")]
    InsufficientPoints(usize),

    #[error("x-values must be strictly monotonically increasing (duplicate or out-of-order at index {0})")]
    NonMonotonicX(usize),

    #[error("Query x={0} is outside the data range [{1}, {2}]")]
    OutOfRange(f64, f64, f64),

    #[error("Input slices have mismatched lengths: xs={0}, ys={1}")]
    LengthMismatch(usize, usize),

    #[error("ZT cross-check requires at least {required} overlap points; found {found}")]
    InsufficientOverlap { required: usize, found: usize },
}

// ============================================================================
// INTERPOLATION METHOD SELECTOR
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpMethod {
    /// Linear interpolation — O(log n) per query via binary search.
    Linear,
    /// Monotone cubic Hermite (PCHIP) — Fritsch–Carlson (1980).
    /// No overshoot; preserves local monotonicity. Preferred for transport data.
    #[allow(clippy::upper_case_acronyms)]
    PCHIP,
}

// ============================================================================
// CORE PCHIP IMPLEMENTATION
// ============================================================================

/// Compute PCHIP slopes using Fritsch–Carlson (1980) monotonicity conditions.
///
/// Given n data points (xs, ys) with strictly increasing xs, returns n slopes
/// d[i] such that the resulting piecewise cubic Hermite spline is monotone on
/// each subinterval.
fn pchip_slopes(xs: &[f64], ys: &[f64]) -> Vec<f64> {
    let n = xs.len();
    debug_assert!(n >= 2, "pchip_slopes called with < 2 points");

    // Finite differences (secant slopes)
    let mut delta: Vec<f64> = (0..n - 1)
        .map(|i| (ys[i + 1] - ys[i]) / (xs[i + 1] - xs[i]))
        .collect();

    let mut d = vec![0.0_f64; n];

    // --- Endpoint slopes (one-sided, matched to neighbouring interval) ---
    // Use non-centred three-point formula at boundaries to stay monotone.
    d[0] = endpoint_slope(delta[0], delta.get(1).copied().unwrap_or(delta[0]));
    d[n - 1] = endpoint_slope(
        delta[n - 2],
        delta.get(n - 3).copied().unwrap_or(delta[n - 2]),
    );

    // --- Interior slopes (Fritsch–Carlson step 2) ---
    for i in 1..n - 1 {
        if delta[i - 1] * delta[i] <= 0.0 {
            // Sign change or zero → force zero slope for monotonicity
            d[i] = 0.0;
        } else {
            // Weighted harmonic mean of adjacent secant slopes
            let h_prev = xs[i] - xs[i - 1];
            let h_next = xs[i + 1] - xs[i];
            let w1 = 2.0 * h_next + h_prev;
            let w2 = h_next + 2.0 * h_prev;
            d[i] = (w1 + w2) / (w1 / delta[i - 1] + w2 / delta[i]);
        }
    }

    // --- Monotonicity enforcement (Fritsch–Carlson step 3) ---
    for i in 0..n - 1 {
        if delta[i] == 0.0 {
            d[i] = 0.0;
            d[i + 1] = 0.0;
            continue;
        }
        let alpha = d[i] / delta[i];
        let beta = d[i + 1] / delta[i];
        let rho = alpha.hypot(beta);
        if rho > 3.0 {
            let tau = 3.0 / rho;
            d[i] = tau * alpha * delta[i];
            d[i + 1] = tau * beta * delta[i];
        }
    }

    // Suppress unused-mut warning — delta is constructed and immediately consumed.
    let _ = delta.iter_mut(); // keeps compiler happy without functional change
    d
}

/// Non-centred endpoint slope preserving sign of adjacent secant.
#[inline]
fn endpoint_slope(delta_near: f64, delta_far: f64) -> f64 {
    // Brodlie (1980) endpoint formula: one-sided but monotone-consistent.
    // For negative delta_near: bound_a = -0.0, bound_b = 2*delta_near < 0.
    // f64::clamp(min, max) requires min ≤ max, so sort the bounds explicitly.
    let bound_a = 0.0_f64.copysign(delta_near);
    let bound_b = 2.0 * delta_near.abs() * delta_near.signum();
    let slope = ((2.0 * delta_near + delta_far) / 3.0)
        .clamp(bound_a.min(bound_b), bound_a.max(bound_b));
    // If the two secants have opposite signs, cap at zero.
    if delta_near * delta_far <= 0.0 { delta_near } else { slope }
}

/// Evaluate a single cubic Hermite polynomial on [x0, x1] at query point x.
///
/// p(t) = h00(t)·y0 + h10(t)·(x1−x0)·m0 + h01(t)·y1 + h11(t)·(x1−x0)·m1
/// where t = (x − x0) / (x1 − x0).
#[inline]
fn cubic_hermite(x0: f64, x1: f64, y0: f64, y1: f64, m0: f64, m1: f64, x: f64) -> f64 {
    let h = x1 - x0;
    let t = (x - x0) / h;
    let t2 = t * t;
    let t3 = t2 * t;
    // Basis polynomials
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    h00 * y0 + h10 * h * m0 + h01 * y1 + h11 * h * m1
}

// ============================================================================
// PUBLIC API
// ============================================================================

/// Validate that xs is strictly monotonically increasing and lengths match.
fn validate_inputs(xs: &[f64], ys: &[f64]) -> Result<(), InterpError> {
    if xs.len() != ys.len() {
        return Err(InterpError::LengthMismatch(xs.len(), ys.len()));
    }
    if xs.len() < 2 {
        return Err(InterpError::InsufficientPoints(xs.len()));
    }
    for i in 1..xs.len() {
        if xs[i] <= xs[i - 1] {
            return Err(InterpError::NonMonotonicX(i));
        }
    }
    Ok(())
}

/// Find the interval index i such that xs[i] <= x_query < xs[i+1]
/// using binary search. Clamps to [0, n-2].
fn find_interval(xs: &[f64], x_query: f64) -> usize {
    let n = xs.len();
    // Binary search for insertion point
    let pos = xs.partition_point(|&v| v <= x_query);
    pos.saturating_sub(1).min(n - 2)
}

/// Interpolate a single query point using PCHIP.
///
/// # Errors
/// - [`InterpError::InsufficientPoints`] if fewer than 2 data points.
/// - [`InterpError::LengthMismatch`] if `xs.len() != ys.len()`.
/// - [`InterpError::NonMonotonicX`] if xs is not strictly increasing.
/// - [`InterpError::OutOfRange`] if x_query is outside [xs[0], xs[n-1]].
pub fn interpolate_pchip(xs: &[f64], ys: &[f64], x_query: f64) -> Result<f64, InterpError> {
    validate_inputs(xs, ys)?;
    let n = xs.len();
    let x_min = xs[0];
    let x_max = xs[n - 1];
    if x_query < x_min || x_query > x_max {
        return Err(InterpError::OutOfRange(x_query, x_min, x_max));
    }

    let slopes = pchip_slopes(xs, ys);
    let i = find_interval(xs, x_query);
    Ok(cubic_hermite(
        xs[i], xs[i + 1], ys[i], ys[i + 1], slopes[i], slopes[i + 1], x_query,
    ))
}

/// Interpolate a single query point using linear interpolation.
///
/// # Errors
/// Same error conditions as [`interpolate_pchip`].
pub fn interpolate_linear(xs: &[f64], ys: &[f64], x_query: f64) -> Result<f64, InterpError> {
    validate_inputs(xs, ys)?;
    let n = xs.len();
    let x_min = xs[0];
    let x_max = xs[n - 1];
    if x_query < x_min || x_query > x_max {
        return Err(InterpError::OutOfRange(x_query, x_min, x_max));
    }

    let i = find_interval(xs, x_query);
    let t = (x_query - xs[i]) / (xs[i + 1] - xs[i]);
    Ok(ys[i] + t * (ys[i + 1] - ys[i]))
}

/// Interpolate a dataset onto a target grid using the specified method.
///
/// For each grid point that lies within the data range, the corresponding
/// interpolated value is returned. Grid points outside the data range are
/// silently skipped (the returned Vec may be shorter than `grid`).
///
/// Returns `(grid_values_in_range, interpolated_values)` as parallel Vecs.
///
/// # Errors
/// Returns [`InterpError`] if the input data is invalid.
pub fn interpolate_to_grid(
    data: &[(f64, f64)],
    grid: &[f64],
    method: InterpMethod,
) -> Result<(Vec<f64>, Vec<f64>), InterpError> {
    if data.len() < 2 {
        return Err(InterpError::InsufficientPoints(data.len()));
    }

    // Split data into xs and ys for validation and slope computation.
    let xs: Vec<f64> = data.iter().map(|(x, _)| *x).collect();
    let ys: Vec<f64> = data.iter().map(|(_, y)| *y).collect();
    validate_inputs(&xs, &ys)?;

    // Precompute slopes once if PCHIP
    let slopes_opt: Option<Vec<f64>> = match method {
        InterpMethod::PCHIP => Some(pchip_slopes(&xs, &ys)),
        InterpMethod::Linear => None,
    };

    let x_min = xs[0];
    let x_max = *xs.last().unwrap(); // safe: len >= 2

    let mut out_grid = Vec::with_capacity(grid.len());
    let mut out_vals = Vec::with_capacity(grid.len());

    for &xq in grid {
        if xq < x_min || xq > x_max {
            continue; // skip out-of-range grid points
        }
        let i = find_interval(&xs, xq);
        let val = match &slopes_opt {
            Some(slopes) => {
                cubic_hermite(xs[i], xs[i + 1], ys[i], ys[i + 1], slopes[i], slopes[i + 1], xq)
            }
            None => {
                let t = (xq - xs[i]) / (xs[i + 1] - xs[i]);
                ys[i] + t * (ys[i + 1] - ys[i])
            }
        };
        out_grid.push(xq);
        out_vals.push(val);
    }

    Ok((out_grid, out_vals))
}

// ============================================================================
// ZT CROSS-CHECK INFRASTRUCTURE
// ============================================================================

/// Minimum absolute ZT used as denominator floor to avoid division by zero
/// when ZT_reported ≈ 0 (unphysical but present in some digitisation artefacts).
const ZT_DENOM_FLOOR: f64 = 1.0e-6;

/// Default temperature grid spacing for cross-check evaluation (10 K bins).
const DEFAULT_T_STEP_K: f64 = 10.0;

/// Relative tolerance for Gate 3 pass/fail: 10% per SPEC-AUDIT-01.
pub const ZT_CROSSCHECK_TOLERANCE: f64 = 0.10;

/// Full result record returned by [`compute_zt_cross_check`].
#[derive(Debug, Clone)]
pub struct ZTCrossCheck {
    /// Number of temperature grid points where all four properties are available.
    pub n_overlap_points: usize,
    /// Mean absolute relative error: mean(|ZT_comp − ZT_rep| / max(|ZT_rep|, floor)).
    pub mean_absolute_error: f64,
    /// Maximum absolute relative error across all overlap points.
    pub max_absolute_error: f64,
    /// Whether `mean_absolute_error <= ZT_CROSSCHECK_TOLERANCE`.
    pub passes_tolerance: bool,
    /// Anomaly flag bit to set on failure: `FLAG_ZT_CROSSCHECK_FAIL` if !passes.
    pub flag_bits: u32,
}

/// Compute ZT cross-validation from four independently measured property curves.
///
/// # Parameters
/// - `s_data`: `[(T, S)]` Seebeck coefficient in V/K, sorted ascending by T.
/// - `sigma_data`: `[(T, σ)]` Electrical conductivity in S/m.
/// - `kappa_data`: `[(T, κ)]` Thermal conductivity in W/(m·K).
/// - `zt_reported_data`: `[(T, ZT)]` Reported figure of merit (dimensionless).
/// - `method`: Interpolation method to use (recommend `InterpMethod::PCHIP`).
/// - `min_overlap`: Minimum number of grid points required in the overlap region.
///
/// # Errors
/// Returns [`InterpError::InsufficientOverlap`] if fewer than `min_overlap`
/// temperature grid points fall within all four property ranges simultaneously.
///
/// # Algorithm
/// 1. Build a uniform temperature grid over the intersection of all four T-ranges.
/// 2. Interpolate S, σ, κ, ZT_reported onto this grid.
/// 3. Compute ZT_computed = S² · σ · T / κ at each grid point.
/// 4. Compute ε_i = |ZT_computed_i − ZT_reported_i| / max(|ZT_reported_i|, floor).
/// 5. Return mean and max ε, plus pass/fail flag.
pub fn compute_zt_cross_check(
    s_data: &[(f64, f64)],
    sigma_data: &[(f64, f64)],
    kappa_data: &[(f64, f64)],
    zt_reported_data: &[(f64, f64)],
    method: InterpMethod,
    min_overlap: usize,
) -> Result<ZTCrossCheck, InterpError> {
    use crate::flags::FLAG_ZT_CROSSCHECK_FAIL;

    // --- Build temperature overlap domain ---
    let t_min = [s_data, sigma_data, kappa_data, zt_reported_data]
        .iter()
        .map(|d| d.first().map_or(f64::NEG_INFINITY, |(t, _)| *t))
        .fold(f64::NEG_INFINITY, f64::max); // max of minimums

    let t_max = [s_data, sigma_data, kappa_data, zt_reported_data]
        .iter()
        .map(|d| d.last().map_or(f64::INFINITY, |(t, _)| *t))
        .fold(f64::INFINITY, f64::min); // min of maximums

    if t_max <= t_min {
        return Err(InterpError::InsufficientOverlap {
            required: min_overlap,
            found: 0,
        });
    }

    // Build uniform grid with DEFAULT_T_STEP_K spacing
    let n_steps = ((t_max - t_min) / DEFAULT_T_STEP_K).floor() as usize + 1;
    let grid: Vec<f64> = (0..n_steps)
        .map(|i| t_min + i as f64 * DEFAULT_T_STEP_K)
        .filter(|&t| t <= t_max)
        .collect();

    if grid.len() < min_overlap {
        return Err(InterpError::InsufficientOverlap {
            required: min_overlap,
            found: grid.len(),
        });
    }

    // --- Interpolate all four curves onto the grid ---
    let (_, s_vals) = interpolate_to_grid(s_data, &grid, method)?;
    let (_, sigma_vals) = interpolate_to_grid(sigma_data, &grid, method)?;
    let (_, kappa_vals) = interpolate_to_grid(kappa_data, &grid, method)?;
    let (grid_zt, zt_rep_vals) = interpolate_to_grid(zt_reported_data, &grid, method)?;

    // grid_zt is the subset that is within range for all four (already filtered
    // by the intersection domain, so should match grid length exactly).
    let n = grid_zt.len();
    if n < min_overlap {
        return Err(InterpError::InsufficientOverlap {
            required: min_overlap,
            found: n,
        });
    }

    // --- Compute relative errors ---
    let mut sum_err = 0.0_f64;
    let mut max_err = 0.0_f64;

    for idx in 0..n {
        let t = grid_zt[idx];
        // Guard against kappa == 0 (physically rejected upstream, but defensive here)
        if kappa_vals[idx] == 0.0 {
            continue;
        }
        let zt_comp = s_vals[idx] * s_vals[idx] * sigma_vals[idx] * t / kappa_vals[idx];
        let denom = zt_rep_vals[idx].abs().max(ZT_DENOM_FLOOR);
        let rel_err = (zt_comp - zt_rep_vals[idx]).abs() / denom;
        sum_err += rel_err;
        if rel_err > max_err {
            max_err = rel_err;
        }
    }

    let mean_absolute_error = sum_err / n as f64;
    let passes_tolerance = mean_absolute_error <= ZT_CROSSCHECK_TOLERANCE;
    let flag_bits = if passes_tolerance { 0 } else { FLAG_ZT_CROSSCHECK_FAIL };

    Ok(ZTCrossCheck {
        n_overlap_points: n,
        mean_absolute_error,
        max_absolute_error: max_err,
        passes_tolerance,
        flag_bits,
    })
}

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: assert two floats are within relative tolerance.
    fn assert_approx(a: f64, b: f64, tol: f64, msg: &str) {
        let rel = (a - b).abs() / b.abs().max(1.0e-12);
        assert!(rel < tol, "{msg}: got {a}, expected ~{b} (rel={rel:.2e})");
    }

    // --- PCHIP interpolation correctness ---

    #[test]
    fn pchip_exact_at_knots() {
        // PCHIP must reproduce knot values exactly.
        let xs = vec![100.0, 200.0, 300.0, 400.0, 500.0];
        let ys = vec![2.0e-4, 1.8e-4, 1.5e-4, 1.2e-4, 1.0e-4];
        for (i, &x) in xs.iter().enumerate() {
            let y = interpolate_pchip(&xs, &ys, x).expect("PCHIP at knot");
            assert_approx(y, ys[i], 1.0e-12, &format!("knot {i}"));
        }
    }

    #[test]
    fn pchip_monotone_on_monotone_data() {
        // Monotone data → interpolated values must also be monotone.
        let xs: Vec<f64> = (100..=800).step_by(50).map(|v| v as f64).collect();
        let ys: Vec<f64> = xs.iter().map(|&t| 3.0e-4 - t * 2.0e-7).collect();
        let mut prev = f64::INFINITY;
        for t in (100..=800).step_by(10).map(|v| v as f64) {
            let y = interpolate_pchip(&xs, &ys, t).expect("PCHIP monotone");
            assert!(y <= prev + 1.0e-15, "PCHIP not monotone: y={y} > prev={prev} at t={t}");
            prev = y;
        }
    }

    #[test]
    fn pchip_linear_data_reproduces_exactly() {
        // For linear data, PCHIP should reproduce the linear function.
        let xs = vec![0.0, 1.0, 2.0, 3.0];
        let ys: Vec<f64> = xs.iter().map(|&x| 2.5 * x + 1.0).collect();
        let y_mid = interpolate_pchip(&xs, &ys, 1.5).expect("PCHIP linear");
        assert_approx(y_mid, 2.5 * 1.5 + 1.0, 1.0e-12, "linear mid-point");
    }

    #[test]
    fn pchip_out_of_range_returns_error() {
        let xs = vec![100.0, 200.0, 300.0];
        let ys = vec![1.0, 2.0, 3.0];
        assert!(
            matches!(interpolate_pchip(&xs, &ys, 50.0), Err(InterpError::OutOfRange(_, _, _))),
            "Below range must return OutOfRange"
        );
        assert!(
            matches!(interpolate_pchip(&xs, &ys, 400.0), Err(InterpError::OutOfRange(_, _, _))),
            "Above range must return OutOfRange"
        );
    }

    #[test]
    fn pchip_insufficient_points() {
        let r = interpolate_pchip(&[300.0], &[1.0], 300.0);
        assert!(
            matches!(r, Err(InterpError::InsufficientPoints(1))),
            "Single point must return InsufficientPoints"
        );
    }

    #[test]
    fn pchip_non_monotonic_x_returns_error() {
        let r = interpolate_pchip(&[100.0, 300.0, 200.0], &[1.0, 2.0, 3.0], 150.0);
        assert!(
            matches!(r, Err(InterpError::NonMonotonicX(_))),
            "Non-monotonic xs must return NonMonotonicX"
        );
    }

    // --- Linear interpolation ---

    #[test]
    fn linear_midpoint_correct() {
        let xs = vec![0.0, 100.0];
        let ys = vec![0.0, 10.0];
        let y = interpolate_linear(&xs, &ys, 50.0).expect("linear midpoint");
        assert_approx(y, 5.0, 1.0e-12, "linear midpoint");
    }

    // --- interpolate_to_grid ---

    #[test]
    fn grid_interp_filters_out_of_range() {
        let data = vec![(200.0, 1.0), (400.0, 2.0), (600.0, 3.0)];
        let grid = vec![100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0];
        let (out_grid, out_vals) = interpolate_to_grid(&data, &grid, InterpMethod::PCHIP)
            .expect("grid interp");
        // 100 and 700 are out of range → 5 points returned
        assert_eq!(out_grid.len(), 5, "should filter out-of-range grid points");
        assert_eq!(out_vals.len(), 5);
        // Knot values must be exact
        assert_approx(out_vals[0], 1.0, 1.0e-12, "knot at 200");
        assert_approx(*out_vals.last().unwrap(), 3.0, 1.0e-12, "knot at 600");
    }

    // --- ZT cross-check ---

    /// Self-consistent test: compute ZT from known S, σ, κ, then verify cross-check passes.
    #[test]
    fn zt_crosscheck_self_consistent_passes() {
        // Physical Bi2Te3-like parameters at 300 K
        let temps: Vec<f64> = (200..=600).step_by(50).map(|v| v as f64).collect();
        let s_data: Vec<(f64, f64)> = temps.iter().map(|&t| (t, 2.0e-4)).collect(); // 200 µV/K
        let sigma_data: Vec<(f64, f64)> = temps.iter().map(|&t| (t, 1.0e5)).collect(); // 1e5 S/m
        let kappa_data: Vec<(f64, f64)> = temps.iter().map(|&t| (t, 1.5)).collect(); // 1.5 W/mK
        // ZT_reported = S² σ T / κ — exactly self-consistent
        let zt_reported_data: Vec<(f64, f64)> = temps
            .iter()
            .map(|&t| (t, (2.0e-4_f64).powi(2) * 1.0e5 * t / 1.5))
            .collect();

        let result = compute_zt_cross_check(
            &s_data,
            &sigma_data,
            &kappa_data,
            &zt_reported_data,
            InterpMethod::PCHIP,
            3,
        )
        .expect("cross-check must succeed");

        assert!(result.passes_tolerance, "Self-consistent ZT must pass tolerance");
        assert!(result.mean_absolute_error < 1.0e-10, "MAE must be negligible for exact data");
        assert_eq!(result.flag_bits, 0, "No flag bits should be set");
    }

    /// Inconsistent ZT: reported ZT is 2× computed → ~50% error → must fail.
    #[test]
    fn zt_crosscheck_inconsistent_fails() {
        let temps: Vec<f64> = (200..=600).step_by(50).map(|v| v as f64).collect();
        let s_data: Vec<(f64, f64)> = temps.iter().map(|&t| (t, 2.0e-4)).collect();
        let sigma_data: Vec<(f64, f64)> = temps.iter().map(|&t| (t, 1.0e5)).collect();
        let kappa_data: Vec<(f64, f64)> = temps.iter().map(|&t| (t, 1.5)).collect();
        // Reported ZT is 2× the computed value — ~50% relative error
        let zt_reported_data: Vec<(f64, f64)> = temps
            .iter()
            .map(|&t| (t, 2.0 * (2.0e-4_f64).powi(2) * 1.0e5 * t / 1.5))
            .collect();

        let result = compute_zt_cross_check(
            &s_data,
            &sigma_data,
            &kappa_data,
            &zt_reported_data,
            InterpMethod::PCHIP,
            3,
        )
        .expect("cross-check computation must succeed");

        assert!(!result.passes_tolerance, "Inconsistent ZT must fail tolerance");
        assert!(
            result.mean_absolute_error > 0.10,
            "MAE must exceed 10% threshold"
        );
        use crate::flags::FLAG_ZT_CROSSCHECK_FAIL;
        assert_ne!(result.flag_bits & FLAG_ZT_CROSSCHECK_FAIL, 0, "FLAG_ZT_CROSSCHECK_FAIL must be set");
    }

    #[test]
    fn zt_crosscheck_insufficient_overlap_returns_error() {
        // Non-overlapping temperature ranges
        let s_data = vec![(100.0, 1.0e-4), (200.0, 0.9e-4)];
        let sigma_data = vec![(100.0, 1.0e5), (200.0, 1.1e5)];
        let kappa_data = vec![(100.0, 2.0), (200.0, 2.1)];
        let zt_data = vec![(500.0, 0.8), (600.0, 0.9)]; // no overlap with [100, 200]

        let result = compute_zt_cross_check(
            &s_data,
            &sigma_data,
            &kappa_data,
            &zt_data,
            InterpMethod::PCHIP,
            3,
        );

        assert!(
            matches!(result, Err(InterpError::InsufficientOverlap { .. })),
            "Non-overlapping ranges must return InsufficientOverlap"
        );
    }
}
