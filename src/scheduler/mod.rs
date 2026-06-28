use chrono::{Days, Local, NaiveDateTime, NaiveTime};
use thiserror::Error;

use crate::{
    config::{AppConfig, ConfigError, ScheduleMode},
    theme::Theme,
};

/// Result of evaluating an automation schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleDecision {
    pub target_theme: Theme,
    pub next_change: NaiveDateTime,
}

/// Pluggable source of schedule decisions.
pub trait ScheduleStrategy: Send + Sync {
    fn evaluate(&self, now: NaiveDateTime) -> Result<ScheduleDecision, SchedulerError>;
}

/// Daily light/dark clock schedule.
#[derive(Debug, Clone, Copy)]
pub struct TimeSchedule {
    light_time: NaiveTime,
    dark_time: NaiveTime,
}

impl TimeSchedule {
    pub fn new(light_time: NaiveTime, dark_time: NaiveTime) -> Result<Self, SchedulerError> {
        if light_time == dark_time {
            return Err(SchedulerError::EqualTimes);
        }
        Ok(Self {
            light_time,
            dark_time,
        })
    }

    pub fn from_config(config: &AppConfig) -> Result<Self, SchedulerError> {
        Self::new(config.light_time()?, config.dark_time()?)
    }

    fn is_light_at(&self, time: NaiveTime) -> bool {
        if self.light_time < self.dark_time {
            time >= self.light_time && time < self.dark_time
        } else {
            time >= self.light_time || time < self.dark_time
        }
    }

    fn next_occurrence(
        now: NaiveDateTime,
        time: NaiveTime,
    ) -> Result<NaiveDateTime, SchedulerError> {
        let today = now.date().and_time(time);
        if today > now {
            Ok(today)
        } else {
            now.date()
                .checked_add_days(Days::new(1))
                .map(|date| date.and_time(time))
                .ok_or(SchedulerError::DateOverflow)
        }
    }
}

impl ScheduleStrategy for TimeSchedule {
    fn evaluate(&self, now: NaiveDateTime) -> Result<ScheduleDecision, SchedulerError> {
        let target_theme = if self.is_light_at(now.time()) {
            Theme::Light
        } else {
            Theme::Dark
        };
        let next_light = Self::next_occurrence(now, self.light_time)?;
        let next_dark = Self::next_occurrence(now, self.dark_time)?;
        Ok(ScheduleDecision {
            target_theme,
            next_change: next_light.min(next_dark),
        })
    }
}

/// Builds an automation strategy from configuration.
pub fn strategy(config: &AppConfig) -> Result<Box<dyn ScheduleStrategy>, SchedulerError> {
    match config.mode {
        ScheduleMode::Time => Ok(Box::new(TimeSchedule::from_config(config)?)),
        mode => Err(SchedulerError::UnsupportedMode(mode)),
    }
}

/// Evaluates the configured strategy at local wall-clock time.
pub fn evaluate_now(config: &AppConfig) -> Result<ScheduleDecision, SchedulerError> {
    strategy(config)?.evaluate(Local::now().naive_local())
}

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("light and dark schedule times must differ")]
    EqualTimes,
    #[error("schedule mode {0} is reserved but not implemented yet")]
    UnsupportedMode(ScheduleMode),
    #[error("could not represent the next scheduled date")]
    DateOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 6, 27)
            .unwrap()
            .and_hms_opt(hour, minute, 0)
            .unwrap()
    }

    fn schedule(light: (u32, u32), dark: (u32, u32)) -> TimeSchedule {
        TimeSchedule::new(
            NaiveTime::from_hms_opt(light.0, light.1, 0).unwrap(),
            NaiveTime::from_hms_opt(dark.0, dark.1, 0).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn daytime_schedule_selects_light_inside_window() {
        let decision = schedule((7, 0), (18, 30)).evaluate(at(12, 0)).unwrap();
        assert_eq!(decision.target_theme, Theme::Light);
        assert_eq!(decision.next_change, at(18, 30));
    }

    #[test]
    fn daytime_schedule_selects_dark_after_window() {
        let decision = schedule((7, 0), (18, 30)).evaluate(at(20, 0)).unwrap();
        assert_eq!(decision.target_theme, Theme::Dark);
        assert_eq!(
            decision.next_change,
            at(7, 0).checked_add_days(Days::new(1)).unwrap()
        );
    }

    #[test]
    fn supports_light_windows_crossing_midnight() {
        assert_eq!(
            schedule((18, 0), (6, 0))
                .evaluate(at(23, 0))
                .unwrap()
                .target_theme,
            Theme::Light
        );
        assert_eq!(
            schedule((18, 0), (6, 0))
                .evaluate(at(12, 0))
                .unwrap()
                .target_theme,
            Theme::Dark
        );
    }

    #[test]
    fn boundary_switches_to_new_theme() {
        assert_eq!(
            schedule((7, 0), (18, 30))
                .evaluate(at(18, 30))
                .unwrap()
                .target_theme,
            Theme::Dark
        );
    }
}
