#!/usr/bin/env bash
# Deploy the game to the server and (re)start it as a user service.
#
#   DEPLOY_REMOTE=user@host bash scripts/deploy.sh
#
# The game serves HTTPS itself on GAME_PORT (default 8081), so nothing else on
# the server (Apache, ports 80/443) is touched. Players open
#   https://$GAME_DOMAIN:$GAME_PORT
#
# One-time setup on the server (needs sudo, done by the server's owner):
#   sudo chown "$USER": /opt/Game          # deploy folder writable by you
#   sudo ufw allow 8081/tcp                 # firewall
#   sudo loginctl enable-linger "$USER"     # service starts at boot, survives logout
# and put the certificate files in $DEPLOY_DIR/certs/ (fullchain.cer, server.key).
#
# Never overwrites an existing .env or certs/. Rebuilds and restarts on every run.
set -euo pipefail

REMOTE="${DEPLOY_REMOTE:?set DEPLOY_REMOTE, e.g. user@host}"
DIR="${DEPLOY_DIR:-/opt/Game}"
DOMAIN="${GAME_DOMAIN:-cleanwaterrun.work.gd}"
PORT="${GAME_PORT:-8081}"
DB="${GAME_DB:-natures_thread}"
UNIT=clearwater-run

cd "$(dirname "$0")/.."

echo "▶ checking $REMOTE:$DIR"
ssh -o BatchMode=yes "$REMOTE" "set -e
  [ -d '$DIR' ] && [ -w '$DIR' ] || { echo '✘ $DIR is missing or not writable — run: sudo chown \$USER: $DIR'; exit 1; }
  mkdir -p '$DIR/certs' && chmod 700 '$DIR/certs'
  [ -s '$DIR/certs/fullchain.cer' ] && [ -s '$DIR/certs/server.key' ] || {
    echo '✘ put the certificate in $DIR/certs/fullchain.cer and the private key in $DIR/certs/server.key'; exit 1; }
  chmod 600 '$DIR/certs/server.key'"

echo "▶ copying the source"
tar czf - \
  --exclude=./target --exclude=./client/node_modules --exclude=./node_modules \
  --exclude=./.git --exclude=./client/dist --exclude=./dist --exclude=./.env \
  --exclude=./certs --exclude=./.claude . |
  ssh -o BatchMode=yes "$REMOTE" "set -e
    cd '$DIR'
    find . -mindepth 1 -maxdepth 1 ! -name target ! -name node_modules ! -name client ! -name .env ! -name certs -exec rm -rf {} +
    if [ -d client ]; then find client -mindepth 1 -maxdepth 1 ! -name node_modules -exec rm -rf {} +; fi
    tar xzf -"

echo "▶ building, migrating and restarting on the server"
ssh -o BatchMode=yes "$REMOTE" "set -e
  export PATH=\$HOME/.cargo/bin:\$PATH
  cd '$DIR'

  if [ ! -f .env ]; then
    echo '  creating .env (production, HTTPS on port $PORT)'
    secret=\$(openssl rand -hex 48)
    sed -e 's#^APP_ENV=.*#APP_ENV=production#' \
        -e 's#^HTTP_PORT=.*#HTTP_PORT=$PORT#' \
        -e 's#^TLS_CERT=.*#TLS_CERT=$DIR/certs/fullchain.cer#' \
        -e 's#^TLS_KEY=.*#TLS_KEY=$DIR/certs/server.key#' \
        -e \"s#^DATABASE_URL=.*#DATABASE_URL=postgres:///$DB?host=/var/run/postgresql\\&user=\$USER#\" \
        -e \"s#^JWT_SECRET=.*#JWT_SECRET=\$secret#\" \
        -e 's#^ALLOWED_ORIGINS=.*#ALLOWED_ORIGINS=https://$DOMAIN:$PORT#' \
        -e 's#^STATIC_DIR=.*#STATIC_DIR=$DIR/client/dist#' \
        .env.example > .env
    chmod 600 .env
  fi

  psql -Atqc \"select 1 from pg_database where datname='$DB'\" 'postgres:///postgres?host=/var/run/postgresql' | grep -q 1 \
    || { echo '  creating database $DB'; createdb -h /var/run/postgresql '$DB'; }

  npm ci --no-audit --no-fund --loglevel=error
  npm run build --silent
  cargo build --release --locked -q --bin natures-thread-server
  ./target/release/natures-thread-server --migrate-only

  mkdir -p ~/.config/systemd/user
  cat > ~/.config/systemd/user/$UNIT.service <<UNIT
[Unit]
Description=Clearwater Run game server
After=network-online.target

[Service]
WorkingDirectory=$DIR
ExecStart=$DIR/target/release/natures-thread-server
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
UNIT
  systemctl --user daemon-reload
  systemctl --user enable -q $UNIT
  systemctl --user restart $UNIT
  sleep 2
  systemctl --user is-active --quiet $UNIT || { journalctl --user -u $UNIT -n 30 --no-pager; exit 1; }
  curl -fsS --max-time 5 --resolve '$DOMAIN:$PORT:127.0.0.1' 'https://$DOMAIN:$PORT/health'; echo
  loginctl show-user \$USER -p Linger | grep -q yes || echo '⚠ linger is off: the game stops when you log out and will not start at boot (sudo loginctl enable-linger \$USER)'"

echo "✔ deployed — https://$DOMAIN:$PORT"
