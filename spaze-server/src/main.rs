use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anyhow::Result;
use clap::Parser;
use spaze_server::{ServerConfig, run};

#[derive(Parser, Debug)]
#[command(version, about = "Spaze chat server daemon")]
struct Cli {
    /// Port to bind on (loopback only). Override binding interface in a future phase.
    #[arg(long, default_value_t = 9876)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), cli.port);

    run(ServerConfig { bind_addr }).await
}
