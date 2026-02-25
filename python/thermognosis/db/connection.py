"""
Thermognosis Engine: Database Connection and Transaction Management
===================================================================
Document IDs: SPEC-DB-POSTGRES-SCHEMA, SPEC-DB-VERSIONING
Layer: spec/08_storage
Status: Normative

This module provides enterprise-grade, mathematically robust connection pools
for PostgreSQL (Relational Core Metadata) and Neo4j (Graph Reasoning Layer).
It implements strict transaction boundaries, ACID compliance, and network
resilience mechanisms (exponential backoff with deterministic jitter).

Mathematical Formalization of Backoff Jitter:
    t_{wait}^{(k)} = \min(d_{max}, d_{base} \cdot 2^k) + \mathcal{U}(0, \epsilon)
    where k is the retry attempt, d_{base} is the initial delay, and
    \mathcal{U} is a uniform distribution for jitter.
"""

import os
import time
import random
import logging
from functools import wraps
from typing import Any, Callable, Generator, Optional, TypeVar, cast
from contextlib import contextmanager

from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker, Session
from sqlalchemy.exc import SQLAlchemyError, OperationalError
from neo4j import GraphDatabase, Driver, Session as Neo4jSession
from neo4j.exceptions import ServiceUnavailable, TransientError

# Configure module-level logger
logger = logging.getLogger(__name__)
logger.setLevel(logging.INFO)
if not logger.handlers:
    handler = logging.StreamHandler()
    formatter = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
    handler.setFormatter(formatter)
    logger.addHandler(handler)

# Type variable for decorator return signatures
F = TypeVar('F', bound=Callable[..., Any])


class ThermognosisDatabaseError(Exception):
    """Base exception for all database-related violations in the Thermognosis Engine."""
    pass


class ConnectionTimeoutError(ThermognosisDatabaseError):
    """Raised when exponential backoff exhausts all retries."""
    pass


class TransactionIntegrityError(ThermognosisDatabaseError):
    """Raised when a transactional guarantee (ACID) is violated."""
    pass


def with_exponential_backoff(
    max_retries: int = 5,
    base_delay: float = 1.0,
    max_delay: float = 30.0,
    jitter_seed: int = 42
) -> Callable[[F], F]:
    """
    Decorator implementing exponential backoff with deterministic jitter.
    
    Implements: SPEC-DB-POSTGRES-SCHEMA (Section 6: Transactional Guarantees)
    
    Formula:
        t_{wait}^{(k)} = \min(d_{max}, d_{base} \cdot 2^k) + \mathcal{U}(0, 0.1)
    
    Parameters
    ----------
    max_retries : int
        Maximum number of connection or transaction retries.
    base_delay : float
        Base multiplier for the exponential delay (seconds).
    max_delay : float
        Absolute maximum delay between retries (seconds).
    jitter_seed : int
        Deterministic seed for the jitter pseudo-random number generator to
        satisfy strict reproducibility constraints.
    """
    rng = random.Random(jitter_seed)

    def decorator(func: F) -> F:
        @wraps(func)
        def wrapper(*args: Any, **kwargs: Any) -> Any:
            attempt = 0
            while attempt <= max_retries:
                try:
                    return func(*args, **kwargs)
                except (OperationalError, ServiceUnavailable, TransientError) as e:
                    attempt += 1
                    if attempt > max_retries:
                        logger.error(f"Exhausted {max_retries} retries for {func.__name__}.")
                        raise ConnectionTimeoutError(
                            f"Failed to execute {func.__name__} after {max_retries} attempts. "
                            f"Last error: {e}"
                        ) from e
                    
                    # Calculate exponential delay with deterministic jitter
                    exponential_delay = min(max_delay, base_delay * (2 ** (attempt - 1)))
                    jitter = rng.uniform(0.0, 0.1)
                    wait_time = exponential_delay + jitter
                    
                    logger.warning(
                        f"Network instability detected in {func.__name__}. "
                        f"Retrying ({attempt}/{max_retries}) in {wait_time:.3f}s..."
                    )
                    time.sleep(wait_time)
            return None # Should be unreachable
        return cast(F, wrapper)
    return decorator


class DatabaseConfig:
    """
    Centralized configuration resolution for database URIs.
    Adapts to execution environments (Arch Linux, Google Colab, Windows).
    """
    
    @staticmethod
    def is_colab_environment() -> bool:
        """Detects if code is executing within Google Colab."""
        return 'COLAB_GPU' in os.environ or 'COLAB_RELEASE_TAG' in os.environ

    @classmethod
    def get_postgres_uri(cls) -> str:
        """Resolves PostgreSQL URI with fallback defaults."""
        default_uri = "postgresql://postgres:postgres@localhost:5432/thermognosis"
        if cls.is_colab_environment():
            # In Colab, we default to an external or mocked in-memory store if unset,
            # but standard is to expect it via secrets.
            default_uri = "postgresql://postgres:postgres@host.docker.internal:5432/thermognosis"
        return os.getenv("THERMOGNOSIS_PG_URI", default_uri)

    @classmethod
    def get_neo4j_uri(cls) -> str:
        """Resolves Neo4j URI with fallback defaults."""
        default_uri = "bolt://localhost:7687"
        if cls.is_colab_environment():
            default_uri = "bolt://host.docker.internal:7687"
        return os.getenv("THERMOGNOSIS_NEO4J_URI", default_uri)
    
    @classmethod
    def get_neo4j_credentials(cls) -> tuple[str, str]:
        """Resolves Neo4j authentication credentials."""
        user = os.getenv("THERMOGNOSIS_NEO4J_USER", "neo4j")
        password = os.getenv("THERMOGNOSIS_NEO4J_PASSWORD", "password")
        return (user, password)


class PostgresManager:
    """
    Singleton connection manager for PostgreSQL.
    
    Implements: SPEC-DB-POSTGRES-SCHEMA (Section 6: Transactional Guarantees)
    Ensures that the relational layer operates under SERIALIZABLE isolation,
    preventing read anomalies and enforcing total transactional ordering.
    """
    _instance: Optional['PostgresManager'] = None

    def __new__(cls) -> 'PostgresManager':
        if cls._instance is None:
            cls._instance = super(PostgresManager, cls).__new__(cls)
            cls._instance._initialize()
        return cls._instance

    @with_exponential_backoff(max_retries=3, base_delay=2.0)
    def _initialize(self) -> None:
        """Initializes the SQLAlchemy engine and connection pool."""
        uri = DatabaseConfig.get_postgres_uri()
        
        # SPEC-DB-POSTGRES-SCHEMA Section 6 requires SERIALIZABLE isolation
        self.engine = create_engine(
            uri,
            pool_size=20,
            max_overflow=10,
            pool_pre_ping=True, # Validates connection liveness before checkout
            isolation_level="SERIALIZABLE"
        )
        self.SessionLocal = sessionmaker(
            autocommit=False,
            autoflush=False,
            bind=self.engine
        )
        logger.info("Initialized PostgreSQL connection pool with SERIALIZABLE isolation.")

    @contextmanager
    def session(self) -> Generator[Session, None, None]:
        """
        Yields a transactional SQLAlchemy session.
        
        Ensures ACID properties via strict rollback on exception.
        
        Yields
        ------
        Session
            Active SQLAlchemy ORM session.
            
        Raises
        ------
        TransactionIntegrityError
            If an exception occurs during the transaction boundary.
        """
        session: Session = self.SessionLocal()
        try:
            yield session
            session.commit()
        except SQLAlchemyError as e:
            session.rollback()
            logger.error(f"PostgreSQL transaction failure. Rolling back. Reason: {e}")
            raise TransactionIntegrityError(f"PostgreSQL ACID violation prevented: {e}") from e
        except Exception as e:
            session.rollback()
            logger.error(f"Unexpected application error. Rolling back transaction. Reason: {e}")
            raise
        finally:
            session.close()


class Neo4jManager:
    """
    Singleton connection manager for the Neo4j Identity Graph.
    
    Implements: SPEC-DB-POSTGRES-SCHEMA (Section 10: Cross-System Consistency)
    Manages connections for the graph reasoning layer complementing relational storage.
    """
    _instance: Optional['Neo4jManager'] = None

    def __new__(cls) -> 'Neo4jManager':
        if cls._instance is None:
            cls._instance = super(Neo4jManager, cls).__new__(cls)
            cls._instance._initialize()
        return cls._instance

    @with_exponential_backoff(max_retries=3, base_delay=2.0)
    def _initialize(self) -> None:
        """Initializes the Neo4j driver connection pool."""
        uri = DatabaseConfig.get_neo4j_uri()
        user, password = DatabaseConfig.get_neo4j_credentials()
        
        self.driver: Driver = GraphDatabase.driver(
            uri,
            auth=(user, password),
            max_connection_lifetime=3600,
            max_connection_pool_size=50,
            connection_acquisition_timeout=60.0
        )
        # Verify connectivity
        self.driver.verify_connectivity()
        logger.info("Initialized Neo4j driver and verified identity graph connectivity.")

    @contextmanager
    def session(self) -> Generator[Neo4jSession, None, None]:
        """
        Yields a Neo4j session for Cypher execution.
        
        Yields
        ------
        Neo4jSession
            Active Neo4j driver session.
            
        Raises
        ------
        TransactionIntegrityError
            If an exception occurs during graph interactions.
        """
        session = self.driver.session()
        try:
            yield session
        except Exception as e:
            logger.error(f"Neo4j graph session failure. Reason: {e}")
            raise TransactionIntegrityError(f"Neo4j graph violation prevented: {e}") from e
        finally:
            session.close()

    def close(self) -> None:
        """Closes the Neo4j driver gracefully."""
        if self.driver:
            self.driver.close()
            logger.info("Neo4j driver connection closed gracefully.")


# Global singleton accessors for syntactic elegance and standard integration
def get_postgres_manager() -> PostgresManager:
    """Returns the globally instantiated PostgreSQL Manager."""
    return PostgresManager()

def get_neo4j_manager() -> Neo4jManager:
    """Returns the globally instantiated Neo4j Manager."""
    return Neo4jManager()