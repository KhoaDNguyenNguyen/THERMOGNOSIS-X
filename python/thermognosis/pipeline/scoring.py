"""
Thermognosis Engine: Unified Quality and Credibility Scoring Framework
======================================================================

Implements:
    - SPEC-QUAL-SCORING: Unified Quality Scoring Specification (UQSS)
    - SPEC-QUAL-CREDIBILITY: Credibility Specification (CRS)

This module evaluates scientific reliability, evidential strength, and 
integrates multiple quality dimensions into a unified, mathematically 
sound decision metric. It strictly governs model training eligibility 
and closed-loop experimentation bounds.

Layer: spec/06_quality
Compliance Level: Research-Grade / Q1 Infrastructure Standard
"""

import math
from enum import Enum
from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple, Union

import numpy as np


class QualityScoreError(Exception):
    """Base exception for SPEC-QUAL-SCORING violations."""
    pass


class CredibilityScoreError(Exception):
    """Base exception for SPEC-QUAL-CREDIBILITY violations."""
    pass


class QualityClass(str, Enum):
    """
    Quality classification thresholds.
    Implements: SPEC-QUAL-SCORING Section 8
    """
    CLASS_A = "Class A"  # S >= 0.90
    CLASS_B = "Class B"  # 0.80 <= S < 0.90
    CLASS_C = "Class C"  # 0.65 <= S < 0.80
    CLASS_D = "Class D"  # 0.50 <= S < 0.65
    REJECT = "Reject"    # S < 0.50


class CredibilityClass(str, Enum):
    """
    Credibility classification thresholds.
    Implements: SPEC-QUAL-CREDIBILITY Section 14
    """
    CLASS_A = "Class A"  # K >= 0.90
    CLASS_B = "Class B"  # 0.75 <= K < 0.90
    CLASS_C = "Class C"  # 0.50 <= K < 0.75
    CLASS_D = "Class D"  # K < 0.50


@dataclass(frozen=True)
class QualityVector:
    """
    Vector representation of record quality components.
    All values must be bounded in [0, 1].
    
    Implements: SPEC-QUAL-SCORING Section 2
    """
    q_comp: float
    q_cred: float
    q_phys: float
    q_err: float
    q_smooth: float
    q_meta: float

    def to_numpy(self) -> np.ndarray:
        """Returns the components as a strictly ordered numpy array."""
        arr = np.array([
            self.q_comp, self.q_cred, self.q_phys,
            self.q_err, self.q_smooth, self.q_meta
        ], dtype=np.float64)
        
        if np.any(np.isnan(arr)):
            raise QualityScoreError("QUAL-SCORE-01: Missing component score (NaN detected).")
        if np.any((arr < 0.0) | (arr > 1.0)):
            raise QualityScoreError("QUAL-SCORE-02: Quality components must strictly lie in [0, 1].")
            
        return arr


class CredibilityScorer:
    """
    Calculates the scientific credibility score K(R) of a given record.
    
    Implements: SPEC-QUAL-CREDIBILITY
    """
    
    @staticmethod
    def calculate_credibility(
        w_source: float,
        n_rep: int,
        w_unc: float,
        delta_phys: float,
        n: int,
        e_cv: float,
        t_current: float,
        t_pub: float,
        alpha: float = 1.0,
        n_0: float = 10.0,
        beta: float = 1.0,
        lambda_time: float = 0.05
    ) -> float:
        r"""
        Computes the composite credibility score.
        
        Math:
            \mathcal{K}(\mathcal{R}) = w_{source} \cdot w_{rep} \cdot w_{unc} \cdot w_{phys} \cdot w_{stat} \cdot w_{model} \cdot w_{time}
            
        Parameters
        ----------
        w_source : float
            Base source weight in [0, 1].
        n_rep : int
            Number of independent confirmations (reproducibility).
        w_unc : float
            Uncertainty transparency weight (1.0, 0.5, or 0.0).
        delta_phys : float
            Magnitude of physical constraint violation.
        n : int
            Sample size for statistical robustness.
        e_cv : float
            Cross-validation error of surrogate model.
        t_current : float
            Current time (e.g., year).
        t_pub : float
            Publication time.
        alpha, n_0, beta, lambda_time : float
            Scaling constants for the respective decay functions.
            
        Returns
        -------
        float
            The overall credibility score bounded in [0, 1].
        """
        # Validate Bounds
        if not (0.0 <= w_source <= 1.0):
            raise CredibilityScoreError("QUAL-CRED-01: Source weight must be in [0, 1].")
        if w_unc not in {0.0, 0.5, 1.0}:
            raise CredibilityScoreError("QUAL-CRED-03: Uncertainty weight must be exactly 0, 0.5, or 1.")
        if n < 0:
            raise CredibilityScoreError("QUAL-CRED-05: Sample size n cannot be negative.")
        
        # Component Calculations
        w_rep = 1.0 - math.exp(-n_rep) if n_rep >= 1 else 0.0
        w_phys = math.exp(-alpha * delta_phys) if delta_phys > 0 else 1.0
        w_stat = n / (n + n_0)
        w_model = math.exp(-beta * e_cv)
        w_time = math.exp(-lambda_time * max(0.0, t_current - t_pub))
        
        # Multiplicative composite score ensures weakest component dominates
        k_score = w_source * w_rep * w_unc * w_phys * w_stat * w_model * w_time
        
        return float(np.clip(k_score, 0.0, 1.0))

    @staticmethod
    def classify(score: float) -> CredibilityClass:
        """Classifies the credibility score according to Section 14."""
        if score >= 0.90:
            return CredibilityClass.CLASS_A
        elif score >= 0.75:
            return CredibilityClass.CLASS_B
        elif score >= 0.50:
            return CredibilityClass.CLASS_C
        else:
            return CredibilityClass.CLASS_D


class QualityScorer:
    """
    Computes unified decision metrics aggregating multiple quality dimensions.
    
    Implements: SPEC-QUAL-SCORING
    """

    DEFAULT_WEIGHTS = np.array([0.25, 0.25, 0.20, 0.15, 0.10, 0.05], dtype=np.float64)

    def __init__(self, weights: Optional[np.ndarray] = None):
        """
        Initializes the scorer and guarantees weight determinism & normalization.
        
        Parameters
        ----------
        weights : np.ndarray, optional
            A 6-element array of weights. Defaults to the specification defaults.
        """
        self.weights = weights if weights is not None else self.DEFAULT_WEIGHTS
        
        if len(self.weights) != 6:
            raise QualityScoreError("QUAL-SCORE-02: Weight misconfiguration. Expected exactly 6 weights.")
        if not np.isclose(np.sum(self.weights), 1.0, atol=1e-6):
            raise QualityScoreError(f"QUAL-SCORE-03: Non-normalized weights. Sum is {np.sum(self.weights):.4f}")

    def score_linear(self, gate: bool, q_vector: QualityVector) -> float:
        r"""
        Computes the standard weighted aggregation model.
        
        Math:
            S(\mathcal{R}) = \mathcal{G}(\mathcal{R}) \sum_{i=1}^n w_i q_i
        """
        if not gate:
            return 0.0
        
        q = q_vector.to_numpy()
        return float(np.dot(self.weights, q))

    def score_multiplicative(self, gate: bool, q_vector: QualityVector) -> float:
        r"""
        Computes the multiplicative risk-sensitive model.
        Sensitive to the weakest dimension; penalizes imbalance.
        
        Math:
            S_{mult}(\mathcal{R}) = \mathcal{G}(\mathcal{R}) \prod_{i=1}^n q_i^{w_i}
        """
        if not gate:
            return 0.0
            
        q = q_vector.to_numpy()
        # Protect against 0^0 or log(0) domain errors by using np.power
        # np.power correctly handles 0^w -> 0 for w>0.
        score = np.prod(np.power(q, self.weights))
        return float(score)

    def score_entropy_regularized(self, gate: bool, q_vector: QualityVector, lambda_reg: float = 0.1) -> float:
        r"""
        Computes the entropy-regularized score.
        
        Math:
            H(\mathbf{q}) = - \sum_i q_i \log q_i
            S_{reg} = S - \lambda H(\mathbf{q})
        """
        if not gate:
            return 0.0
            
        q = q_vector.to_numpy()
        s_base = float(np.dot(self.weights, q))
        
        # Calculate entropy safely (lim_{x->0} x*log(x) = 0)
        with np.errstate(divide='ignore', invalid='ignore'):
            entropy_terms = np.where(q > 0, q * np.log(q), 0.0)
            
        if np.any(np.isnan(entropy_terms)):
            raise QualityScoreError("QUAL-SCORE-04: Entropy instability detected during computation.")
            
        entropy = -np.sum(entropy_terms)
        s_reg = s_base - (lambda_reg * entropy)
        
        # Ensure mathematically bounded in [0, 1] after regularization
        return float(np.clip(s_reg, 0.0, 1.0))

    def score_risk_adjusted(self, gate: bool, q_mu: QualityVector, q_sigma: QualityVector, gamma: float = 1.0) -> float:
        r"""
        Computes the uncertainty-weighted adjustment score.
        
        Math:
            \mathbb{E}[S] = \sum_i w_i \mu_i
            S_{risk} = \mathbb{E}[S] - \gamma \sqrt{\mathrm{Var}(S)}
            \text{where } \mathrm{Var}(S) = \sum_i w_i^2 \sigma_i^2
        """
        if not gate:
            return 0.0
            
        mu = q_mu.to_numpy()
        sigma = q_sigma.to_numpy()
        
        expected_s = np.dot(self.weights, mu)
        var_s = np.dot(self.weights**2, sigma**2)
        
        if var_s < 0:
            raise QualityScoreError("QUAL-SCORE-05: Risk-adjusted miscalculation. Negative variance detected.")
            
        s_risk = expected_s - gamma * np.sqrt(var_s)
        
        return float(np.clip(s_risk, 0.0, 1.0))

    @staticmethod
    def classify(score: float) -> QualityClass:
        """
        Classifies the unified quality score according to Section 8.
        Default modeling eligibility typically requires >= 0.80 (Class B).
        """
        if score >= 0.90:
            return QualityClass.CLASS_A
        elif score >= 0.80:
            return QualityClass.CLASS_B
        elif score >= 0.65:
            return QualityClass.CLASS_C
        elif score >= 0.50:
            return QualityClass.CLASS_D
        else:
            return QualityClass.REJECT

    @staticmethod
    def pareto_dominates(q1: QualityVector, q2: QualityVector) -> bool:
        r"""
        Evaluates Pareto-Dominance Analysis (Section 9).
        
        Math:
            \mathcal{R}_1 \succ \mathcal{R}_2 \iff q_i^{(1)} \ge q_i^{(2)} \forall i \text{ and } \exists j : q_j^{(1)} > q_j^{(2)}
        """
        v1 = q1.to_numpy()
        v2 = q2.to_numpy()
        
        strictly_greater_any = np.any(v1 > v2)
        greater_equal_all = np.all(v1 >= v2)
        
        return bool(greater_equal_all and strictly_greater_any)