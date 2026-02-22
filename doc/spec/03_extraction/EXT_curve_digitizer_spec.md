# EXT — Curve Digitizer Specification  
**Document ID:** SPEC-EXT-CURVE-DIGITIZER  
**Layer:** spec/03_extraction  
**Status:** Normative — Quantitative Figure-to-Data Extraction Protocol  
**Compliance Level:** Research-Grade / Q1 Infrastructure Standard  

---

# 1. Purpose

This document defines the **Curve Digitizer Specification (CDS)** governing the extraction of quantitative data from graphical figures (plots, charts, phase diagrams, response curves) embedded in scientific documents.

The Curve Digitizer is responsible for transforming visual representations into numerically structured datasets while preserving:

1. Deterministic reproducibility.
2. Coordinate system integrity.
3. Uncertainty quantification.
4. Provenance traceability.
5. Auditability of transformation steps.

Curve digitization is not an approximate convenience.  
It is a controlled inverse problem under geometric and numerical constraints.

---

# 2. Formal Problem Definition

Let a scientific figure be represented as an image:

\[
I : \Omega \subset \mathbb{R}^2 \rightarrow \mathbb{R}^3
\]

where:

- \( \Omega \) = pixel domain,
- \( \mathbb{R}^3 \) = RGB color space.

Goal:

Extract numerical curve:

\[
\mathcal{C}
=
\{ (x_i, y_i) \}_{i=1}^N
\subset \mathbb{R}^2
\]

Digitizer defines mapping:

\[
\mathcal{D} : I \rightarrow \mathcal{C}
\]

Subject to geometric calibration and uncertainty bounds.

---

# 3. Coordinate Calibration

Let pixel coordinate be:

\[
(u, v) \in \Omega
\]

Let real-world coordinate be:

\[
(x, y)
\]

Calibration transformation:

\[
\begin{pmatrix}
x \\
y
\end{pmatrix}
=
\mathbf{A}
\begin{pmatrix}
u \\
v
\end{pmatrix}
+
\mathbf{b}
\]

where:

- \( \mathbf{A} \in \mathbb{R}^{2 \times 2} \),
- \( \mathbf{b} \in \mathbb{R}^{2} \).

For nonlinear axes (log-scale):

\[
x = \exp(a u + b)
\]

Calibration must use at least 2 known reference points per axis.

---

# 4. Axis Type Identification

Axis scaling type:

\[
S \in \{ \text{linear}, \text{log}, \text{semilog}, \text{loglog} \}
\]

Transformation rule applied accordingly:

Linear:

\[
x = a u + b
\]

Log:

\[
x = 10^{(a u + b)}
\]

Axis type must be stored in metadata.

---

# 5. Curve Detection

Let binary mask of curve pixels be:

\[
M(u, v) =
\begin{cases}
1, & \text{if pixel belongs to curve} \\
0, & \text{otherwise}
\end{cases}
\]

Detection methods may include:

- Color thresholding
- Edge detection
- Morphological filtering

Determinism requirement:

\[
\mathcal{D}(I; v_1) = \mathcal{D}(I; v_2)
\]

Given identical digitizer version and parameters.

---

# 6. Sampling Strategy

Extracted pixel set:

\[
\mathcal{P} = \{ (u_j, v_j) \}
\]

Sampling to ordered sequence:

\[
\mathcal{P}_{sorted}
=
\text{sort by } x
\]

Downsampling rule:

\[
N_{\text{final}} \le N_{\max}
\]

Uniform sampling or adaptive curvature-based sampling permitted.

---

# 7. Interpolation

If necessary, interpolate discrete points:

Linear interpolation:

\[
y(x) = y_i + 
\frac{y_{i+1} - y_i}{x_{i+1} - x_i}
(x - x_i)
\]

Spline interpolation:

\[
y(x) = \sum_{k} \alpha_k B_k(x)
\]

Interpolation method must be logged.

---

# 8. Uncertainty Quantification

Pixel-level uncertainty:

\[
\delta u = \pm 0.5
\]

Propagation to real coordinate:

\[
\delta x =
\left|
\frac{\partial x}{\partial u}
\right|
\delta u
\]

Total uncertainty:

\[
\sigma_y^2 =
\sigma_{\text{digitization}}^2
+
\sigma_{\text{calibration}}^2
\]

Each extracted point must carry uncertainty estimate.

---

# 9. Resolution Constraint

Let image resolution:

\[
R_x \times R_y
\]

Maximum extractable precision bounded by:

\[
\delta x_{\min} \ge \frac{x_{\max} - x_{\min}}{R_x}
\]

Digitizer must not report precision beyond resolution limit.

---

# 10. Multi-Curve Handling

If figure contains \( K \) curves:

\[
\mathcal{C} =
\{ \mathcal{C}_k \}_{k=1}^{K}
\]

Curves distinguished by:

- Color segmentation,
- Legend mapping,
- User-specified ROI.

Each curve assigned unique identifier.

---

# 11. OCR Integration

Axis labels extracted via OCR:

\[
\text{Label} = \mathcal{O}(I_{\text{text}})
\]

Units normalized to SI before storage.

OCR uncertainty must be logged.

---

# 12. Provenance Metadata

Digitized artifact metadata:

\[
\mathcal{M}
=
(
\text{PDF\_checksum},
\text{Page\_number},
\text{Figure\_index},
\text{Calibration\_points},
\text{Digitizer\_version}
)
\]

Checksum coupling:

\[
H_{\mathcal{C}} = H(\mathcal{S}(\mathcal{C}))
\]

---

# 13. Deterministic Reproducibility

Given:

- Identical image \( I \),
- Identical parameters,
- Identical digitizer version,

Must satisfy:

\[
\mathcal{D}(I) = \mathcal{C}
\]

Bitwise-identical output required.

---

# 14. Error Threshold Policy

If estimated uncertainty:

\[
\sigma_y > \sigma_{\max}
\]

Digitization flagged as low-confidence.

Not automatically rejected, but marked.

---

# 15. Manual Override Protocol

Manual calibration allowed only if:

- Points explicitly recorded.
- Transformation parameters stored.
- Audit trail preserved.

No undocumented manual adjustment permitted.

---

# 16. Security Constraints

Digitizer must operate in:

- Sandboxed image-processing environment.
- No external network calls.
- Memory-limited execution.

---

# 17. Logging Requirements

Each digitization event must log:

- Image checksum
- Calibration parameters
- Sampling method
- Interpolation method
- Uncertainty model
- Output checksum
- Environment fingerprint

---

# 18. Compliance Requirement

Digitized curve \( \mathcal{C} \) accepted only if:

\[
\mathcal{C} \models \text{SPEC-ACQ-CHECKSUM}
\land
\mathcal{C} \models \text{SPEC-CONTRACT-RAW-MEASUREMENT}
\]

Failure blocks integration into dataset.

---

# 19. Strategic Interpretation

The Curve Digitizer transforms graphical knowledge into structured data.

It ensures:

- Geometric consistency,
- Quantified uncertainty,
- Deterministic reproducibility,
- Audit-ready transformation,
- Scientific defensibility.

Digitization is a mathematical reconstruction problem, not a heuristic shortcut.

---

# 20. Concluding Statement

In the Thermognosis Engine, every extracted curve must be:

- Geometrically calibrated,
- Numerically validated,
- Uncertainty-annotated,
- Cryptographically identified,
- Fully reproducible.

Graphical data is scientific data.  
Its extraction must meet the same rigor as experimental measurement.
