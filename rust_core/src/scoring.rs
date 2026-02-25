//! Quality Scoring and Decision Framework
//!
//! Implements the formal evaluation of scientific reliability, evidential strength,
//! and trustworthiness of data and derived results.
//! 
//! Layer: spec/06_quality
//! Status: Normative

use rayon::prelude::*;
use std::f64;
use thiserror::Error;

/// Formal Error Hierarchy for the Quality Scoring Module.
/// Implements: SPEC-QUAL-SCORING Section 15 (Error Classification)
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ScoringError {
    #[error("QUAL-SCORE-01: Component score out of bounds [0,1]. Component index: {0}, Value: {1}")]
    InvalidComponentBound(usize, f64),
    
    #[error("QUAL-SCORE-02: Weight misconfiguration. Weights must sum to 1.0. Current sum: {0}")]
    InvalidWeights(f64),
    
    #[error("QUAL-SCORE-04: Entropy instability. A component value resulted in an undefined state.")]
    EntropyInstability,
}

/// Enumeration of permissible Quality Classes.
/// Implements: SPEC-QUAL-SCORING Section 8 (Threshold Classification)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum QualityClass {
    ClassA = 1, // S >= 0.90
    ClassB = 2, // 0.80 <= S < 0.90
    ClassC = 3, // 0.65 <= S < 0.80
    ClassD = 4, // 0.50 <= S < 0.65
    Reject = 5, // S < 0.50
}

impl QualityClass {
    /// Classifies a real-valued score into a definitive Quality Class.
    /// Implements: SPEC-QUAL-SCORING Section 8
    #[inline]
    pub fn from_score(score: f64) -> Self {
        if score >= 0.90 {
            QualityClass::ClassA
        } else if score >= 0.80 {
            QualityClass::ClassB
        } else if score >= 0.65 {
            QualityClass::ClassC
        } else if score >= 0.50 {
            QualityClass::ClassD
        } else {
            QualityClass::Reject
        }
    }
}

/// The weight vector configuration for the Aggregation Model.
/// Implements: SPEC-QUAL-SCORING Section 4 (Weighted Aggregation Model)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoringWeights {
    pub comp: f64,
    pub cred: f64,
    pub phys: f64,
    pub err: f64,
    pub smooth: f64,
    pub meta: f64,
}

impl Default for ScoringWeights {
    /// Returns the normative default weights for the Thermognosis Engine.
    #[inline]
    fn default() -> Self {
        Self {
            comp: 0.25,
            cred: 0.25,
            phys: 0.20,
            err: 0.15,
            smooth: 0.10,
            meta: 0.05,
        }
    }
}

impl ScoringWeights {
    /// Validates that the sum of weights is mathematically exactly 1.0 (within epsilon).
    #[inline]
    pub fn validate(&self) -> Result<(), ScoringError> {
        let sum = self.comp + self.cred + self.phys + self.err + self.smooth + self.meta;
        if (sum - 1.0).abs() > f64::EPSILON {
            return Err(ScoringError::InvalidWeights(sum));
        }
        Ok(())
    }
}

/// Represents the n-dimensional quality state of a given scientific record \mathcal{R}.
/// Implements: SPEC-QUAL-SCORING Section 2 (Quality Vector Representation)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QualityVector {
    pub completeness: f64,
    pub credibility: f64,
    pub physics_consistency: f64,
    pub error_magnitude: f64,
    pub smoothness: f64,
    pub metadata: f64,
    pub hard_constraint_gate: bool,
}

impl QualityVector {
    /// Validates the epistemological bounds of the quality vector components ($q_i \in [0,1]$).
    #[inline]
    pub fn validate(&self) -> Result<(), ScoringError> {
        let components = [
            self.completeness,
            self.credibility,
            self.physics_consistency,
            self.error_magnitude,
            self.smoothness,
            self.metadata,
        ];
        
        for (i, &q) in components.iter().enumerate() {
            if !(0.0..=1.0).contains(&q) {
                return Err(ScoringError::InvalidComponentBound(i, q));
            }
        }
        Ok(())
    }

    /// Computes the Information Entropy of the Quality Vector: $H(\mathbf{q}) = - \sum q_i \ln q_i$
    /// Implements: SPEC-QUAL-SCORING Section 6 (Entropy-Regularized Score)
    #[inline]
    pub fn compute_entropy(&self) -> f64 {
        let components = [
            self.completeness,
            self.credibility,
            self.physics_consistency,
            self.error_magnitude,
            self.smoothness,
            self.metadata,
        ];

        let mut entropy = 0.0;
        for &q in &components {
            if q > 0.0 {
                // Ensure q_i ln(q_i) is mathematically bounded; 0 ln(0) resolves to 0.
                entropy += q * q.ln();
            }
        }
        -entropy
    }

    /// Computes the base Weighted Aggregation Score $S(\mathcal{R}) = \sum w_i q_i$
    /// Implements: SPEC-QUAL-SCORING Section 4
    #[inline]
    pub fn compute_base_score(&self, weights: &ScoringWeights) -> f64 {
        (self.completeness * weights.comp)
            + (self.credibility * weights.cred)
            + (self.physics_consistency * weights.phys)
            + (self.error_magnitude * weights.err)
            + (self.smoothness * weights.smooth)
            + (self.metadata * weights.meta)
    }
}

/// The resultant struct from the scoring engine containing decoupled metric analysis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoringResult {
    pub base_score: f64,
    pub regularized_score: f64,
    pub entropy: f64,
    pub class: QualityClass,
}

/// Primary orchestrator for the UQSS mathematical implementations.
pub struct QualityEvaluator {
    pub weights: ScoringWeights,
    pub lambda: f64,
}

impl QualityEvaluator {
    /// Instantiates a new Evaluator. Verifies constraints upon creation.
    pub fn new(weights: ScoringWeights, lambda: f64) -> Result<Self, ScoringError> {
        weights.validate()?;
        Ok(Self { weights, lambda })
    }

    /// Evaluates a singular record ($\mathcal{R}$), mapping constraints, entropy, and baseline.
    /// Implements: SPEC-QUAL-SCORING Sections 3, 4, 6, and 8
    #[inline]
    pub fn evaluate_record(&self, vector: &QualityVector) -> Result<ScoringResult, ScoringError> {
        vector.validate()?;

        // Implements: SPEC-QUAL-SCORING Section 3 (Hard Constraint Gate)
        // If mandatory constraints are violated, the final score drops strictly to 0.
        if !vector.hard_constraint_gate {
            return Ok(ScoringResult {
                base_score: 0.0,
                regularized_score: 0.0,
                entropy: 0.0,
                class: QualityClass::Reject,
            });
        }

        let base_score = vector.compute_base_score(&self.weights);
        let entropy = vector.compute_entropy();
        
        // S_reg = S - \lambda * H(\mathbf{q})
        let regularized_score = base_score - (self.lambda * entropy);
        
        // Ensure final bounded constraints theoretically map properly within [0,1]
        let bounded_score = regularized_score.clamp(0.0, 1.0);
        let class = QualityClass::from_score(bounded_score);

        Ok(ScoringResult {
            base_score,
            regularized_score: bounded_score,
            entropy,
            class,
        })
    }

    /// Executes highly parallelized, deterministic batch scoring on millions of records.
    /// Powered by `rayon`, avoiding CPU bottlenecks on FFI interop transitions.
    pub fn evaluate_batch(
        &self,
        vectors: &[QualityVector],
    ) -> Result<Vec<ScoringResult>, ScoringError> {
        vectors
            .par_iter()
            .map(|vector| self.evaluate_record(vector))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_record() {
        let eval = QualityEvaluator::new(ScoringWeights::default(), 0.01).unwrap();
        let perfect_vector = QualityVector {
            completeness: 1.0,
            credibility: 1.0,
            physics_consistency: 1.0,
            error_magnitude: 1.0,
            smoothness: 1.0,
            metadata: 1.0,
            hard_constraint_gate: true,
        };

        let result = eval.evaluate_record(&perfect_vector).unwrap();
        // Since lambda is non-zero, entropy of all 1s (-1*ln(1)*6) is 0. 
        // Hence regularized_score remains 1.0.
        assert!((result.base_score - 1.0).abs() < f64::EPSILON);
        assert_eq!(result.class, QualityClass::ClassA);
    }

    #[test]
    fn test_hard_constraint_failure() {
        let eval = QualityEvaluator::new(ScoringWeights::default(), 0.1).unwrap();
        let vector = QualityVector {
            completeness: 0.9, credibility: 0.9, physics_consistency: 0.9,
            error_magnitude: 0.9, smoothness: 0.9, metadata: 0.9,
            hard_constraint_gate: false, // SPEC-QUAL-SCORING Section 3 Trigger
        };

        let result = eval.evaluate_record(&vector).unwrap();
        assert_eq!(result.base_score, 0.0);
        assert_eq!(result.class, QualityClass::Reject);
    }
}