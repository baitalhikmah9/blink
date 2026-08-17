use serde::{Deserialize, Serialize};

use crate::scheduler::blink::{
    Jitter, MAX_INTERVAL_SECS as BLINK_MAX, MIN_INTERVAL_SECS as BLINK_MIN,
};
use crate::scheduler::break_timer;

const MIN_DURATION_MS: u32 = 300;
const MAX_DURATION_MS: u32 = 5000;
const MIN_THICKNESS_PX: i32 = 8;
const MAX_THICKNESS_PX: i32 = 200;
const MIN_INTENSITY: u8 = 0;
const MAX_INTENSITY: u8 = 255;
const MIN_IDLE_THRESHOLD_SECS: u32 = 10;
const MAX_IDLE_THRESHOLD_SECS: u32 = 3600;
const MIN_BREAK_DURATION_SECS: u32 = 1;
const MAX_BREAK_DURATION_SECS: u32 = 300;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
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
#[serde(default)]
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
#[serde(default)]
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
#[serde(default)]
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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Statistics {
    pub enabled: bool,
    pub blink_cues_shown: u64,
    pub eye_break_reminders_shown: u64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub blink: BlinkSettings,
    pub appearance: AppearanceSettings,
    pub eye_break: BreakSettings,
    pub behaviour: BehaviourSettings,
    pub statistics: Statistics,
}

impl Settings {
    /// Clamp and fix all values after loading from disk.
    pub fn normalize(&mut self) {
        self.blink.interval_secs = self
            .blink
            .interval_secs
            .clamp(BLINK_MIN as f32, BLINK_MAX as f32);

        self.appearance.intensity = self
            .appearance
            .intensity
            .clamp(MIN_INTENSITY, MAX_INTENSITY);
        self.appearance.thickness_px = self
            .appearance
            .thickness_px
            .clamp(MIN_THICKNESS_PX, MAX_THICKNESS_PX);
        self.appearance.duration_ms = self
            .appearance
            .duration_ms
            .clamp(MIN_DURATION_MS, MAX_DURATION_MS);

        self.eye_break.interval_secs = self.eye_break.interval_secs.clamp(
            break_timer::MIN_INTERVAL_SECS as f32,
            break_timer::MAX_INTERVAL_SECS as f32,
        );
        self.eye_break.duration_secs = self
            .eye_break
            .duration_secs
            .clamp(MIN_BREAK_DURATION_SECS, MAX_BREAK_DURATION_SECS);

        self.behaviour.idle_threshold_secs = self
            .behaviour
            .idle_threshold_secs
            .clamp(MIN_IDLE_THRESHOLD_SECS, MAX_IDLE_THRESHOLD_SECS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiny_duration_is_clamped() {
        let mut s = Settings::default();
        s.appearance.duration_ms = 5;
        s.normalize();
        assert_eq!(s.appearance.duration_ms, MIN_DURATION_MS);
    }

    #[test]
    fn huge_blink_interval_is_clamped() {
        let mut s = Settings::default();
        s.blink.interval_secs = 9999.0;
        s.normalize();
        assert_eq!(s.blink.interval_secs, BLINK_MAX as f32);
    }

    #[test]
    fn default_unchanged() {
        let mut s = Settings::default();
        s.normalize();
        assert_eq!(s.blink.interval_secs, 30.0);
        assert_eq!(s.appearance.duration_ms, 800);
    }
}
