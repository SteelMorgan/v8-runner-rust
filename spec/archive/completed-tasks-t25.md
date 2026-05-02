# Completed Task T25

## T25: Generate JSON Schema field descriptions and remove config aliases

Status: completed on `2026-05-03`.

Implemented scope:

- Added concise JSON Schema descriptions for user-facing config fields in the main config and
  local overlay schemas.
- Kept descriptions in the schema boundary model instead of parsing prose from
  `docs/CONFIGURATION.md`.
- Covered nested config sections including `infobase`, `source-set`, `tools`, `mcp`, `tests`, and
  `tools.client_mcp.extension`.
- Removed YAML alias support from the config model, schema boundary, generated schema artifacts,
  documentation, and tests.
- Preserved unrelated null-reset and numeric-bound schema semantics.
- Regenerated `docs/schemas/v8project.schema.json` and
  `docs/schemas/v8project.local.schema.json`.

Verification:

- `cargo test --locked generated_schema`
- `cargo test --locked removed_alias_keys`
