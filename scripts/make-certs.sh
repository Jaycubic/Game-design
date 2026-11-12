#!/usr/bin/env bash
# Create a local certificate authority and an HTTPS certificate for the game
# server, so browsers on the LAN treat the game as a secure context (WebGPU).
#
#   scripts/make-certs.sh [DIR] [EXTRA_NAMES...]
#     DIR          output folder (default ./certs)
#     EXTRA_NAMES  extra IPs / hostnames to include (the host's LAN IPs and
#                  hostname are detected automatically)
#
# Players import DIR/natures-thread-ca.crt into their browser/OS trust store
# once. The CA private key never leaves the server; keep DIR out of git.
set -euo pipefail

DIR="${1:-./certs}"
shift || true
command -v openssl >/dev/null || { echo "openssl is required" >&2; exit 1; }
mkdir -p "$DIR"
chmod 700 "$DIR"
umask 077

CA_KEY="$DIR/natures-thread-ca.key"
CA_CRT="$DIR/natures-thread-ca.crt"
KEY="$DIR/server.key"
CRT="$DIR/server.crt"

if [ ! -f "$CA_KEY" ]; then
  echo "▶ Creating local certificate authority"
  openssl req -x509 -newkey rsa:3072 -sha256 -days 825 -nodes \
    -keyout "$CA_KEY" -out "$CA_CRT" \
    -subj "/CN=Nature's Thread Local CA" \
    -addext "basicConstraints=critical,CA:TRUE" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" >/dev/null 2>&1
fi

# Subject alternative names: localhost, this host's name and LAN addresses, plus extras.
SAN="DNS:localhost,IP:127.0.0.1"
HOST="$(hostname 2>/dev/null || true)"
[ -n "$HOST" ] && SAN="$SAN,DNS:$HOST,DNS:$HOST.local"
for ip in $(hostname -I 2>/dev/null || true); do
  case "$ip" in *:*) ;; *) SAN="$SAN,IP:$ip" ;; esac
done
for extra in "$@"; do
  if [[ "$extra" =~ ^[0-9.]+$ ]]; then SAN="$SAN,IP:$extra"; else SAN="$SAN,DNS:$extra"; fi
done

echo "▶ Issuing server certificate for: $SAN"
openssl req -newkey rsa:2048 -nodes -keyout "$KEY" -out "$DIR/server.csr" -subj "/CN=natures-thread" >/dev/null 2>&1
cat > "$DIR/server.ext" <<EXT
basicConstraints=CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectAltName=$SAN
EXT
openssl x509 -req -in "$DIR/server.csr" -CA "$CA_CRT" -CAkey "$CA_KEY" -CAcreateserial \
  -out "$CRT" -days 397 -sha256 -extfile "$DIR/server.ext" >/dev/null 2>&1
rm -f "$DIR/server.csr" "$DIR/server.ext"
chmod 644 "$CA_CRT" "$CRT"

echo "✔ Certificates written to $DIR"
echo "  Set in .env:   TLS_CERT=$CRT   TLS_KEY=$KEY"
echo "  Give players:  $CA_CRT  (import once as a trusted root, then open https://<server-ip>:<port>)"
