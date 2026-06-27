---
name: v8-runner
description: "Use when Codex needs to operate v8-runner on local 1C projects from the CLI: configure v8project.yaml, initialize infobases or EDT workspaces, build Designer or EDT sources, run syntax checks and tests, dump infobase changes, convert source formats, load or export artifacts, launch 1C clients, or choose safe 1C automation command sequences."
---

# v8-runner

Use this skill to operate `v8-runner` as the automation layer for local 1C development projects.

Keep this file as the decision entrypoint. Load only the reference file that matches the task:

- `references/command-selection.md` for choosing the right command sequence.
- `references/bootstrap.md` for generating `v8project.yaml` from an existing repository — what to detect yourself and what to ask the user (decision tree for `format`, `builder`, `connection`).
- `references/config-and-backends.md` for `v8project.yaml`, source sets, formats, builders, and backend limits.
- `references/project-workflows.md` for common build, syntax, dump, launch, and source sync workflows across Designer and EDT projects.
- `references/file-and-artifact-workflows.md` for dump, convert, load, make/artifacts, and staged publication.
- `references/testing.md` for YaXUnit, Vanessa Automation, syntax checks, and artifacts.
- `references/troubleshooting.md` for setup failures, stale state, and environment diagnostics.

## Command Form

Use the available `v8-runner` binary directly. If it is not on `PATH`, ask for the binary path or use a project-provided wrapper script.

`v8project.yaml` is the default project config name. A sibling `v8project.local.yaml` is loaded automatically for machine-local paths, credentials, tools, tests, and MCP settings. Do not pass `--config v8project.yaml` unless the user explicitly wants a non-default command shape or the active config path differs from the default; never pass `v8project.local.yaml` as `--config`.

Generated `v8project.yaml` files include a `yaml-language-server` modeline that points to the versioned JSON Schema for the current `v8-runner` release. For `v8project.local.yaml`, use the matching `docs/schemas/v8project.local.schema.json` raw GitHub tag URL in editor settings when schema-assisted editing matters.

Use JSON output only when another tool, script, or final answer needs structured results:

```bash
v8-runner --json-message build
```

Use text output for direct human diagnostics.

Useful global flags:

- `--config <CONFIG>` when the active config is not `./v8project.yaml`.
- `--json-message` for machine-readable CLI envelopes.
- `--workdir <WORKDIR>` to override `workPath`; it wins over `v8project.local.yaml`.
- `--clean-before-execution` to clear logs before execution.
- `--log-level <error|warn|info|debug|trace>` for diagnostics.
- `--no-color` for plain text output.

## First Pass

1. Check whether `v8project.yaml` exists in the 1C project root.
2. If it is missing, run the narrowest `v8-runner config init ...` command that fits the project shape.
3. Inspect the generated config before running mutating commands.
4. Run `v8-runner init` only when the file infobase or EDT workspace needs to be created.
5. Run the narrowest validation command that answers the user's goal.

Useful bootstrap commands:

```bash
v8-runner config init
v8-runner config init --connection "File=build/ib"
v8-runner config init --format edt
v8-runner config init --builder IBCMD
v8-runner init
```

## Default Use-Case Routing

- Source files changed and infobase may be stale: run `v8-runner build`.
- Only one source-set changed: use commands that accept `--source-set <NAME>` instead of rebuilding or materializing everything.
- Branch switch, rebase, large object moves, stale source-backed tool extension state, or suspicious incremental state: run `v8-runner build --full-rebuild`.
- Syntax check: inspect `format` and `builder`, then choose `syntax designer-modules`, `syntax designer-config`, or `syntax edt`.
- Behavior validation: run the relevant `v8-runner test ...` command; tests build first.
- Vanessa Automation debugging or scenario authoring: use `v8-runner launch mcp va ...`
  to start the VA test-manager for MCP tools. The runner adds `/TESTMANAGER` and
  `/DisableUnsafeActionProtection`; WS mode passes `mcpMode=ws;...;VAParams=<runtime params>`
  in `/C`, while local HTTP MCP fallback passes `runMcp...;VAParams=<runtime params>`
  without `StartFeaturePlayer`. Transport defaults to `auto`: WS is selected when the
  session-manager is reachable, while local HTTP MCP is selected only with
  `--mcp-transport=mcp` or as the `auto` fallback.
- Extension properties need synchronization: use `v8-runner extensions` or `extensions --name <SOURCE_SET>`.
- Infobase changes need to become Git-visible files: check `git status`, then run the relevant `v8-runner dump ...` command.
- Source files need conversion between Designer and EDT: use `v8-runner convert`; this is CLI-only and does not use the infobase.
- Existing `.cf` or `.cfe` artifacts need to be applied to an infobase: use `v8-runner load ...`.
- Release artifacts need to be exported or external artifacts published: use `v8-runner make ...` or the `artifacts` alias.
- Need a 1C UI session: use `v8-runner launch designer`, `launch thin`, `launch thick`, or `launch ordinary`.
- Need onec-client-mcp-devkit launched inside 1C without VA authoring: use `v8-runner launch mcp ...`.
- Pair the launched 1С-client with a running [v8-client-session-manager](https://github.com/SteelMorgan/v8-client-session-manager) over WebSocket: rely on `--mcp-transport=auto` (default — TCP-probes `manager_url` for 200 ms). Force WS with `--mcp-transport=ws` (fails if manager is down) or use local HTTP MCP with `--mcp-transport=mcp`. WS-only flags: `--manager-url`, `--client-uid`, `--corr-id`, `--mcp-log-level`, `--mcp-ws-timeout-ms`. The internal `kind` mapping (`v8_runner_client` / `vanessa_test_client` / `yaxunit_runner` / `vanessa_test_client`) is fixed by entry-point and **not** overridable from CLI. Read `references/project-workflows.md` (section «WS-режим к session-manager») for the full payload, defaults, and `--json-message` shape.

## Launch Modes

| Mode | Purpose | MCP/VA behavior |
|---|---|---|
| `launch designer` | Open Designer. | No client MCP tools; enterprise additional keys are not applied. |
| `launch thin`, `launch thick`, `launch ordinary` | Open a regular 1C UI client. | Optional WS base client tools; no VA tools by itself. |
| `launch mcp` | Start onec-client-mcp-devkit inside 1C without Vanessa. | `kind=v8_runner_client` in WS, or local HTTP MCP with `--mcp-transport=mcp` / `auto` fallback. |
| `launch mcp va` | Start the Vanessa test-manager for research, authoring, and VA client MCP tools. | `kind=vanessa_test_client`; runner adds `/TESTMANAGER`, `/DisableUnsafeActionProtection`, `/Execute <vanessa-automation.epf>`; WS mode uses `mcpMode=ws;...;VAParams=<runtime params>` in `/C`, local MCP fallback uses `runMcp...;VAParams=<runtime params>`, and neither mode uses `StartFeaturePlayer`. |
| `test yaxunit ...` | Run YAxUnit tests. | `kind=yaxunit_runner` in WS; not an interactive UI session. |
| `test va` | Run Vanessa feature scenarios. | `kind=vanessa_test_client`, but payload is `StartFeaturePlayer;VAParams=...`; this is scenario execution, not the manager research mode. |

For `launch mcp va --mcp-transport=ws`, wait for the `kind=vanessa_test_client`
session in `v8-client-session-manager`: normal startup/tool registration takes 10-90 seconds.
Poll `session_list` every 5-10 seconds and use 120 seconds as a diagnostic limit. After the
VA tools appear, call `connect_test_client` with the required `profileName` argument; then the VA
client MCP tools become available. After the investigation or on error, call `close_test_client`
with the same `profileName` (or without it to close the current connected profile) so extra 1C
test-client sessions are not kept alive.

## Guardrails

- Do not delete or recreate an infobase, workspace, temp directory, or generated state unless the user explicitly asks or the command itself is the documented recovery path.
- Do not invent raw `1cv8`, `ibcmd`, or `1cedtcli` flags; prefer the `v8-runner` command surface.
- Check `git status` before `dump` when the result may overwrite or mix with existing source changes.
- Preserve failed test artifacts under `workPath/temp/<runner-id>/runs/<run-id>/` for diagnosis instead of cleaning them immediately.
- Report missing local 1C utilities as environment/setup issues, not as project source failures.
- Keep final answers concrete: command run, result, relevant artifact path, and any follow-up command.

## Output Discipline

When reporting results, distinguish:

- project source failures;
- v8-runner command/config failures;
- local 1C platform, EDT, IBCMD, or tool discovery failures;
- test failures and their artifact paths.
