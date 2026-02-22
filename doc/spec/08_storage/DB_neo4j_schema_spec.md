# DB — Neo4j Schema Specification  
**Document ID:** SPEC-DB-NEO4J-SCHEMA  
**Layer:** spec/08_storage  
**Status:** Normative — Graph Database Schema and Integrity Framework  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Neo4j Graph Database Schema Specification (NGDSS)** governing:

- Logical schema design for the Identity Graph,
- Node and relationship taxonomies,
- Property constraints,
- Indexing and performance guarantees,
- Integrity, versioning, and reproducibility rules.

The Neo4j instance constitutes the persistent backbone of the Thermognosis Identity Graph.  
Schema design must support:

- Physical rigor,
- Canonical material identity,
- Credibility-aware relationships,
- Deterministic query semantics,
- Scalable graph analytics.

---

# 2. Conceptual Graph Model

The graph is formally defined as:

\[
G = (V, E, \mathcal{P}_V, \mathcal{P}_E)
\]

where:

- \( V \) — nodes  
- \( E \subseteq V \times V \) — directed relationships  
- \( \mathcal{P}_V \) — node property maps  
- \( \mathcal{P}_E \) — edge property maps  

Neo4j implements:

\[
v \in V \equiv (:Label \{properties\})
\]

\[
e \in E \equiv (v_i)-[:TYPE \{properties\}]->(v_j)
\]

---

# 3. Core Node Labels

Mandatory node labels:

### 3.1 `Material`
Represents canonicalized material identity.

### 3.2 `Property`
Represents physical quantity (e.g., Seebeck coefficient).

### 3.3 `Measurement`
Represents experimental or simulated measurement event.

### 3.4 `Publication`
Represents scientific source.

### 3.5 `Condition`
Represents environmental parameters (e.g., temperature, pressure).

### 3.6 `Phase`
Represents crystallographic or thermodynamic phase.

### 3.7 `Model`
Represents computational surrogate or predictive model.

---

# 4. Node Property Schema

## 4.1 `Material`

Required properties:

- `material_id` (string, unique)
- `formula_canonical`
- `composition_vector`
- `structure_descriptor`
- `phase_descriptor`
- `created_at`
- `canon_version`

Uniqueness constraint:

\[
\forall m_i, m_j:
m_i.\text{material\_id}
=
m_j.\text{material\_id}
\Rightarrow
m_i = m_j
\]

---

## 4.2 `Property`

Required:

- `property_name`
- `symbol`
- `unit`
- `dimension_vector`

Dimensional constraint:

\[
\mathbf{d}(\text{unit}) = \text{dimension\_vector}
\]

---

## 4.3 `Measurement`

Required:

- `value`
- `uncertainty`
- `unit`
- `timestamp`
- `credibility_score`
- `quality_score`

Constraint:

\[
\text{uncertainty} \ge 0
\]

---

## 4.4 `Publication`

Required:

- `doi`
- `title`
- `year`
- `journal`
- `credibility_prior`

Uniqueness constraint on `doi`.

---

# 5. Core Relationship Types

### 5.1 `(:Material)-[:HAS_PROPERTY]->(:Property)`

### 5.2 `(:Material)-[:MEASURED_AS]->(:Measurement)`

### 5.3 `(:Measurement)-[:REPORTED_IN]->(:Publication)`

### 5.4 `(:Measurement)-[:UNDER_CONDITION]->(:Condition)`

### 5.5 `(:Material)-[:IN_PHASE]->(:Phase)`

### 5.6 `(:Model)-[:PREDICTS]->(:Property)`

### 5.7 `(:Publication)-[:CONFIRMS]->(:Measurement)`

Relationships must include:

- `weight`
- `created_at`
- `version_tag`

---

# 6. Edge Weight Semantics

Edge weight defined as:

\[
w_{ij}
=
\mathcal{K}(\mathcal{R}_{ij})
\cdot
q_{\text{quality}}
\]

Stored property:

`weight ∈ [0,1]`

Weight must be recomputable.

---

# 7. Indexing Strategy

Mandatory indices:

- `Material(material_id)`
- `Publication(doi)`
- `Property(property_name)`
- `Measurement(timestamp)`

Expected query time:

\[
\mathcal{O}(\log |V|)
\]

---

# 8. Constraint Enforcement

Neo4j schema must enforce:

### 8.1 Uniqueness

CREATE CONSTRAINT material_id_unique
FOR (m:Material)
REQUIRE m.material_id IS UNIQUE;


### 8.2 Required Property Constraints

All mandatory properties must be validated at application layer.

---

# 9. Temporal Versioning

Each node and edge must include:

- `valid_from`
- `valid_to`
- `version`

Graph state at time \( t \):

\[
G_t =
\{ v,e \mid v.\text{valid\_from} \le t < v.\text{valid\_to} \}
\]

Supports historical reconstruction.

---

# 10. Query Determinism

Given identical database snapshot:

\[
\text{Query}(G) = \text{deterministic}
\]

No random ordering without explicit `ORDER BY`.

---

# 11. Graph Integrity Rules

### Referential Integrity

\[
\forall e=(v_i,v_j):
v_i,v_j \in V
\]

### No Orphan Measurement

\[
\exists \text{ path from Measurement to Publication}
\]

### No Orphan Material

\[
\exists \text{ at least one Measurement or Model link}
\]

---

# 12. Dimensional Validation Hook

Measurement insertion must validate:

\[
\mathbf{d}(\text{unit}) = \mathbf{d}(\text{property})
\]

Dimension mismatch blocks transaction.

---

# 13. Embedding Cache Storage

Optional property:

`embedding_vector: [float]`

Constraint:

\[
\text{length} = d
\]

Embedding must store associated `embedding_version`.

---

# 14. Backup and Recovery

Full snapshot frequency:

\[
\ge 1 \text{ per } 24h
\]

Checksum validation:

\[
\text{hash}(G_{\text{backup}})
=
\text{hash}(G_{\text{production}})
\]

---

# 15. Performance Targets

Node retrieval:

\[
< 10 \text{ ms}
\]

Graph traversal (depth ≤ 3):

\[
< 50 \text{ ms}
\]

Embedding query latency:

\[
< 100 \text{ ms}
\]

---

# 16. Scalability Constraints

Expected growth:

\[
|V| \sim 10^6
\]

\[
|E| \sim 10^7
\]

Schema must support horizontal scaling.

---

# 17. Security and Access Control

Role-based access:

- Reader
- Analyst
- Writer
- Admin

Mutation operations logged with:

- User ID
- Timestamp
- Affected nodes

---

# 18. Error Classification

- DB-SCHEMA-01: Missing required property
- DB-SCHEMA-02: Duplicate unique key
- DB-SCHEMA-03: Orphan node
- DB-SCHEMA-04: Weight inconsistency
- DB-SCHEMA-05: Dimensional mismatch
- DB-SCHEMA-06: Version conflict

All violations must abort transaction.

---

# 19. Formal Soundness Condition

Schema is sound if:

1. Referential integrity preserved,
2. Unique constraints enforced,
3. Deterministic reconstruction possible,
4. Dimensional validation integrated,
5. All identities canonicalized,
6. Version history immutable.

---

# 20. Strategic Interpretation

Neo4j schema is not merely a storage decision — it is:

- The structural memory of scientific reasoning,
- The backbone of embedding computation,
- The substrate for credibility-aware ranking,
- The foundation of closed-loop experimentation.

Schema discipline ensures scientific traceability and computational reproducibility.

---

# 21. Concluding Statement

All graph persistence in Neo4j must satisfy:

\[
G \models \text{SPEC-DB-NEO4J-SCHEMA}
\]

The database schema must be as rigorous as the physics and mathematics it encodes.

Infrastructure integrity is a prerequisite for scientific excellence.
