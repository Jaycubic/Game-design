#!/usr/bin/env bash
# Development helper: mirror this working tree to a scratch folder on the build
# server and run a command there (the server has Rust, PostgreSQL and make).
#
#   scripts/dev-remote.sh "make test"
#
# Configure with DEV_REMOTE (ssh target) and DEV_REMOTE_DIR (scratch folder).
# Never points at the deployment folder; build caches (target/, node_modules/)
# are kept on the remote side between runs.
set -euo pipefail

REMOTE="${DEV_REMOTE:?set DEV_REMOTE, e.g. user@host}"
REMOTE_DIR="${DEV_REMOTE_DIR:-natures-thread-dev}"
CMD="${1:-make check}"

cd "$(dirname "$0")/.."

case "$REMOTE_DIR" in
  /opt/*|/|"") echo "refusing to sync into '$REMOTE_DIR' — use a scratch folder" >&2; exit 2 ;;
esac

tar czf - \
  --exclude=./target --exclude=./client/node_modules --exclude=./node_modules \
  --exclude=./.git --exclude=./client/dist --exclude=./dist --exclude=./.env \
  --exclude=./certs . |
  ssh -o BatchMode=yes "$REMOTE" "set -e
    mkdir -p '$REMOTE_DIR'
    cd '$REMOTE_DIR'
    find . -mindepth 1 -maxdepth 1 ! -name target ! -name node_modules ! -name client ! -name .env ! -name certs -exec rm -rf {} +
    if [ -d client ]; then find client -mindepth 1 -maxdepth 1 ! -name node_modules -exec rm -rf {} +; fi
    tar xzf -"

ssh -o BatchMode=yes "$REMOTE" "export PATH=\$HOME/.cargo/bin:\$PATH; cd '$REMOTE_DIR' && $CMD"
