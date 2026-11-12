# Nature's Thread — build, test, database and packaging targets.
# Shell: bash (Linux / macOS / WSL / Git Bash). Run `make help` for the list.

SHELL := /bin/bash
.SHELLFLAGS := -eu -o pipefail -c
.DEFAULT_GOAL := help

CARGO   ?= cargo
NPM     ?= npm
NODE    ?= node
PSQL    ?= psql
PYTHON  ?= python3
ENV_FILE ?= .env
SERVER_BIN := target/release/natures-thread-server
CLASSIFY_BIN := target/release/classify
ZIP     ?= dist/Natures-Thread.zip
CERT_DIR ?= certs

# Load .env into recipes that need it (never echoed).
LOAD_ENV = if [ -f "$(ENV_FILE)" ]; then set -a; . "./$(ENV_FILE)"; set +a; fi
SCHEME = $$( [ -n "$${TLS_CERT:-}" ] && echo https || echo http )
BASE_URL_CMD = echo "$(SCHEME)://localhost:$${HTTP_PORT:-8070}"

.PHONY: help check-tools setup client-dev server-dev dev fmt lint check test test-full db-check db-migrate \
        db-reset build-client build-server build run health test-room package verify-package clean certs classify

help: ## List targets
	@echo "Nature's Thread — make targets"
	@echo "Prerequisites: Rust 1.82+ (cargo), Node 22+ (npm), PostgreSQL 14+ (psql), Python 3, OpenSSL"
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*## "}; {printf "  make %-15s %s\n", $$1, $$2}'

check-tools: ## Verify required tools and print versions
	@ok=1; for t in $(CARGO) rustc $(NODE) $(NPM) $(PSQL) $(PYTHON) openssl curl; do \
	  if command -v $$t >/dev/null 2>&1; then v=$$($$t --version 2>/dev/null || $$t version 2>/dev/null || true); printf "  ✔ %-8s %s\n" $$t "$$(echo "$$v" | head -1)"; \
	  else printf "  ✘ %-8s missing\n" $$t; ok=0; fi; done; \
	node_major=$$($(NODE) -p 'process.versions.node.split(".")[0]' 2>/dev/null || echo 0); \
	if [ "$$node_major" -lt 22 ]; then echo "  ✘ Node 22 or newer is required"; ok=0; fi; \
	[ $$ok = 1 ] && echo "All required tools found." || { echo "Install the missing tools (see README)."; exit 1; }

setup: ## Fetch Rust crates and install client packages from the lockfiles
	$(CARGO) fetch --locked
	$(NPM) ci --no-audit --no-fund
	@[ -f "$(ENV_FILE)" ] || echo "Next: cp .env.example .env and fill in DATABASE_URL and JWT_SECRET (see README)."

client-dev: ## Start the Vite dev server on :5173 (proxies to the Rust server)
	$(NPM) run dev

server-dev: ## Run the Rust server in development mode
	@[ -f "$(ENV_FILE)" ] || { echo "Missing $(ENV_FILE) — cp .env.example .env first"; exit 1; }
	$(CARGO) run -p natures-thread-server

dev: ## Run server and client together (Ctrl+C stops both)
	@[ -f "$(ENV_FILE)" ] || { echo "Missing $(ENV_FILE) — cp .env.example .env first"; exit 1; }
	@trap 'kill 0' INT TERM EXIT; \
	  $(CARGO) run -p natures-thread-server & \
	  $(NPM) run dev & \
	  wait

fmt: ## Check Rust formatting and content JSON formatting
	$(CARGO) fmt --all -- --check
	$(NODE) scripts/format-content.mjs
	@git diff --quiet -- content 2>/dev/null || { echo "content/ was reformatted — review and commit"; }

lint: ## Clippy (warnings are errors) and strict TypeScript checks
	$(CARGO) clippy --workspace --all-targets --locked -- -D warnings
	$(NPM) run lint

check: ## Compile/type-check the workspace and client
	$(CARGO) check --workspace --all-targets --locked
	$(NPM) run typecheck

test: ## Rust unit + level tests (solver proofs), server tests, client tests
	$(CARGO) test --workspace --release --locked
	$(NPM) test

test-full: test test-room ## Everything, including the live two-client room test

classify: ## Print the solver's report for every level and ruleset
	$(CARGO) run --release --locked -p natures-thread-core --bin classify

db-check: ## Check DATABASE_URL is set and PostgreSQL answers (credentials never printed)
	@$(LOAD_ENV); \
	[ -n "$${DATABASE_URL:-}" ] || { echo "DATABASE_URL is not set (see .env.example)"; exit 1; }; \
	host=$$(echo "$$DATABASE_URL" | sed -E 's#^[^@]*@##; s#\?.*##'); \
	if $(PSQL) "$$DATABASE_URL" -Atqc 'select 1' >/dev/null 2>&1; then echo "✔ PostgreSQL reachable ($$host)"; \
	else echo "✘ cannot connect to PostgreSQL ($$host) — is it running, and are the credentials right?"; exit 1; fi

db-migrate: build-server ## Apply versioned migrations (stops on the first error)
	@$(LOAD_ENV); ./$(SERVER_BIN) --migrate-only

db-reset: ## DESTRUCTIVE dev-only reset: make db-reset CONFIRM=yes
	@$(LOAD_ENV); \
	[ "$${APP_ENV:-development}" != "production" ] || { echo "Refusing: APP_ENV=production"; exit 1; }; \
	[ "$(CONFIRM)" = "yes" ] || { echo "This deletes all game data. Re-run with CONFIRM=yes"; exit 1; }; \
	$(PSQL) "$$DATABASE_URL" -v ON_ERROR_STOP=1 -qc 'DROP SCHEMA public CASCADE; CREATE SCHEMA public;'
	@$(MAKE) --no-print-directory db-migrate

build-client: ## Production client build → client/dist
	$(NPM) run build

build-server: ## Release build of the server and the classifier
	$(CARGO) build --release --locked -p natures-thread-server -p natures-thread-core

build: build-client build-server ## Build everything

run: build ## Run the release server (serves the built client)
	@[ -f "$(ENV_FILE)" ] || { echo "Missing $(ENV_FILE) — cp .env.example .env first"; exit 1; }
	@$(LOAD_ENV); exec ./$(SERVER_BIN)

health: ## Call /health and /ready on the configured port
	@$(LOAD_ENV); base=$$($(BASE_URL_CMD)); \
	curl -fsS --max-time 5 -k "$$base/health" && echo && curl -fsS --max-time 5 -k "$$base/ready" && echo || \
	{ echo "✘ server not healthy at $$base"; exit 1; }

test-room: build-server ## Live two-client shared-room test (starts the server if needed)
	@$(LOAD_ENV); base=$$($(BASE_URL_CMD)); started=""; \
	if ! curl -fsk --max-time 2 "$$base/health" >/dev/null 2>&1; then \
	  echo "▶ starting server for the test"; ./$(SERVER_BIN) > /tmp/nt-test-room.log 2>&1 & started=$$!; \
	  for i in $$(seq 1 40); do curl -fsk --max-time 1 "$$base/health" >/dev/null 2>&1 && break; sleep 0.25; done; \
	fi; \
	trap '[ -n "$$started" ] && kill $$started' EXIT; \
	NODE_TLS_REJECT_UNAUTHORIZED=0 BASE_URL="$$base" CLASSIFY=./$(CLASSIFY_BIN) $(NODE) tests/e2e/room.mjs

certs: ## Create a local CA + HTTPS certificate in ./certs (WebGPU on LAN devices)
	scripts/make-certs.sh $(CERT_DIR) $(EXTRA_NAMES)

package: ## Create dist/Natures-Thread.zip (no secrets, caches or build output)
	$(PYTHON) scripts/package.py build $(ZIP)

verify-package: package ## Extract the ZIP to a temp folder and build + test from scratch
	$(PYTHON) scripts/package.py verify $(ZIP)

clean: ## Remove build outputs (never source, content, migrations or data)
	$(CARGO) clean
	rm -rf client/dist dist
