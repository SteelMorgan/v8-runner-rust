#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CODEX_BIN="${CODEX_BIN:-codex}"

COUNT=""
PROMPT=""
PROMPT_FILE=""
WORK_DIR="$ROOT_DIR"
OUTPUT_DIR=""
SLEEP_SECONDS="0"
CONTINUE_ON_ERROR=0

declare -a CODEX_ARGS=()

usage() {
  cat <<'EOF'
Usage: scripts/codex-loop.sh --count N (--prompt TEXT | --prompt-file FILE) [options] [-- codex exec options]
       scripts/codex-loop.sh --count N [options] [-- codex exec options] < prompt.txt

Runs `codex exec` in a loop with the same prompt.

Required:
  -n, --count N          Number of Codex runs.

Prompt input:
  -p, --prompt TEXT      Prompt text.
  -f, --prompt-file FILE Read prompt from file.
  stdin                  Used when neither --prompt nor --prompt-file is provided.

Options:
  -C, --cd DIR           Working directory passed to `codex exec -C`.
                          Defaults to the repository root.
  -o, --output-dir DIR   Write each run's final Codex message to DIR/run-NNN.md.
  --sleep SECONDS        Sleep between runs. Defaults to 0.
  --continue-on-error    Keep running after a failed Codex invocation.
  -h, --help             Show this help.

Everything after `--` is forwarded to `codex exec`.

Examples:
  scripts/codex-loop.sh --count 3 --prompt "Summarize current git status" -- --sandbox read-only
  scripts/codex-loop.sh -n 5 -f prompt.md -o var/codex-loop -- --model gpt-5.4 --sandbox workspace-write
  printf '%s\n' "Run repo checks and report failures" | scripts/codex-loop.sh -n 2 -- --full-auto
EOF
}

log() {
  printf '[codex-loop] %s\n' "$*"
}

die() {
  printf '[codex-loop] %s\n' "$*" >&2
  exit 1
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    die "Missing required command: $1"
  fi
}

is_positive_integer() {
  [[ "$1" =~ ^[1-9][0-9]*$ ]]
}

is_non_negative_integer() {
  [[ "$1" =~ ^[0-9]+$ ]]
}

read_stdin_prompt() {
  if [[ -t 0 ]]; then
    die "Prompt is required. Use --prompt, --prompt-file, or pipe prompt text to stdin."
  fi

  PROMPT="$(cat)"
}

parse_args() {
  while (($#)); do
    case "$1" in
      -n|--count)
        [[ $# -ge 2 ]] || die "$1 requires a value"
        COUNT="$2"
        shift 2
        ;;
      -p|--prompt)
        [[ $# -ge 2 ]] || die "$1 requires a value"
        PROMPT="$2"
        shift 2
        ;;
      -f|--prompt-file)
        [[ $# -ge 2 ]] || die "$1 requires a value"
        PROMPT_FILE="$2"
        shift 2
        ;;
      -C|--cd)
        [[ $# -ge 2 ]] || die "$1 requires a value"
        WORK_DIR="$2"
        shift 2
        ;;
      -o|--output-dir)
        [[ $# -ge 2 ]] || die "$1 requires a value"
        OUTPUT_DIR="$2"
        shift 2
        ;;
      --sleep)
        [[ $# -ge 2 ]] || die "$1 requires a value"
        SLEEP_SECONDS="$2"
        shift 2
        ;;
      --continue-on-error)
        CONTINUE_ON_ERROR=1
        shift
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      --)
        shift
        CODEX_ARGS=("$@")
        break
        ;;
      *)
        die "Unknown argument: $1"
        ;;
    esac
  done
}

validate_config() {
  [[ -n "$COUNT" ]] || die "--count is required"
  is_positive_integer "$COUNT" || die "--count must be a positive integer"
  is_non_negative_integer "$SLEEP_SECONDS" || die "--sleep must be a non-negative integer"

  if [[ -n "$PROMPT" && -n "$PROMPT_FILE" ]]; then
    die "Use only one prompt source: --prompt or --prompt-file"
  fi

  if [[ -n "$PROMPT_FILE" ]]; then
    [[ -f "$PROMPT_FILE" ]] || die "Prompt file does not exist: $PROMPT_FILE"
    PROMPT="$(<"$PROMPT_FILE")"
  elif [[ -z "$PROMPT" ]]; then
    read_stdin_prompt
  fi

  [[ -n "$PROMPT" ]] || die "Prompt cannot be empty"
  [[ -d "$WORK_DIR" ]] || die "Working directory does not exist: $WORK_DIR"

  if [[ -n "$OUTPUT_DIR" ]]; then
    mkdir -p "$OUTPUT_DIR"
  fi
}

run_codex_once() {
  local run_number="$1"
  local -a cmd

  cmd=("$CODEX_BIN" exec -C "$WORK_DIR")

  if [[ -n "$OUTPUT_DIR" ]]; then
    printf -v output_file '%s/run-%03d.md' "$OUTPUT_DIR" "$run_number"
    cmd+=("--output-last-message" "$output_file")
  fi

  cmd+=("${CODEX_ARGS[@]}" -)

  log "Run $run_number/$COUNT"
  printf '%s\n' "$PROMPT" | "${cmd[@]}"
}

main() {
  parse_args "$@"
  validate_config
  require_cmd "$CODEX_BIN"

  local failures=0
  local run_number

  for ((run_number = 1; run_number <= COUNT; run_number++)); do
    if ! run_codex_once "$run_number"; then
      failures=$((failures + 1))
      log "Run $run_number failed"

      if ((CONTINUE_ON_ERROR == 0)); then
        exit 1
      fi
    fi

    if ((run_number < COUNT && SLEEP_SECONDS > 0)); then
      sleep "$SLEEP_SECONDS"
    fi
  done

  if ((failures > 0)); then
    die "$failures run(s) failed"
  fi

  log "Completed $COUNT run(s)"
}

main "$@"
