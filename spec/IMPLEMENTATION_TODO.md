# Active TODO For `v8-runner`

This file tracks open implementation work only.

## Current Status

- Open tasks as of `2026-05-03`: 1.

## Open Tasks

### T25: Generate JSON Schema field descriptions and remove config aliases

Status: open.

Trigger: generated `docs/schemas/v8project.schema.json` and
`docs/schemas/v8project.local.schema.json` are useful for YAML editing, but fields do not currently
carry `description` text for editor hover/help. The same schema generation path also carries YAML
aliases that make the public config contract broader and harder to reason about.

Scope:

- Add concise descriptions to the YAML-boundary schema structs in `src/config/schema.rs`, preferably
  through Rust doc comments consumed by `schemars`.
- Keep descriptions close to the schema model; do not parse prose from `docs/CONFIGURATION.md`.
- Cover both main config and local overlay schemas, including nested sections such as `infobase`,
  `source-set`, `tools`, `mcp`, `tests`, and `tools.client_mcp.extension`.
- Remove YAML aliases from `src/config/model.rs` and `src/config/schema.rs`; keep only canonical
  public keys.
- Remove alias documentation from `docs/CONFIGURATION.md` and update examples/tests that still use
  alias keys.
- Preserve null-reset, numeric-bound and validation semantics unrelated to aliases.
- Regenerate `docs/schemas/v8project.schema.json` and
  `docs/schemas/v8project.local.schema.json`.

Acceptance:

- Schema artifacts contain `description` entries for user-facing config fields.
- Schema artifacts no longer contain removed alias-only properties for execution timeout, EDT CLI
  section naming, enterprise launch keys, or EDT timeout spellings.
- Loader rejects removed alias keys with normal unknown-field/schema errors.
- `cargo test --locked generated_schema_artifacts_are_current` passes without
  `UPDATE_CONFIG_SCHEMAS=1`.
- Existing schema/loader parity tests for main config and local overlay remain green.

## Rules

- Keep this file short and active-only.
- Move closed task detail into `spec/archive/`.
- If a task changes a public or architectural contract, update the ADR and active docs layer
  before implementation.
- Promote only immediately executable work here; keep broader ADR reconciliation in
  `ADR_DERIVED_BACKLOG.md`.

## Historical Records

- [spec/archive/IMPLEMENTATION_TODO_2026-04-30.md](archive/IMPLEMENTATION_TODO_2026-04-30.md):
  closed task ledger moved out of the active file.
- [spec/archive/MCP_IMPLEMENTATION_PLAN_2026-03-21.md](archive/MCP_IMPLEMENTATION_PLAN_2026-03-21.md):
  closed MCP rollout history.
- [spec/archive/completed-tasks-t22.md](archive/completed-tasks-t22.md):
  closed universal tool extension preparation task.
- [spec/archive/completed-tasks-t21.md](archive/completed-tasks-t21.md):
  closed local config overlay task.
- [spec/archive/completed-tasks-t23.md](archive/completed-tasks-t23.md):
  closed YAML schema support for config editing.
- [spec/archive/completed-tasks-t24.md](archive/completed-tasks-t24.md):
  closed source-backed tool extension change-detection task.
