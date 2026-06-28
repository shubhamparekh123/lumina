use std::process::ExitCode;

use clap::Parser;
use lumina::{app, cli::Cli, utils::logging};

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    if !cli.is_daemon_process() {
        logging::init_console();
    }

    match app::run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("lumina: {error}");
            tracing::error!(%error, "command failed");
            ExitCode::FAILURE
        }
    }
}
