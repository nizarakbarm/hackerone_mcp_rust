use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Transport {
    Stdio,
    Http,
}

#[derive(Debug, Parser)]
#[command(version, about = "MCP server for the HackerOne API")]
struct Cli {
    /// Config file to merge last (highest precedence).
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
    #[arg(long, value_name = "USERNAME")]
    api_username: Option<String>,
    /// HackerOne API token; prefer `HACKERONE_API_TOKEN` to avoid shell history.
    #[arg(long, value_name = "TOKEN")]
    api_token: Option<String>,
    #[arg(long, value_name = "HANDLE")]
    program_handle: Option<String>,
    #[arg(long, value_name = "URL")]
    base_url: Option<String>,
    /// HTTP listen address, e.g. `127.0.0.1:8080` (`--transport http`).
    #[arg(long, value_name = "ADDR")]
    bind: Option<String>,
    /// How MCP clients reach the server.
    #[arg(long, value_enum, default_value_t = Transport::Stdio)]
    transport: Transport,
}

impl Cli {
    fn flags(&self) -> hackerone_mcp::config::Flags {
        hackerone_mcp::config::Flags {
            config: self.config.clone(),
            api_username: self.api_username.clone(),
            api_token: self.api_token.clone(),
            program_handle: self.program_handle.clone(),
            base_url: self.base_url.clone(),
            bind: self.bind.clone(),
        }
    }
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        // Startup failures are read by humans: print the message, not the debug struct.
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    hackerone_mcp::init_tracing();
    let config = hackerone_mcp::config::load(&cli.flags(), &std::env::current_dir()?, |key| {
        std::env::var(key).ok()
    })?;
    if config.program_handle.is_none() {
        // Not fatal: the program handle becomes mandatory only when a tool that
        // needs it is called.
        tracing::debug!("no program handle configured");
    }
    match cli.transport {
        Transport::Stdio => {
            tracing::info!("starting stdio MCP server");
            hackerone_mcp::serve_stdio(config).await
        }
        Transport::Http => {
            tracing::info!("starting streamable HTTP MCP server");
            hackerone_mcp::serve_http(config).await
        }
    }
}
