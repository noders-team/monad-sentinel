# Sentinel Setup in 10 Minutes

## 1. Build

```bash
cargo build --release
```

The binary is placed at `target/release/sentinel-agent`.

## 2. Install the Binary

```bash
cp target/release/sentinel-agent /usr/local/bin/sentinel-agent
```

## 3. Create the Secrets File

Create `/etc/sentinel/secrets.env` and restrict its permissions to root-only:

```bash
mkdir -p /etc/sentinel
chmod 700 /etc/sentinel

cat > /etc/sentinel/secrets.env <<'EOF'
SENTINEL_TELEGRAM_TOKEN=...
SENTINEL_TELEGRAM_CHAT_ID=...
EOF

chmod 600 /etc/sentinel/secrets.env
```

Never store secrets inside the config file — the `EnvironmentFile` directive in the systemd unit loads them at runtime.

## 4. Install the Config

```bash
cp sentinel.example.toml /etc/sentinel/sentinel.toml
```

Edit `/etc/sentinel/sentinel.toml` to set your node's RPC URL and any rule overrides.

## 5. Dry-Run

Verify the configuration and rule evaluation without starting the alerting loop:

```bash
sentinel-agent --config /etc/sentinel/sentinel.toml check
```

Fix any reported errors before proceeding.

## 6. Install the systemd Unit

Create `/etc/systemd/system/sentinel-agent.service`:

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

## 7. Enable and Start

```bash
systemctl daemon-reload
systemctl enable --now sentinel-agent
```

Check that the service started cleanly:

```bash
systemctl status sentinel-agent
journalctl -u sentinel-agent -f
```
