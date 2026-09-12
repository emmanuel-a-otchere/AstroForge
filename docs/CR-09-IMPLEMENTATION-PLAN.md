# CR-09 Implementation Plan — Resource, Hardware & Execution Intelligence

**Source:** [`CR-09-RESOURCE-HARDWARE-EXECUTION-INTELLIGENCE.md`](CR-09-RESOURCE-HARDWARE-EXECUTION-INTELLIGENCE.md)
**Audit:** [`CR-09-AUDIT.md`](CR-09-AUDIT.md) — 29% shipped, 30% partial, 41% missing (145 sub-items)
**Status:** Proposed → 6 bundles staged
**Target:** AstroForge v1.0

CR-09 has substantial existing surface from slice CD (`gpu_providers.rs`, 233 LOC) + the pre-existing `resource.rs` (~600 LOC) + `hardware.rs` (190 LOC) + `resource_estimate.rs` (223 LOC). The audit confirms coverage of §4 hardware discovery + §13 backend selection + §7-§9 resource budgeting + §18 processing estimate.

The 6-bundle plan addresses the remaining gaps.

## Bundle order

| # | Bundle | Slices | PR scope | LOC est. |
|---|---|---|---|---|
| **E1** | **ExecutionPlan data type** | §22 partial + §23 + §24 partial | `ExecutionPlan` aggregate struct (Operation / Hardware / Backend / Model / Precision / Tile / Concurrency / Estimated Memory / Fallback); `ExecutionStrategy` enum (Optimal / ReducedConcurrency / Tiled / Streaming / QuantizedModel / AlternativeBackend / CpuFallback / LowerCost); `select_execution_strategy()` function (the §10 8-step ladder); wire `ExecutionPlan` ↔ `AiOperation`; 9 ADRs (§34 ADR-09.1..09.9) | 1,500 |
| **E2** | **IPC surface for execution intelligence** | §24 | 17 new Tauri commands in `commands_execution.rs`: `get_hardware_profile`, `get_resource_snapshot`, `get_resource_budget`, `get_available_backends`, `estimate_operation_resources`, `create_execution_plan`, `get_execution_plan`, `select_execution_strategy`, `get_model_execution_options`, `get_execution_metrics`, etc. | 800 |
| **E3** | **Execution scheduler** | §8 partial + §16 + §21 events | `ExecutionScheduler` module: priority queue (Preview / UserProcessing / BackgroundAnalysis / CacheGeneration / ModelOptimization); concurrency control (configurable worker count); resource reservation; cancellation; release. 17 §21 events (Tauri event emitters). | 1,200 |
| **E4** | **Thermal + benchmarking + pressure** | §14 + §15 + §21 | `benchmark_execution_backend(model, w, h)` runs a 1-tile probe and caches result; thermal monitor (CPU temp via sysinfo or platform API; 4-state ladder NORMAL/ELEVATED/CONSTRAINED/CRITICAL); resource pressure sampling (continuous background task); cache invalidation when hardware/runtime changes | 1,000 |
| **E5** | **UX surface for execution intelligence** | §11 + §17 + §18 partial + §20 + §28 | `ProcessingOptimizedToast.svelte` (§17 "Good UX"); `QualityTradeoffPrompt.svelte` (§11 "Resource adaptation"); `ExecutionInspector.svelte` expert-mode (§20 detailed view); resource estimate presentation (§18 "Estimated time / Memory / Execution / Strategy / Start"); failure-recovery prompt (§28 "Processing paused / Retry Safely / Cancel") | 1,500 |
| **E6** | **Test coverage + telemetry** | §29 + §30 + §32 acceptance + §33 test strategy | Hardware matrix tests (4GB CPU / 8GB CPU / 8GB+iGPU / 16GB+ / discrete GPU / Apple Silicon / Windows accelerated / Linux CPU fallback); capability-profile simulation; resource tests (insufficient memory / pressure / large dims / multi-op / cancellation); fallback tests (GPU → smaller tile → CPU → user); performance tests (startup / model load / inference / memory peak / tile overhead / throughput / sustained); telemetry data type (§30) | 1,500 |

**Total: ~7,500 LOC over 6 PRs.**

## Implementation locations

```
crates/
├── astroforge-core/
│   ├── resource.rs                 (existing, E1 + E4 extend)
│   ├── execution/
│   │   ├── mod.rs
│   │   ├── plan.rs                 (NEW, E1 — ExecutionPlan)
│   │   ├── strategy.rs             (NEW, E1 — ExecutionStrategy enum + ladder)
│   │   ├── scheduler.rs            (NEW, E3)
│   │   ├── estimates.rs            (NEW, E1 — wraps resource_estimate.rs)
│   │   ├── fallback.rs             (NEW, E1 — fallback history)
│   │   └── benchmark.rs            (NEW, E4)
│   ├── resources/
│   │   ├── mod.rs
│   │   ├── budget.rs               (existing surface in resource.rs)
│   │   ├── snapshot.rs             (existing surface)
│   │   ├── pressure.rs             (NEW, E4)
│   │   └── thermal.rs              (NEW, E4)
│   └── recipe.rs                   (existing — link to ExecutionPlan via R7)
│
├── astroforge-ai/
│   ├── hardware.rs                 (existing)
│   ├── gpu_providers.rs            (existing, E1 wires to ExecutionPlan)
│   ├── recommendations/
│   │   ├── resource_estimate.rs    (existing)
│   │   └── execution_profiles.rs   (NEW, E1)
│   └── execution/
│       ├── model_selection.rs      (NEW, E1 — SelectionMatrix)
│       └── telemetry.rs            (NEW, E6)
│
└── src-tauri/src/
    ├── commands_execution.rs       (NEW, E2)
    ├── events_execution.rs         (NEW, E3 — 17 events)
    └── resource_state.rs           (NEW, E4 — IPC state)

src/
├── components/
│   ├── ExecutionInspector.svelte        (NEW, E5)
│   ├── ProcessingOptimizedToast.svelte   (NEW, E5)
│   ├── QualityTradeoffPrompt.svelte     (NEW, E5)
│   ├── FailureRecoveryPrompt.svelte     (NEW, E5)
│   └── ResourceEstimateCard.svelte      (NEW, E5)
└── lib/
    ├── execution-store.ts               (NEW, E2)
    └── resource-store.ts                (NEW, E4 — pressure + thermal subscriptions)

docs/adr/
├── 0021-cr09-hardware-execution-concern.md     (E1)
├── 0022-cr09-capability-over-device.md         (E1)
├── 0023-cr09-resource-budget-first-class.md    (E1)
├── 0024-cr09-quality-before-performance.md     (E1)
├── 0025-cr09-strategy-separate-from-intent.md  (E1)
├── 0026-cr09-automatic-backend-selection.md    (E1)
├── 0027-cr09-graceful-degradation.md            (E1)
├── 0028-cr09-expert-visibility.md              (E1)
└── 0029-cr09-local-performance-intelligence.md (E1)
```

## Key audit-driven decisions

1. **ExecutionPlan aggregates the existing `RecommendedExecution` +
   `ResourceEstimate` + `BackendCapability` + `ExecutionProvider`.** No
   parallel data model; this is a thin composition.
2. **The §10 8-step adaptation ladder** is implemented as
   `ExecutionStrategy::from_constraints(budget, plan)` returning the
   most-aggressive strategy that fits.
3. **The §16 scheduler is priority-based, not preemption-based.** User
   processing is highest priority; background tasks yield. This avoids
   the complexity of preemption while still protecting the user.
4. **The §15 thermal monitor is best-effort** — sysinfo provides CPU
   temp on Linux + Windows; macOS requires additional API. Where
   unavailable, the monitor emits ResourcePressureChanged events
   based on memory pressure only.
5. **The §14 benchmark is cached with a TTL** (default 1 hour)
   invalidated on hardware-profile change or model load error.
6. **The §17 + §18 UX is progressive disclosure**: normal users see
   "Processing optimized for available memory" toast; experts see the
   full ExecutionInspector. Quality trade-off prompts only fire when
   the strategy degradation crosses a meaningful threshold (e.g.,
   CPU fallback when GPU was preferred).

## §32 acceptance gap-fill (most-impactful)

The audit surfaced 11 missing + 9 partial acceptance criteria. Per bundle:
- CPU capabilities detection → E1 (extend HardwareProbe)
- Hardware profile persisted → E1 (db.rs column)
- Resource pressure detected → E4 (continuous monitor)
- Concurrency dynamically controlled → E3 (scheduler)
- Execution strategy adapts to pressure → E4 (subscribe to pressure events)
- Safe fallback strategies → E1 (ExecutionStrategy ladder)
- Material quality trade-offs disclosed → E5 (QualityTradeoffPrompt)
- Expert inspect execution details → E5 (ExecutionInspector)
- Resource failures actionable → E5 (FailureRecoveryPrompt)
- Execution environment recorded → E1 (ExecutionPlan persisted per Pipeline Run)
- Execution strategy in provenance → E1 (link to AiOperation)

## First concrete slice

**E1 — ExecutionPlan data type + strategy selection.** Paperwork + Rust
types only:

1. `crates/astroforge-core/src/execution/plan.rs` — `ExecutionPlan`
   struct (§22 + §23).
2. `crates/astroforge-core/src/execution/strategy.rs` —
   `ExecutionStrategy` enum + `from_constraints(budget, plan)`
   implementing the §10 8-step ladder.
3. Wire `ExecutionPlan` ↔ `AiOperation::resource_metrics` (record
   strategy on completion).
4. 9 ADRs (§34 ADR-09.1..09.9).
5. Tests: strategy selection (8 scenarios covering the full ladder),
   plan serialization, strategy persistence.

No behavior change; just richer data + decisions. Unblocks E2 + E3 + E4.