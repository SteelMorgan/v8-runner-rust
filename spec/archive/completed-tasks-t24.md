# Completed Task T24

## T24: Skip unchanged source-backed tool extension preparation

Status: completed on `2026-05-03`.

Source ADR: [ADR-0022](../decisions/0022-universalnyy-mehanizm-podgotovki-rasshireniy-i-client-mcp-extension.md).

Implemented scope:

- Added private change-detection context for `tools.client_mcp.extension.source` under
  `workPath/hash-storages/tool-<extension-name>-source.redb`.
- Kept tool extensions outside project `source-set` and `--source-set` selection.
- Made unchanged source-backed tool extensions return a skipped build step without EDT export,
  Designer load, or apply.
- Preserved conservative refresh for changed sources, recoverable fallback state, and
  `build --full-rebuild`.
- Committed tool-extension source snapshots only after successful export/load/apply.
- Left `.cfe` artifact behavior unchanged.

Verification:

- `build_skips_unchanged_edt_client_mcp_source_tool_extension`
- `build_refreshes_changed_edt_client_mcp_source_tool_extension_and_commits_state`
- `full_rebuild_refreshes_edt_client_mcp_source_tool_extension`
- `failed_edt_client_mcp_source_export_does_not_commit_tool_extension_state`
- `recoverable_tool_extension_storage_fallback_refreshes_and_commits_after_success`
- `failed_edt_client_mcp_source_load_does_not_commit_tool_extension_state`
- `failed_edt_client_mcp_source_update_does_not_commit_tool_extension_state`
- `repeated_test_skips_unchanged_source_backed_tool_extension_build`
