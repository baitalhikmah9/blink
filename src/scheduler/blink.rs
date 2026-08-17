//! Timing-only interval scheduler. No OS calls.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

pub const MIN_INTERVAL_SECS: u64 = 1;
pub const MAX_INTERVAL_SECS: u64 = 5 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Jitter {
    Off,
    Pct5,
    Pct10,
    Pct20,
}

impl Jitter {
    pub fn fraction(self) -> f32 {
        match self {
            Jitter::Off => 0.0,
            Jitter::Pct5 => 0.05,
            Jitter::Pct10 => 0.10,
            Jitter::Pct20 => 0.20,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerConfig {
    pub enabled: bool,
    pub base_interval: Duration,
    pub jitter: Jitter,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_interval: Duration::from_secs(30),
            jitter: Jitter::Pct10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Disabled,
    Waiting,
    Paused,
}

pub struct IntervalScheduler {
    config: SchedulerConfig,
    min_interval: Duration,
    max_interval: Duration,
    state: State,
    next_fire: Option<Instant>,
    paused_until: Option<Instant>,
    rng: u64,
}

impl IntervalScheduler {
    pub fn new(config: SchedulerConfig, min_interval: Duration, max_interval: Duration) -> Self {
        let mut config = config;
        config.base_interval = config.base_interval.clamp(min_interval, max_interval);
        let mut scheduler = Self {
            config,
            min_interval,
            max_interval,
            state: State::Disabled,
            next_fire: None,
            paused_until: None,
            rng: seed(),
        };
        scheduler.apply_enabled();
        scheduler
    }

    pub fn is_paused(&self) -> bool {
        self.state == State::Paused
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
        self.paused_until = None;
        self.apply_enabled();
    }

    pub fn pause_for(&mut self, duration: Duration) {
        if self.state == State::Disabled {
            return;
        }
        self.state = State::Paused;
        self.paused_until = Some(Instant::now() + duration);
        self.next_fire = None;
    }

    pub fn resume(&mut self) {
        if self.state == State::Disabled {
            return;
        }
        self.paused_until = None;
        self.state = State::Waiting;
        self.reset_timer_from_now();
    }

    /// Atomic poll. Returns true only when the waiting timer has fired.
    /// Handles pause expiry internally. Callers cannot fire early.
    pub fn poll_due(&mut self) -> bool {
        match self.state {
            State::Disabled => false,
            State::Paused => {
                if self.paused_until.is_some_and(|t| Instant::now() >= t) {
                    self.paused_until = None;
                    self.state = State::Waiting;
                    self.reset_timer_from_now();
                }
                false
            }
            State::Waiting => {
                if self.next_fire.is_some_and(|t| Instant::now() >= t) {
                    self.reset_timer_from_now();
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn reset_without_firing(&mut self) {
        if self.state == State::Waiting {
            self.reset_timer_from_now();
        }
    }

    fn apply_enabled(&mut self) {
        if self.config.enabled {
            self.state = State::Waiting;
            self.reset_timer_from_now();
        } else {
            self.state = State::Disabled;
            self.next_fire = None;
        }
    }

    fn reset_timer_from_now(&mut self) {
        self.next_fire = Some(Instant::now() + self.jittered_interval());
    }

    fn jittered_interval(&mut self) -> Duration {
        let base = self.config.base_interval;
        let frac = self.config.jitter.fraction();
        if frac <= 0.0 {
            return base.clamp(self.min_interval, self.max_interval);
        }
        let base_ms = base.as_millis() as i64;
        let max_delta = ((base_ms as f32) * frac) as i64;
        let delta = self.next_rand_range(-max_delta, max_delta);
        let jittered_ms = (base_ms + delta).max(1000);
        Duration::from_millis(jittered_ms as u64).clamp(self.min_interval, self.max_interval)
    }

    fn next_rand_range(&mut self, min: i64, max: i64) -> i64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        if max <= min {
            return min;
        }
        let span = (max - min + 1) as u64;
        min + (self.rng % span) as i64
    }
}

fn seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    (nanos ^ 0x2545_F491_4F6C_DD1D) | 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_scheduler(config: SchedulerConfig) -> IntervalScheduler {
        IntervalScheduler::new(
            config,
            Duration::from_secs(MIN_INTERVAL_SECS),
            Duration::from_secs(MAX_INTERVAL_SECS),
        )
    }

    #[test]
    fn default_is_30s() {
        assert_eq!(
            SchedulerConfig::default().base_interval,
            Duration::from_secs(30)
        );
    }

    #[test]
    fn poll_only_fires_once_per_interval() {
        let mut s = new_scheduler(SchedulerConfig {
            enabled: true,
            base_interval: Duration::from_secs(30),
            jitter: Jitter::Off,
        });
        s.next_fire = Some(Instant::now() - Duration::from_millis(1));
        assert!(s.poll_due());
        assert!(!s.poll_due());
    }

    #[test]
    fn pause_expires_without_firing() {
        let mut s = new_scheduler(SchedulerConfig {
            enabled: true,
            base_interval: Duration::from_secs(30),
            jitter: Jitter::Off,
        });
        s.state = State::Paused;
        s.paused_until = Some(Instant::now() - Duration::from_millis(1));
        assert!(!s.poll_due());
        assert!(!s.is_paused());
    }

    #[test]
    fn pause_does_not_fire() {
        let mut s = new_scheduler(SchedulerConfig::default());
        s.state = State::Paused;
        s.paused_until = Some(Instant::now() + Duration::from_secs(60));
        assert!(!s.poll_due());
    }

    #[test]
    fn disabled_poll_returns_false() {
        let mut s = new_scheduler(SchedulerConfig {
            enabled: false,
            base_interval: Duration::from_secs(30),
            jitter: Jitter::Off,
        });
        assert!(!s.poll_due());
        s.set_enabled(true);
        assert!(!s.poll_due());
    }

    #[test]
    fn jittered_interval_is_clamped() {
        let mut s = new_scheduler(SchedulerConfig {
            enabled: true,
            base_interval: Duration::from_secs(MAX_INTERVAL_SECS),
            jitter: Jitter::Pct20,
        });
        s.next_fire = Some(Instant::now() - Duration::from_millis(1));
        assert!(s.poll_due());
        let before = Instant::now();
        let after = Instant::now();
        let next = s.next_fire.unwrap();
        assert!(next >= before + s.min_interval);
        assert!(next <= after + s.max_interval);
    }
}
