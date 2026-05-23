# Rustico

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Discord](https://img.shields.io/badge/Discord-%235865F2.svg?style=for-the-badge&logo=discord&logoColor=white)

**Rustico** is a lightning-fast, asynchronous bot written in Rust that automatically monitors Anime News Network (ANN) and AniList for the latest anime news and episodes, pushing beautiful and formatted updates directly to your Discord server via Webhooks.

## 🚀 Features

- **AniList Integration**: Automatically fetches newly aired episodes within the last 24 hours.
- **Anime News Network (ANN) Integration**: Parses the official ANN RSS feed for the latest breaking anime news.
- **Discord Webhooks**: Sends rich embeds (with thumbnails, colors, and timestamps) straight to your Discord channel.
- **Fully Asynchronous**: Built on `tokio` and `reqwest` with HTTP connection pooling for high performance.
- **Cron Scheduler**: Runs automatically at a configured interval (default: every 15 minutes).
- **State Tracking**: Remembers previously sent news and episodes to avoid spamming your channel.

## 📦 Installation & Setup

1. **Clone the repository:**
   ```bash
   git clone https://github.com/gonzyui/rustico.git
   cd rustico
   ```

2. **Configure the environment:**
   Create a `.env` file in the root of the project with your Discord Webhook URL:
   ```env
   DISCORD_WEBHOOK_URL="https://discord.com/api/webhooks/your_webhook_id/your_webhook_token"
   # Optional configurations:
   # ANN_RSS_URL="https://www.animenewsnetwork.com/all/rss.xml"
   # CHECK_INTERVAL_MINUTES="15"
   ```

3. **Build & Run (Locally):**
   Ensure you have [Rust](https://rustup.rs/) installed.
   ```bash
   cargo run --release
   ```

### 🐳 Running with Docker

If you prefer using Docker, you can build and run Rustico without installing Rust on your machine.

1. **Build the Docker image:**
   ```bash
   docker build -t rustico .
   ```

2. **Run the container:**
   You can pass your environment variables directly when running the container:
   ```bash
   docker run -d \
     --name rustico-bot \
     -e DISCORD_WEBHOOK_URL="https://discord.com/api/webhooks/your_webhook_id/your_webhook_token" \
     -e CHECK_INTERVAL_MINUTES="15" \
     rustico
   ```
   *(Note: You don't need a `.env` file when running via Docker if you pass the variables with `-e`, though you can also use `--env-file .env` if you prefer).*

## 📝 Usage

When you first launch the application, **Rustico** will run in "Demo Mode" and send the 3 most recent ANN articles and AniList episodes to verify that the Discord Webhook is working perfectly. 

After this initial run, it will silently wait and check for *new* items at your defined interval (`CHECK_INTERVAL_MINUTES`).

## 🛠️ Built With
- [Tokio](https://tokio.rs/) - Async runtime
- [Reqwest](https://docs.rs/reqwest/latest/reqwest/) - HTTP Client
- [Serde](https://serde.rs/) - Serialization/Deserialization
- [RSS](https://docs.rs/rss/latest/rss/) - RSS parsing
- [Tokio-cron-scheduler](https://docs.rs/tokio-cron-scheduler/latest/tokio_cron_scheduler/) - Task scheduling

## 📄 License
This project is open-source and available to use.
