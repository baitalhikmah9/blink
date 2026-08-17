pub mod blink;
pub mod break_timer;

pub use blink::{
    IntervalScheduler, Jitter, Preset, SchedulerConfig, MAX_INTERVAL_SECS, MIN_INTERVAL_SECS,
};
