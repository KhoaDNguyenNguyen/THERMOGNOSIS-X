# filepath: python/thermognosis/pipeline/ingestion.py
"""
Thermognosis Engine: Raw Measurement Ingestion Module
Document ID: SPEC-CONTRACT-RAW-MEASUREMENT
Layer: spec/01_contracts (Implementation)

This module governs the ingestion, validation, storage, and traceability
of primary experimental measurements. It ensures identity consistency,
enforces SI unit compliance, and constructs immutable dataclass entities
for downstream statistical and graph representations.
"""

import hashlib
import json
import logging
import warnings
from dataclasses import dataclass
from typing import Dict, Any, List, Optional, Union
from pathlib import Path

import numpy as np
import pandas as pd

# -----------------------------------------------------------------------------
# Global Conventions & System Constants
# Implements: SPEC-GOV-GLOBAL-CONVENTIONS
# -----------------------------------------------------------------------------
SPEC_VERSION = "v1.0.0"
LORENTZ_NUMBER = 2.44e-8  # W * Ohm / K^2 (Wiedemann-Franz constant L_0)

logger = logging.getLogger(__name__)
logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")


# Try to import canonical hashing, fallback to rigorous deterministic local implementation
try:
    from thermognosis.utils.hashing import generate_canonical_hash
except ImportError:
    def generate_canonical_hash(*args) -> str:
        """
        Computes a deterministic SHA-256 hash from canonical JSON serialization.
        
        Implements: SPEC-CONTRACT-MATERIAL, Section 3.1
        """
        payload = json.dumps(args, sort_keys=True, ensure_ascii=True, separators=(',', ':'))
        return hashlib.sha256(payload.encode('utf-8')).hexdigest()


# -----------------------------------------------------------------------------
# Error Hierarchy
# Implements: SPEC-GOV-ERROR-HIERARCHY
# -----------------------------------------------------------------------------
class ThermognosisError(Exception):
    """Base exception for all Thermognosis Engine mathematical/physical violations."""
    pass

class ThermognosisMissingDataError(ThermognosisError):
    """Raised when mandatory epistemic or measured fields are missing."""
    pass

class ThermognosisPhysicalConstraintError(ThermognosisError):
    """Raised when measurements violate fundamental thermodynamic bounds."""
    pass

class ThermognosisUncertaintyError(ThermognosisError):
    """Raised when uncertainty metrics violate non-negativity constraints."""
    pass


# -----------------------------------------------------------------------------
# Immutable Entities
# Implements: SPEC-CONTRACT-RAW-MEASUREMENT
# -----------------------------------------------------------------------------
@dataclass(frozen=True)
class MeasurementQuantities:
    """
    Core physical quantities. All must be in SI units.
    
    Q = (T, S, sigma, kappa)
    """
    T: float      # Temperature [K]
    S: float      # Seebeck coefficient [V/K]
    sigma: float  # Electrical conductivity [S/m]
    kappa: float  # Thermal conductivity [W/(m*K)]


@dataclass(frozen=True)
class MeasurementUncertainties:
    """
    Measurement uncertainty structure.
    
    U = (sigma_T, sigma_S, sigma_sigma, sigma_kappa)
    Constraint: sigma_x >= 0
    """
    sigma_T: float
    sigma_S: float
    sigma_sigma: float
    sigma_kappa: float


@dataclass(frozen=True)
class MeasurementContext:
    """
    Contextual metadata defining the experimental environment.
    """
    pressure: str
    atmosphere: str
    sample_orientation: str
    doping_level: str


@dataclass(frozen=True)
class MeasurementMethod:
    """
    Methodological metadata for experimental traceability.
    """
    instrument: str
    calibration_date: str
    technique: str
    resolution: str


@dataclass(frozen=True)
class RawMeasurement:
    """
    The atomic unit of experimental data. Immutable and deterministically hashed.
    
    Implements: SPEC-CONTRACT-RAW-MEASUREMENT, Section 2
    Formula: R = (ID, MAT_ID, PAPER_ID, Q, U, C, M)
    """
    id: str
    mat_id: str
    paper_id: str
    quantities: MeasurementQuantities
    uncertainties: MeasurementUncertainties
    context: Optional[MeasurementContext] = None
    method: Optional[MeasurementMethod] = None

    @property
    def zT(self) -> float:
        """
        Derived Thermoelectric Figure of Merit.
        Formula: zT = (S^2 * sigma * T) / kappa
        
        Implements: SPEC-CONTRACT-RAW-MEASUREMENT, Section 4
        """
        return (self.quantities.S**2 * self.quantities.sigma * self.quantities.T) / self.quantities.kappa


# -----------------------------------------------------------------------------
# Ingestion Logic
# -----------------------------------------------------------------------------
class MeasurementIngestor:
    """
    Governs the ingestion, validation, and instantiation of RawMeasurement entities.
    Ensures that no physical impossibilities or epistemic gaps enter the system.
    """
    
    def __init__(self, check_wiedemann_franz: bool = True):
        self.check_wiedemann_franz = check_wiedemann_franz

    def _validate_mandatory_fields(self, record: Dict[str, Any]) -> None:
        """Ensures all mathematically required fields are present."""
        required = [
            'mat_id', 'paper_id', 
            'T', 'S', 'sigma', 'kappa', 
            'u_T', 'u_S', 'u_sigma', 'u_kappa'
        ]
        missing = [field for field in required if field not in record or pd.isna(record[field])]
        if missing:
            raise ThermognosisMissingDataError(
                f"Record missing mandatory fields: {missing}. "
                "Orphan or incomplete measurements are prohibited."
            )

    def _validate_physical_constraints(self, q: MeasurementQuantities) -> None:
        """
        Enforces strict thermodynamic and logical boundaries on raw signals.
        Implements: SPEC-CONTRACT-RAW-MEASUREMENT, Section 9
        """
        if q.T <= 0:
            raise ThermognosisPhysicalConstraintError(f"Temperature must be strictly positive. Got T={q.T} K.")
        if q.sigma <= 0:
            raise ThermognosisPhysicalConstraintError(f"Electrical conductivity must be strictly positive. Got sigma={q.sigma} S/m.")
        if q.kappa <= 0:
            raise ThermognosisPhysicalConstraintError(f"Thermal conductivity must be strictly positive. Got kappa={q.kappa} W/(m*K).")

        # Optional Wiedemann-Franz consistency check: kappa_e <= L_0 * sigma * T
        # Because total kappa = kappa_e + kappa_l, total kappa must be > kappa_e
        if self.check_wiedemann_franz:
            kappa_e_estimate = LORENTZ_NUMBER * q.sigma * q.T
            if q.kappa < kappa_e_estimate:
                warnings.warn(
                    f"Wiedemann-Franz violation detected: kappa ({q.kappa}) < kappa_e_estimate ({kappa_e_estimate}). "
                    "This triggers a governance flag for review.", 
                    RuntimeWarning
                )

    def _validate_uncertainties(self, u: MeasurementUncertainties) -> None:
        """
        Ensures non-negativity of statistical variances/standard deviations.
        Implements: SPEC-CONTRACT-RAW-MEASUREMENT, Section 6
        """
        if any(val < 0 for val in (u.sigma_T, u.sigma_S, u.sigma_sigma, u.sigma_kappa)):
            raise ThermognosisUncertaintyError(
                f"Uncertainty cannot be negative. Got: U=({u.sigma_T}, {u.sigma_S}, {u.sigma_sigma}, {u.sigma_kappa})"
            )

    def ingest_record(self, record: Dict[str, Any]) -> RawMeasurement:
        """
        Ingests a single raw measurement dictionary, applying all formal validation.
        
        Parameters
        ----------
        record : Dict[str, Any]
            Raw dictionary containing 'mat_id', 'paper_id', physical quantities, and uncertainties.
            
        Returns
        -------
        RawMeasurement
            An immutable, canonical measurement entity.
        """
        self._validate_mandatory_fields(record)

        q = MeasurementQuantities(
            T=float(record['T']),
            S=float(record['S']),
            sigma=float(record['sigma']),
            kappa=float(record['kappa'])
        )
        
        u = MeasurementUncertainties(
            sigma_T=float(record['u_T']),
            sigma_S=float(record['u_S']),
            sigma_sigma=float(record['u_sigma']),
            sigma_kappa=float(record['u_kappa'])
        )

        self._validate_physical_constraints(q)
        self._validate_uncertainties(u)

        # Deterministic ID Generation
        # ID = Hash(MAT_ID, PAPER_ID, T, S, sigma, kappa)
        meas_hash = generate_canonical_hash(
            record['mat_id'], 
            record['paper_id'], 
            q.T, q.S, q.sigma, q.kappa
        )
        entity_id = f"MEAS_{meas_hash}"

        # Optional Context Hydration
        ctx = None
        if all(k in record for k in ['pressure', 'atmosphere', 'sample_orientation', 'doping_level']):
            ctx = MeasurementContext(
                pressure=str(record['pressure']),
                atmosphere=str(record['atmosphere']),
                sample_orientation=str(record['sample_orientation']),
                doping_level=str(record['doping_level'])
            )

        # Optional Method Hydration
        method = None
        if all(k in record for k in ['instrument', 'calibration_date', 'technique', 'resolution']):
            method = MeasurementMethod(
                instrument=str(record['instrument']),
                calibration_date=str(record['calibration_date']),
                technique=str(record['technique']),
                resolution=str(record['resolution'])
            )

        entity = RawMeasurement(
            id=entity_id,
            mat_id=str(record['mat_id']),
            paper_id=str(record['paper_id']),
            quantities=q,
            uncertainties=u,
            context=ctx,
            method=method
        )
        
        return entity

    def ingest_dataframe(self, df: pd.DataFrame) -> List[RawMeasurement]:
        """
        Ingests a tabular pandas DataFrame into a strictly validated list of RawMeasurement objects.
        Optimized for high-throughput batch ingestion while preserving entity guarantees.
        
        Parameters
        ----------
        df : pd.DataFrame
            The dataframe containing batch experimental data.
            
        Returns
        -------
        List[RawMeasurement]
            A list of instantiated, immutable measurement objects.
        """
        if df.empty:
            logger.warning("Attempted to ingest an empty dataframe. Returning empty list.")
            return []
            
        # Pre-validate structure prior to iterative hydration to fail fast
        required_cols = {'mat_id', 'paper_id', 'T', 'S', 'sigma', 'kappa', 'u_T', 'u_S', 'u_sigma', 'u_kappa'}
        missing = required_cols - set(df.columns)
        if missing:
            raise ThermognosisMissingDataError(f"DataFrame is missing structurally required columns: {missing}")

        entities = []
        for row in df.itertuples(index=False):
            record_dict = row._asdict()
            try:
                entity = self.ingest_record(record_dict)
                entities.append(entity)
            except ThermognosisError as e:
                # In strict environments, any single failure poisons the batch to prevent partial-state corruption.
                logger.error(f"Ingestion failed for MAT_ID={record_dict.get('mat_id')} at row indexing. Error: {str(e)}")
                raise

        logger.info(f"Successfully ingested {len(entities)} raw measurements. Spec Version: {SPEC_VERSION}")
        return entities