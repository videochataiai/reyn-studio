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
