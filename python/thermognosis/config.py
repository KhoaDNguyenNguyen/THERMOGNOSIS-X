"""
Thermognosis Engine - Core Configuration Module
Document ID: SPEC-INFRA-CONF-000

This module defines the pure configuration architecture for THERMOGNOSIS-X.
It strictly enforces type-safe parsing, environment resolution, and
computational determinism.

No business logic or circular dependencies are permitted in this layer.
"""

import os
import random
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

import numpy as np
import yaml


class ConfigurationError(Exception):
    """
    Exception raised for critical failures during configuration resolution.
    
    Document ID: SPEC-GOV-ERROR-HIERARCHY
    
    This exception guarantees that no silent failures occur when loading
    infrastructure constraints, ensuring pipeline integrity.
    """
    pass


@dataclass(frozen=True)
class ThermoConfig:
    """
    Immutable representation of the THERMOGNOSIS-X runtime state.
    
    Document ID: SPEC-INFRA-CONF-001
    
    Attributes:
        db_url (str): Connection string for the underlying data store.
        db_echo (bool): Flag to enable/disable database query echoing.
        environment (str): Runtime context ("development" | "production" | "test").
        deterministic (bool): If True, global random states are fixed.
        random_seed (int): The seed utilized when deterministic mode is active.
        log_level (str): Threshold for structured logging.
        max_batch_size (int): Upper bound of items processed per batch.
        max_workers (int): Hardware concurrency limits for parallel pipelines.
        rust_strict_mode (bool): Dictates failure tolerance in the Rust FFI layer.
        rust_timeout_seconds (int): Maximum execution time allowed for Rust extensions.
    """
    # Database
    db_url: str
    db_echo: bool

    # Runtime mode
    environment: str  

    # Determinism
    deterministic: bool
    random_seed: int

    # Logging
    log_level: str

    # Performance
    max_batch_size: int
    max_workers: int

    # Rust execution
    rust_strict_mode: bool
    rust_timeout_seconds: int


def load_config(config_path: Optional[str] = None) -> ThermoConfig:
    """
    Resolves, validates, and initializes the global engine configuration.
    
    Document ID: SPEC-INFRA-CONF-002
    
    This function parses the configuration YAML and strictly maps it to the 
    ThermoConfig immutable dataclass. It is responsible for cross-platform 
    path resolution and executing the global determinism enforcement protocol.
    
    Args:
        config_path (Optional[str]): Explicit path to a YAML configuration file.
            If omitted, the function queries the `THERMOGNOSIS_ENV` environment 
            variable (defaulting to "default") and resolves the path relative 
            to the project root (`config/{env}.yaml`).
            
    Returns:
        ThermoConfig: A validated, immutable instance of the system configuration.
        
    Raises:
        ConfigurationError: If the file is missing, the YAML is malformed, 
            or critical schema keys are absent.
    """
    # 1. Path Resolution (Cross-platform compliant)
    if config_path is None:
        # Assuming __file__ is located at `<project_root>/python/thermognosis/config.py`
        # `parents[2]` reliably targets `<project_root>`
        project_root = Path(__file__).resolve().parents[2]
        env_mode = os.environ.get("THERMOGNOSIS_ENV", "default")
        target_path = project_root / "config" / f"{env_mode}.yaml"
    else:
        target_path = Path(config_path)

    if not target_path.is_file():
        raise ConfigurationError(f"Configuration file not found at: {target_path}")

    # 2. Strict YAML Parsing
    try:
        with target_path.open("r", encoding="utf-8") as f:
            raw_data = yaml.safe_load(f)
    except yaml.YAMLError as exc:
        raise ConfigurationError(f"Malformed YAML detected in {target_path}: {exc}") from exc

    if not isinstance(raw_data, dict):
        raise ConfigurationError(f"Invalid schema at {target_path}. Expected a top-level mapping.")

    # 3. Explicit Schema Extraction (No silent fallbacks)
    try:
        config = ThermoConfig(
            db_url=str(raw_data["db"]["url"]),
            db_echo=bool(raw_data["db"]["echo"]),
            environment=str(raw_data["runtime"]["environment"]),
            deterministic=bool(raw_data["runtime"]["deterministic"]),
            random_seed=int(raw_data["runtime"]["random_seed"]),
            log_level=str(raw_data["logging"]["level"]),
            max_batch_size=int(raw_data["performance"]["max_batch_size"]),
            max_workers=int(raw_data["performance"]["max_workers"]),
            rust_strict_mode=bool(raw_data["rust"]["strict_mode"]),
            rust_timeout_seconds=int(raw_data["rust"]["timeout_seconds"]),
        )
    except KeyError as exc:
        raise ConfigurationError(f"Missing critical configuration key: {exc}") from exc
    except (TypeError, ValueError) as exc:
        raise ConfigurationError(f"Type coercion failure in configuration values: {exc}") from exc

    # 4. Global Determinism Enforcement (SPEC-GOV-CODE-GENERATION-PROTOCOL)
    if config.deterministic:
        random.seed(config.random_seed)
        np.random.seed(config.random_seed)

    return config