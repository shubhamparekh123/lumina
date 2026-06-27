use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use thiserror::Error;
use tokio::time;

use crate::{
    backends::BackendError,
    config::{ConfigError, ConfigStore},
    notifications::{NotificationError, NotificationService},
    scheduler::{self, SchedulerError},
    theme::ThemeBackend,
    utils::{logging, paths},
};

const POLL_INTERVAL: Duration = Duration::from_secs(30);

pub fn start() -> Result<u32, DaemonError> {
    let pid_path = pid_file()?;
    if let Some(pid) = read_running_pid(&pid_path)? {
        return Err(DaemonError::AlreadyRunning(pid));
    }

    let executable = std::env::current_exe().map_err(DaemonError::CurrentExecutable)?;
    let mut command = Command::new(executable);
    command
        .args(["daemon", "run"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: `setsid` is async-signal-safe and does not access Rust-owned memory.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    let mut child = command.spawn().map_err(DaemonError::Spawn)?;
    let child_pid = child.id();
    for _ in 0..40 {
        if pid_path.exists() {
            return Ok(child_pid);
        }
        if let Some(status) = child.try_wait().map_err(DaemonError::Spawn)? {
            return Err(DaemonError::ExitedEarly(status));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
    let _ = child.wait();
    Err(DaemonError::StartTimeout)
}

pub fn stop() -> Result<u32, DaemonError> {
    let path = pid_file()?;
    let pid = read_running_pid(&path)?.ok_or(DaemonError::NotRunning)?;

    #[cfg(unix)]
    {
        // SAFETY: the PID is parsed as a positive i32 and SIGTERM has no pointer arguments.
        let result = unsafe { libc::kill(pid, libc::SIGTERM) };
        if result != 0 {
            return Err(DaemonError::Signal(io::Error::last_os_error()));
        }
        for _ in 0..40 {
            if !process_exists(pid) {
                let _ = fs::remove_file(&path);
                return Ok(pid as u32);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        return Err(DaemonError::StopTimeout(pid));
    }
    #[cfg(not(unix))]
    return Err(DaemonError::UnsupportedPlatform);
}

pub async fn run(
    backend: &dyn ThemeBackend,
    notifier: &dyn NotificationService,
) -> Result<(), DaemonError> {
    let state_dir = state_dir()?;
    logging::init_file(&state_dir.join("lumina.log")).map_err(DaemonError::Logging)?;
    let pid_path = state_dir.join("lumina.pid");
    let _pid_guard = PidGuard::acquire(pid_path)?;
    let store = ConfigStore::standard()?;
    tracing::info!(pid = std::process::id(), "daemon started");

    let mut interval = time::interval(POLL_INTERVAL);
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = interval.tick() => apply_schedule(&store, backend, notifier),
            result = &mut shutdown => {
                result?;
                tracing::info!("daemon stopping");
                return Ok(());
            }
        }
    }
}

fn apply_schedule(
    store: &ConfigStore,
    backend: &dyn ThemeBackend,
    notifier: &dyn NotificationService,
) {
    let result = (|| -> Result<(), DaemonError> {
        let config = store.load_or_create()?;
        if !config.automation {
            tracing::info!("automation is disabled");
            return Ok(());
        }
        let decision = scheduler::evaluate_now(&config)?;
        let current = backend.current_theme()?;
        if current != decision.target_theme {
            backend.set_theme(decision.target_theme)?;
            tracing::info!(theme = %decision.target_theme, next_change = %decision.next_change, "theme changed");
            if let Err(error) = notifier.theme_changed(decision.target_theme) {
                tracing::warn!(%error, "theme changed but notification failed");
            }
        }
        Ok(())
    })();

    if let Err(error) = result {
        tracing::error!(%error, "schedule evaluation failed");
    }
}

#[cfg(unix)]
async fn shutdown_signal() -> Result<(), DaemonError> {
    use tokio::signal::unix::{signal, SignalKind};
    let mut terminate = signal(SignalKind::terminate()).map_err(DaemonError::SignalSetup)?;
    let mut interrupt = signal(SignalKind::interrupt()).map_err(DaemonError::SignalSetup)?;
    tokio::select! {
        _ = terminate.recv() => Ok(()),
        _ = interrupt.recv() => Ok(()),
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() -> Result<(), DaemonError> {
    tokio::signal::ctrl_c().await.map_err(DaemonError::SignalSetup)
}

fn state_dir() -> Result<PathBuf, DaemonError> {
    paths::state_dir().ok_or(DaemonError::MissingStateDirectory)
}

fn pid_file() -> Result<PathBuf, DaemonError> {
    Ok(state_dir()?.join("lumina.pid"))
}

fn read_running_pid(path: &Path) -> Result<Option<i32>, DaemonError> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(DaemonError::PidFile(error)),
    };
    let pid: i32 = contents.trim().parse().map_err(|_| DaemonError::InvalidPidFile)?;
    if pid <= 0 {
        return Err(DaemonError::InvalidPidFile);
    }

    #[cfg(unix)]
    {
        if process_exists(pid) {
            return Ok(Some(pid));
        }
    }
    #[cfg(not(unix))]
    return Ok(Some(pid));

    let _ = fs::remove_file(path);
    Ok(None)
}

#[cfg(unix)]
fn process_exists(pid: i32) -> bool {
    // SAFETY: signal 0 only checks process existence and has no pointer arguments.
    if unsafe { libc::kill(pid, 0) } == 0 {
        return true;
    }
    io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

struct PidGuard {
    path: PathBuf,
}

impl PidGuard {
    fn acquire(path: PathBuf) -> Result<Self, DaemonError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(DaemonError::PidFile)?;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                if error.kind() == io::ErrorKind::AlreadyExists {
                    DaemonError::PidFileExists
                } else {
                    DaemonError::PidFile(error)
                }
            })?;
        writeln!(file, "{}", std::process::id()).map_err(DaemonError::PidFile)?;
        Ok(Self { path })
    }
}

impl Drop for PidGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("could not locate the user state directory")]
    MissingStateDirectory,
    #[error("daemon is already running with PID {0}")]
    AlreadyRunning(i32),
    #[error("daemon is not running")]
    NotRunning,
    #[error("daemon PID file already exists")]
    PidFileExists,
    #[error("daemon PID file is invalid")]
    InvalidPidFile,
    #[error("could not access daemon PID file: {0}")]
    PidFile(#[source] io::Error),
    #[error("could not find the Lumina executable: {0}")]
    CurrentExecutable(#[source] io::Error),
    #[error("could not start daemon: {0}")]
    Spawn(#[source] io::Error),
    #[error("daemon exited during startup with status {0}")]
    ExitedEarly(std::process::ExitStatus),
    #[error("daemon did not become ready within two seconds; inspect lumina.log")]
    StartTimeout,
    #[error("daemon with PID {0} did not stop within two seconds")]
    StopTimeout(i32),
    #[error("could not signal daemon: {0}")]
    Signal(#[source] io::Error),
    #[error("could not register shutdown signal: {0}")]
    SignalSetup(#[source] io::Error),
    #[error("could not initialize daemon logging: {0}")]
    Logging(#[source] io::Error),
    #[error("daemon process management is only supported on Unix")]
    UnsupportedPlatform,
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Scheduler(#[from] SchedulerError),
    #[error(transparent)]
    Backend(#[from] BackendError),
    #[error(transparent)]
    Notification(#[from] NotificationError),
}
