# AstroForge — Local Working Notes

This folder is a clone of https://github.com/emmanuel-a-otchere/AstroForge.
It is the local development workspace for Phase 1.5 (Guided Processing Train).

## Source of truth (re-validate before edits)
- Repo: github.com/emmanuel-a-otchere/AstroForge
- Spec: AstroForge_Spec_v1.1.0.md
- Plan: docs/PROJECT_PLAN.md
- Active CR: docs/CR_AF-CR-2026-09-01-IMG-PIPELINE.md
- Project board: github.com/users/emmanuel-a-otchere/projects/11

## Milestone scheme (used by plan, issues, and commits)
- Phase N → coarse bucket (0..4)
- Milestone N.M → phase N, milestone M (e.g. P1.5-M5 = Phase 1.5, Milestone 5)
- Task N.M-K → milestone N.M, task K (e.g. P1.5-M5-T3)

"Milestone 1.5.1" in conversation = P1.5-M1 (Processing Train State Machine).
"Milestone 1.5.5" in conversation = P1.5-M5 (Backend AI Service Layer).

## Local conventions
- `local/` is gitignored scratch space for ad-hoc scripts, dumps, and probes.
- `scratch/` is gitignored scratch space for throwaway experiments.
- All real work happens on a feature branch + PR. Never commit to main.
- Before pushing: re-validate state from GitHub (issues, project board, latest main).
