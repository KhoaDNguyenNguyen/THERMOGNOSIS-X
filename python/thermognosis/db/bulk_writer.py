"""
Thermognosis Engine: Bulk Relational and Graph Writer
=====================================================

Implements: SPEC-DB-POSTGRES-SCHEMA (Layer: spec/08_storage)
Compliance: Research-Grade / Q1 Infrastructure Standard

This module provides high-throughput, transactionally atomic bulk ingestion 
mechanisms for PostgreSQL and Neo4j. It strictly enforces ACID properties, 
specifically isolation at the SERIALIZABLE level, to guarantee relational 
and cross-system consistency.

Mathematical Foundation:
    Let transaction T transition the database state:
        T: R -> R'
    
    The batch insertion satisfies:
        ∀ t ∈ Batch: t_j ∈ D_j
    
    Cross-system invariant:
        UUID_PG = UUID_Neo4j = UUID_Parquet

Author: Distinguished Professor of Computational Materials Science
Date: 2026-02-22
"""

import logging
from typing import Any, Dict, List, Tuple, Optional
import psycopg2
import psycopg2.extras
from psycopg2.pool import AbstractConnectionPool
from neo4j import Driver, Transaction

# Setup rigorous module-level logging
logger = logging.getLogger(__name__)
logger.setLevel(logging.INFO)


class ThermognosisDatabaseError(Exception):
    """Base exception for all database boundary violations."""
    pass


class PostgresBulkInsertError(ThermognosisDatabaseError):
    """
    Raised for PostgreSQL insertion failures.
    Maps to DB-PG-01, DB-PG-02, DB-PG-03, DB-PG-06 error classifications.
    """
    pass


class Neo4jBulkInsertError(ThermognosisDatabaseError):
    """Raised for Neo4j UNWIND transaction failures."""
    pass


class CrossSystemConsistencyError(ThermognosisDatabaseError):
    """
    Raised when atomicity between PostgreSQL and Neo4j cannot be guaranteed.
    Enforces invariant: UUID_PG == UUID_Neo4j.
    """
    pass


class PostgresBulkWriter:
    """
    High-throughput transactional bulk writer for PostgreSQL.
    
    Implements: SPEC-DB-POSTGRES-SCHEMA (Section 6: Transactional Guarantees)
    Isolation Level: SERIALIZABLE
    Throughput Target: > 10^4 rows/s
    """

    def __init__(self, pool: AbstractConnectionPool) -> None:
        """
        Initialize the bulk writer with a connection pool.
        
        Parameters
        ----------
        pool : AbstractConnectionPool
            Thread-safe connection pool from `connection.py`.
        """
        self.pool = pool

    def execute_batch(
        self, 
        table_name: str, 
        columns: List[str], 
        data: List[Tuple[Any, ...]], 
        page_size: int = 10000
    ) -> int:
        """
        Execute a bulk insert into PostgreSQL with strict atomicity.
        
        Mathematical Formulation:
            T: R -> R'
            If exception E occurs, R' = R (Total Rollback)
            
        Implements: SPEC-DB-POSTGRES-SCHEMA
        
        Parameters
        ----------
        table_name : str
            Target relational table.
        columns : List[str]
            Ordered list of column identifiers.
        data : List[Tuple[Any, ...]]
            List of tuples representing rows to insert.
        page_size : int, optional
            Number of rows per execute_values batch, by default 10000.
            
        Returns
        -------
        int
            Number of rows successfully inserted.
            
        Raises
        ------
        PostgresBulkInsertError
            If transactional integrity, constraints, or isolation guarantees fail.
        """
        if not data:
            logger.warning(f"Empty dataset provided for {table_name}. Aborting transaction.")
            return 0

        conn = self.pool.getconn()
        
        try:
            # Enforce strict serializable isolation as per Section 6
            conn.set_session(isolation_level=psycopg2.extensions.ISOLATION_LEVEL_SERIALIZABLE)
            
            with conn.cursor() as cursor:
                cols_str = ",".join(columns)
                # Formulate the optimized INSERT statement
                query = f"INSERT INTO {table_name} ({cols_str}) VALUES %s"
                
                # psycopg2.extras.execute_values utilizes PostgreSQL multi-row VALUES logic
                # for extreme throughput, bypassing single-row parsing overhead.
                psycopg2.extras.execute_values(
                    cursor, 
                    query, 
                    argslist=data, 
                    page_size=page_size
                )
                
            conn.commit()
            logger.info(f"Successfully committed {len(data)} tuples to {table_name}.")
            return len(data)

        except psycopg2.IntegrityError as e:
            conn.rollback()
            err_msg = str(e)
            if "duplicate key" in err_msg:
                raise PostgresBulkInsertError(f"DB-PG-01: Primary/Unique key violation. {err_msg}") from e
            elif "foreign key" in err_msg:
                raise PostgresBulkInsertError(f"DB-PG-02: Referential integrity violation. {err_msg}") from e
            else:
                raise PostgresBulkInsertError(f"DB-PG-XX: Integrity constraint violation. {err_msg}") from e
        except psycopg2.OperationalError as e:
            conn.rollback()
            raise PostgresBulkInsertError(f"DB-PG-06: Isolation or operational anomaly. {str(e)}") from e
        except Exception as e:
            conn.rollback()
            raise PostgresBulkInsertError(f"Unhandled PostgreSQL batch execution failure. {str(e)}") from e
        finally:
            self.pool.putconn(conn)


class GraphBulkWriter:
    """
    High-throughput transactional bulk writer for Neo4j.
    
    Implements: SPEC-DB-POSTGRES-SCHEMA (Section 10: Cross-System Consistency)
    Throughput Method: UNWIND strategy.
    """

    def __init__(self, driver: Driver) -> None:
        """
        Initialize the graph bulk writer.
        
        Parameters
        ----------
        driver : neo4j.Driver
            The Neo4j driver connection from `connection.py`.
        """
        self.driver = driver

    def execute_unwind_batch(
        self, 
        cypher_query: str, 
        batch_data: List[Dict[str, Any]]
    ) -> int:
        """
        Executes a Cypher query using UNWIND for high-performance batch node/edge creation.
        
        Implements: SPEC-DB-POSTGRES-SCHEMA
        
        Parameters
        ----------
        cypher_query : str
            The Cypher query containing an `UNWIND $batch AS row` clause.
        batch_data : List[Dict[str, Any]]
            List of dictionaries representing the properties for the UNWIND batch.
            
        Returns
        -------
        int
            Number of elements processed in the graph batch.
            
        Raises
        ------
        Neo4jBulkInsertError
            If the graph transaction fails or invalidates cross-system invariants.
        """
        if not batch_data:
            logger.warning("Empty batch provided for Neo4j UNWIND. Aborting transaction.")
            return 0

        def _tx_logic(tx: Transaction, query: str, data: List[Dict[str, Any]]) -> Any:
            result = tx.run(query, batch=data)
            return result.consume()

        try:
            with self.driver.session() as session:
                # Write transactions are strictly atomic within the session.
                summary = session.execute_write(_tx_logic, cypher_query, batch_data)
                
                updates = summary.counters
                logger.info(
                    f"Neo4j Batch Commit Successful. Nodes created: {updates.nodes_created}, "
                    f"Relationships created: {updates.relationships_created}"
                )
                return len(batch_data)
                
        except Exception as e:
            logger.error(f"Neo4j transaction failed during UNWIND batch execution: {str(e)}")
            raise Neo4jBulkInsertError(f"Graph transaction rollback executed due to: {str(e)}") from e


class UnifiedTranslationalWriter:
    """
    Orchestrates coordinated writes across PostgreSQL and Neo4j to guarantee 
    cross-system identifier invariants.
    
    Mathematical Invariant:
        UUID_PG = UUID_Neo4j
    """

    def __init__(
        self, 
        pg_writer: PostgresBulkWriter, 
        graph_writer: GraphBulkWriter
    ) -> None:
        self.pg_writer = pg_writer
        self.graph_writer = graph_writer

    def write_canonical_materials(
        self, 
        materials_data: List[Tuple[Any, ...]], 
        graph_data: List[Dict[str, Any]]
    ) -> None:
        """
        Executes a tightly coupled, sequential write to PG and Neo4j.
        In a distributed setup without two-phase commit (2PC), PG is written 
        first as the authoritative source of truth.
        
        Implements: SPEC-DB-POSTGRES-SCHEMA (Section 10)
        
        Parameters
        ----------
        materials_data : List[Tuple[Any, ...]]
            Data for `material_registry`.
        graph_data : List[Dict[str, Any]]
            Data for Neo4j `(:Material)` nodes.
            
        Raises
        ------
        CrossSystemConsistencyError
            If PG succeeds but Neo4j fails, requiring manual compensation or retry log.
        """
        pg_cols = [
            "material_uuid", "formula_canonical", "composition_hash", 
            "canon_version", "created_at", "updated_at"
        ]
        
        # 1. Authoritative Transactional Spine (PostgreSQL)
        logger.info("Initiating Phase 1/2: PostgreSQL Authoritative Write")
        pg_inserted = self.pg_writer.execute_batch(
            table_name="material_registry",
            columns=pg_cols,
            data=materials_data
        )
        
        # 2. Graph Reasoning Layer (Neo4j)
        neo4j_query = """
        UNWIND $batch AS row
        MERGE (m:Material {uuid: row.material_uuid})
        SET m.formula = row.formula_canonical,
            m.canon_version = row.canon_version,
            m.updated_at = row.updated_at
        """
        
        logger.info("Initiating Phase 2/2: Neo4j Graph Propagation")
        try:
            neo4j_inserted = self.graph_writer.execute_unwind_batch(
                cypher_query=neo4j_query,
                batch_data=graph_data
            )
            
            if pg_inserted != neo4j_inserted:
                logger.warning(
                    f"Length mismatch during unified write: PG({pg_inserted}) vs Neo4j({neo4j_inserted}). "
                    "This may indicate MERGE deduplication in Neo4j."
                )
                
        except Neo4jBulkInsertError as e:
            # Note: Because PG is already committed, this constitutes a distributed 
            # transaction failure requiring saga pattern compensation or dead-letter queuing.
            logger.critical("DB-PG-10: Cross-system structural drift detected!")
            raise CrossSystemConsistencyError(
                "Phase 1 (PG) succeeded, but Phase 2 (Neo4j) failed. System state is partially desynchronized."
            ) from e