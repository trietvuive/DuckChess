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

python3 -c "
import yaml, random

with open('config.yml') as f:
    cfg = yaml.safe_load(f)

hellos = [
    'Hi! I\'m {me}. Can you beat a duck at chess? Quack!',
    'Hello from {me}! Let\'s see what you\'ve got, {opponent}.',
    '{me} reporting for duty. Prepare to be outplayed... or not. Good luck!',
    'Quack quack! {me} here. No take-backs! Well, maybe some.',
    'Hey {opponent}! I\'m {me}, a Rust-powered chess engine. Let\'s go!',
    '{me} has entered the pond. Ready to play, {opponent}?',
    'Beep boop. I\'m {me}. I think in centipawns. Good luck, {opponent}!',
    'Hi {opponent}! {me} here. May the best silicon win.',
]

goodbyes = [
    'Good game, {opponent}! Thanks for playing!',
    'GG! That was fun. Rematch anytime!',
    'Thanks for the game, {opponent}! Quack!',
    'Well played! See you next time, {opponent}.',
    'Good game! {me} needs a nap now.',
    'GG WP! Hope you had fun, {opponent}!',
]

hellos_spec = [
    'Hi spectators! I\'m {me}, a UCI engine written in Rust. Enjoy the show!',
    'Welcome! {me} here. Grab some popcorn!',
    'Hey there! Thanks for watching {me} play.',
    'Quack! Thanks for tuning in to watch {me}.',
]

goodbyes_spec = [
    'Thanks for watching!',
    'Hope you enjoyed the game! See you next time.',
    'Thanks for spectating! Quack!',
    'That\'s all folks! Thanks for watching.',
]

cfg['greeting'] = {
    'hello': random.choice(hellos),
    'goodbye': random.choice(goodbyes),
    'hello_spectators': random.choice(hellos_spec),
    'goodbye_spectators': random.choice(goodbyes_spec),
}

with open('config.yml', 'w') as f:
    yaml.dump(cfg, f, default_flow_style=False, sort_keys=False)
"

exec python3 lichess-bot.py "$@"
