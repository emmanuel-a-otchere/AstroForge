# Contributing to AstroForge

## Spec-Driven Development (Unapologetically Enforced)

AstroForge is a **spec-driven project**. The specification is the single source
of truth for the project's behavior, architecture, and feature set. No code is
written or modified without a corresponding spec that describes the change.

### The Rule

> **The spec changes first. Then the plan. Then the code. Never the reverse.**

Any change — a new feature, an enhancement, a bug fix that alters behavior, an
architectural decision, a pipeline stage addition or removal — must be reflected
in a spec update **before** the corresponding code is written or modified.

### How to Make a Spec-Driven Change

1. **Read the active spec.** The current specification lives in
   `docs/specs/AstroForge_Spec_v1.1.0.md`. The index of all specs is at
   `docs/specs/SPEC_INDEX.md`.

2. **Propose the change.** Create a new version of the spec file following
   semantic versioning (see below). Place it in `docs/specs/`.

3. **Update the spec index.** Add the new version to `docs/specs/SPEC_INDEX.md`,
   mark the previous version as superseded, and note what changed.

4. **Update the project plan.** If the spec change adds, removes, or alters
   tasks, update `docs/PROJECT_PLAN.md` to reflect the new task breakdown.
   The project plan is a living document that is rebased frequently against
   actual work progress.

5. **Review.** The spec and plan changes should be reviewed and agreed upon
   before any implementation begins.

6. **Implement.** Write or modify code to match the approved spec. If you
   discover during implementation that the spec is wrong or incomplete, stop,
   update the spec, then continue coding.

7. **Verify.** Confirm the implementation matches the spec. If reality diverges
   from the spec, the spec is updated first — not the code.

8. **Rebase the project plan.** After every completed milestone, update the
   project plan: mark completed tasks as done, adjust remaining estimates,
   re-evaluate blocked items, and re-prioritize if needed.

### Semantic Versioning for Specs

| Version bump | When to use | Example |
|---|---|---|
| **MAJOR** (2.0.0) | Fundamental architecture, pipeline, or product-direction changes. Existing recipes may break. | Replacing ONNX with a different inference runtime |
| **MINOR** (1.2.0) | New stages, features, or capabilities added backward-compatibly. | Adding a new Stage 18: `lens_degradation_correction` |
| **PATCH** (1.1.1) | Clarifications, typo fixes, refinements to existing stages. No new features. | Correcting a parameter default value in Stage 7 |

### File Naming

Spec files follow the pattern: `AstroForge_Spec_v{MAJOR}.{MINOR}.{PATCH}.md`

Example: `AstroForge_Spec_v1.2.0.md`

### What Counts as a Spec Change

- Adding, removing, or reordering pipeline stages
- Changing a stage's algorithm, parameters, or defaults
- Adding or removing AI models from the registry
- Changing the architecture, tech stack, or platform targets
- Altering the UI/UX workflow or dialog system
- Modifying the recipe format or sharing mechanism
- Changing performance targets or constraints

### What Does NOT Require a Spec Change

- Code refactors that preserve behavior
- Performance optimizations within existing parameter bounds
- Bug fixes that bring behavior in line with the existing spec
- Test additions or improvements
- Documentation typos or clarifications that don't change meaning

## The Living Project Plan

The project plan (`docs/PROJECT_PLAN.md`) translates the spec into actionable
work. It is **rebased frequently** against actual progress:

- **After every completed milestone:** mark tasks done, adjust remaining
  estimates, re-evaluate blocked items.
- **After every spec change:** review the plan against the spec diff and add,
  remove, or re-prioritize tasks as needed.
- **When a task is blocked:** note the blocker, adjust dependent tasks, and
  re-evaluate the phase timeline.

The plan is structured into 5 phases (0–4), each with milestones and individual
tasks. Every task references the spec section it implements, ensuring full
traceability from spec to code.

## General Contribution Guidelines

*(Build instructions, dependency lists, and development setup will be added
here as the repository structure is finalized.)*
