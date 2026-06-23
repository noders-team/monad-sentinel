# Sentinel за 10 минут

1. Собрать: `cargo build --release` → `target/release/sentinel-agent`.
2. Секреты: создать `/etc/sentinel/secrets.env` (chmod 600):
   ```
   SENTINEL_TELEGRAM_TOKEN=...
   SENTINEL_TELEGRAM_CHAT_ID=...
   ```
3. Конфиг: `cp sentinel.example.toml /etc/sentinel/sentinel.toml`.
4. Dry-run: `sentinel-agent --config /etc/sentinel/sentinel.toml check`.
5. systemd `/etc/systemd/system/sentinel-agent.service`:
   ```ini
   [Unit]
   Description=Sentinel Alerting Agent
   After=network.target

   [Service]
   EnvironmentFile=/etc/sentinel/secrets.env
   ExecStart=/usr/local/bin/sentinel-agent --config /etc/sentinel/sentinel.toml run
   Restart=always

   [Install]
   WantedBy=multi-user.target
   ```
6. `systemctl enable --now sentinel-agent`.
