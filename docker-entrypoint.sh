#!/usr/bin/env bash
set -euo pipefail

if [ ! -f /app/config.yml ]; then
    cp /app/config.yml.template /app/config.yml
fi

if [ -n "${LICHESS_BOT_TOKEN:-}" ]; then
    python3 -c "
import yaml, sys
with open('/app/config.yml') as f:
    cfg = yaml.safe_load(f)
cfg['token'] = sys.argv[1]
with open('/app/config.yml', 'w') as f:
    yaml.dump(cfg, f, default_flow_style=False, sort_keys=False)
" "$LICHESS_BOT_TOKEN"
fi

exec python3 lichess-bot.py "$@"
