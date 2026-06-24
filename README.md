# monad-sentinel

A lightweight, self-hostable alerting agent for Monad validator operators. Scrapes the native metrics endpoint at `:8889`, evaluates declarative rules with defaults tuned for VDP thresholds (98% uptime / 48-hour upgrade window), and sends alerts to Telegram. No Prometheus stack required — ships as a single static Rust binary.

Maintained by Noders as a public good for the Monad validator community. Licensed under Apache-2.0.

---

## What It Does

- Scrapes the node's native OpenTelemetry metrics from `:8889`.
- Runs an independent RPC liveness check against `eth_blockNumber` (default `:8080`).
- Evaluates rules defined in `rules/default.toml` against a simple alert state machine.
- Sends firing and resolved notifications to a Telegram bot.

No Prometheus, Grafana, or any other external stack is needed.

---

## Quick Start

```bash
export SENTINEL_TELEGRAM_TOKEN=...      # obtain from @BotFather
export SENTINEL_TELEGRAM_CHAT_ID=...

cp sentinel.example.toml sentinel.toml

cargo run --release -- --config sentinel.toml check   # dry-run: evaluate rules once and print results
cargo run --release -- --config sentinel.toml run     # start the alerting loop
```

Rules are declared in `rules/default.toml`. Secrets must be supplied exclusively via environment variables — never hardcode them in the config file.

---

## Zombie-Node Detection

`metrics_stale` fires when `:8889` stops updating — the otel/waltrace pipeline inside the node process has died while the process itself remains alive.

`rpc_block_stall` independently monitors `eth_blockNumber` via the configured `rpc_url` (default `:8080`). It also fires when the RPC endpoint is unreachable: the agent records the last-known block height, which then stops advancing. This means a dead RPC is detected even from a cold start.

Combined interpretation:

| `metrics_stale` | `rpc_block_stall` | Meaning |
|-----------------|-------------------|---------|
| firing | silent | Metrics pipeline is dead; the chain is alive — restart the node. |
| firing | firing | The node has genuinely halted. |

**Clock-skew caveat:** `metrics_stale` compares the otel timestamp embedded in `:8889` output against the agent's local clock. Run the agent on the same host as the node so both share the same clock. The `max_age_ms` threshold (default 60 s) absorbs minor skew.

### Co-fire During a Metrics Freeze (M0.5 Behaviour)

When the metrics pipeline stalls (the timestamp from `:8889` freezes), `metrics_stale` fires immediately. At the same time, `sync_stall` and `commit_stall` also fire because they read `monad_execution_ledger_block_num` and `monad_state_consensus_events_commit_block` from `:8889` — and those values are now frozen too. This co-fire is expected in M0.5.

**How to distinguish the root cause:**

- If `rpc_block_stall` is **silent** — the chain is alive; only the metrics pipeline is frozen — restart the node.
- If `rpc_block_stall` is **firing** — the chain itself has stalled.

Automatic co-fire suppression via a correlation engine is planned for M0.6.

---

## Consensus Participation / Uptime

`participation_loss` fires when the node is alive and rounds are advancing, but the vote-rate has dropped below 50% of the round-rate — the validator has stopped participating in consensus and is burning VDP uptime.

Key points:

- This is an **early warning**, not the exact foundation uptime percentage (which is calculated per epoch).
- Works only on a **staked validator**; a full node produces no participation signal.
- Suppressed by `metrics_stale`: if the metrics pipeline is frozen, the vote-rate appears falsely zero and the rule is silenced to avoid spurious alerts.

---

## Console (control plane)

`sentinel-web` is a privileged HTTP control plane that lets an operator view node status, metrics, logs, alerts, and issue safe restart operations — all protected by password + TOTP + CSRF.

### Security model

- **VPN-only**: bind `sentinel-web` to `127.0.0.1` (default) or a private interface; never expose it to the public internet.
- **Session authentication**: password login sets a `sid` (HttpOnly) session cookie.
- **CSRF protection**: every mutating request requires the `x-csrf` header to match the `csrf` cookie value set at login.
- **TOTP (second factor)**: every restart operation requires a current TOTP code from the enrolled authenticator app.
- **Allowlist**: only the three managed units (`monad-bft.service`, `monad-execution.service`, `monad-rpc.service`) can be restarted. The command is always a fixed argv — no shell interpolation.
- **Unprivileged user**: `sentinel-web` runs as the `sentinel` system user; it gains `NOPASSWD` access to `systemctl restart` for the allowlisted units only via the sudoers snippet.
- **Audit log**: every restart attempt (including failures) is written to the SQLite audit table with actor, unit, result, and detail.

### Bootstrap

1. **Create the system user and directories:**

   ```bash
   sudo useradd -r -s /usr/sbin/nologin sentinel
   sudo mkdir -p /var/lib/sentinel /etc/sentinel
   sudo chown sentinel:sentinel /var/lib/sentinel
   ```

2. **Add sentinel to the journal group** (required for log reads):

   ```bash
   sudo usermod -aG systemd-journal sentinel
   ```

3. **Install the sudoers allowlist:**

   ```bash
   sudo install -o root -g root -m 440 deploy/sudoers.d-sentinel /etc/sudoers.d/sentinel
   sudo visudo -c   # validate syntax
   ```

4. **Write the environment file** (never commit secrets):

   ```bash
   sudo tee /etc/sentinel/sentinel-web.env <<'EOF'
   SENTINEL_ADMIN_PASSWORD=<strong-random-password>
   SENTINEL_SESSION_KEY=<32-random-bytes-hex>
   # Optional: Telegram alert forwarding
   # SENTINEL_TELEGRAM_TOKEN=...
   # SENTINEL_TELEGRAM_CHAT_ID=...
   EOF
   sudo chmod 600 /etc/sentinel/sentinel-web.env
   sudo chown sentinel:sentinel /etc/sentinel/sentinel-web.env
   ```

5. **Write the config file** (`/etc/sentinel/sentinel-web.toml`) — see `WebConfig` defaults for all fields.

6. **Copy the binary and install the systemd unit:**

   ```bash
   sudo install -o root -g root -m 755 target/release/sentinel-web /usr/local/bin/
   sudo install -o root -g root -m 644 deploy/sentinel-web.service /etc/systemd/system/
   sudo systemctl daemon-reload
   sudo systemctl enable --now sentinel-web
   ```

7. **Enroll your TOTP authenticator**: on first start, `sentinel-web` prints a `otpauth://` enrollment URI to stderr. Scan it once with your authenticator app (Google Authenticator, Aegis, etc.). The TOTP secret is regenerated on each startup in Phase 1; persist it in the env file if you need a stable secret across restarts.

### Environment variables

| Variable | Required | Description |
|---|---|---|
| `SENTINEL_ADMIN_PASSWORD` | Yes | Password for the `admin` account |
| `SENTINEL_SESSION_KEY` | Yes (via env file) | Session signing key (32+ bytes) |
| `SENTINEL_TELEGRAM_TOKEN` | Optional | Telegram bot token for alert forwarding |
| `SENTINEL_TELEGRAM_CHAT_ID` | Optional | Telegram chat/channel ID |
