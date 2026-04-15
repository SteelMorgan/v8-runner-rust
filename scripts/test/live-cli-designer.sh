#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

DESIGNER_CONFIG_PATH="${V8TR_DESIGNER_REAL_CONFIG:-}"
BIN_PATH="${V8TR_BIN:-$ROOT_DIR/target/debug/v8-test-runner}"
OUTPUT_ROOT="$ROOT_DIR/target/manual-tests/live-cli-designer"
FIXTURE_BASE_PATH="$ROOT_DIR/tests/fixtures/designer"

die() {
    echo "$*" >&2
    exit 2
}

assert_file_exists() {
    local path="$1"
    if [[ ! -f "$path" ]]; then
        die "Expected file was not produced: $path"
    fi
}

assert_file_nonempty() {
    local path="$1"
    if [[ ! -s "$path" ]]; then
        die "Expected non-empty file was not produced: $path"
    fi
}

assert_dir_exists() {
    local path="$1"
    if [[ ! -d "$path" ]]; then
        die "Expected directory was not produced: $path"
    fi
}

snapshot_dir() {
    local source_dir="$1"
    local target_dir="$2"
    rm -rf "$target_dir"
    mkdir -p "$target_dir"
    cp -R "$source_dir/." "$target_dir/"
}

trim_yaml_scalar() {
    sed -e "s/^[[:space:]]*//" -e "s/[[:space:]]*$//" -e "s/^['\"]//" -e "s/['\"]$//"
}

extract_yaml_scalar() {
    local key="$1"
    awk -v key="$key" '
        $0 ~ "^[[:space:]]*" key ":[[:space:]]*" {
            sub("^[[:space:]]*" key ":[[:space:]]*", "", $0)
            print
            exit
        }
    ' "$DESIGNER_CONFIG_PATH" | trim_yaml_scalar
}

extract_platform_path() {
    awk '
        /^[[:space:]]*tools:[[:space:]]*$/ { in_tools=1; next }
        in_tools && /^[^[:space:]]/ { in_tools=0 }
        in_tools && /^[[:space:]]{2}platform:[[:space:]]*$/ { in_platform=1; next }
        in_platform && /^[[:space:]]{2}[^[:space:]]/ { in_platform=0 }
        in_platform && /^[[:space:]]{4}path:[[:space:]]*/ {
            sub(/^[[:space:]]{4}path:[[:space:]]*/, "", $0)
            print
            exit
        }
    ' "$DESIGNER_CONFIG_PATH" | trim_yaml_scalar
}

assert_json_step_ok() {
    local json_path="$1"
    local source_set="$2"
    python3 - "$json_path" "$source_set" <<'PY'
import json
import sys

json_path, source_set = sys.argv[1], sys.argv[2]
with open(json_path, "r", encoding="utf-8") as fh:
    payload = json.load(fh)

steps = payload.get("data", {}).get("steps", [])
for step in steps:
    if step.get("source_set") == source_set:
        if step.get("ok") is True:
            raise SystemExit(0)
        raise SystemExit(f"build step for '{source_set}' is not successful: {step}")

raise SystemExit(f"build output does not contain step for '{source_set}'")
PY
}

extract_connection_file_path() {
    python3 - "$DESIGNER_CONFIG_PATH" <<'PY'
import pathlib
import re
import shlex
import sys

config_path = pathlib.Path(sys.argv[1])
text = config_path.read_text(encoding="utf-8")

match = re.search(r"^connection:\s*(.+)$", text, re.MULTILINE)
if not match:
    raise SystemExit("connection must use File=... or raw /F ...")

connection = match.group(1).strip().strip("'\"")
if connection.startswith("/") or connection.startswith("-"):
    parts = shlex.split(connection)
    for index, part in enumerate(parts):
        if part.lower() in ("/f", "-f") and index + 1 < len(parts):
            print(pathlib.Path(parts[index + 1]).expanduser())
            raise SystemExit(0)
    raise SystemExit("connection must use File=... or raw /F ...")

for part in connection.split(";"):
    part = part.strip()
    if part.lower().startswith("file="):
        print(pathlib.Path(part[5:]).expanduser())
        raise SystemExit(0)

raise SystemExit("connection must use File=... or raw /F ...")
PY
}

extract_launch_pid() {
    local json_path="$1"
    python3 - "$json_path" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as fh:
    payload = json.load(fh)

pid = payload.get("data", {}).get("pid")
if not isinstance(pid, int) or pid <= 0:
    raise SystemExit("launch output does not contain a valid pid")
print(pid)
PY
}

run_cli() {
    echo
    echo "==> $*"
    "$BIN_PATH" --config "$DESIGNER_CONFIG_PATH" "$@"
}

run_cli_json_to_file() {
    local json_path="$1"
    shift
    echo
    echo "==> --output json $*"
    "$BIN_PATH" --config "$DESIGNER_CONFIG_PATH" --output json "$@" | tee "$json_path"
}

LAUNCH_PID=""
cleanup() {
    if [[ -n "$LAUNCH_PID" ]]; then
        kill "$LAUNCH_PID" >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

if [[ -z "$DESIGNER_CONFIG_PATH" ]]; then
    echo "SKIPPED: V8TR_DESIGNER_REAL_CONFIG is not set."
    echo "Set a dedicated DESIGNER live config for tests/fixtures/designer (see examples/live-cli-designer.fixture.yaml)."
    exit 0
fi

if [[ ! -f "$DESIGNER_CONFIG_PATH" ]]; then
    die "Live Designer config not found: $DESIGNER_CONFIG_PATH"
fi

if ! rg -q "^format:\\s*DESIGNER\\s*$" "$DESIGNER_CONFIG_PATH"; then
    die "Live Designer config must contain 'format: DESIGNER': $DESIGNER_CONFIG_PATH"
fi

if ! rg -q "^builder:\\s*DESIGNER\\s*$" "$DESIGNER_CONFIG_PATH"; then
    die "Live Designer config must contain 'builder: DESIGNER': $DESIGNER_CONFIG_PATH"
fi

if ! rg -q "^connection:\\s*['\"]?(File=|/F[[:space:]]+)" "$DESIGNER_CONFIG_PATH"; then
    die "Live Designer config must use a file-based connection ('File=...' or raw '/F ...'): $DESIGNER_CONFIG_PATH"
fi

for source_set in configuration "Расширение1" external-processor external-report; do
    if ! rg -q "name:\\s*${source_set}\\s*$" "$DESIGNER_CONFIG_PATH"; then
        die "Live Designer config must declare source-set '${source_set}': $DESIGNER_CONFIG_PATH"
    fi
done

base_path="$(extract_yaml_scalar "basePath")"
if [[ -z "$base_path" ]]; then
    die "Live Designer config must define basePath: $DESIGNER_CONFIG_PATH"
fi

fixture_base_real="$(realpath "$FIXTURE_BASE_PATH")"
config_base_real="$(realpath "$base_path" 2>/dev/null || true)"
if [[ "$config_base_real" != "$fixture_base_real" ]]; then
    die "Live Designer config must point basePath to '$fixture_base_real', got '${base_path}'"
fi

platform_path="$(extract_platform_path)"
if [[ -z "$platform_path" ]]; then
    die "Live Designer config must define tools.platform.path: $DESIGNER_CONFIG_PATH"
fi

if [[ ! -e "$platform_path" ]]; then
    die "tools.platform.path does not exist: $platform_path"
fi

WORK_BASE_PATH="$OUTPUT_ROOT/workspace/basePath"
WORK_CONFIG_PATH="$OUTPUT_ROOT/json/live-designer.config.yaml"

if [[ ! -x "$BIN_PATH" ]]; then
    echo "Building v8-test-runner binary..." >&2
    (cd "$ROOT_DIR" && cargo build --locked --bin v8-test-runner >/dev/null)
fi

rm -rf "$OUTPUT_ROOT"
mkdir -p \
    "$WORK_BASE_PATH" \
    "$OUTPUT_ROOT/dump/full" \
    "$OUTPUT_ROOT/dump/incremental" \
    "$OUTPUT_ROOT/dump/partial" \
    "$OUTPUT_ROOT/artifacts/external-processor" \
    "$OUTPUT_ROOT/artifacts/external-report" \
    "$OUTPUT_ROOT/json"

cp -R "$FIXTURE_BASE_PATH/." "$WORK_BASE_PATH/"
python3 - "$DESIGNER_CONFIG_PATH" "$WORK_CONFIG_PATH" "$WORK_BASE_PATH" <<'PY'
import pathlib
import re
import sys

source = pathlib.Path(sys.argv[1])
target = pathlib.Path(sys.argv[2])
base_path = pathlib.Path(sys.argv[3])
text = source.read_text(encoding="utf-8")
replacement = f"basePath: {base_path.as_posix()}"

if re.search(r"^\s*basePath:\s*.*$", text, re.MULTILINE):
    text = re.sub(r"^\s*basePath:\s*.*$", replacement, text, count=1, flags=re.MULTILINE)
else:
    raise SystemExit("live config must define basePath")

target.write_text(text, encoding="utf-8")
PY

DESIGNER_CONFIG_PATH="$WORK_CONFIG_PATH"

run_cli init
assert_file_exists "$(extract_connection_file_path)/1Cv8.1CD"

build_json="$OUTPUT_ROOT/json/build.json"
run_cli_json_to_file "$build_json" build --full-rebuild
assert_json_step_ok "$build_json" "configuration"
assert_json_step_ok "$build_json" "Расширение1"

rm -f \
    "$WORK_BASE_PATH/configuration/Configuration.xml" \
    "$WORK_BASE_PATH/configuration/ConfigDumpInfo.xml"
run_cli dump --mode full --source-set configuration
assert_file_exists "$WORK_BASE_PATH/configuration/Configuration.xml"
assert_file_exists "$WORK_BASE_PATH/configuration/ConfigDumpInfo.xml"
snapshot_dir "$WORK_BASE_PATH/configuration" "$OUTPUT_ROOT/dump/full"

rm -f "$WORK_BASE_PATH/configuration/ConfigDumpInfo.xml"
run_cli dump --mode incremental --source-set configuration
assert_dir_exists "$WORK_BASE_PATH/configuration"
assert_file_exists "$WORK_BASE_PATH/configuration/ConfigDumpInfo.xml"
snapshot_dir "$WORK_BASE_PATH/configuration" "$OUTPUT_ROOT/dump/incremental"

rm -f "$WORK_BASE_PATH/configuration/Catalogs/Справочник1.xml"
run_cli dump --mode partial --source-set configuration --object Catalog.Справочник1
assert_file_exists "$WORK_BASE_PATH/configuration/Catalogs/Справочник1.xml"
snapshot_dir "$WORK_BASE_PATH/configuration" "$OUTPUT_ROOT/dump/partial"

run_cli syntax designer-config --all-extensions
run_cli syntax designer-modules --server --all-extensions

run_cli artifacts --output "$OUTPUT_ROOT/artifacts/configuration.cf"
assert_file_nonempty "$OUTPUT_ROOT/artifacts/configuration.cf"

run_cli artifacts \
    --output "$OUTPUT_ROOT/artifacts/extension.cfe" \
    --source-set "Расширение1" \
    --extension "Расширение1"
assert_file_nonempty "$OUTPUT_ROOT/artifacts/extension.cfe"

run_cli artifacts \
    --output "$OUTPUT_ROOT/artifacts/external-processor" \
    --source-set external-processor
assert_file_nonempty "$OUTPUT_ROOT/artifacts/external-processor/ВнешняяОбработка1.epf"

run_cli artifacts \
    --output "$OUTPUT_ROOT/artifacts/external-report" \
    --source-set external-report
assert_file_nonempty "$OUTPUT_ROOT/artifacts/external-report/ВнешнийОтчет1.erf"

launch_json="$OUTPUT_ROOT/json/launch-designer.json"
run_cli_json_to_file "$launch_json" launch --mode designer
LAUNCH_PID="$(extract_launch_pid "$launch_json")"
sleep 1
kill -0 "$LAUNCH_PID" >/dev/null 2>&1 || die "Designer process is not running after launch: pid $LAUNCH_PID"

echo
echo "Live CLI Designer smoke completed successfully."
