#!/usr/bin/env bash
set -euo pipefail

# ── Autotrain: train NNUE → install net.bin → rebuild engine ──────────────
#
# Usage:
#   ./scripts/train.sh                          # train with defaults
#   ./scripts/train.sh --superbatches 800       # override any train_nnue flag
#   ./scripts/train.sh --plot checkpoints/…/log.txt  # just chart a previous run
#
# Environment variables (optional):
#   CARGO   – path to cargo binary (auto-detected from rustup if unset)
#   THREADS – CPU threads for training (default: all cores)

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_DIR"

CARGO="${CARGO:-cargo}"

THREADS="${THREADS:-$(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 4)}"
DATA_DIR="$REPO_DIR/data"
NET_DST="$REPO_DIR/src/engine/eval/nnue/net.bin"

# ── Download training data if missing ─────────────────────────────────────
#
# Smallnet binpacks from official-stockfish, curated for small architectures:
#   test77  ~63 MB   (quick runs / CI)
#   test79  ~908 MB  (full training)
HF_BASE="https://huggingface.co/datasets/official-stockfish/master-smallnet-binpacks/resolve/main"

download() {
    local name="$1"
    local url="$2"
    local dest="$DATA_DIR/$name"
    if [[ -f "$dest" ]]; then
        return
    fi
    echo "⏬  Downloading $name …"
    curl -fSL --progress-bar "$url" -o "$dest"
    echo "✅  $name ($(du -h "$dest" | cut -f1))"
}

ensure_data() {
    mkdir -p "$DATA_DIR"
    download "test77-jan2022.binpack" \
        "$HF_BASE/test77-jan2022-2tb7p.high-simple-eval-1k.min-v2.binpack"
    download "test79-may2022.binpack" \
        "$HF_BASE/test79-may2022-16tb7p-filter-v6-dd.min-mar2023.unmin.high-simple-eval-1k.min-v2.binpack"
}

# ── If --plot was passed, just chart and exit ─────────────────────────────
for arg in "$@"; do
    if [[ "$arg" == "--plot" ]]; then
        exec "$CARGO" run --bin train_nnue --features train --release -- "$@"
    fi
done

# ── Train ─────────────────────────────────────────────────────────────────
ensure_data

echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  NNUE Training"
echo "  threads: $THREADS"
echo "  data:    $DATA_DIR"
echo "══════════════════════════════════════════════════════════════"
echo ""

"$CARGO" run --bin train_nnue --features train --release -- \
    --threads "$THREADS" \
    "$@"

# ── Find the latest checkpoint ────────────────────────────────────────────
LATEST_CKPT=$(ls -dt checkpoints/duckchess_nnue-*/ 2>/dev/null | head -1)

if [[ -z "$LATEST_CKPT" ]]; then
    echo "❌  No checkpoint found in checkpoints/"
    exit 1
fi

QUANTISED="$LATEST_CKPT/quantised.bin"
if [[ ! -f "$QUANTISED" ]]; then
    echo "❌  Missing $QUANTISED"
    exit 1
fi

# ── Install net.bin ───────────────────────────────────────────────────────
echo ""
echo "📦  Installing $(basename "$LATEST_CKPT")/quantised.bin → net.bin"
cp "$QUANTISED" "$NET_DST"

# ── Rebuild engine ────────────────────────────────────────────────────────
echo "🔨  Rebuilding engine…"
"$CARGO" build --release 2>&1

echo ""
echo "══════════════════════════════════════════════════════════════"
echo "  ✅  Done!  net.bin updated, engine rebuilt."
echo ""
echo "  Quick smoke test:"
echo "    echo 'uci' | ./target/release/duck_chess"
echo "══════════════════════════════════════════════════════════════"
