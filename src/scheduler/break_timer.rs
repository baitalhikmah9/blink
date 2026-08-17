//! Eye-break (20-20-20-style) timer constants. Scheduling reuses
//! `IntervalScheduler` with wider bounds.

pub const MIN_INTERVAL_SECS: u64 = 60;
pub const MAX_INTERVAL_SECS: u64 = 180 * 60;
pub const DEFAULT_INTERVAL_SECS: u64 = 20 * 60;
pub const DEFAULT_DURATION_SECS: u32 = 20;

pub const MESSAGE: &str = "Look at something far away";
pub const SECONDARY_MESSAGE: &str = "Around 6 m / 20 ft if practical";

pub const INTERVAL_PRESETS_MIN: [u32; 6] = [10, 15, 20, 30, 45, 60];
pub const DURATION_PRESETS_SEC: [u32; 4] = [10, 20, 30, 60];
