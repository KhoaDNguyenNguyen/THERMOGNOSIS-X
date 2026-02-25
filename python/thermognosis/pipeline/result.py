"""
Thermognosis Engine: Pipeline Result Structures.

This module defines the strictly immutable data structures responsible for
capturing, validating, and formatting the terminal metrics of a high-throughput
computational materials pipeline execution.

Implements: SPEC-PIPELINE-ORCHESTRATION, SPEC-PIPELINE-END-TO-END
"""

from dataclasses import dataclass
from typing import Any


class PipelineMetricValidationError(ValueError):
    """
    Structured exception raised when pipeline metrics violate physical,
    mathematical, or logical constraints (e.g., negative time or counts).
    
    Implements: SPEC-GOV-ERROR-HIERARCHY
    """
    pass


@dataclass(frozen=True)
class PipelineResult:
    """
    An immutable record of pipeline execution metrics.

    This data structure captures the terminal state of a Thermognosis
    pipeline run. By utilizing a frozen dataclass, we mathematically 
    guarantee that post-execution metrics cannot be silently mutated 
    by downstream logging or persistence processes.

    Implements: SPEC-PIPELINE-ORCHESTRATION, SPEC-PIPELINE-END-TO-END

    Parameters
    ----------
    total_processed : int
        The total number of materials or molecular structures evaluated.
    total_failed : int
        The total number of structures that failed parsing, processing, 
        or mathematical convergence.
    total_inserted : int
        The total number of structures successfully written to the 
        persistence layer (database).
    average_score : float
        The mean thermodynamic or fitness score (e.g., zT, stability) 
        across all successfully processed entities.
    physics_violations : int
        The number of structures rejected due to violating fundamental 
        physical constraints (e.g., non-positive entropy, invalid 
        stoichiometry).
    processing_time_seconds : float
        The wall-clock time in seconds consumed by the entire pipeline 
        execution.

    Raises
    ------
    PipelineMetricValidationError
        If any count or temporal metric strictly evaluates to a negative 
        number, violating logical bounds.
    """

    total_processed: int
    total_failed: int
    total_inserted: int
    average_score: float
    physics_violations: int
    processing_time_seconds: float

    def __post_init__(self) -> None:
        """
        Validates the fundamental integrity of the pipeline metrics.
        
        Since this dataclass is frozen, validation must occur post-initialization.
        No metric representing a physical count or temporal duration may be negative.
        """
        if self.total_processed < 0:
            raise PipelineMetricValidationError("total_processed cannot be negative.")
        if self.total_failed < 0:
            raise PipelineMetricValidationError("total_failed cannot be negative.")
        if self.total_inserted < 0:
            raise PipelineMetricValidationError("total_inserted cannot be negative.")
        if self.physics_violations < 0:
            raise PipelineMetricValidationError("physics_violations cannot be negative.")
        if self.processing_time_seconds < 0.0:
            raise PipelineMetricValidationError("processing_time_seconds cannot be negative.")
        
        # Logical consistency check
        if self.total_inserted + self.total_failed > self.total_processed:
            raise PipelineMetricValidationError(
                "Sum of inserted and failed structures cannot exceed total_processed."
            )

    def __repr__(self) -> str:
        """
        Generates a highly readable, structured string representation of 
        the pipeline metrics suitable for scientific execution logs and 
        standard output.

        Returns
        -------
        str
            A strictly formatted string containing the aligned summary of 
            the pipeline execution.
        """
        border = "=" * 60
        title = "THERMOGNOSIS PIPELINE EXECUTION SUMMARY".center(60)
        
        return (
            f"\n{border}\n"
            f"{title}\n"
            f"{border}\n"
            f"{'Total Processed':<30}: {self.total_processed}\n"
            f"{'Total Inserted':<30}: {self.total_inserted}\n"
            f"{'Total Failed':<30}: {self.total_failed}\n"
            f"{'Physics Violations':<30}: {self.physics_violations}\n"
            f"{'Average Score':<30}: {self.average_score:.6f}\n"
            f"{'Processing Time (s)':<30}: {self.processing_time_seconds:.4f}\n"
            f"{border}\n"
        )