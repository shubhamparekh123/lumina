use clap::{Parser, Subcommand};

/// Lightweight Linux desktop theme control.
#[derive(Debug, Parser)]
#[command(name = "lumina", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn is_daemon_process(&self) -> bool {
        matches!(
            self.command,
            Command::Daemon {
                command: DaemonCommand::Run
            }
        )
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Switch to the light color scheme.
    Light,
    /// Switch to the dark color scheme.
    Dark,
    /// Switch to the opposite color scheme.
    Toggle,
    /// Display theme and automation state.
    Status,
    /// Manage the background scheduler.
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum DaemonCommand {
    /// Start the scheduler in the background.
    Start,
    /// Stop the background scheduler.
    Stop,
    /// Internal foreground entry point used by `daemon start`.
    #[command(hide = true)]
    Run,
}
