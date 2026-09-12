Yes. CR-09 is the execution-intelligence layer that makes the architecture established in CR-05, CR-06 and CR-08 practical across different machines.

The key principle should remain:

AstroForge adapts execution to the machine; the user should not have to adapt the workflow to the machine.

CR-09 — Resource, Hardware & Execution Intelligence

Status: Proposed
Target: AstroForge v1.0
Priority: Critical
Depends on: CR-02, CR-05, CR-06, CR-08
Enables: CR-10, CR-12, CR-16, CR-21, CR-25

⸻

1. Intent

CR-09 establishes AstroForge’s ability to understand the machine on which it is running and intelligently determine how processing should be executed.

AstroForge is explicitly designed for a broad hardware range—from modest 4 GB systems through higher-memory machines with GPUs or dedicated accelerators.

The same astrophotography workflow therefore cannot assume:

* large system memory;
* discrete GPU;
* CUDA;
* Apple Silicon;
* high-end CPU;
* NPU;
* constant thermal capacity;
* unlimited storage;
* high compute throughput.

Instead, AstroForge must dynamically determine:

1. What hardware is available?
2. What resources are currently available?
3. Which processing strategy is appropriate?
4. Which AI model/backend can safely execute?
5. How much memory is required?
6. Can the operation run now?
7. Should it be tiled, serialized, downscaled, deferred or substituted?
8. What quality/performance trade-off is being introduced?

This must happen behind the workflow.

⸻

2. Product Decision

2.1 Hardware complexity is not a user workflow

The normal user should never need to think:

“Should I use CUDA?”

or:

“Should I select CoreML?”

or:

“How much VRAM does this operation require?”

AstroForge should determine the best execution path automatically.

The user should see:

AstroForge optimized this operation for your system.

Expert users may inspect the details.

⸻

3. Execution Intelligence Model

CR-09 introduces a new conceptual layer:

                 User Intent
                     │
                     ▼
                   Recipe
                     │
                     ▼
             Dataset Understanding
                     │
                     ▼
                  Pipeline
                     │
                     ▼
          ┌──────────────────────┐
          │ Execution Intelligence│
          └──────────┬───────────┘
                     │
       ┌─────────────┼──────────────┐
       ▼             ▼              ▼
   Hardware       Resources       Models
       │             │              │
       └─────────────┼──────────────┘
                     ▼
              Execution Plan
                     │
                     ▼
              Engine / Runtime
                     │
                     ▼
               Image Version

This is deliberately not another user-visible DAG.

It is an execution decision system beneath the Studio experience.

⸻

4. Hardware Discovery

At application startup AstroForge should discover relevant execution capabilities.

CPU

Record:

* architecture;
* core count;
* logical processor count;
* SIMD capabilities where relevant;
* performance characteristics where available.

Memory

Record:

* physical RAM;
* currently available RAM;
* AstroForge budget;
* memory pressure.

GPU

Where available:

* GPU vendor;
* device;
* memory;
* compute capability;
* supported runtime/provider.

Accelerators

Detect supported:

* NPU;
* Apple Neural Engine where exposed;
* integrated GPU;
* discrete GPU;
* platform-specific accelerators.

Operating System

Record:

* OS;
* architecture;
* relevant runtime capabilities.

⸻

5. Capability Rather Than Device-Centric Design

The engine should not primarily reason:

“This is an NVIDIA GPU.”

It should reason:

“A GPU capable of executing this operation with these constraints is available.”

This creates a more portable architecture.

Conceptually:

Execution Capability
├── CPU
├── GPU
├── NPU
├── Accelerator
└── None

Each capability declares:

* supported operations;
* supported precision;
* memory;
* performance characteristics;
* compatibility;
* reliability;
* availability.

⸻

6. Execution Profiles

AstroForge may internally classify execution environments.

Entry

Typical characteristics:

* 4 GB RAM;
* CPU-focused;
* limited acceleration.

Strategy:

* streaming;
* small tiles;
* reduced concurrency;
* compact AI models;
* aggressive buffer reuse.

⸻

Standard

Typical characteristics:

* 8 GB RAM;
* modern CPU;
* integrated GPU/accelerator.

Strategy:

* moderate parallelism;
* tiled AI;
* accelerated inference where reliable.

⸻

Performance

Typical characteristics:

* 16+ GB RAM;
* strong CPU;
* GPU/discrete accelerator.

Strategy:

* larger tiles;
* greater concurrency;
* high-resolution inference;
* faster previews.

⸻

Accelerated

Systems with a particularly suitable AI accelerator.

Strategy:

* prioritize compatible accelerator;
* benchmark/select optimal model/backend;
* preserve quality objectives.

Important: these profiles are execution strategies, not user restrictions.

A 4 GB system must still be a legitimate AstroForge system.

⸻

7. Resource Budgeting

AstroForge should establish a dynamic processing budget.

For example:

System RAM
     ↓
Reserved OS/application allowance
     ↓
AstroForge safe budget
     ↓
Current workload
     ↓
Available processing budget

The engine should never simply assume:

“8 GB RAM means AstroForge can use 8 GB.”

A safety margin is required.

⸻

8. Memory-Aware Execution

This is especially important for astrophotography because images can become extremely large.

AstroForge should use:

* tiled image processing;
* streaming;
* bounded queues;
* memory-mapped data where appropriate;
* buffer reuse;
* lazy loading;
* preview-resolution processing;
* controlled concurrency;
* intermediate artifact release.

Large images should not automatically be duplicated in memory.

⸻

9. Memory Estimation

Before expensive processing, AstroForge should estimate resource requirements.

Example:

AI Super Resolution

Estimated memory: 3.2 GB

Available processing budget: 2.1 GB

AstroForge recommendation:
Run using tiled inference.

Expected impact:

* Slightly longer processing time
* Same target output resolution
* No reduction in source quality

This is substantially better than allowing the application to crash from memory exhaustion.

⸻

10. Intelligent Execution Strategies

When resources are constrained, AstroForge can adapt execution in this order:

Optimal execution
       ↓
Reduce concurrency
       ↓
Reduce tile size
       ↓
Enable streaming
       ↓
Use smaller/quantized model
       ↓
Use alternative backend
       ↓
Use CPU fallback
       ↓
Use lower-cost processing strategy
       ↓
Ask user if quality trade-off is material

The exact ordering should be operation-specific.

⸻

11. Quality Must Be the Constraint

Performance optimization must not silently degrade astronomical image quality.

For example, AstroForge should not silently replace:

High-quality AI SR

with:

Low-quality model

merely because it is faster.

Instead:

Resource adaptation

Your system cannot safely run the preferred model at full resolution.

AstroForge recommends tiled inference.

Quality: preserved
Speed: approximately slower

If the only available alternative materially changes quality:

Quality trade-off detected

The available model is faster but may reconstruct less detail.

[Use Alternative] [Cancel]

⸻

12. AI Model Selection

CR-09 connects directly to CR-06 and future CR-16.

Model selection should consider:

* model size;
* quantization;
* precision;
* input dimensions;
* output dimensions;
* supported backend;
* memory requirements;
* performance;
* target image characteristics;
* expected quality;
* device capabilities.

Therefore:

Image Intelligence
       +
Operation Intent
       +
Resource Profile
       +
Model Registry
       ↓
Optimal Model Configuration

⸻

13. Backend Selection

AstroForge’s ONNX Runtime layer may support different execution providers depending on platform/build.

Conceptually:

Preferred
   ↓
Accelerated Backend
   ↓
GPU Backend
   ↓
Integrated GPU
   ↓
CPU

Potential backends include:

* CPU;
* CUDA;
* DirectML;
* CoreML;
* OpenVINO;
* other supported ONNX Runtime providers.

The implementation must validate actual runtime availability rather than assuming a provider exists.

⸻

14. Backend Benchmarking

AstroForge should optionally perform a lightweight capability benchmark.

Instead of assuming:

“GPU = faster”

it can determine:

“For this model and image size, CPU is currently more efficient than this GPU path.”

This is important because:

* integrated GPUs differ significantly;
* driver/runtime overhead matters;
* small models may run faster on CPU;
* memory transfers can dominate;
* thermal throttling can change performance.

The benchmark should be cached and periodically invalidated when hardware/runtime changes.

⸻

15. Thermal and Sustained Performance

For long astrophotography processing sessions, instantaneous performance is insufficient.

Where platform APIs permit, AstroForge should detect relevant system pressure.

Possible states:

NORMAL
ELEVATED
CONSTRAINED
CRITICAL

If sustained processing causes thermal/resource pressure, AstroForge can reduce concurrency rather than destabilize the application.

⸻

16. Execution Scheduler

CR-09 should introduce an execution scheduler responsible for:

* queueing;
* prioritization;
* concurrency;
* memory reservation;
* accelerator selection;
* cancellation;
* resource release.

Example:

Queue
│
├── Preview — High Priority
├── User Processing — High Priority
├── Background Analysis — Normal
├── Cache Generation — Low
└── Model Optimization — Idle

This prevents background tasks from starving the user’s active workflow.

⸻

17. User-Visible Resource Intelligence

The UI should expose consequences, not technical internals.

Good:

Processing optimized for available memory.

Good:

This operation will take longer because AstroForge is using tiled processing.

Poor:

ONNX Runtime allocated 1,832 MiB and selected DirectML EP.

The latter belongs in Expert Diagnostics.

⸻

18. Processing Estimate

Before long operations:

Preparing operation
Estimated time: ~3 min
Memory: ~2.4 GB
Execution: Accelerated
Strategy: Tiled
[Start]

The estimate should be explicitly qualified:

Estimates may vary depending on image complexity and system load.

⸻

19. Execution Controls

The existing CR-05 controls remain:

* Start;
* Pause;
* Resume;
* Cancel;
* Retry;
* Skip where safe.

CR-09 adds:

* execution strategy;
* resource estimate;
* backend selection;
* memory-safe fallback;
* execution priority.

The user should not normally have to configure these.

⸻

20. Expert Execution Inspector

Expert mode may expose:

Execution
CPU
Apple Silicon / x86 / ARM
Cores: 8
Memory
System: 16 GB
AstroForge budget: 8.5 GB
Current: 5.7 GB
Accelerator
GPU: Available
Backend: CoreML
Model
Model: AstroForge-SR-Compact
Precision: INT8
Execution
Tile: 512 × 512
Overlap: 32 px
Concurrency: 2
Estimated:
Memory: 2.8 GB
Time: 01:42

This should be an inspection surface, not the primary workflow.

⸻

21. Execution Events

Introduce:

HardwareProfileDetected
HardwareProfileChanged
ResourceBudgetCalculated
ResourcePressureChanged
ExecutionPlanCreated
ExecutionStrategySelected
BackendSelected
ModelConfigurationSelected
ResourceEstimateCreated
ExecutionStarted
ExecutionProgress
ExecutionPaused
ExecutionResumed
ExecutionCompleted
ExecutionFailed
FallbackTriggered
ExecutionStrategyChanged

⸻

22. Data Model

CR-09 introduces or formalizes:

hardware_profile
hardware_capability
execution_provider
execution_profile
resource_budget
resource_snapshot
resource_reservation
execution_plan
execution_strategy
execution_estimate
model_execution_profile
backend_capability
execution_metric
performance_benchmark
fallback_event

Important distinction

hardware_profile describes the machine.

execution_profile describes how AstroForge intends to use it.

execution_plan describes how a particular operation will actually execute.

⸻

23. Example Execution Plan

Conceptually:

ExecutionPlan
├── Operation
│   └── AI Super Resolution
│
├── Hardware
│   └── Standard Integrated GPU
│
├── Backend
│   └── CoreML
│
├── Model
│   └── Compact-SR INT8
│
├── Precision
│   └── INT8
│
├── Tile
│   └── 512 × 512
│
├── Concurrency
│   └── 2
│
├── Estimated Memory
│   └── 2.8 GB
│
└── Fallback
    ├── smaller tiles
    └── CPU

⸻

24. Semantic API

Conceptual commands:

get_hardware_profile
get_execution_capabilities
get_resource_budget
get_resource_snapshot
estimate_operation_resources
create_execution_plan
get_execution_plan
get_available_backends
get_backend_capabilities
select_execution_strategy
validate_execution_plan
get_model_execution_options
benchmark_execution_backend
pause_execution
resume_execution
cancel_execution
get_execution_metrics
get_fallback_history

Again, exact names should follow the repository’s existing API conventions.

⸻

25. Architecture

CR-09 creates a clear separation:

             USER
               │
               ▼
        Processing Intent
               │
               ▼
            Pipeline
               │
               ▼
     ┌─────────────────────┐
     │ Execution Intelligence│
     └──────────┬──────────┘
                │
       ┌────────┼─────────┐
       ▼        ▼         ▼
   Hardware  Resources   Models
       │        │         │
       └────────┼─────────┘
                ▼
        Execution Strategy
                │
                ▼
       Backend / Runtime
                │
                ▼
         AstroForge Engine
                │
                ▼
          Image Version

This is a critical architectural separation.

The processing intent remains stable while execution strategy changes.

⸻

26. Relationship to CR-08

CR-08 says:

“Run this Recipe.”

CR-09 determines:

“Given this machine, what is the safest and most effective way to run it?”

Therefore:

Recipe ≠ Execution Strategy

A Recipe should remain portable.

Example:

Recipe
Galaxy — Natural v1.3

could execute as:

Machine A
GPU
FP16
1024 tiles
4 workers

while the same recipe executes as:

Machine B
CPU
INT8
256 tiles
1 worker

The intent remains the same.

⸻

27. Relationship to CR-11

This distinction becomes especially important with CR-11.

CR-09 asks:

What can this machine execute?

CR-11 asks:

What can this machine execute without relying on external services?

So:

CR-09 = hardware/execution intelligence

CR-11 = connectivity/service intelligence

Neither should alter the user’s fundamental workflow.

⸻

28. Failure and Recovery

Resource failures should become actionable.

Instead of:

CUDA out of memory

AstroForge should say:

Processing paused

The selected execution strategy exceeded the available memory.

Your previous Image Version is safe.

AstroForge can retry using smaller tiles.

Expected impact: slower processing; output quality preserved.

[Retry Safely] [Cancel]

This directly builds on CR-10’s recovery model.

⸻

29. Implementation Map

Conceptual placement:

crates/
├── astroforge-core/
│   ├── execution/
│   │   ├── plan.rs
│   │   ├── strategy.rs
│   │   ├── scheduler.rs
│   │   ├── estimates.rs
│   │   └── fallback.rs
│   │
│   └── resources/
│       ├── budget.rs
│       ├── snapshot.rs
│       └── pressure.rs
│
├── astroforge-runtime/
│   ├── hardware/
│   ├── backends/
│   └── capabilities/
│
├── astroforge-ai/
│   ├── model_selection/
│   └── execution_profiles/
│
├── astroforge-persistence/
│   ├── hardware/
│   ├── execution/
│   └── performance/
│
└── astroforge-app/
    ├── execution_commands.rs
    ├── execution_events.rs
    └── resource_state.rs

These are conceptual boundaries; the actual repository structure should be preserved wherever existing modules already provide the capability.

⸻

30. Performance Telemetry

AstroForge should record execution telemetry locally for optimization and diagnostics.

Examples:

* operation duration;
* peak memory;
* average memory;
* backend;
* model;
* precision;
* tile dimensions;
* concurrency;
* throughput;
* fallback count;
* retry count;
* resource pressure;
* success/failure.

This data should feed future execution optimization.

It should not require cloud telemetry.

⸻

31. Privacy

Hardware and execution telemetry should remain local by default.

CR-09 must not introduce a hidden telemetry dependency.

If future anonymous performance telemetry is introduced, it must be:

* explicitly controlled;
* clearly disclosed;
* independent from core processing;
* compatible with CR-11’s offline-first model.

⸻

32. Acceptance Criteria

CR-09 is complete when:

Hardware

* [ ]	CPU capabilities are detected.
* [ ]	System memory is detected.
* [ ]	Available accelerators are detected.
* [ ]	Supported execution providers are detected.
* [ ]	Hardware profile is persisted appropriately.

Resource Management

* [ ]	AstroForge calculates a safe memory budget.
* [ ]	Processing operations can estimate resource requirements.
* [ ]	Memory-heavy operations use bounded resource allocation.
* [ ]	Large images can be processed using tiles/streaming.
* [ ]	Resource pressure is detected.
* [ ]	The application avoids uncontrolled memory exhaustion.

Execution

* [ ]	AstroForge selects an appropriate backend automatically.
* [ ]	Model selection considers available resources.
* [ ]	Concurrency is dynamically controlled.
* [ ]	Execution strategy can adapt to resource pressure.
* [ ]	Safe fallback strategies exist.
* [ ]	User intent remains unchanged when execution adapts.

UX

* [ ]	Normal users do not need to configure GPU/backend settings.
* [ ]	Meaningful resource estimates are shown.
* [ ]	Material quality trade-offs are disclosed.
* [ ]	Expert users can inspect execution details.
* [ ]	Resource failures produce actionable guidance.

Reproducibility

* [ ]	Execution environment is recorded.
* [ ]	Backend is recorded.
* [ ]	Model configuration is recorded.
* [ ]	Precision and tile configuration are recorded.
* [ ]	Execution strategy is included in provenance.

⸻

33. Test Strategy

Hardware Matrix

Test at minimum:

* 4 GB RAM / CPU;
* 8 GB RAM / CPU;
* 8 GB RAM / integrated GPU;
* 16 GB+ RAM;
* supported discrete GPU;
* Apple Silicon;
* Windows accelerated backend;
* Linux CPU fallback.

Where actual hardware isn’t available in CI, capability profiles should be simulated.

⸻

Resource Tests

Test:

* insufficient memory;
* memory pressure during execution;
* large image dimensions;
* multiple simultaneous operations;
* model loading/unloading;
* cancellation during inference;
* repeated processing;
* long-running workloads.

⸻

Fallback Tests

Example:

Preferred GPU
      ↓ failure
Smaller tile
      ↓ insufficient
CPU
      ↓ insufficient
User decision

No fallback should silently change a material quality characteristic.

⸻

Performance Tests

Measure:

* startup hardware detection;
* model load time;
* inference time;
* memory peak;
* tile overhead;
* throughput;
* concurrency scaling;
* sustained processing.

⸻

34. ADRs

ADR-09.1 — Hardware Is an Execution Concern

Hardware details should not shape the normal user workflow.

ADR-09.2 — Capability Over Device

Execution decisions are based on capabilities rather than vendor-specific assumptions.

ADR-09.3 — Resource Budget Is First-Class

Every significant operation should respect a dynamic resource budget.

ADR-09.4 — Quality Before Performance

Performance optimization must not silently compromise astronomical image quality.

ADR-09.5 — Execution Strategy Is Separate From Processing Intent

The same Recipe/Pipeline intent may execute differently on different machines.

ADR-09.6 — Automatic Backend Selection

AstroForge chooses the most appropriate supported execution backend automatically.

ADR-09.7 — Graceful Resource Degradation

Resource constraints result in adaptation, not application failure.

ADR-09.8 — Expert Visibility Without Expert Dependency

Technical execution details are inspectable but never required for normal use.

ADR-09.9 — Local Performance Intelligence

Execution optimization must work without cloud services.

⸻

35. Definition of Done

CR-09 is complete when:

A user can take the same AstroForge Project and Recipe to substantially different machines, and AstroForge automatically determines how to execute the workflow based on available CPU, memory, GPU/NPU, model capabilities and current resource conditions—while preserving the user’s processing intent, protecting image quality, avoiding uncontrolled resource exhaustion, and recording the execution conditions as provenance.

⸻

36. Strategic Outcome

CR-09 establishes a particularly important abstraction for AstroForge:

The user’s workflow is hardware-independent; execution is hardware-aware.

That gives the architecture this clean progression:

CR-04 — Understand the data
↓
CR-05 — Determine what processing it needs
↓
CR-06 — Determine how the image can be intelligently enhanced
↓
CR-07 — Determine which result is better
↓
CR-08 — Capture the knowledge so it can be reused
↓
CR-09 — Determine how the machine should execute it

The resulting product model is:

Intent → Intelligence → Execution → Evidence → Decision

rather than:

User → Configure GPU → Configure model → Configure memory → Configure pipeline → Hope it works

That distinction is fundamental to making AstroForge feel like an intelligent astrophotography application, rather than a graphical wrapper around a collection of image-processing libraries.

CR-10 should then formalize the application lifecycle, standalone runtime, crash recovery, project recovery, safe shutdown and runtime integrity—turning all of these capabilities into a genuinely installable, self-contained AstroForge application.