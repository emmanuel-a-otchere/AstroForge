# AstroForge
A cross-platform app that turns raw smart-telescope data into publication-ready deep-sky images through an interactive, AI-augmented pipeline.

**Automated, AI-augmented astrophotography processing for the smart telescope era.**

## The Origin
This project began as a solo endeavor driven by my personal hobby. As an hobbist astrophotographer using modern smart telescopes (Dwarf, Seestar, iPhone ...), I constantly navigated the frustrating gap between basic native stacking apps and highly complex professional software. AstroForge is the tool I wanted to exist: a bridge between one-click simplicity and expert-level control, built specifically for the unique quirks of modern automated hardware.

## Unique Positioning
How does AstroForge compare to existing solutions in the astrophotography ecosystem?

* **Professional Closed Source (e.g., PixInsight):** Unmatched power and flexibility; however, it carries a steep learning curve, high financial cost, and requires heavy manual workflow overhead.
* **Traditional Open Source (e.g., Siril, DeepSkyStacker):** Fantastic calibration and stacking engines; yet, they often require exporting to general-purpose photo editors for final polishing and lack integrated, modern AI restoration models.
* **Native Smart Telescope Apps:** Provide instant gratification and ease of use; but they operate as locked black boxes, failing to support advanced narrowband compositions, custom AI enhancement, or expert parameter tweaking.
* **The AstroForge Advantage:** Purpose-built for the modern smart telescope output. It natively handles raw Bayer PNG and JPG files without relying on missing metadata, features a lightweight AI-augmented processing graph, and scales its interface from beginner-friendly wizards to expert parameter panels. Furthermore, it is explicitly engineered to run advanced AI models on modest 4GB to 8GB RAM systems.

## Core Capabilities
* **Intelligent Ingestion:** Automatically detects raw Bayer patterns in FITS, DNG, PNG, and JPG formats using statistical analysis and autocorrelation when camera metadata is absent.
* **Target-Aware Routing:** Dynamically switches between deep-sky stacking and planetary lucky-imaging pipelines based on exposure time and frame count heuristics.
* **Narrowband Composition:** Seamlessly extracts and combines Ha, OIII, and SII channels from OSC sensors into HOO, SHO, or custom palettes.
* **AI Model Hub:** Integrates lightweight, tile-based models like SwinIR for denoising and super-resolution, alongside Deep Image Prior for zero-shot deconvolution. 
* **Interactive Pipeline:** Every processing stage offers a preview dialog. Users can choose a global verbosity level: Auto for beginners; Confirm for intermediates; and Manual for experts.
* **Recipe Sharing:** Exports sanitized JSON sidecar files, allowing users to share exact processing workflows and parameter sets with the community.

## Forward-Looking Vision and Delivery
AstroForge is designed to evolve alongside hardware capabilities and community needs. 

* **Primary Delivery:** A lightweight, cross-platform desktop application built on Tauri and Rust. This ensures a tiny memory footprint, native GPU access, and avoids the heavy overhead of Electron-based frameworks.
* **Headless and CLI Modes:** Future support for command-line execution is planned to enable automated processing on remote observatories, Raspberry Pi setups, and cloud instances.
* **Plugin Ecosystem:** Future architecture will support Python and WASM plugins; this will allow the community to add custom calibration steps, experimental AI models, or direct integrations with telescope control software.
* **Community Model Training:** As the project grows, we intend to release open datasets and training scripts, allowing users to fine-tune restoration models on their specific camera sensors and local light pollution conditions.

## Call for Contributions
While this started as a solo hobby project, building a robust astrophotography engine requires a community. Contributions are highly encouraged and welcomed across all skill levels!

* **Test Data:** Donate your raw smart telescope sessions (especially unusual PNG and JPG raw Bayer files) to help train and test our ingestion heuristics.
* **AI Optimization:** Help quantize, prune, and optimize ONNX models to run smoothly on low-memory integrated GPUs.
* **Frontend and UX:** Assist in designing intuitive Svelte-based wizard dialogs, interactive histogram tools, and live-preview canvases.
* **Backend Engineering:** Contribute to the Rust-based DAG orchestrator, FITS parsing, and high-performance image math routines.
* **Documentation and Translation:** Help write user guides, document the pipeline stages, and translate the interface for the global astrophotography community.

## Getting Started
*(Build instructions, dependency lists, and contribution guidelines will be added here as the repository structure is finalized.)*

## License
*(To be determined; targeting a permissive open-source license such as MIT or Apache 2.0 to ensure free distribution and community adoption.)*
