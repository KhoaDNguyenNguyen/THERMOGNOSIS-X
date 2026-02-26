-- ============================================================================
-- File: python/thermognosis/db/schema.sql
-- Description: Core Relational Schema for the Thermognosis Engine
-- Author: Distinguished Professor of Computational Materials Science & Chief Software Architect
-- Status: Normative — Relational Core Metadata and Transactional Integrity Framework
-- Compliance Level: Research-Grade / Q1 Infrastructure Standard
-- ============================================================================
-- Implements: SPEC-DB-POSTGRES-SCHEMA, SPEC-DB-VERSIONING, SPEC-GOV-NAMING-RULES

BEGIN;

-- Enable pgcrypto for UUID generation if on older PostgreSQL versions, 
-- though gen_random_uuid() is native in PG13+.
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ============================================================================
-- 1. AUDIT LOGGING (Implements: SPEC-DB-POSTGRES-SCHEMA Section 11)
-- ============================================================================

-- Implements: SPEC-DB-POSTGRES-SCHEMA Section 11 (Audit Logging)
CREATE TABLE audit_log (
    audit_uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_id TEXT NOT NULL,
    operation_type TEXT NOT NULL CHECK (operation_type IN ('INSERT', 'UPDATE', 'DELETE')),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    affected_uuid UUID NOT NULL,
    previous_state JSONB,
    table_name TEXT NOT NULL
);

-- Index for temporal and identity-based queries on the audit log
CREATE INDEX idx_audit_log_timestamp ON audit_log USING btree (timestamp);
CREATE INDEX idx_audit_log_affected_uuid ON audit_log USING btree (affected_uuid);

-- ============================================================================
-- 2. CORE REGISTRY TABLES (Implements: SPEC-DB-POSTGRES-SCHEMA Section 4)
-- ============================================================================

-- Implements: SPEC-DB-POSTGRES-SCHEMA Section 4.1 (material_registry)
-- Implements: SPEC-DB-VERSIONING Section 2 (Temporal Data Model)
CREATE TABLE material_registry (
    material_uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    formula_canonical TEXT NOT NULL,
    composition_hash TEXT NOT NULL,
    canon_version INTEGER NOT NULL,
    
    -- Temporal Versioning Fields
    valid_from TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    valid_to TIMESTAMPTZ NOT NULL DEFAULT 'infinity'::timestamptz,
    version_number INTEGER NOT NULL DEFAULT 1,
    
    -- Metadata Fields
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Uniqueness constraint: UNIQUE(composition_hash, canon_version)
    CONSTRAINT uq_material_composition_version UNIQUE (composition_hash, canon_version)
);

-- Implements: SPEC-DB-POSTGRES-SCHEMA Section 4.2 (property_registry)
-- Implements: SPEC-DB-VERSIONING Section 2 (Temporal Data Model)
CREATE TABLE property_registry (
    property_uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    symbol TEXT,
    unit TEXT NOT NULL,
    dimension_vector JSONB NOT NULL,
    
    -- Temporal Versioning Fields
    valid_from TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    valid_to TIMESTAMPTZ NOT NULL DEFAULT 'infinity'::timestamptz,
    version_number INTEGER NOT NULL DEFAULT 1,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Implements: SPEC-DB-POSTGRES-SCHEMA Section 4.3 (dataset_registry)
-- Implements: SPEC-DB-VERSIONING Section 2 (Temporal Data Model)
CREATE TABLE dataset_registry (
    dataset_uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parquet_path TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    quality_score DOUBLE PRECISION NOT NULL,
    checksum TEXT NOT NULL,
    
    -- Temporal Versioning Fields
    valid_from TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    valid_to TIMESTAMPTZ NOT NULL DEFAULT 'infinity'::timestamptz,
    version_number INTEGER NOT NULL DEFAULT 1,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Implements: SPEC-DB-POSTGRES-SCHEMA Section 4.4 (publication_registry)
-- Implements: SPEC-DB-VERSIONING Section 2 (Temporal Data Model)
CREATE TABLE publication_registry (
    publication_uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    doi TEXT UNIQUE NOT NULL,
    title TEXT NOT NULL,
    journal TEXT,
    year INTEGER,
    credibility_prior DOUBLE PRECISION NOT NULL,
    
    -- Temporal Versioning Fields
    valid_from TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    valid_to TIMESTAMPTZ NOT NULL DEFAULT 'infinity'::timestamptz,
    version_number INTEGER NOT NULL DEFAULT 1,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Constraint: 0 <= credibility_prior <= 1
    CONSTRAINT chk_publication_credibility CHECK (credibility_prior >= 0.0 AND credibility_prior <= 1.0)
);

-- Implements: SPEC-DB-POSTGRES-SCHEMA Section 4.5 (measurement_metadata)
-- Implements: SPEC-DB-POSTGRES-SCHEMA Section 5 (Referential Integrity: ON DELETE RESTRICT)
-- Implements: SPEC-DB-VERSIONING Section 2 (Temporal Data Model)
CREATE TABLE measurement_metadata (
    measurement_uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    material_uuid UUID NOT NULL,
    property_uuid UUID NOT NULL,
    publication_uuid UUID NOT NULL,
    
    value DOUBLE PRECISION NOT NULL,
    uncertainty DOUBLE PRECISION NOT NULL,
    unit TEXT NOT NULL,
    
    -- Temporal Versioning Fields
    valid_from TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    valid_to TIMESTAMPTZ NOT NULL DEFAULT 'infinity'::timestamptz,
    version_number INTEGER NOT NULL DEFAULT 1,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    CONSTRAINT fk_measurement_material 
        FOREIGN KEY (material_uuid) REFERENCES material_registry(material_uuid) ON DELETE RESTRICT,
    CONSTRAINT fk_measurement_property 
        FOREIGN KEY (property_uuid) REFERENCES property_registry(property_uuid) ON DELETE RESTRICT,
    CONSTRAINT fk_measurement_publication 
        FOREIGN KEY (publication_uuid) REFERENCES publication_registry(publication_uuid) ON DELETE RESTRICT,
        
    -- Constraint: uncertainty >= 0
    CONSTRAINT chk_measurement_uncertainty CHECK (uncertainty >= 0.0)
);

-- ============================================================================
-- 3. INDEXING STRATEGY (Implements: SPEC-DB-POSTGRES-SCHEMA Section 8)
-- ============================================================================

-- B-tree indexes on all Foreign Keys
CREATE INDEX idx_fk_measurement_material_uuid ON measurement_metadata USING btree (material_uuid);
CREATE INDEX idx_fk_measurement_property_uuid ON measurement_metadata USING btree (property_uuid);
CREATE INDEX idx_fk_measurement_publication_uuid ON measurement_metadata USING btree (publication_uuid);

-- GIN index on JSONB dimension vectors
CREATE INDEX idx_gin_property_dimension_vector ON property_registry USING gin (dimension_vector);

-- B-tree indexes on created_at for expected lookup complexity O(log n)
CREATE INDEX idx_material_registry_created_at ON material_registry USING btree (created_at);
CREATE INDEX idx_property_registry_created_at ON property_registry USING btree (created_at);
CREATE INDEX idx_dataset_registry_created_at ON dataset_registry USING btree (created_at);
CREATE INDEX idx_publication_registry_created_at ON publication_registry USING btree (created_at);
CREATE INDEX idx_measurement_metadata_created_at ON measurement_metadata USING btree (created_at);

-- ============================================================================
-- 4. PL/PGSQL TRIGGERS & FUNCTIONS (Implements: SPEC-DB-POSTGRES-SCHEMA Section 11)
-- ============================================================================

-- Implements: SPEC-DB-POSTGRES-SCHEMA Section 11 (Audit Logic)
CREATE OR REPLACE FUNCTION tf_capture_audit_log()
RETURNS TRIGGER AS $$
DECLARE
    v_actor_id TEXT;
    v_affected_uuid UUID;
    v_previous_state JSONB := NULL;
BEGIN
    -- Resolve actor_id (use session context variable if available, fallback to DB user)
    BEGIN
        v_actor_id := current_setting('thermognosis.current_actor_id', true);
        IF v_actor_id IS NULL OR v_actor_id = '' THEN
            v_actor_id := current_user;
        END IF;
    EXCEPTION WHEN OTHERS THEN
        v_actor_id := current_user;
    END;

    -- Extract previous state for updates and deletes
    IF (TG_OP = 'DELETE' OR TG_OP = 'UPDATE') THEN
        v_previous_state := row_to_json(OLD)::JSONB;
    END IF;

    -- Dynamically resolve the primary key based on the active table
    IF TG_RELNAME = 'material_registry' THEN
        v_affected_uuid := COALESCE(NEW.material_uuid, OLD.material_uuid);
    ELSIF TG_RELNAME = 'property_registry' THEN
        v_affected_uuid := COALESCE(NEW.property_uuid, OLD.property_uuid);
    ELSIF TG_RELNAME = 'dataset_registry' THEN
        v_affected_uuid := COALESCE(NEW.dataset_uuid, OLD.dataset_uuid);
    ELSIF TG_RELNAME = 'publication_registry' THEN
        v_affected_uuid := COALESCE(NEW.publication_uuid, OLD.publication_uuid);
    ELSIF TG_RELNAME = 'measurement_metadata' THEN
        v_affected_uuid := COALESCE(NEW.measurement_uuid, OLD.measurement_uuid);
    ELSE
        -- Fallback defense logic
        RAISE EXCEPTION 'Audit trigger attached to unsupported table: %', TG_RELNAME;
    END IF;

    -- Insert into immutable audit log
    INSERT INTO audit_log (
        actor_id, 
        operation_type, 
        timestamp, 
        affected_uuid, 
        previous_state, 
        table_name
    ) VALUES (
        v_actor_id, 
        TG_OP, 
        CURRENT_TIMESTAMP, 
        v_affected_uuid, 
        v_previous_state, 
        TG_RELNAME
    );

    IF (TG_OP = 'DELETE') THEN
        RETURN OLD;
    ELSE
        RETURN NEW;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- Attach triggers to all core registry tables
CREATE TRIGGER trg_audit_material_registry
AFTER INSERT OR UPDATE OR DELETE ON material_registry
FOR EACH ROW EXECUTE FUNCTION tf_capture_audit_log();

CREATE TRIGGER trg_audit_property_registry
AFTER INSERT OR UPDATE OR DELETE ON property_registry
FOR EACH ROW EXECUTE FUNCTION tf_capture_audit_log();

CREATE TRIGGER trg_audit_dataset_registry
AFTER INSERT OR UPDATE OR DELETE ON dataset_registry
FOR EACH ROW EXECUTE FUNCTION tf_capture_audit_log();

CREATE TRIGGER trg_audit_publication_registry
AFTER INSERT OR UPDATE OR DELETE ON publication_registry
FOR EACH ROW EXECUTE FUNCTION tf_capture_audit_log();

CREATE TRIGGER trg_audit_measurement_metadata
AFTER INSERT OR UPDATE OR DELETE ON measurement_metadata
FOR EACH ROW EXECUTE FUNCTION tf_capture_audit_log();

COMMIT;