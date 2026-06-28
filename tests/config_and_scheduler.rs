use chrono::{NaiveDate, NaiveTime};
use lumina::{
    config::{AppConfig, ConfigStore},
    scheduler::{ScheduleStrategy, TimeSchedule},
    theme::Theme,
};

#[test]
fn persisted_configuration_drives_scheduler() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConfigStore::at(directory.path().join("config.toml"));
    let config = AppConfig {
        light_time: "08:15".to_owned(),
        dark_time: "20:45".to_owned(),
        ..AppConfig::default()
    };
    store.save(&config).unwrap();

    let loaded = store.load_or_create().unwrap();
    let schedule = TimeSchedule::from_config(&loaded).unwrap();
    let noon = NaiveDate::from_ymd_opt(2026, 6, 27)
        .unwrap()
        .and_time(NaiveTime::from_hms_opt(12, 0, 0).unwrap());
    assert_eq!(schedule.evaluate(noon).unwrap().target_theme, Theme::Light);
}
