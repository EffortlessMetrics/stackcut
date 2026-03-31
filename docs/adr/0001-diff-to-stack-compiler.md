# ADR 0001: Build `stackcut` as a diff-to-stack compiler

## Status

Accepted.

## Context

The problem is not lack of review tools. The gap is earlier in the flow: one AI-sized or human-sized oversized diff needs to become a humane stack before review infrastructure can help.

Existing tools are strong once a stack exists. `stackcut` therefore sits upstream and compiles one large change into a reviewable series.

## Decision

Build `stackcut` as:

- local-first
- deterministic
- artifact-first
- overrideable
- validation-heavy

Start at file granularity and prove exact recomposition before adding deeper semantic slicing.

## Consequences

### Positive

- trust boundary is clear
- behavior is explainable
- artifacts are portable
- patch materialization is straightforward

### Negative

- mixed concerns inside one file remain unresolved in v0.1
- richer semantic cuts require future work
- some cases will remain explicitly ambiguous

## Why this is still the right first cut

The first product bar is not elegance. It is trust.

A deterministic file-scoped planner with exact recomposition is a stronger foundation than a flashier semantic slicer that cannot prove what it did.
