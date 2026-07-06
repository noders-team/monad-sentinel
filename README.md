# Monad Sentinel — Validator Operations Suite

Self-hostable operations tooling for **Monad validator** operators, maintained by [Noders](https://github.com/noders-team) as a public good for the validator community. Apache-2.0.

Sentinel has two parts that share one codebase:

1. **`sentinel-agent`** — a lightweight **alerting agent**. Scrapes the node's native metrics (`:8889`), cross-checks RPC liveness (`:8080`), evaluates declarative rules tuned for VDP thresholds (98% uptime / 48-hour upgrade window), and sends Telegram alerts. No Prometheus/Grafana stack required — a single static Rust binary.
2. **`sentinel-web`** — a **browser control plane** ("Console"). A Rust/axum service + React dashboard that shows live metrics, status, logs, alerts and the audit log, and (optionally) performs gated node operations — per-service restart and `apt` upgrade/rollback — behind password + TOTP + CSRF. VPN-only, single-tenant.

> Designed for self-hosting on the validator host. Everything binds to localhost by default; reach the Console over an SSH tunnel or a private VPN (Tailscale/WireGuard) — never expose it publicly.

---

## Repository layout

```
.
├── crates/
│   ├── sentinel-agent/      # alerting library + CLI (lib `sentinel_agent`, bin `sentinel-agent`)
│   └── sentinel-web/        # Console backend (axum) — serves the API + the built SPA
├── frontend/                # React + TypeScript + Vite SPA (the dashboard)
├── deploy/                  # systemd unit, sudoers allowlist, upgrade wrapper script
└── Cargo.toml               # workspace
```

---

## Part 1 — Alerting agent (`sentinel-agent`)

### What it does

- Scrapes the node's native OpenTelemetry metrics from `:8889`.
- Runs an independent RPC liveness check against `eth_blockNumber` (default `:8080`).
- Evaluates rules from `crates/sentinel-agent/rules/default.toml` against an alert state machine.
- Sends firing and resolved notifications to a Telegram bot (with retry/backoff; the bot token is never logged).

### Quick start

```bash
export SENTINEL_TELEGRAM_TOKEN=...      # from @BotFather
export SENTINEL_TELEGRAM_CHAT_ID=...

cp crates/sentinel-agent/sentinel.example.toml sentinel.toml

# dry-run: evaluate rules once and print results
cargo run --release -p sentinel-agent -- --config sentinel.toml check
# start the alerting loop
cargo run --release -p sentinel-agent -- --config sentinel.toml run
```

Secrets are supplied exclusively via environment variables — never hardcode them in the config.

### Zombie-node detection

`metrics_stale` fires when `:8889` stops updating — the otel/waltrace pipeline inside the node process has died while the process itself remains alive.

`rpc_block_stall` independently monitors `eth_blockNumber` via the configured `rpc_url` (default `:8080`). It also fires when the RPC endpoint is unreachable: the agent records the last-known block height, which then stops advancing — so a dead RPC is detected even from a cold start.

| `metrics_stale` | `rpc_block_stall` | Meaning |
|---|---|---|
| firing | silent | Metrics pipeline is dead; the chain is alive — restart the node. |
| firing | firing | The node has genuinely halted. |

**Clock-skew caveat:** `metrics_stale` compares the otel timestamp embedded in `:8889` output against the agent's local clock. Run the agent on the same host as the node. The `max_age_ms` threshold (default 60 s) absorbs minor skew.

### Consensus participation / uptime

`participation_loss` fires when the node is alive and rounds are advancing, but the vote-rate has dropped below 50% of the round-rate — the validator has stopped participating in consensus and is burning VDP uptime.

- An **early warning**, not the exact per-epoch foundation uptime percentage.
- Works only on a **staked validator**; a full node produces no participation signal.
- Suppressed by `metrics_stale`: if the metrics pipeline is frozen, the vote-rate appears falsely zero and the rule is silenced.

---

## Part 2 — Console (`sentinel-web` + `frontend`)

A single-tenant, VPN-only web control plane over the node. The axum backend serves a JSON API under `/api/*` and the built React SPA as static files (the SPA falls back to `index.html` for client-side routes; `/api/*` always takes precedence).

### Dashboard (React SPA)

Dark "Mission Control" theme, top-tab navigation, polling (no streaming). Seven screens:

| Screen | Shows |
|---|---|
| **Login** | password login → session cookie |
| **Overview** | per-service status, KPIs (participation, vote/round, peers, height), recent alerts, an upgrade banner, per-service restart |
| **Metrics** | time-series charts with a 1h / 24h / 7d window |
| **Logs** | journald tail per managed unit, level filter |
| **Alerts** | active + recent Sentinel rule fires |
| **Upgrades** | current vs candidate version, manual target/deadline, Run / Rollback |
| **Operations** | the audit log |

### Security model

- **VPN-only**: binds to `127.0.0.1:8088` by default; reach it via SSH tunnel or a private VPN — never the public internet.
- **Session auth**: password login sets an `sid` (HttpOnly, SameSite=Strict) cookie carrying an opaque 256-bit CSPRNG token; sessions are server-side in memory with absolute + idle expiry.
- **CSRF**: every mutating request must send the `x-csrf` header matching the JS-readable `csrf` cookie (double-submit) — works because the SPA is served same-origin.
- **TOTP** second factor: every state-changing operation (restart / upgrade / rollback) requires a fresh 6-digit code, plus typing the node name in the confirm dialog. Accepted codes are single-use (replays are rejected and audited), and 5 failed codes within 5 minutes lock the ops endpoints for the rest of the window.
- **Privilege separation**: `sentinel-web` runs as an unprivileged `sentinel` user and **never builds a command from user input**. Privileged actions go through a fixed allowlisted argv (no shell), granted narrowly via sudoers.
- **Audit log**: every privileged attempt (success and every rejection) is written to the SQLite audit table.
- Secrets via `SENTINEL_*` env only.

### Read-only vs. full

The entire **read-only** dashboard (metrics, status, logs, alerts, upgrade *view*) needs **no sudo at all** — `systemctl is-active`, journald (via the `systemd-journal` group), the metrics/RPC ports, and `apt-cache policy` are all unprivileged. Operations (restart/upgrade/rollback) are the *only* thing that needs the sudoers grant. So a safe first deployment installs the service **without** the sudoers file: a fully functional dashboard with zero privileged surface. Add the sudoers allowlist later to enable the buttons.

### Upgrades

A single `monad` apt package owns all three node binaries (`monad-node`, `monad`, `monad-rpc`). The upgrade op installs an exact version of this package via the privileged wrapper `deploy/monad-upgrade.sh` (`apt install monad=<VER> --allow-change-held-packages` → `apt-mark hold` → restart the three services → verify `monad-rpc -V`). The candidate version comes from `apt-cache policy monad`. Rollback re-installs the version recorded immediately before the last upgrade (not free-form input). Both require a fresh TOTP.

---

## Building

```bash
# Backend (workspace)
cargo build --release -p sentinel-web        # Console binary
cargo test  --workspace                      # full Rust test suite

# Frontend (served by sentinel-web)
cd frontend
npm install
npm run build                                # → frontend/dist
VITE_NODE_NAME=my-validator npm run build    # brand the type-to-confirm string
npm run test                                 # Vitest + RTL
```

**Cross-compiling the deploy binary** (e.g. from macOS/ARM to the validator's Linux x86-64). The project produces a fully static binary, so the cleanest path is [`cargo-zigbuild`](https://github.com/rust-cross/cargo-zigbuild):

```bash
brew install zig
rustup target add x86_64-unknown-linux-musl
cargo install cargo-zigbuild
cargo zigbuild --release --target x86_64-unknown-linux-musl -p sentinel-web
# → target/x86_64-unknown-linux-musl/release/sentinel-web  (static, scp-and-run)
```

---

## Deploying the Console

Build the binary + `frontend/dist`, copy them to the node, then install the service. Artifacts live in `deploy/`.

```bash
# 1. system user (no login) + journal read access
sudo useradd -r -s /usr/sbin/nologin -d /opt/sentinel sentinel
sudo usermod -aG systemd-journal sentinel
sudo mkdir -p /var/lib/sentinel /etc/sentinel /opt/sentinel/frontend
sudo chown -R sentinel:sentinel /var/lib/sentinel /opt/sentinel

# 2. binary + SPA bundle
sudo install -o root -g root -m 755 sentinel-web /usr/local/bin/sentinel-web
sudo tar -xzf frontend-dist.tgz -C /opt/sentinel/frontend     # → /opt/sentinel/frontend/dist

# 3. config (only the overrides; everything else defaults to the node's real layout)
sudo tee /etc/sentinel/sentinel-web.toml >/dev/null <<'EOF'
db_path = "/var/lib/sentinel/sentinel-web.sqlite"
frontend_dist = "/opt/sentinel/frontend/dist"
EOF

# 4. secrets (env file; systemd reads it as root before dropping privileges)
sudo tee /etc/sentinel/sentinel-web.env >/dev/null <<'EOF'
SENTINEL_ADMIN_PASSWORD=<strong-random-password>
# Optional Telegram alert forwarding:
# SENTINEL_TELEGRAM_TOKEN=...
# SENTINEL_TELEGRAM_CHAT_ID=...
EOF
sudo chmod 600 /etc/sentinel/sentinel-web.env

# 5. systemd unit, enable + start
sudo install -o root -g root -m 644 deploy/sentinel-web.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now sentinel-web

# 6. first start writes the TOTP enrollment banner to an owner-only file —
#    scan it once, then delete it (it is never printed to the journal):
sudo cat /var/lib/sentinel/totp-enroll.txt
sudo rm /var/lib/sentinel/totp-enroll.txt
```

> Steps 1–6 give the **read-only** deployment (no sudoers). To enable operations, additionally install `deploy/sudoers.d-sentinel` (and, for upgrades, `deploy/monad-upgrade.sh`):
> ```bash
> sudo install -o root -g root -m 440 deploy/sudoers.d-sentinel /etc/sudoers.d/sentinel && sudo visudo -c
> sudo install -o root -g root -m 750 deploy/monad-upgrade.sh /usr/local/bin/monad-upgrade.sh
> ```

**Access** (the service binds to `127.0.0.1:8088`):

```bash
ssh -L 8088:localhost:8088 user@validator-host    # then open http://localhost:8088
```

Log in with `admin` + the password from the env file. (TOTP is requested only for operations, not for viewing.)

**TOTP enrollment:** on first start, `sentinel-web` generates a TOTP secret, persists it in the SQLite DB (`creds` table), and writes the `otpauth://` URI to `totp-enroll.txt` (mode 0600) next to the DB — the secret never touches the journal. Scan it once, then delete the file. On later starts the stored secret is reused — no re-enrollment. To reset, delete the DB file and restart.

### Configuration & environment

`sentinel-web.toml` (all fields have sensible defaults matching a standard Monad host):

| Key | Default | Notes |
|---|---|---|
| `listen_addr` | `127.0.0.1:8088` | localhost only; reach via tunnel/VPN |
| `db_path` | `sentinel-web.sqlite` | metric history + audit + creds |
| `frontend_dist` | `frontend/dist` | the built SPA to serve |
| `metrics_url` | `http://localhost:8889/metrics` | node otel metrics |
| `rpc_url` | `http://localhost:8080` | JSON-RPC for `rpc_block_stall` |
| `scrape_interval_ms` | `5000` | |
| `package` | `monad` | apt package for upgrades |
| `upgrade_script` | `/usr/local/bin/monad-upgrade.sh` | privileged wrapper |
| `services` | bft / execution / rpc | managed units (allowlist source of truth) |

| Env var | Required | Description |
|---|---|---|
| `SENTINEL_ADMIN_PASSWORD` | first run only | bootstraps the `admin` password (then persisted, hashed) |
| `SENTINEL_TELEGRAM_TOKEN` / `SENTINEL_TELEGRAM_CHAT_ID` | optional | Telegram alert forwarding |

### Managing / removing

```bash
sudo systemctl status|restart|stop sentinel-web
sudo journalctl -u sentinel-web -f

# full uninstall (reversible)
sudo systemctl disable --now sentinel-web
sudo rm /etc/systemd/system/sentinel-web.service /usr/local/bin/sentinel-web
sudo rm -rf /etc/sentinel /opt/sentinel /var/lib/sentinel
sudo userdel sentinel
```

---

## License

Apache-2.0. © Noders.
