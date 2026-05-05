use anyhow::Result;
use clap::Parser;
use spaze_client::{ClientConfig, identity::derive_identity, run};
use spaze_proto::RoomId;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(version, about = "Spaze TUI client (Phase 1.B: ratatui)")]
struct Cli {
    /// Display name. Defaults to $USER, or "anon" if unset.
    #[arg(long)]
    name: Option<String>,

    /// WebSocket URL of the spaze-server.
    #[arg(long, default_value = "ws://127.0.0.1:9876/")]
    server: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    let name = cli
        .name
        .unwrap_or_else(|| std::env::var("USER").unwrap_or_else(|_| "anon".to_string()));
    let (user_id, device_id) = derive_identity(&name);

    // Hardcoded room ID for Phase 1.A. Server uses its own; both ends agree to put
    // messages in this single Room. Server doesn't currently care which RoomId
    // a message is tagged with — it broadcasts everything.
    let room_id = RoomId::from_uuid(Uuid::nil());

    let config = ClientConfig {
        server_url: cli.server,
        user_id,
        device_id,
        display_name: name,
        room_id,
    };

    run(config).await
}
