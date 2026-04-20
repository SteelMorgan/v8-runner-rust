#!/bin/sh

set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
BASE_CONFIG="$ROOT_DIR/scripts/test/live-cli-designer.fixture.yaml"
FIXTURE_BASE="$ROOT_DIR/tests/fixtures/designer"
LIVE_SCRIPT="$ROOT_DIR/scripts/test/live-cli-ibcmd.sh"
OUTPUT_ROOT="$ROOT_DIR/target/manual-tests/live-cli-ibcmd"
WORK_BASE_PATH="$OUTPUT_ROOT/workspace/basePath"
WORK_CONFIG_PATH="$ROOT_DIR/target/manual-tests/live-cli-ibcmd.yaml"
BIN_PATH="${V8TR_BIN:-$ROOT_DIR/target/debug/v8-runner}"
PLATFORM_PATH="${V8TR_PLATFORM_PATH:-}"

die() {
    echo "$*" >&2
    exit 2
}

stage() {
    echo
    echo "==> UAT IBCMD: $1"
}

detect_platform_path() {
    if [ -n "$PLATFORM_PATH" ]; then
        printf '%s\n' "$PLATFORM_PATH"
        return 0
    fi

    if command -v ibcmd >/dev/null 2>&1; then
        command -v ibcmd
        return 0
    fi

    if command -v 1cv8 >/dev/null 2>&1; then
        command -v 1cv8
        return 0
    fi

    if command -v 1cv8.exe >/dev/null 2>&1; then
        command -v 1cv8.exe
        return 0
    fi

    python3 - <<'PY'
import os
import pathlib

candidate_roots = [
    pathlib.Path("/opt/1cv8"),
    pathlib.Path("/c/Program Files/1cv8"),
    pathlib.Path("/c/Program Files (x86)/1cv8"),
    pathlib.Path("C:/Program Files/1cv8"),
    pathlib.Path("C:/Program Files (x86)/1cv8"),
]

for env_name in ("ProgramFiles", "ProgramFiles(x86)"):
    env_value = os.environ.get(env_name)
    if env_value:
        candidate_roots.append(pathlib.Path(env_value) / "1cv8")

patterns = (
    "**/ibcmd",
    "**/ibcmd.exe",
    "**/1cv8",
    "**/1cv8.exe",
)

for root in candidate_roots:
    if not root.exists():
        continue
    for pattern in patterns:
        for path in sorted(root.glob(pattern), reverse=True):
            if path.is_file():
                print(path)
                raise SystemExit(0)

raise SystemExit(1)
PY
}

materialize_ibcmd_config() {
    platform_path="$1"

    python3 - "$BASE_CONFIG" "$WORK_CONFIG_PATH" "$ROOT_DIR" "$OUTPUT_ROOT" "$platform_path" <<'PY'
import pathlib
import re
import sys

source = pathlib.Path(sys.argv[1])
target = pathlib.Path(sys.argv[2])
root_dir = pathlib.Path(sys.argv[3])
output_root = pathlib.Path(sys.argv[4])
platform_path = pathlib.Path(sys.argv[5])

text = source.read_text(encoding="utf-8")
replacements = {
    "__ROOT_DIR__": root_dir.as_posix(),
    "__OUTPUT_ROOT__": output_root.as_posix(),
    "AUTO_PLATFORM": platform_path.as_posix(),
    "__VANESSA_EPF__": (root_dir / "tests/fixtures/vanessa-automation-single.epf").as_posix(),
    "__VANESSA_PARAMS_TEMPLATE__": (root_dir / "scripts/test/live-cli-designer.va-params.json").as_posix(),
    "__VANESSA_FEATURE_PATH__": (root_dir / "scripts/test/features/live-cli-designer").as_posix(),
}

for old, new in replacements.items():
    text = text.replace(old, new)

text = re.sub(
    r"^builder:\s*DESIGNER\s*$",
    "builder: IBCMD",
    text,
    count=1,
    flags=re.MULTILINE,
)
text = re.sub(
    r"\n  - name: external-processor\n    purpose: EXTERNAL_DATA_PROCESSORS\n    path: external/processor",
    "",
    text,
)
text = re.sub(
    r"\n  - name: external-report\n    purpose: EXTERNAL_REPORTS\n    path: external/report",
    "",
    text,
)

target.write_text(text, encoding="utf-8")
PY
}

[ -f "$BASE_CONFIG" ] || die "Base fixture config not found: $BASE_CONFIG"
[ -d "$FIXTURE_BASE" ] || die "Fixture source directory not found: $FIXTURE_BASE"

if ! command -v python3 >/dev/null 2>&1; then
    die "python3 is required for fixture config materialization"
fi

platform_path="$(detect_platform_path)" || die "1C platform tools were not found. Put ibcmd/1cv8 in PATH or set optional V8TR_PLATFORM_PATH."

stage "prepare IBCMD fixture config"
rm -rf "$OUTPUT_ROOT"
mkdir -p "$OUTPUT_ROOT"
materialize_ibcmd_config "$platform_path"

stage "build cargo binary"
(cd "$ROOT_DIR" && cargo build --locked --bin v8-runner)

stage "run real live-cli-ibcmd scenario"
V8TR_BIN="$BIN_PATH" V8TR_LIVE_CLI_OUTPUT_ROOT="$OUTPUT_ROOT" V8TR_IBCMD_REAL_CONFIG="$WORK_CONFIG_PATH" bash "$LIVE_SCRIPT"

echo
echo "UAT CLI IBCMD live scenario completed successfully."
