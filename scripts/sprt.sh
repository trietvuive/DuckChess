#!/usr/bin/env bash
# Run SPRT (Sequential Probability Ratio Test) with fast-chess for DuckChess.
# Compares a baseline engine vs a test engine and stops early when the result is statistically clear.
#
# Prerequisites:
#   - fast-chess: https://github.com/Disservin/fastchess (build or install, must be in PATH)
#   - Two engine binaries: baseline (e.g. current master) and test (e.g. your patch)
#
# Usage:
#   ./scripts/sprt.sh [OPTIONS] [--base BASELINE] [--test TEST]
#
# Options:
#   --base PATH    Path to baseline engine (default: target/release/duck_chess_base)
#   --test PATH    Path to test engine (default: target/release/duck_chess)
#   --tc TC        Time control, Cutechess format (default: 10+0.1)
#   --elo0 ELO     SPRT H0: draw elo (default: 0)
#   --elo1 ELO     SPRT H1: improvement elo (default: 5)
#   --alpha A      Type I error (default: 0.05)
#   --beta B       Type II error (default: 0.05)
#   --concurrency N  Games in parallel (default: 4)
#   --book PATH    EPD opening book (optional; omit to use start position only)
#   --rounds N     Max rounds (default: 100000; SPRT usually stops earlier)
#   --help         Show this help

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# Defaults
BASELINE=""
TEST=""
TC="10+0.1"
ELO0="0"
ELO1="5"
ALPHA="0.05"
BETA="0.05"
CONCURRENCY="4"
BOOK=""
ROUNDS="100000"

# Detect engine extension for Windows
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
  EXE=".exe"
else
  EXE=""
fi

usage() {
  sed -n '2,28p' "$0" | sed 's/^# \?//'
  exit 0
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base)   BASELINE="$2"; shift 2 ;;
    --test)   TEST="$2";     shift 2 ;;
    --tc)     TC="$2";       shift 2 ;;
    --elo0)   ELO0="$2";     shift 2 ;;
    --elo1)   ELO1="$2";     shift 2 ;;
    --alpha)  ALPHA="$2";    shift 2 ;;
    --beta)   BETA="$2";     shift 2 ;;
    --concurrency) CONCURRENCY="$2"; shift 2 ;;
    --book)   BOOK="$2";     shift 2 ;;
    --rounds) ROUNDS="$2";   shift 2 ;;
    --help)   usage ;;
    *) echo "Unknown option: $1"; usage ;;
  esac
done

# Default paths if not set
if [[ -z "$BASELINE" ]]; then
  BASELINE="$REPO_ROOT/target/release/duck_chess_base${EXE}"
fi
if [[ -z "$TEST" ]]; then
  TEST="$REPO_ROOT/target/release/duck_chess${EXE}"
fi

# Resolve to absolute paths
BASELINE="$(cd "$(dirname "$BASELINE")" && pwd)/$(basename "$BASELINE")"
TEST="$(cd "$(dirname "$TEST")" && pwd)/$(basename "$TEST")"

if ! command -v fastchess &>/dev/null; then
  echo "Error: fastchess not found in PATH."
  echo "Install from: https://github.com/Disservin/fastchess"
  echo "  git clone https://github.com/Disservin/fastchess && cd fastchess && make -j"
  exit 1
fi

for path in "$BASELINE" "$TEST"; do
  if [[ ! -f "$path" ]]; then
    echo "Error: Engine not found: $path"
    echo "Build with: cargo build --release"
    echo "For baseline vs test: build baseline, copy to duck_chess_base, then build test."
    exit 1
  fi
done

OPENINGS=""
if [[ -n "$BOOK" ]]; then
  if [[ ! -f "$BOOK" ]]; then
    echo "Error: Book not found: $BOOK"
    exit 1
  fi
  OPENINGS="-openings file=$BOOK format=epd order=random"
fi

echo "SPRT test: Base vs Test"
echo "  Baseline: $BASELINE"
echo "  Test:     $TEST"
echo "  TC:       $TC  Concurrency: $CONCURRENCY"
echo "  SPRT:     elo0=$ELO0 elo1=$ELO1 alpha=$ALPHA beta=$BETA"
echo ""

exec fastchess \
  -engine "cmd=$BASELINE" name=Base \
  -engine "cmd=$TEST" name=Test \
  -each "tc=$TC" proto=uci \
  -sprt "elo0=$ELO0" "elo1=$ELO1" "alpha=$ALPHA" "beta=$BETA" \
  -rounds "$ROUNDS" \
  -repeat \
  -concurrency "$CONCURRENCY" \
  $OPENINGS
