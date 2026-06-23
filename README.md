# monad-sentinel

**Self-hostable monitoring & alerting agent for Monad validator operators.**
Scrapes the node's native `:8889` metrics + independent RPC liveness, evaluates VDP-aware rules (uptime, sync/commit stalls, timeouts, peers, metrics-freeze "zombie node" detection, consensus participation), and pages Telegram. Single static Rust binary, no Prometheus stack required.

Maintained by [Noders](https://github.com/noders-team) as a public good for the Monad validator community. Licensed under Apache-2.0.

---

# sentinel-agent (Sentinel M0 — Alerting)

Лёгкий self-hostable агент для операторов валидаторов Monad: скрейпит нативный `:8889`,
гоняет правила с дефолтами под пороги VDP (uptime 98% / upgrade 48ч) и пейджит в Telegram.
Не требует стек Prometheus.

## Быстрый старт
```bash
export SENTINEL_TELEGRAM_TOKEN=...      # из @BotFather
export SENTINEL_TELEGRAM_CHAT_ID=...
cp sentinel.example.toml sentinel.toml
cargo run --release -- --config sentinel.toml check   # dry-run
cargo run --release -- --config sentinel.toml run     # цикл алертинга
```

Правила — в `rules/default.toml` (декларативно). Секреты — только в env.

## Обнаружение «зомби-ноды»
`metrics_stale` срабатывает, когда `:8889` перестаёт обновляться (otel/waltrace-поток умер, а процесс жив). Независимый `rpc_block_stall` следит за `eth_blockNumber` (`rpc_url`, дефолт :8080). Комбинация:
- `metrics_stale` + НЕ `rpc_block_stall` → метрики мертвы, цепочка жива → рестарт ноды.
- оба → нода реально встала.

Примечание: `metrics_stale` сравнивает otel-timestamp ноды с часами агента — запускайте агент НА ноде (общие часы). Порог `max_age_ms` (дефолт 60с) поглощает мелкий перекос.

### М0.5: Совместное срабатывание `sync_stall`/`commit_stall` при замёрзнутых метриках
При остановке metrics-pipeline (замёрзнутый timestamp `:8889`) сразу срабатывает `metrics_stale`, но **также горят** `sync_stall` и `commit_stall` (читают замёрзшие `monad_execution_ledger_block_num` и `monad_state_consensus_events_commit_block` с `:8889`). Это ожидаемое совместное срабатывание в М0.5.

**Как различить:**
- Если `rpc_block_stall` **молчит** → цепочка живая, горит только metrics-pipeline → рестартни ноду.
- Если `rpc_block_stall` **горит** → цепочка реально встала.

Полное автоматическое подавление co-fire приходит в М0.6 (Correlation engine).

## Участие в консенсусе (uptime)
`participation_loss` срабатывает, когда нода жива и раунды идут, но vote-rate упал ниже 50% от round-rate — валидатор перестал участвовать (жжётся VDP-uptime). Это раннее предупреждение, не точный фондовый uptime% (тот считается по эпохам). Работает на ЗАСТЕЙКАННОМ валидаторе (на full-node сигнала участия нет). Подавляется `metrics_stale` (если метрики замёрзли, vote-rate ложно «нулевой»).
