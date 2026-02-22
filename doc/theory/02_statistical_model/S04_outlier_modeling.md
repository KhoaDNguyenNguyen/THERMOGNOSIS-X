# S04 — Outlier Modeling and Robust Inference  
**Document ID:** S04-OUTLIER-MODELING  
**Layer:** Statistical Modeling / Robustness Layer  
**Status:** Normative — Anomaly-Resilient Inference  
**Dependencies:**  
- T00-SYS-AXIOMS  
- T03-UNCERTAINTY-PROPAGATION  
- P03-PHYSICAL-CONSTRAINTS  
- S01-BAYESIAN-CREDIBILITY-MODEL  
- S02-WEIGHTED-LIKELIHOOD-FORMULATION  
- S03-UNCERTAINTY-WEIGHTING  

---

# 1. Purpose

This document formalizes the **Outlier Modeling Framework (OMF)** within the Thermognosis Engine.

Its objectives are:

1. To define outliers probabilistically rather than heuristically.
2. To distinguish between high-uncertainty data and anomalous data.
3. To integrate outlier modeling into Bayesian inference.
4. To prevent instability from corrupted or non-physical observations.
5. To preserve scientific integrity under heterogeneous data quality.

Outliers are not deleted by default.  
They are modeled as alternative generative processes.

---

# 2. Definition of Outlier

Let observation:

\[
y_i = f_\theta(x_i) + \epsilon_i
\]

Define residual:

\[
r_i = y_i - f_\theta(x_i)
\]

An outlier is an observation whose residual distribution deviates from the assumed noise model.

Formally:

\[
p(y_i \mid \theta) \not\sim \mathcal{N}(f_\theta(x_i), \sigma_i^2)
\]

---

# 3. Mixture Model Formulation

Introduce latent indicator:

\[
z_i \in \{0,1\}
\]

where:

- \( z_i = 1 \): inlier  
- \( z_i = 0 \): outlier  

Model:

\[
p(y_i \mid \theta)
=
z_i \cdot \mathcal{N}(f_\theta(x_i), \sigma_i^2)
+
(1 - z_i) \cdot p_{out}(y_i)
\]

Outlier distribution \( p_{out} \) may be:

\[
p_{out}(y_i)
=
\mathcal{N}(\mu_{out}, \tau^2)
\]

with \( \tau^2 \gg \sigma_i^2 \).

---

# 4. Prior on Outlier Probability

Let:

\[
z_i \sim \mathrm{Bernoulli}(\pi)
\]

where:

\[
\pi = P(\text{inlier})
\]

Hyperprior:

\[
\pi \sim \mathrm{Beta}(\alpha, \beta)
\]

Posterior inference jointly estimates:

\[
p(\theta, z_i, \pi \mid \mathcal{D})
\]

---

# 5. Marginal Likelihood

Marginalizing \( z_i \):

\[
p(y_i \mid \theta)
=
\pi \mathcal{N}(f_\theta(x_i), \sigma_i^2)
+
(1-\pi) p_{out}(y_i)
\]

Log-likelihood:

\[
\log p(\mathcal{D} \mid \theta)
=
\sum_i
\log
\left[
\pi \mathcal{N}(y_i \mid f_\theta(x_i), \sigma_i^2)
+
(1-\pi) p_{out}(y_i)
\right]
\]

This yields soft classification.

---

# 6. Student-t Robust Alternative

Heavy-tailed alternative:

\[
y_i \sim \mathrm{StudentT}
(\nu, f_\theta(x_i), \sigma_i^2)
\]

Density:

\[
p(y_i)
\propto
\left(
1 +
\frac{(y_i - f_\theta(x_i))^2}{\nu \sigma_i^2}
\right)^{-\frac{\nu+1}{2}}
\]

As \( \nu \to \infty \), reduces to Gaussian.

Small \( \nu \) increases robustness.

---

# 7. Posterior Outlier Probability

Posterior probability of inlier:

\[
P(z_i = 1 \mid y_i, \theta)
=
\frac{
\pi \mathcal{N}(y_i \mid f_\theta(x_i), \sigma_i^2)
}{
\pi \mathcal{N}(y_i \mid f_\theta(x_i), \sigma_i^2)
+
(1-\pi) p_{out}(y_i)
}
\]

This becomes dynamic credibility adjustment.

---

# 8. Distinction from Uncertainty Weighting

Important distinction:

- High uncertainty \( \sigma_i^2 \) reduces weight gradually.
- Outlier modeling handles structural deviations.

Uncertainty weighting assumes model correctness.  
Outlier modeling allows model violation.

---

# 9. Physical Constraint Integration

If:

\[
\mathbf{x}_i \notin \mathcal{C}
\]

then prior on \( z_i \) shifts:

\[
P(z_i = 1) \downarrow
\]

Physical violation increases probability of outlier classification.

---

# 10. Influence Function Analysis

For Gaussian model:

\[
\psi(r_i) = r_i
\]

Unbounded influence.

For Student-t:

\[
\psi(r_i)
=
\frac{r_i}{1 + \frac{r_i^2}{\nu \sigma_i^2}}
\]

Bounded influence ensures extreme residuals do not dominate.

---

# 11. Information Geometry Impact

Fisher information under mixture:

\[
\mathcal{I}(\theta)
=
\sum_i
P(z_i = 1 \mid y_i)
\frac{1}{\sigma_i^2}
\mathcal{J}_i
\]

Thus outlier probability scales information curvature.

---

# 12. Asymptotic Consistency

If fraction of true outliers:

\[
\epsilon < 0.5
\]

Robust estimator converges to:

\[
\theta^*
\]

Gaussian-only model may diverge under adversarial contamination.

---

# 13. Decision Policy

Outlier handling must follow hierarchy:

1. Soft down-weighting (default).
2. Human review if persistent anomaly.
3. Hard exclusion only with justification.

Binary deletion without probabilistic basis is prohibited.

---

# 14. Diagnostics

System must log:

- Posterior \( P(z_i = 1) \)
- Residual magnitude
- Credibility interaction
- Physical violation flags

High-confidence outliers must trigger audit.

---

# 15. Stability Safeguards

To prevent collapse:

Require:

\[
0 < \pi < 1
\]

Regularization:

\[
\alpha, \beta > 1
\]

Avoid degeneracy toward trivial all-outlier or all-inlier state.

---

# 16. Interaction with Credibility Model

Combined weight:

\[
w_i =
C_i \cdot P(z_i = 1 \mid y_i)
\]

Final effective likelihood:

\[
\log p(\mathcal{D} \mid \theta)
=
\sum_i
w_i
\log
\mathcal{N}(y_i \mid f_\theta(x_i), \sigma_i^2)
\]

This unifies credibility, uncertainty, and anomaly modeling.

---

# 17. Strategic Interpretation

Outlier modeling:

- Protects inference from corruption,
- Maintains statistical efficiency,
- Preserves scientific fairness.

It prevents:

- Overreaction to noise,
- Silent deletion of inconvenient data,
- Instability from extreme residuals.

Robustness is structural, not reactive.

---

# 18. Compliance Requirement

All inference engines must satisfy:

\[
\text{Module} \models \text{S04-OUTLIER-MODELING}
\]

Violation leads to:

- Statistical fragility,
- Susceptibility to data corruption,
- Loss of credibility governance.

---

# 19. Concluding Statement

Outlier modeling transforms anomaly handling from manual intervention into probabilistic reasoning.

The Thermognosis Engine must remain:

- Physically constrained,
- Uncertainty aware,
- Credibility weighted,
- Robust to contamination.

Scientific resilience emerges from principled modeling of deviation,  
not from rigid adherence to idealized assumptions.
