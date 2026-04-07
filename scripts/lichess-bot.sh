#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LICHESS_BOT_DIR="$REPO_ROOT/lichess-bot"

if [ ! -d "$LICHESS_BOT_DIR" ]; then
    echo "Error: lichess-bot not found at $LICHESS_BOT_DIR"
    echo "Run: git clone https://github.com/lichess-bot-devs/lichess-bot.git lichess-bot"
    exit 1
fi

if [ ! -f "$REPO_ROOT/target/release/duck_chess" ]; then
    echo "Building engine (release)..."
    cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml"
fi

if [ ! -d "$LICHESS_BOT_DIR/venv" ]; then
    echo "Setting up Python virtual environment..."
    python3 -m venv "$LICHESS_BOT_DIR/venv"
    "$LICHESS_BOT_DIR/venv/bin/pip" install -r "$LICHESS_BOT_DIR/requirements.txt"
fi

cd "$LICHESS_BOT_DIR"
source venv/bin/activate
exec python3 lichess-bot.py "$@"
