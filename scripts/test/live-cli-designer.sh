#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

DEFAULT_DESIGNER_CONFIG_PATH="$ROOT_DIR/scripts/test/live-cli-designer.fixture.yaml"
DESIGNER_CONFIG_PATH="${V8TR_DESIGNER_REAL_CONFIG:-$DEFAULT_DESIGNER_CONFIG_PATH}"
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

strip_shell_quotes() {
    local value="$1"
    value="${value#"${value%%[![:space:]]*}"}"
    value="${value%"${value##*[![:space:]]}"}"
    value="${value#\'}"
    value="${value%\'}"
    value="${value#\"}"
    value="${value%\"}"
    printf '%s\n' "$value"
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

config_matches() {
    local pattern="$1"
    local path="$2"

    if command -v rg >/dev/null 2>&1; then
        rg -q "$pattern" "$path"
        return $?
    fi

    grep -Eq "$pattern" "$path"
}

detect_platform_path() {
    local configured_path="${1:-}"

    if [[ -n "$configured_path" && "$configured_path" != "AUTO_PLATFORM" && -e "$configured_path" ]]; then
        printf '%s\n' "$configured_path"
        return 0
    fi

    if command -v 1cv8 >/dev/null 2>&1; then
        command -v 1cv8
        return 0
    fi

    python3 - <<'PY'
import pathlib

candidates = sorted(pathlib.Path("/opt/1cv8").glob("**/1cv8"), reverse=True)
for candidate in candidates:
    if candidate.is_file():
        print(candidate)
        raise SystemExit(0)

raise SystemExit(1)
PY
}

extract_source_sets() {
    python3 - "$DESIGNER_CONFIG_PATH" <<'PY'
import pathlib
import re
import sys


def clean(value: str) -> str:
    return value.strip().strip("'\"")


config_path = pathlib.Path(sys.argv[1])
lines = config_path.read_text(encoding="utf-8").splitlines()
items = []
current = None
in_block = False

for line in lines:
    if not in_block:
        if re.match(r"^\s*source-set:\s*$", line):
            in_block = True
        continue

    if re.match(r"^\S", line):
        break

    name_match = re.match(r"^\s*-\s*name:\s*(.+?)\s*$", line)
    if name_match:
        if current is not None:
            items.append(current)
        current = {"name": clean(name_match.group(1))}
        continue

    field_match = re.match(r"^\s+(purpose|path):\s*(.+?)\s*$", line)
    if field_match and current is not None:
        current[field_match.group(1)] = clean(field_match.group(2))

if current is not None:
    items.append(current)

for item in items:
    print(
        "\t".join(
            [
                item.get("name", ""),
                item.get("purpose", ""),
                item.get("path", ""),
            ]
        )
    )
PY
}

extract_artifact_root_name() {
    local relative_path="$1"
    python3 - "$WORK_BASE_PATH" "$relative_path" <<'PY'
import pathlib
import sys

base_path = pathlib.Path(sys.argv[1])
relative_path = pathlib.Path(sys.argv[2])
root = base_path / relative_path

if not root.is_dir():
    raise SystemExit(f"source-set path is not a directory: {root}")

names = sorted(
    path.stem
    for path in root.glob("*.xml")
    if path.is_file() and path.name not in {"Configuration.xml", "ConfigDumpInfo.xml"}
)

if len(names) != 1:
    raise SystemExit(
        f"expected exactly one root xml artifact in {root}, found {len(names)}"
    )

print(names[0])
PY
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

print_test_step() {
    local title="$1"
    echo
    echo "============================================================"
    echo "TEST: $title"
    echo "============================================================"
}

materialize_live_config() {
    local source_config="$1"
    local target_config="$2"
    local platform_path="$3"
    local output_root="$4"
    local work_base_path="$5"

    python3 - "$source_config" "$target_config" "$ROOT_DIR" "$output_root" "$work_base_path" "$platform_path" <<'PY'
import pathlib
import re
import sys

source = pathlib.Path(sys.argv[1])
target = pathlib.Path(sys.argv[2])
root_dir = pathlib.Path(sys.argv[3])
output_root = pathlib.Path(sys.argv[4])
work_base_path = pathlib.Path(sys.argv[5])
platform_path = sys.argv[6]
text = source.read_text(encoding="utf-8")

replacements = {
    "__ROOT_DIR__": root_dir.as_posix(),
    "__OUTPUT_ROOT__": output_root.as_posix(),
    "AUTO_PLATFORM": platform_path,
}

for needle, replacement in replacements.items():
    text = text.replace(needle, replacement)

if re.search(r"^\s*basePath:\s*.*$", text, re.MULTILINE):
    text = re.sub(
        r"^\s*basePath:\s*.*$",
        f"basePath: {work_base_path.as_posix()}",
        text,
        count=1,
        flags=re.MULTILINE,
    )
else:
    raise SystemExit("live config must define basePath")

target.write_text(text, encoding="utf-8")
PY
}

LAUNCH_PID=""
cleanup() {
    if [[ -n "$LAUNCH_PID" ]]; then
        kill "$LAUNCH_PID" >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

if [[ ! -f "$DESIGNER_CONFIG_PATH" ]]; then
    die "Live Designer config not found: $DESIGNER_CONFIG_PATH"
fi

if ! config_matches "^format:\\s*DESIGNER\\s*$" "$DESIGNER_CONFIG_PATH"; then
    die "Live Designer config must contain 'format: DESIGNER': $DESIGNER_CONFIG_PATH"
fi

if ! config_matches "^builder:\\s*DESIGNER\\s*$" "$DESIGNER_CONFIG_PATH"; then
    die "Live Designer config must contain 'builder: DESIGNER': $DESIGNER_CONFIG_PATH"
fi

if ! config_matches "^connection:\\s*['\"]?(File=|/F[[:space:]]+)" "$DESIGNER_CONFIG_PATH"; then
    die "Live Designer config must use a file-based connection ('File=...' or raw '/F ...'): $DESIGNER_CONFIG_PATH"
fi

declare -A SOURCE_SET_NAME_BY_PURPOSE=()
declare -A SOURCE_SET_PATH_BY_PURPOSE=()
required_purposes=(
    CONFIGURATION
    EXTENSION
    EXTERNAL_DATA_PROCESSORS
    EXTERNAL_REPORTS
)

while IFS=$'\t' read -r source_set_name source_set_purpose source_set_path; do
    source_set_name="$(strip_shell_quotes "$source_set_name")"
    source_set_purpose="$(strip_shell_quotes "$source_set_purpose")"
    source_set_path="$(strip_shell_quotes "$source_set_path")"

    if [[ -z "$source_set_name" || -z "$source_set_purpose" || -z "$source_set_path" ]]; then
        die "Each source-set must define name, purpose, and path: $DESIGNER_CONFIG_PATH"
    fi

    if [[ -n "${SOURCE_SET_NAME_BY_PURPOSE[$source_set_purpose]:-}" ]]; then
        die "Live Designer config must define only one source-set with purpose '$source_set_purpose': $DESIGNER_CONFIG_PATH"
    fi

    SOURCE_SET_NAME_BY_PURPOSE["$source_set_purpose"]="$source_set_name"
    SOURCE_SET_PATH_BY_PURPOSE["$source_set_purpose"]="$source_set_path"
done < <(extract_source_sets)

for purpose in "${required_purposes[@]}"; do
    if [[ -z "${SOURCE_SET_NAME_BY_PURPOSE[$purpose]:-}" ]]; then
        die "Live Designer config must declare a source-set with purpose '$purpose': $DESIGNER_CONFIG_PATH"
    fi
done

base_path="$(extract_yaml_scalar "basePath")"
if [[ -z "$base_path" ]]; then
    die "Live Designer config must define basePath: $DESIGNER_CONFIG_PATH"
fi

resolved_base_path="${base_path//__ROOT_DIR__/$ROOT_DIR}"
resolved_base_path="${resolved_base_path//__OUTPUT_ROOT__/$OUTPUT_ROOT}"

fixture_base_real="$(realpath "$FIXTURE_BASE_PATH")"
config_base_real="$(realpath "$resolved_base_path" 2>/dev/null || true)"
if [[ "$config_base_real" != "$fixture_base_real" ]]; then
    die "Live Designer config must point basePath to '$fixture_base_real', got '${base_path}'"
fi

platform_path="$(extract_platform_path)"
if [[ -z "$platform_path" ]]; then
    die "Live Designer config must define tools.platform.path: $DESIGNER_CONFIG_PATH"
fi

platform_path="$(detect_platform_path "$platform_path")" || die "Unable to detect 1cv8 platform binary automatically"

CONFIGURATION_SOURCE_SET_NAME="${SOURCE_SET_NAME_BY_PURPOSE[CONFIGURATION]}"
CONFIGURATION_SOURCE_SET_PATH="${SOURCE_SET_PATH_BY_PURPOSE[CONFIGURATION]}"
EXTENSION_SOURCE_SET_NAME="${SOURCE_SET_NAME_BY_PURPOSE[EXTENSION]}"
EXTENSION_SOURCE_SET_PATH="${SOURCE_SET_PATH_BY_PURPOSE[EXTENSION]}"
EXTERNAL_PROCESSOR_SOURCE_SET_NAME="${SOURCE_SET_NAME_BY_PURPOSE[EXTERNAL_DATA_PROCESSORS]}"
EXTERNAL_PROCESSOR_SOURCE_SET_PATH="${SOURCE_SET_PATH_BY_PURPOSE[EXTERNAL_DATA_PROCESSORS]}"
EXTERNAL_REPORT_SOURCE_SET_NAME="${SOURCE_SET_NAME_BY_PURPOSE[EXTERNAL_REPORTS]}"
EXTERNAL_REPORT_SOURCE_SET_PATH="${SOURCE_SET_PATH_BY_PURPOSE[EXTERNAL_REPORTS]}"

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
materialize_live_config "$DESIGNER_CONFIG_PATH" "$WORK_CONFIG_PATH" "$platform_path" "$OUTPUT_ROOT" "$WORK_BASE_PATH"

DESIGNER_CONFIG_PATH="$WORK_CONFIG_PATH"

for purpose in "${required_purposes[@]}"; do
    source_set_path="${SOURCE_SET_PATH_BY_PURPOSE[$purpose]}"
    if [[ ! -d "$WORK_BASE_PATH/$source_set_path" ]]; then
        die "Configured source-set path does not exist under fixture basePath: $source_set_path"
    fi
done

EXTERNAL_PROCESSOR_ARTIFACT_NAME="$(extract_artifact_root_name "$EXTERNAL_PROCESSOR_SOURCE_SET_PATH")"
EXTERNAL_REPORT_ARTIFACT_NAME="$(extract_artifact_root_name "$EXTERNAL_REPORT_SOURCE_SET_PATH")"

print_test_step "init infobase"
run_cli init
assert_file_exists "$(extract_connection_file_path)/1Cv8.1CD"

build_json="$OUTPUT_ROOT/json/build.json"
print_test_step "build full rebuild"
run_cli_json_to_file "$build_json" build --full-rebuild
assert_json_step_ok "$build_json" "$CONFIGURATION_SOURCE_SET_NAME"
assert_json_step_ok "$build_json" "$EXTENSION_SOURCE_SET_NAME"

print_test_step "dump full configuration"
rm -f \
    "$WORK_BASE_PATH/$CONFIGURATION_SOURCE_SET_PATH/Configuration.xml" \
    "$WORK_BASE_PATH/$CONFIGURATION_SOURCE_SET_PATH/ConfigDumpInfo.xml"
run_cli dump --mode full --source-set "$CONFIGURATION_SOURCE_SET_NAME"
assert_file_exists "$WORK_BASE_PATH/$CONFIGURATION_SOURCE_SET_PATH/Configuration.xml"
assert_file_exists "$WORK_BASE_PATH/$CONFIGURATION_SOURCE_SET_PATH/ConfigDumpInfo.xml"
snapshot_dir "$WORK_BASE_PATH/$CONFIGURATION_SOURCE_SET_PATH" "$OUTPUT_ROOT/dump/full"

print_test_step "dump incremental configuration"
rm -f "$WORK_BASE_PATH/$CONFIGURATION_SOURCE_SET_PATH/ConfigDumpInfo.xml"
run_cli dump --mode incremental --source-set "$CONFIGURATION_SOURCE_SET_NAME"
assert_dir_exists "$WORK_BASE_PATH/$CONFIGURATION_SOURCE_SET_PATH"
assert_file_exists "$WORK_BASE_PATH/$CONFIGURATION_SOURCE_SET_PATH/ConfigDumpInfo.xml"
snapshot_dir "$WORK_BASE_PATH/$CONFIGURATION_SOURCE_SET_PATH" "$OUTPUT_ROOT/dump/incremental"

print_test_step "dump partial configuration"
rm -f "$WORK_BASE_PATH/$CONFIGURATION_SOURCE_SET_PATH/Catalogs/Справочник1.xml"
run_cli dump --mode partial --source-set "$CONFIGURATION_SOURCE_SET_NAME" --object Catalog.Справочник1
assert_file_exists "$WORK_BASE_PATH/$CONFIGURATION_SOURCE_SET_PATH/Catalogs/Справочник1.xml"
snapshot_dir "$WORK_BASE_PATH/$CONFIGURATION_SOURCE_SET_PATH" "$OUTPUT_ROOT/dump/partial"

print_test_step "syntax designer config"
run_cli syntax designer-config --all-extensions
print_test_step "syntax designer modules"
run_cli syntax designer-modules --server --all-extensions

print_test_step "artifacts configuration cf"
run_cli artifacts --output "$OUTPUT_ROOT/artifacts/configuration.cf"
assert_file_nonempty "$OUTPUT_ROOT/artifacts/configuration.cf"

print_test_step "artifacts extension cfe"
run_cli artifacts \
    --output "$OUTPUT_ROOT/artifacts/extension.cfe" \
    --source-set "$EXTENSION_SOURCE_SET_NAME" \
    --extension "$EXTENSION_SOURCE_SET_NAME"
assert_file_nonempty "$OUTPUT_ROOT/artifacts/extension.cfe"

print_test_step "artifacts external processor epf"
run_cli artifacts \
    --output "$OUTPUT_ROOT/artifacts/external-processor" \
    --source-set "$EXTERNAL_PROCESSOR_SOURCE_SET_NAME"
assert_file_nonempty "$OUTPUT_ROOT/artifacts/external-processor/${EXTERNAL_PROCESSOR_ARTIFACT_NAME}.epf"

print_test_step "artifacts external report erf"
run_cli artifacts \
    --output "$OUTPUT_ROOT/artifacts/external-report" \
    --source-set "$EXTERNAL_REPORT_SOURCE_SET_NAME"
assert_file_nonempty "$OUTPUT_ROOT/artifacts/external-report/${EXTERNAL_REPORT_ARTIFACT_NAME}.erf"

launch_json="$OUTPUT_ROOT/json/launch-designer.json"
print_test_step "launch designer"
run_cli_json_to_file "$launch_json" launch --mode designer
LAUNCH_PID="$(extract_launch_pid "$launch_json")"
sleep 1
kill -0 "$LAUNCH_PID" >/dev/null 2>&1 || die "Designer process is not running after launch: pid $LAUNCH_PID"

echo
echo "Live CLI Designer smoke completed successfully."
