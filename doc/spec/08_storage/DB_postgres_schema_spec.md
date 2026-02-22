# DB — PostgreSQL Schema Specification  
**Document ID:** SPEC-DB-POSTGRES-SCHEMA  
**Layer:** spec/08_storage  
**Status:** Normative — Relational Core Metadata and Transactional Integrity Framework  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **PostgreSQL Schema Specification (PGSS)** governing:

- Relational storage of structured metadata,
- Transactional integrity guarantees,
- Canonical identifiers and cross-system consistency,
- Auditability and reproducibility controls,
- Strong typing and constraint enforcement.

PostgreSQL serves as the **transactional backbone** of the system, complementing:

- Neo4j (graph reasoning layer),
- Parquet (analytical columnar layer).

The relational layer is the authoritative source of:

- Canonical identifiers,
- Versioned metadata,
- Governance state,
- Access control mappings,
- Dataset registry records.

---

# 2. Relational Model Formalization

A relational database instance is defined as:

\[
\mathcal{R} =
\{ R_1, R_2, \dots, R_k \}
\]

where each relation:

\[
R_i \subseteq D_1 \times D_2 \times \dots \times D_m
\]

Each tuple:

\[
t \in R_i
\]

must satisfy domain constraints and key constraints.

---

# 3. Core Schema Domains

Mandatory domains:

- `UUID` — canonical identifiers
- `TEXT` — descriptive metadata
- `NUMERIC(precision, scale)` — high-precision quantities
- `DOUBLE PRECISION` — scientific floating-point values
- `TIMESTAMP WITH TIME ZONE`
- `JSONB` — structured extensible metadata

Domain consistency condition:

\[
\forall t \in R_i:
t_j \in D_j
\]

---

# 4. Core Tables

## 4.1 `material_registry`

**Primary Key:** `material_uuid`

Required fields:

- `material_uuid UUID PRIMARY KEY`
- `formula_canonical TEXT NOT NULL`
- `composition_hash TEXT NOT NULL`
- `canon_version INTEGER NOT NULL`
- `created_at TIMESTAMPTZ NOT NULL`
- `updated_at TIMESTAMPTZ NOT NULL`

Uniqueness constraint:

\[
\text{UNIQUE}(composition\_hash, canon\_version)
\]

---

## 4.2 `property_registry`

**Primary Key:** `property_uuid`

Fields:

- `property_uuid UUID PRIMARY KEY`
- `name TEXT NOT NULL`
- `symbol TEXT`
- `unit TEXT NOT NULL`
- `dimension_vector JSONB NOT NULL`
- `created_at TIMESTAMPTZ NOT NULL`

Dimensional constraint:

\[
\mathbf{d}(\text{unit}) = \text{dimension\_vector}
\]

---

## 4.3 `dataset_registry`

**Primary Key:** `dataset_uuid`

Fields:

- `dataset_uuid UUID PRIMARY KEY`
- `parquet_path TEXT NOT NULL`
- `schema_version INTEGER NOT NULL`
- `quality_score DOUBLE PRECISION NOT NULL`
- `checksum TEXT NOT NULL`
- `created_at TIMESTAMPTZ NOT NULL`

Integrity condition:

\[
\text{checksum} =
\text{SHA256}(\text{binary dataset})
\]

---

## 4.4 `publication_registry`

**Primary Key:** `publication_uuid`

Fields:

- `publication_uuid UUID PRIMARY KEY`
- `doi TEXT UNIQUE NOT NULL`
- `title TEXT NOT NULL`
- `journal TEXT`
- `year INTEGER`
- `credibility_prior DOUBLE PRECISION NOT NULL`

Constraint:

\[
0 \le \text{credibility\_prior} \le 1
\]

---

## 4.5 `measurement_metadata`

**Primary Key:** `measurement_uuid`

Foreign keys:

- `material_uuid REFERENCES material_registry`
- `property_uuid REFERENCES property_registry`
- `publication_uuid REFERENCES publication_registry`

Fields:

- `value DOUBLE PRECISION NOT NULL`
- `uncertainty DOUBLE PRECISION NOT NULL`
- `unit TEXT NOT NULL`
- `created_at TIMESTAMPTZ NOT NULL`

Uncertainty constraint:

\[
\text{uncertainty} \ge 0
\]

---

# 5. Referential Integrity

For every foreign key:

\[
t_{\text{child}}.\text{fk}
\in
\pi_{\text{pk}}(R_{\text{parent}})
\]

ON DELETE policy:

- `RESTRICT` for registry tables,
- `CASCADE` only where explicitly justified.

---

# 6. Transactional Guarantees

All operations must satisfy ACID properties.

Let transaction \( T \):

\[
T: \mathcal{R} \to \mathcal{R}'
\]

Properties:

- Atomicity  
- Consistency  
- Isolation  
- Durability  

Isolation level:

\[
\text{SERIALIZABLE}
\]

for registry-modifying operations.

---

# 7. Versioning Model

Versioned tables must include:

- `valid_from`
- `valid_to`
- `version_number`

Temporal reconstruction:

\[
R^{(t)} =
\{ t \in R \mid
t.\text{valid\_from} \le t < t.\text{valid\_to}
\}
\]

Historical states must remain immutable.

---

# 8. Indexing Strategy

Mandatory indices:

- B-tree on primary keys,
- B-tree on foreign keys,
- GIN index on JSONB dimension vectors,
- Index on `created_at`.

Expected lookup complexity:

\[
\mathcal{O}(\log n)
\]

---

# 9. Numerical Integrity

Floating storage must ensure:

\[
|x_{\text{stored}} - x_{\text{true}}|
\le \epsilon_{\text{mach}} |x_{\text{true}}|
\]

High-precision quantities requiring exact arithmetic must use:

\[
NUMERIC(p, s)
\]

---

# 10. Cross-System Consistency

PostgreSQL UUID must match:

- Neo4j node identifiers,
- Parquet dataset metadata.

Consistency invariant:

\[
\text{UUID}_{PG}
=
\text{UUID}_{Neo4j}
=
\text{UUID}_{Parquet}
\]

---

# 11. Audit Logging

Every mutation must record:

- `actor_id`
- `operation_type`
- `timestamp`
- `affected_uuid`
- `previous_state JSONB`

Audit completeness:

\[
\forall \text{mutation}:
\exists \text{audit record}
\]

---

# 12. Integrity Checks

Daily verification:

\[
\text{COUNT}(R_i)_{\text{Postgres}}
=
\text{COUNT}_{\text{derived layer}}
\]

Checksum verification between registry and Parquet datasets mandatory.

---

# 13. Access Control

Role hierarchy:

- `reader`
- `analyst`
- `editor`
- `admin`

Permission model:

\[
\text{role} \rightarrow \{\text{allowed operations}\}
\]

No direct table writes outside controlled services.

---

# 14. Performance Targets

Single-row lookup:

\[
< 5 \text{ ms}
\]

Foreign-key join (≤ 10^5 rows):

\[
< 50 \text{ ms}
\]

Bulk insert throughput:

\[
> 10^4 \text{ rows/s}
\]

---

# 15. Error Classification

- DB-PG-01: Primary key violation
- DB-PG-02: Foreign key violation
- DB-PG-03: Dimensional inconsistency
- DB-PG-04: Version conflict
- DB-PG-05: Audit log missing
- DB-PG-06: Isolation anomaly

All violations must abort transaction.

---

# 16. Backup and Recovery

Backup frequency:

\[
\ge 1 \text{ per 24 hours}
\]

Point-in-time recovery (PITR) enabled.

Recovery correctness condition:

\[
\mathcal{R}_{\text{restored}}
=
\mathcal{R}_{\text{snapshot}}
\]

---

# 17. Formal Soundness Condition

PostgreSQL schema is sound if:

1. All keys and constraints enforced,
2. Referential integrity preserved,
3. ACID compliance verified,
4. Cross-layer UUID consistency maintained,
5. Version history immutable,
6. Audit trail complete.

---

# 18. Strategic Interpretation

PostgreSQL is the **governance spine** of the system.

It ensures:

- Deterministic identity resolution,
- Reliable transaction boundaries,
- Canonical metadata authority,
- Reproducible dataset lineage.

Relational rigor protects scientific infrastructure from structural drift.

---

# 19. Concluding Statement

All relational operations must satisfy:

\[
\mathcal{R} \models \text{SPEC-DB-POSTGRES-SCHEMA}
\]

Scientific credibility depends not only on mathematical correctness —  
but also on transactional and structural integrity.
