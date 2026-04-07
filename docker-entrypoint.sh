#!/usr/bin/env bash
set -euo pipefail

cp /app/config.yml.template /app/config.yml

python3 -c "
import yaml, random, sys

with open('/app/config.yml') as f:
    cfg = yaml.safe_load(f)

token = sys.argv[1] if len(sys.argv) > 1 and sys.argv[1] else None
if token:
    cfg['token'] = token

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

with open('/app/config.yml', 'w') as f:
    yaml.dump(cfg, f, default_flow_style=False, sort_keys=False)
" "${LICHESS_BOT_TOKEN:-}"

exec python3 lichess-bot.py "$@"
