# Figures Definition — Publication Draft  
**Document ID:** PUB-FIGURES-DEFINITION  
**Layer:** 99_publication_draft  
**Status:** Normative — Figure Design, Mathematical Consistency, and Reproducibility Standard  
**Scope:** Q1-Level Journal Submission  

---

# 1. Purpose

This document defines the formal specification for all figures included in the publication draft of the Thermognosis Engine.

Its objectives are:

1. To ensure scientific rigor and reproducibility.
2. To guarantee mathematical consistency with formal framework documents.
3. To align visual representation with theoretical constructs.
4. To satisfy Q1-level journal expectations in computational materials science and AI-driven discovery.

Figures are not decorative elements.  
They are compressed mathematical arguments.

---

# 2. General Figure Principles

Every figure must satisfy:

1. **Theoretical Traceability**  
   Each plotted quantity must correspond to a formally defined variable in foundational documents.

2. **Dimensional Consistency**  
   Units must satisfy:
   \[
   [y] = \text{physically valid unit}
   \]

3. **Reproducibility Constraint**  
   A figure must be regenerable from:
   - Versioned dataset,
   - Logged hyperparameters,
   - Documented random seed.

4. **Statistical Transparency**  
   All uncertainty must be explicitly visualized:
   \[
   \sigma, \quad \text{CI}_{95\%}, \quad \text{or posterior intervals}
   \]

---

# 3. Figure Category Overview

Figures are divided into six categories:

1. System Architecture Figures
2. Physical Model Validation Figures
3. Statistical Model Performance Figures
4. Graph-Theoretic Structure Figures
5. Closed-Loop Dynamics Figures
6. Convergence and Stability Diagnostics

---

# 4. Figure 1 — System Architecture Overview

### Objective

Visualize closed-loop operator:

\[
\mathcal{S}_{t+1} = \mathcal{O}(\mathcal{S}_t)
\]

### Required Elements

- Nodes:
  - Data Fusion \( \mathcal{F} \)
  - Estimation \( \mathcal{E} \)
  - Ranking \( \mathcal{R} \)
  - Acquisition \( \mathcal{A} \)
  - Update \( \mathcal{U} \)

- Directed edges representing operator composition:
  \[
  \mathcal{O} = \mathcal{U} \circ \mathcal{A} \circ \mathcal{R} \circ \mathcal{E} \circ \mathcal{F}
  \]

### Requirement

Diagram must reflect formal operator structure defined in CL01.

---

# 5. Figure 2 — Thermoelectric Physical Model Validation

### Objective

Validate consistency with:

\[
zT = \frac{S^2 \sigma T}{\kappa}
\]

### Plots

1. Measured vs predicted \( zT \)
2. Residual distribution:
   \[
   r_i = y_i - \hat{y}_i
   \]
3. Uncertainty bands:
   \[
   \hat{y}_i \pm 2\sigma_i
   \]

### Acceptance Criteria

- Zero-centered residual distribution.
- No systematic bias across temperature.

---

# 6. Figure 3 — Uncertainty Propagation Analysis

### Objective

Demonstrate error propagation:

\[
\sigma_{zT}^2 =
\left(\frac{\partial zT}{\partial S}\right)^2 \sigma_S^2
+
\left(\frac{\partial zT}{\partial \sigma}\right)^2 \sigma_\sigma^2
+
\left(\frac{\partial zT}{\partial \kappa}\right)^2 \sigma_\kappa^2
\]

### Visualization

- Contribution bar chart.
- Sensitivity heatmap.

---

# 7. Figure 4 — Citation Graph Structure

### Objective

Visualize influence distribution.

Plot:

- Degree distribution:
  \[
  P(k) \propto k^{-\gamma}
  \]
- PageRank distribution.

### Diagnostic

Check scale-free behavior:

\[
2 < \gamma < 3
\]

---

# 8. Figure 5 — Material Identity Embedding Space

### Objective

Visualize embedding:

\[
\phi(v) \in \mathbb{R}^d
\]

2D projection via PCA or spectral embedding.

Requirements:

- Color-coded by credibility score.
- Highlight high \( zT \) region.
- Annotate cluster boundaries.

---

# 9. Figure 6 — Ranking Stability

### Objective

Track ranking evolution:

\[
R_t(v)
\]

Plot:

\[
R_t(v_{top})
\quad \text{vs iteration } t
\]

Stability condition:

\[
|R_{t+1}(v) - R_t(v)| < \epsilon
\]

---

# 10. Figure 7 — Information Gain Trajectory

### Objective

Demonstrate entropy decay:

\[
H_t = H[p(\theta | \mathcal{D}_t)]
\]

Plot:

- \( H_t \) vs iteration.
- Cumulative information gain:
  \[
  \sum_{i=1}^t IG(v_i)
  \]

Monotonic decrease expected.

---

# 11. Figure 8 — Convergence Diagnostics

### Objective

Validate Lyapunov condition:

\[
V_{t+1} - V_t \le 0
\]

Plot:

- Prediction error vs iteration.
- Parameter norm:
  \[
  \|\theta_{t+1} - \theta_t\|
  \]

---

# 12. Figure 9 — Exploration–Exploitation Balance

Plot acquisition score:

\[
A(v) = \alpha IG(v) + (1-\alpha) U(v)
\]

Demonstrate transition from high-variance exploration to exploitation.

---

# 13. Figure Design Standards

### Typography

- Axis labels in SI units.
- Mathematical symbols italicized.
- Consistent font across figures.

### Resolution

- Minimum 300 dpi.
- Vector format preferred (PDF/SVG).

### Color Policy

- Accessible color palette.
- Distinguishable in grayscale.
- Uncertainty shown via shading.

---

# 14. Statistical Reporting Requirements

Every plot must include:

- Sample size \( n \)
- Confidence interval definition
- Model hyperparameters used
- Random seed for reproducibility

---

# 15. Mathematical Integrity Check

Before submission, verify:

\[
\text{Figure Output} \models \text{Formal Equation}
\]

No plotted quantity may contradict theoretical definitions.

---

# 16. Reproducibility Specification

Each figure must be generated by:

\[
\texttt{generate\_figure\_X.py}
\]

with:

- Fixed seed:
  \[
  \texttt{seed} = 42
  \]
- Versioned dataset hash.
- Logged hyperparameters.

---

# 17. Audit Trail

Store:

1. Data version.
2. Model version.
3. Graph snapshot.
4. Acquisition state.

Figure must be reproducible years later.

---

# 18. Strategic Interpretation

Figures are:

- Mathematical evidence,
- Stability demonstration,
- Credibility validation,
- Convergence proof.

They communicate:

- Physical correctness,
- Statistical robustness,
- Graph-theoretic intelligence,
- Closed-loop convergence.

---

# 19. Compliance Requirement

All figures must satisfy:

\[
\text{Figure} \models \text{PUB-FIGURES-DEFINITION}
\]

Non-compliance results in:

- Rejection risk,
- Reviewer skepticism,
- Loss of scientific credibility.

---

# 20. Concluding Statement

The figures of this publication must serve as condensed formal proofs.

They must demonstrate:

- Physical validity,
- Statistical rigor,
- Graph intelligence,
- Closed-loop convergence.

Visual clarity must reflect mathematical precision.

In a Q1-level submission, figures are not illustrations.  
They are structured evidence of theoretical integrity.
