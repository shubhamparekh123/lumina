# Lumina

Lumina is a small, open-source Linux utility for switching between light and dark desktop themes. The MVP supports GNOME through `gsettings`; its core is desktop-independent so KDE Plasma, XFCE, Cinnamon, COSMIC, MATE, and Budgie can be added as adapters rather than rewrites.

## Features

- Light, dark, toggle, and status CLI commands
- GNOME `org.gnome.desktop.interface color-scheme` support
- Validated TOML configuration with safe defaults
- Time-based automatic switching, including schedules across midnight
- Background daemon with PID-file lifecycle and graceful shutdown
- Structured INFO, WARN, and ERROR logging through `tracing`
- Freedesktop desktop notifications through `notify-rust`/libnotify-compatible servers
- Testable backend and scheduler boundaries

## Requirements

- Linux with a GNOME-based desktop
- `gsettings` (normally provided by GLib)
- A notification daemon implementing the Freedesktop notifications specification
- Rust 1.75 or newer to build from source

## Installation

Download or clone this repository, then run:

```bash
cd lumina
cargo install --path .
```

The installed binary is normally placed in `~/.cargo/bin`. Ensure that directory is on `PATH`.

## Build and test

```bash
cargo build --release
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

The release binary will be at `target/release/lumina`.

## CLI usage

```text
lumina light
lumina dark
lumina toggle
lumina status
lumina daemon start
lumina daemon stop
```

Example status:

```text
Current Theme: Dark
Automation: Enabled
Mode: Time
Next Change: 18:30
```

Set `RUST_LOG` to adjust logging, for example `RUST_LOG=lumina=info`.

## Configuration

Lumina creates `~/.config/lumina/config.toml` on first use:

```toml
theme = "dark"
auto = true
mode = "time"
light_time = "07:00"
dark_time = "18:30"
```

Times use the local timezone and 24-hour `HH:MM` format. `light_time` and `dark_time` must differ. The daemon reloads this file on every scheduling cycle, so it does not need a restart after edits.

Daemon state and logs follow the platform user-state directory (normally `~/.local/state/lumina/`).

## Architecture

Lumina follows ports-and-adapters/clean-architecture boundaries:

```text
CLI / future GTK UI
        |
 application orchestration
        |
 ThemeBackend + ScheduleStrategy + NotificationService
        |                 |                 |
 GNOME/gsettings      TimeSchedule      libnotify
```

- `theme` defines the desktop-neutral `ThemeBackend` port.
- `backends` contains desktop-specific adapters; no UI code invokes `gsettings`.
- `scheduler` contains pure scheduling policy behind `ScheduleStrategy`.
- `config` owns persistence and validation.
- `daemon` owns background lifecycle and periodically orchestrates those ports.
- `app` maps CLI requests to application operations.
- `gui` documents the boundary for a future GTK4/Libadwaita frontend.

Adding another desktop means implementing `ThemeBackend` and selecting it in a future backend factory. Adding sunrise, battery, weather, or rule automation means implementing `ScheduleStrategy` and extending configuration; neither change requires modifying presentation code.

## Implementation notes and technical debt

1. **Structure:** domain ports are separate from adapters. Automatic desktop detection is deferred to a backend factory.
2. **Configuration:** defaults are atomically persisted and input is validated. Config migrations/versioning remain future work.
3. **Backend abstraction:** default toggle logic depends only on the trait. Backends are synchronous because `gsettings` is a short local process.
4. **GNOME:** command execution is injectable and detection is tested. Native GIO bindings may eventually remove process-spawn overhead.
5. **CLI:** output is stable and script-friendly. Machine-readable output is not yet offered.
6. **Scheduler:** time logic is pure and supports midnight crossing. Sunrise, sunset, battery, weather, and rules are reserved but return clear unsupported errors.
7. **Daemon:** config is polled every 30 seconds and failures are logged without terminating the service. A systemd user unit and event-based reload are the next lifecycle improvements.
8. **Tests:** config, strategy boundaries, trait toggle behavior, and GNOME parsing are covered without requiring GNOME. Live desktop tests belong in opt-in Linux CI.
9. **Documentation:** source installation is documented. Distribution packages and man pages remain roadmap items.

## Roadmap

- systemd user service with socket-safe lifecycle management
- GNOME desktop auto-detection and backend factory
- KDE Plasma, XFCE, Cinnamon, COSMIC, MATE, and Budgie adapters
- Sunrise/sunset, battery, weather, and composable rule strategies
- GTK4/Libadwaita GUI
- Package formats for Debian, Fedora, Arch, and Flatpak
- Configuration schema migration and JSON status output

## Contributing

Contributions are welcome. Keep desktop-specific behavior behind `ThemeBackend`, scheduling policy behind `ScheduleStrategy`, avoid panics on user-controlled input, and include tests for behavior changes. Before opening a pull request, run formatting, tests, and Clippy:

```bash
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Please open an issue before starting a large architectural change so the approach can be discussed.

## License

Lumina is available under the [MIT License](LICENSE).
