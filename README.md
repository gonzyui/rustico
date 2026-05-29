# Rustico

<p align="center">
  <img src="assets/logo.png" alt="Rustico Logo" width="150"/>
</p>

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Discord](https://img.shields.io/badge/Discord-%235865F2.svg?style=for-the-badge&logo=discord&logoColor=white)

**Rustico** is a lightning-fast, asynchronous bot written in Rust that automatically monitors Anime News Network (ANN) and AniList for the latest anime news and episodes, pushing beautiful and formatted updates directly to your Discord server via Webhooks.

## 🚀 Features

- **AniList Integration**: Automatically fetches newly aired episodes within the last 24 hours
- **Anime News Network (ANN) Integration**: Parses multiple RSS feeds for the latest breaking anime news
- **Multiple Discord Webhooks**: Send to multiple Discord servers simultaneously
- **Persistent State**: Automatically saves seen articles/episodes in YAML format to prevent duplicates
- **Advanced HTML Parsing**: Intelligent HTML entity decoding and tag stripping
- **Discord Components V2**: Beautiful formatted messages with colors, thumbnails, and rich components
- **Fully Asynchronous**: Built on `tokio` and `reqwest` with HTTP connection pooling for high performance
- **Cron Scheduler**: Runs automatically at a configured interval (default: every 15 minutes)
- **Health Check API**: REST API for monitoring bot status (`/health`, `/metrics`, `/stats`)
- **Configurable Message Templates**: Customize formatting via YAML configuration files
- **Demo Mode**: On first run, sends sample items to verify Discord webhook is working

## 📦 Installation & Setup

### 1. Clone the repository:
```bash
git clone https://github.com/gonzyui/rustico.git
cd rustico
```

### 2. Configure the environment:
Create a `.env` file in the root of the project. Use `example.env` as a template:

```bash
cp example.env .env
```

Edit `.env` with your settings:
```env
# Required
DISCORD_WEBHOOK_URL="https://discord.com/api/webhooks/WEBHOOK"

# Multiple webhooks (comma-separated)
# DISCORD_WEBHOOK_URL="https://..., https://..."

# Optional - RSS Feeds (comma-separated)
ANN_RSS_URLS="https://www.animenewsnetwork.com/all/rss.xml"

# Optional - AniList
ANILIST_ENABLED=true

# Optional - Scheduling
CHECK_INTERVAL_MINUTES=15
DEMO_MODE_ITEM_LIMIT=3
DELAY_BETWEEN_MESSAGES_MS=800

# Optional - Health API
API_ENABLED=true
API_HOST=127.0.0.1
API_PORT=3000

# Optional - Logging
RUST_LOG=info

# Optional - Message templates
MESSAGES_CONFIG_FILE=messages.yaml
```

### 3. Customize Messages (Optional):
Edit `config/messages.yaml` to customize Discord message formatting:

```yaml
colors:
  ann: 0x1E90FF        # Dodger Blue for ANN
  anilist: 0x8A2BE2    # Blue Violet for AniList

formatting:
  ann:
    title_prefix: "📰"
    truncate_description: 400
  anilist:
    title_prefix: "🎬"
    truncate_description: 300
    show_score: true
```

### 4. Build & Run (Locally):

Ensure you have [Rust](https://rustup.rs/) installed (version 1.70+).

```bash
# Debug build
cargo run

# Release build (optimized)
cargo run --release
```

### 🐳 Running with Docker

```bash
# Build the image
docker build -t rustico .

# Run the container
docker run -d \
  --name rustico-bot \
  -e DISCORD_WEBHOOK_URL="https://discord.com/api/webhooks/your_id/your_token" \
  -e ANILIST_ENABLED=true \
  -e CHECK_INTERVAL_MINUTES=15 \
  -v rustico-data:/app/data \
  rustico

# View logs
docker logs -f rustico-bot
```

## 📊 Monitoring & Health Check

Rustico exposes a REST API for monitoring (default: `http://127.0.0.1:3000`):

### Health Check
```bash
curl http://127.0.0.1:3000/health | jq
```

Response:
```json
{
  "status": "🟢 healthy",
  "version": "0.4.0",
  "uptime_seconds": 3600,
  "stats": {
    "articles_sent": 5,
    "episodes_sent": 3,
    "errors": 0,
    "seen_articles": 50,
    "seen_episodes": 15,
    "last_check": "2026-05-29T10:30:00Z"
  }
}
```

### Metrics
```bash
curl http://127.0.0.1:3000/metrics | jq
curl http://127.0.0.1:3000/stats | jq
```

## 💾 State Persistence

Rustico automatically saves and loads state from `data/rustico_state.yaml`:

```yaml
seen_ann:
  - "guid1"
  - "guid2"
seen_anilist:
  - 12345
  - 67890
initialized: true
stats:
  total_articles_sent: 15
  total_episodes_sent: 8
  total_errors: 0
  last_check: "2026-05-29T10:30:00Z"
```

This file is:
- **Automatically created** on first run
- **Updated** after each check cycle
- **Preserved** on restart to avoid re-sending old content
- **Human-readable** YAML format

## 🔧 Configuration Validation

Rustico validates all configuration at startup:

✅ Discord webhook URLs must be valid  
✅ At least one data source (ANN or AniList) must be enabled  
✅ Check interval must be >= 1 minute  
✅ Port numbers must be valid  

Invalid configurations will show clear error messages.

## 📝 Usage Example

**First run - Demo Mode:**
```
🚀 Starting Rustico v0.4.0
📝 Configuration:
   Webhooks: 1 webhook(s) configured
   ANN RSS: 1 feed(s)
   AniList: enabled
   API: enabled
   Interval: 15 min
   Delay between messages: 800 ms
⏱️ Executing initial pass...
✅ Webhook avatar configured from assets/logo.png
🆕 First run → sending up to 3 articles as demo
📤 [ANN] Sending: "New Attack on Titan Season 5 Announced"
📤 [AniList] Sending: "Jujutsu Kaisen Season 2 EP20"
✅ Initial pass completed — state initialized
⏰ Cron configured: '0 */15 * * * *'
🌐 Health API listening on http://127.0.0.1:3000
✅ Scheduler started — press Ctrl+C to stop
```

**Subsequent runs:**
- Checks every 15 minutes
- Only sends new articles/episodes
- State is automatically saved
- Graceful shutdown on Ctrl+C

## 🛠️ Built With

- [Tokio](https://tokio.rs/) - Async runtime
- [Reqwest](https://docs.rs/reqwest/latest/reqwest/) - HTTP Client with connection pooling
- [Serde](https://serde.rs/) - Serialization/Deserialization
- [Serde YAML](https://docs.rs/serde_yaml/) - YAML parsing
- [RSS](https://docs.rs/rss/latest/rss/) - RSS parsing
- [Tokio-cron-scheduler](https://docs.rs/tokio-cron-scheduler/) - Task scheduling
- [Scraper](https://docs.rs/scraper/) - HTML parsing
- [Axum](https://docs.rs/axum/) - Web framework for health API
- [Chrono](https://docs.rs/chrono/) - Date/time handling
- [Anyhow](https://docs.rs/anyhow/) - Error handling

## 🐛 Troubleshooting

### Bot not sending messages
1. Check `.env` file - ensure `DISCORD_WEBHOOK_URL` is valid
2. Check logs - run with `RUST_LOG=debug`
3. Test health API: `curl http://127.0.0.1:3000/health`

### Messages not appearing in Discord
1. Verify webhook hasn't expired (Discord webhooks can expire after 7 days of inactivity)
2. Check webhook permissions in Discord server settings
3. Check firewall/network - make sure bot can reach Discord API

### State file issues
- Delete `data/rustico_state.yaml` to reset and start fresh in demo mode
- File is created automatically on first run in the `data/` directory

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🤝 Contributing

Contributions are welcome! Feel free to open issues and pull requests on GitHub.
