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

`rpc_block_stall` independently monitors `eth_blockNumber` via the configured `rpc_url` (default `:8080`). `rpc_block_stall` also fires when the RPC endpoint is unreachable (the agent records the last-known block height, which then stops advancing), so a dead RPC is detected even from a cold start.

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
