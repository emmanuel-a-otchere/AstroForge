# ADR-0019 — CR-08 ADR-08.8: Recipes Cannot Execute Arbitrary Code

- Status: Accepted (2026-09-22)
- Gates: `CR-08 §30`, `CR-08 §20`, `CR-08 §21`
- Closes: CR-08 §30 architectural decision records (sub-record 8/9)
- Source spec: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30

## Context

A Recipe is a structured artifact: it carries
typed fields (name, target_type, stages,
parameters) and is parsed by AstroForge at apply
time. Could a Recipe carry arbitrary executable
code (a Python script, a shell command, a binary
payload)? The §30 spec is firm: "Recipes are
declarative processing specifications."

Three options were on the table:

1. **Free-form code allowed.** A Recipe can carry
   a `code: Option<String>` field that the apply
   round evaluates.
2. **Pure declarative only.** A Recipe is a
   structured document; the apply round rejects
   any payload that resembles executable code.
3. **Inline DSL.** A Recipe can carry a
   limited-purpose DSL (e.g. a tiny expression
   language) that is statically analyzed.

## Decision

**Adopt option 2: Pure declarative only.** A
Recipe is a structured document. The §20
validation pipeline (PR #392) runs five classes of
checks against every Recipe before the apply
round: parameter range, dependency, filesystem
reference rejection, executable payload rejection,
resource budget. Executable payload rejection
catches: shebangs (`#!/`), Python signatures
(`os.system(`, `subprocess.`, `eval(`, `exec(`),
shell signatures (`shell_exec`), HTML script tags
(`<script`, `</script`).

### Why option 2

- **Safety.** A Recipe is shareable (per the §19
  import/export). A Recipe that carries code is a
  Recipe that ships arbitrary execution to any
  user who imports it. Option 1 + 3 both expose
  users to remote-code-execution risks.
- **Reproducibility.** Code-as-data obscures the
  decision (which Python version? which lib
  version?). The declarative structure makes
  reproducibility verification mechanical.
- **Audit-grade trail.** Per ADR-0015 + 0016,
  every Recipe is auditable; arbitrary code is
  opaque to audit.

### Why not option 1

- RCE risk. A malicious Recipe can `eval()` or
  `subprocess.run()` arbitrary commands; the §19
  import path becomes an attack vector.

### Why not option 3

- DSL surface area. Any inline DSL eventually
  grows escape hatches (the user wants `eval`,
  the DSL author adds it); option 2 keeps the
  Recipe pure.

## Consequences

- The §20 validation pipeline (PR #392) enforces
  executable payload rejection as a hard rule.
  Recipes that contain `#!/`, `os.system(`,
  `subprocess.`, `eval(`, `exec(`, `<script`,
  `</script`, `shell_exec` are rejected.
- The Recipe struct's `params: HashMap<String,
  serde_json::Value>` accepts only JSON values;
  string values are scanned by the §20
  executable + filesystem checks before the
  apply round.

## Related

- ADR-0020 (Provenance Is Human-Readable)
- ADR-08.8 source: `docs/CR-08-RECIPES-REPRODUCIBILITY-PROVENANCE.md` §30
- §20 audit row: `docs/CR-08-AUDIT.md`