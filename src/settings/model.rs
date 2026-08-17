use serde::{Deserialize, Serialize};

use crate::scheduler::blink::Jitter;
use crate::scheduler::break_timer;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BlinkSettings {
    pub enabled: bool,
    pub interval_secs: f32,
    pub jitter: Jitter,
}

impl Default for BlinkSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 30.0,
            jitter: Jitter::Pct10,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AppearanceSettings {
    pub color: (u8, u8, u8),
    pub intensity: u8,
    pub thickness_px: i32,
    pub duration_ms: u32,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            color: (167, 139, 250),
            intensity: 140,
            thickness_px: 35,
            duration_ms: 800,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BreakSettings {
    pub enabled: bool,
    pub interval_secs: f32,
    pub duration_secs: u32,
    pub countdown_enabled: bool,
}

impl Default for BreakSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: break_timer::DEFAULT_INTERVAL_SECS as f32,
            duration_secs: break_timer::DEFAULT_DURATION_SECS,
            countdown_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BehaviourSettings {
    pub start_at_login: bool,
    pub pause_when_idle: bool,
    pub idle_threshold_secs: u32,
}

impl Default for BehaviourSettings {
    fn default() -> Self {
        Self {
            start_at_login: false,
            pause_when_idle: true,
            idle_threshold_secs: 5 * 60,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Statistics {
    pub enabled: bool,
    pub blink_cues_shown: u64,
    pub eye_break_reminders_shown: u64,
}

impl Default for Statistics {
    fn default() -> Self {
        Self {
            enabled: true,
            blink_cues_shown: 0,
            eye_break_reminders_shown: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Settings {
    pub blink: BlinkSettings,
    pub appearance: AppearanceSettings,
    pub eye_break: BreakSettings,
    pub behaviour: BehaviourSettings,
    pub statistics: Statistics,
}
