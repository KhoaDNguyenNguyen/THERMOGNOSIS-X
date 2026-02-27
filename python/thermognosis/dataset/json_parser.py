"""
Thermognosis Engine: JSON Epistemic Bridge
==========================================

This module strictly normalizes raw, heterogeneous JSON files from materials science 
databases into physically constrained, immutable Python objects. It acts as the 
epistemic bridge between dirty external data and the Thermognosis ML pipeline.

Performance Constraints:
    - O(1) Memory Footprint per file stream using pure Generators.
    - Class instantiation overhead minimized via `slots=True`.
    - Deterministic across Linux, Windows, and Colab environments via `pathlib`.
"""

import json
import logging
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Generator, List, Tuple, Any, Dict, Union

# Initialize strict module-level logger per SPEC-GOV-ERROR-HIERARCHY
logger = logging.getLogger("thermognosis.logging")


class ThermognosisError(Exception):
    """Base exception for the Thermognosis computational engine."""
    pass


class ParserError(ThermognosisError):
    """Raised when critical metadata parsing fails, necessitating a file drop."""
    pass


@dataclass(frozen=True, slots=True)
class SampleRecord:
    """
    Immutable representation of a material sample's metadata.
    
    Parameters
    ----------
    sample_id : int
        Unique identifier for the sample.
    composition : str
        Chemical composition (e.g., 'PbTe').
    paper_id : int
        Identifier of the publication source.
    figure_ids : Tuple[int, ...]
        Identifiers of the figures containing the sample data.
    """
    sample_id: int
    composition: str
    paper_id: int
    figure_ids: Tuple[int, ...]
    measurement_type: str


@dataclass(frozen=True, slots=True)
class DataPointRecord:
    """
    Immutable representation of a single physical property measurement.
    
    Parameters
    ----------
    sample_id : int
        Corresponding sample identifier.
    property_x : str
        Name of the independent variable (e.g., 'Temperature').
    property_y : str
        Name of the dependent variable (e.g., 'Seebeck Coefficient').
    unit_x : str
        Physical unit of X.
    unit_y : str
        Physical unit of Y.
    x : float
        Scalar value of the independent variable.
    y : float
        Scalar value of the dependent variable.
    """
    sample_id: int
    property_x: str
    property_y: str
    unit_x: str
    unit_y: str
    x: float
    y: float


@dataclass(frozen=True, slots=True)
class PaperRecord:
    """
    Immutable representation of publication metadata.
    
    Parameters
    ----------
    paper_id : int
        Unique identifier for the paper.
    doi : str
        Digital Object Identifier.
    year : int
        Year of publication.
    journal : str
        Name of the scientific journal.
    """
    paper_id: int
    doi: str
    year: int
    journal: str


def parse_sample_json(file_path: Path) -> Tuple[SampleRecord, List[DataPointRecord]]:
    """
    Parses a heterogenous sample JSON file into immutable records.
    
    Enforces strict mathematical typing and skips malformed floating-point
    artifacts without crashing the broader pipeline batch.

    Parameters
    ----------
    file_path : pathlib.Path
        Absolute or relative path to the sample JSON file.

    Returns
    -------
    Tuple[SampleRecord, List[DataPointRecord]]
        The normalized sample metadata and a list of validated data points.
        
    Raises
    ------
    ParserError
        If IO operations fail, JSON is fundamentally corrupted, or critical 
        sample metadata is missing.
    """
    try:
        with file_path.open('r', encoding='utf-8') as f:
            raw_data = json.load(f)
    except json.JSONDecodeError as e:
        logger.error(f"[FATAL] Malformed JSON structure in {file_path}: {e}")
        raise ParserError(f"JSON decode failure: {file_path}") from e
    except OSError as e:
        logger.error(f"[FATAL] IO error reading {file_path}: {e}")
        raise ParserError(f"IO accessibility failure: {file_path}") from e

    # Stage 1: Critical Metadata Extraction
    try:
        # sample_id = int(raw_data['sample_id'])
        # sample_id = int(raw_data.get('sample_id', raw_data.get('sampleid')))
        # # composition = str(raw_data['composition'])
        # composition = str(raw_data.get('composition', raw_data.get('chemical_formula', 'UNKNOWN')))
        # paper_id = int(raw_data['paper_id'])
        # ---- Robust sample_id extraction ----
        # raw_sample_id = raw_data.get('sample_id') or raw_data.get('sampleid')
        # if raw_sample_id is None:
        #     raise ValueError("Missing sample_id")
        # sample_id = int(raw_sample_id)

        # # ---- Composition ----
        # composition = str(
        #     raw_data.get('composition')
        #     or raw_data.get('chemical_formula')
        #     or "UNKNOWN"
        # )

        # # ---- Robust paper_id extraction ----
        # raw_paper_id = raw_data.get('paper_id') or raw_data.get('paperid')
        # if raw_paper_id is None:
        #     raise ValueError("Missing paper_id")
        # paper_id = int(raw_paper_id)

        # # safely coerce figures into an immutable tuple of ints
        # raw_figures = raw_data.get('figure_ids',[])
        # figure_ids = tuple(int(fid) for fid in raw_figures)
        # Starrydata format: sample is a list
        sample_block = raw_data["sample"][0]
        data_type = (
            sample_block
            .get("sampleinfo",{})
            .get("DataType",{})
            .get("category","Unknown")
        )
        sample_id = int(sample_block["sampleid"])

        composition = str(
            sample_block.get("composition","UNKNOWN")
        )

        paper_id = int(sample_block["paperid"])

        # figure ids extracted from rawdata
        figure_ids = tuple(
            int(d["figureid"])
            for d in raw_data.get("rawdata",[])
        )

        sample_record = SampleRecord(
            sample_id=sample_id,
            composition=composition,
            paper_id=paper_id,
            figure_ids=figure_ids,
            measurement_type=data_type
        )
    except (KeyError, ValueError, TypeError) as e:
        logger.error(f"[FATAL] Schema mismatch in sample metadata {file_path}: {e}")
        raise ParserError(f"Critical metadata extraction failed in {file_path}") from e

    # Stage 2: Data Point Normalization
    data_points: List[DataPointRecord] = []
    # raw_points = raw_data.get('data_points',[])
    raw_points = raw_data.get("rawdata",[])
    
    if not isinstance(raw_points, list):
        logger.warning(f"[WARN] 'data_points' is not iterable in {file_path}. Defaulting to empty.")
        raw_points =[]

    for idx, pt in enumerate(raw_points):
        try:
            x_val = float(pt['x'])
            y_val = float(pt['y'])
            
            # Physics Constraint Check: Nan / Inf representations are unacceptable
            if math.isnan(x_val) or math.isnan(y_val) or math.isinf(x_val) or math.isinf(y_val):
                raise ValueError("Encountered NaN or Inf thermodynamic artifact.")

            # dp = DataPointRecord(
            #     sample_id=sample_id,
            #     property_x=str(pt['property_x']),
            #     property_y=str(pt['property_y']),
            #     unit_x=str(pt['unit_x']),
            #     unit_y=str(pt['unit_y']),
            #     x=x_val,
            #     y=y_val
            # )
            # Build property lookup
            property_lookup = {
                p["propertyid"]: (p["propertyname"], p["unit"])
                for p in raw_data.get("property",[])
            }

            prop_x,unit_x = property_lookup.get(
                pt["propertyid_x"],
                ("UNKNOWN","")
            )

            prop_y,unit_y = property_lookup.get(
                pt["propertyid_y"],
                ("UNKNOWN","")
            )

            dp = DataPointRecord(
                sample_id=sample_id,
                property_x=prop_x,
                property_y=prop_y,
                unit_x=unit_x,
                unit_y=unit_y,
                x=x_val,
                y=y_val
            )

            data_points.append(dp)
            # if (
            #     dp.property_x == "Temperature"
            #     and dp.property_y in ALLOWED_Y
            # ):
            #     data_points.append(dp)

        except (KeyError, ValueError, TypeError) as e:
            # Graceful degradation: Log and skip only the specific malformed point
            logger.warning(f"[WARN] Skipping malformed point index {idx} in {file_path}: {e}")
            continue

    return sample_record, data_points


def parse_paper_json(file_path: Path) -> PaperRecord:
    """
    Parses a JSON file containing publication metadata into a PaperRecord.

    Parameters
    ----------
    file_path : pathlib.Path
        Path to the paper metadata JSON.

    Returns
    -------
    PaperRecord
        Immutable representation of the paper.
        
    Raises
    ------
    ParserError
        If JSON is invalid or required attributes are absent.
    """
    try:
        with file_path.open('r', encoding='utf-8') as f:
            raw_data = json.load(f)
            
        return PaperRecord(
            paper_id=int(raw_data['paper_id']),
            doi=str(raw_data['doi']),
            year=int(raw_data['year']),
            journal=str(raw_data['journal'])
        )
    except (json.JSONDecodeError, OSError) as e:
        logger.error(f"[FATAL] Cannot read paper JSON {file_path}: {e}")
        raise ParserError(f"Read error on {file_path}") from e
    except (KeyError, ValueError, TypeError) as e:
        logger.error(f"[FATAL] Invalid paper schema in {file_path}: {e}")
        raise ParserError(f"Schema error on {file_path}") from e

ALLOWED_Y = {
    "Thermal conductivity",
    "Seebeck coefficient",
    "Electrical resistivity",
    "Electrical conductivity",
    "ZT"
}

def stream_samples(
                    directory: Path,
                    allowed_types=("Experiment",)
                )-> Generator[Tuple[SampleRecord, DataPointRecord], None, None]:
    """
    Lazy-loads and yields parsed data point tuples sequentially from a directory.
    
    Guarantees O(1) memory complexity during the processing of highly scaled 
    datasets (e.g., 50,000+ files). Corrupt files are skipped and logged.

    Parameters
    ----------
    directory : pathlib.Path
        Target directory containing sample JSON files.

    Yields
    ------
    Tuple[SampleRecord, DataPointRecord]
        A mathematically robust pair representing metadata and a single observation.
    """
    if not directory.exists() or not directory.is_dir():
        logger.error(f"[FATAL] Invalid target directory for streaming: {directory}")
        raise ParserError(f"Directory not found: {directory}")

    # Use rglob for recursive path yields - minimizing RAM footprint
    for file_path in directory.rglob("*.json"):
        try:
            sample_record, data_points = parse_sample_json(file_path)
            if sample_record.measurement_type not in allowed_types:
                logger.info(
                    f"[FILTER] Reject sample {sample_record.sample_id} "
                    f"type={sample_record.measurement_type}"
                )
                continue

            for dp in data_points:
                yield (sample_record, dp)
        except ParserError as e:
            logger.warning(f"[WARN] Evicting file from stream due to critical parse failure: {e}")
            continue


# ==============================================================================
# Pytest Coverage (Unit & Integration)
# Append these tests at the bottom of the file or extract them to `tests/test_json_parser.py`
# ==============================================================================

if __name__ == "pytest":
    import pytest

    @pytest.fixture
    def mock_data_dir(tmp_path: Path) -> Path:
        """Fixture generating strictly governed, temporary scientific data."""
        data_dir = tmp_path / "thermognosis_data"
        data_dir.mkdir()
        
        # 1. Valid File
        valid_json = {
            "sample_id": 101,
            "composition": "Bi2Te3",
            "paper_id": 99,
            "figure_ids": [1, 2],
            "data_points":[
                {"property_x": "T", "property_y": "S", "unit_x": "K", "unit_y": "uV/K", "x": 300.0, "y": 210.5},
                {"property_x": "T", "property_y": "S", "unit_x": "K", "unit_y": "uV/K", "x": 400.0, "y": 250.1}
            ]
        }
        (data_dir / "valid_sample.json").write_text(json.dumps(valid_json), encoding='utf-8')
        
        # 2. File with missing/malformed fields
        dirty_json = {
            "sample_id": 102,
            "composition": "PbTe",
            "paper_id": 100,
            # figure_ids missing to test robustness
            "data_points":[
                {"property_x": "T", "property_y": "S", "unit_x": "K", "unit_y": "uV/K", "x": 300.0, "y": "NaN"}, # Malformed Y
                {"property_x": "T", "property_y": "S", "unit_x": "K", "unit_y": "uV/K", "x": 400.0, "y": 180.0}, # Valid
                {"property_x": "T", "unit_x": "K", "unit_y": "uV/K", "x": 500.0, "y": 190.0} # Missing property_y
            ]
        }
        (data_dir / "dirty_sample.json").write_text(json.dumps(dirty_json), encoding='utf-8')
        
        # 3. Completely Corrupted File (simulating broken IO)
        (data_dir / "corrupted_sample.json").write_text("{ broken json \n", encoding='utf-8')

        return data_dir

    def test_parse_sample(mock_data_dir: Path):
        """Tests optimal deterministic parsing on strictly conformant data."""
        target = mock_data_dir / "valid_sample.json"
        sample, points = parse_sample_json(target)
        
        assert sample.sample_id == 101
        assert sample.composition == "Bi2Te3"
        assert sample.figure_ids == (1, 2)
        assert len(points) == 2
        assert points[0].x == 300.0
        assert points[1].y == 250.1

    def test_missing_fields(mock_data_dir: Path):
        """Tests pipeline resilience: gracefully skips dirty points without crashing."""
        target = mock_data_dir / "dirty_sample.json"
        sample, points = parse_sample_json(target)
        
        # Validates tuple conversion defaults when missing
        assert sample.figure_ids == ()
        
        # The parser should discard the 'NaN' and the missing 'property_y' points
        # Retaining only the single valid data point.
        assert len(points) == 1
        assert points[0].x == 400.0
        assert points[0].y == 180.0

    def test_streaming(mock_data_dir: Path):
        """Tests the lazily evaluated data stream for exact row extraction."""
        # Across the three files, only 3 valid data points exist
        # 2 in valid_sample, 1 in dirty_sample, 0 in corrupted_sample
        stream = stream_samples(mock_data_dir)
        results = list(stream)
        
        assert len(results) == 3
        # Ensure correct type output
        assert isinstance(results[0][0], SampleRecord)
        assert isinstance(results[0][1], DataPointRecord)