use thiserror::Error;

use crate::{
    backends::{BackendError, GnomeBackend},
    cli::{Cli, Command, DaemonCommand},
    config::{ConfigError, ConfigStore},
    daemon::{self, DaemonError},
    notifications::{LibnotifyService, NotificationError, NotificationService},
    scheduler::{self, SchedulerError},
    theme::{Theme, ThemeBackend},
};

/// Executes one parsed CLI command.
pub async fn run(cli: Cli) -> Result<(), AppError> {
    let backend = GnomeBackend::new();
    let notifier = LibnotifyService;

    match cli.command {
        Command::Light => change_theme(&backend, &notifier, Theme::Light)?,
        Command::Dark => change_theme(&backend, &notifier, Theme::Dark)?,
        Command::Toggle => {
            let theme = backend.toggle()?;
            notify_best_effort(&notifier, theme);
            println!("Current Theme: {theme}");
        }
        Command::Status => print_status(&backend)?,
        Command::Daemon { command } => match command {
            DaemonCommand::Start => println!("Lumina daemon started (PID {})", daemon::start()?),
            DaemonCommand::Stop => println!("Lumina daemon stopped (PID {})", daemon::stop()?),
            DaemonCommand::Run => daemon::run(&backend, &notifier).await?,
        },
    }
    Ok(())
}

fn change_theme(
    backend: &dyn ThemeBackend,
    notifier: &dyn NotificationService,
    theme: Theme,
) -> Result<(), AppError> {
    backend.set_theme(theme)?;
    notify_best_effort(notifier, theme);
    println!("Current Theme: {theme}");
    Ok(())
}

fn notify_best_effort(notifier: &dyn NotificationService, theme: Theme) {
    if let Err(error) = notifier.theme_changed(theme) {
        tracing::warn!(%error, "theme changed but notification failed");
    }
}

fn print_status(backend: &dyn ThemeBackend) -> Result<(), AppError> {
    let config = ConfigStore::standard()?.load_or_create()?;
    let current = backend.current_theme()?;
    let automation = if config.automation { "Enabled" } else { "Disabled" };
    let next_change = if config.automation {
        scheduler::evaluate_now(&config)?.next_change.format("%H:%M").to_string()
    } else {
        "Disabled".to_owned()
    };

    println!("Current Theme: {current}");
    println!("Automation: {automation}");
    println!("Mode: {}", config.mode);
    println!("Next Change: {next_change}");
    Ok(())
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Backend(#[from] BackendError),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Scheduler(#[from] SchedulerError),
    #[error(transparent)]
    Daemon(#[from] DaemonError),
    #[error(transparent)]
    Notification(#[from] NotificationError),
}

