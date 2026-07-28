# Reyn Studio Product

## Register

product

## Users

Reyn Studio serves simulation and verification leads, physics-ML teams, and design
partners evaluating fast external-flow prediction. It also serves independent
researchers, educators, and advanced technical learners. They work locally with
geometry, checkpoints, operating points, fields, loads, and evidence, often under
confidentiality or offline constraints. Their core job is to reach a useful result
quickly while retaining enough lineage and method detail to explain what produced it.

## Product Purpose

Reyn Studio is a local-first neural-CFD scientific instrument for creating,
interrogating, comparing, and preserving defensible incompressible-flow predictions.
It combines a native Rust interface, GPU scientific views, and a Python model engine
with a durable Project → Case → immutable Run → Evidence workflow.

Success means a technical user can import supported geometry, qualify the setup, run
within the model envelope, inspect engineering-relevant fields and fluid loads, and
export evidence without confusing model output, recovered quantities, derivations,
references, integrity, or authenticity. Reyn assists engineering judgment; it does not
pretend to replace general-purpose CFD, structural analysis, or human review.

## Brand Personality

Calm, mechanical, exacting. Reyn should feel like a premium laboratory instrument:
confident because it is precise, restrained because the work is complex, and candid
about every boundary. It is warm enough to feel authored, never decorative enough to
distract from the evidence.

## Anti-references

- Cheap generic SaaS dashboards, account furniture, upgrade prompts, support buttons,
  oversized KPI cards, and interchangeable card grids.
- Decorative sci-fi control rooms, cyber-blue glow, glassmorphism, haze, ornamental
  grids, animated “AI” motifs, or confidence theater.
- Ribbon clones and universal simulation trees that expose every control at once.
- Toy-like educational simplification that hides units, provenance, or model limits.
- Marketing language or UI states that imply “AI replaces CFD,” general CAD support,
  structural stress, independent validation, or release readiness when those claims
  are not supported.
- Unfamiliar custom affordances where standard desktop interactions are clearer.

## Design Principles

1. **Evidence before spectacle.** Visualization exists to measure, compare, and decide.
2. **Honesty is structural.** Unsupported, unknown, recovered, derived, referenced,
   stale, unsigned, and verified states remain visibly distinct.
3. **Fast expert interaction.** Keep the shell responsive, keyboard-reachable, and
   predictable; progressive disclosure should reveal depth without slowing the main
   engineering path.
4. **Durable objects over screen state.** Project, source, case, run, and evidence
   lineage survive reopening and remain attached to exports.
5. **Premium through precision.** Hierarchy, typography, alignment, calibrated color,
   motion, and microcopy must be exceptionally consistent; decoration never substitutes
   for interaction quality.
6. **Local ownership.** Creation, execution, review, and export work without an account
   or mandatory cloud dependency.

## Accessibility & Inclusion

Target WCAG 2.2 AA-equivalent contrast and interaction quality within the constraints
of the native egui surface. Every workflow and consequential action should be keyboard
reachable with a visible focus state. Status is never color-only. Reduced-motion and
high-contrast preferences are first-class, and scientific colormaps include a
colorblind-safe option without changing stored evidence. Labels, units, error reasons,
and unavailable states use readable text rather than icon-only or hover-only disclosure.
# Product

## Register

product

## Users

Physics-ML researchers, simulation and verification leads, and educators using a local desktop instrument to inspect, compare, and validate neural CFD predictions. Their primary workflow is loading a model, generating or importing a flow case, interrogating fields and recovered quantities, and deciding whether the model is trustworthy for a declared horizon and regime.

## Product Purpose

Reyn Studio is a local-first neural-CFD workbench that combines native GPU visualization with a Python inference and solver engine. It exists to make fast surrogate predictions inspectable rather than opaque: every important result should expose physical context, provenance, solver comparisons, uncertainty or trust evidence, and exportable verification artifacts.

Success means a technical user can move from model import to a defensible flow analysis without leaving the app, while the interface remains responsive when the engine is unavailable and never presents validation data as independent evidence.

## Brand Personality

Precise, trustworthy, and restrained. Reyn Studio should feel like a calibrated scientific instrument: quiet native chrome, dense but legible measurements, and expressive flow data without cinematic excess.

## Anti-references

- Generic enterprise-CFD brochureware or a clone of ParaView/Ansys chrome.
- Decorative “AI” visuals, stock imagery, or effects that obscure the underlying field.
- Blue-on-black cyber dashboards, excessive glass, oversized KPI cards, or gratuitous animation.
- Interfaces that hide provenance, reuse validation data as test evidence, or imply certainty unsupported by the experiment.

## Design Principles

1. Evidence before spectacle: visualization serves measurement and verification.
2. Honest by construction: provenance, leakage risks, baselines, and limits remain visible.
3. Expert speed: preserve dense native workflows, predictable controls, and low-latency interaction.
4. Graceful degradation: the shell stays useful and explains failures when inference or solver services are unavailable.
5. One physical vocabulary: colors, units, controls, and trust states remain consistent across 2D, 3D, benchmarks, and reports.

## Accessibility & Inclusion

Target WCAG 2.1 AA contrast where applicable in the native interface. Respect reduced-motion system settings, maintain visible keyboard focus, do not encode trust or error states by color alone, and pair scientific color scales with calibrated legends and numeric values.
