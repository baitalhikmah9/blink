//! Pure time → opacity cue animation.

pub trait CueAnimation {
    fn total_duration_ms(&self) -> u32;
    fn opacity_at(&self, elapsed_ms: u32) -> f32;
}

/// Soft double pulse: blink, blink.
#[derive(Debug, Clone, Copy)]
pub struct DoubleEdgePulse {
    pub fade_in_ms: u32,
    pub fade_out_ms: u32,
    pub gap_ms: u32,
}

impl Default for DoubleEdgePulse {
    fn default() -> Self {
        Self {
            fade_in_ms: 150,
            fade_out_ms: 150,
            gap_ms: 200,
        }
    }
}

impl DoubleEdgePulse {
    pub fn from_total_duration(total_ms: u32, gap_ms: u32) -> Self {
        let gap_ms = gap_ms.min(total_ms.saturating_sub(100));
        let pulse_len = total_ms.saturating_sub(gap_ms) / 2;
        let half = (pulse_len / 2).max(1);
        Self {
            fade_in_ms: half,
            fade_out_ms: pulse_len - half,
            gap_ms,
        }
    }

    fn single_pulse_len(&self) -> u32 {
        self.fade_in_ms + self.fade_out_ms
    }

    fn opacity_within_pulse(&self, t: u32) -> f32 {
        if t < self.fade_in_ms {
            t as f32 / self.fade_in_ms.max(1) as f32
        } else {
            1.0 - ((t - self.fade_in_ms) as f32 / self.fade_out_ms.max(1) as f32)
        }
    }
}

impl CueAnimation for DoubleEdgePulse {
    fn total_duration_ms(&self) -> u32 {
        self.single_pulse_len() * 2 + self.gap_ms
    }

    fn opacity_at(&self, elapsed_ms: u32) -> f32 {
        let pulse_len = self.single_pulse_len();
        let second_pulse_start = pulse_len + self.gap_ms;
        let opacity = if elapsed_ms < pulse_len {
            self.opacity_within_pulse(elapsed_ms)
        } else if elapsed_ms < second_pulse_start {
            0.0
        } else if elapsed_ms < second_pulse_start + pulse_len {
            self.opacity_within_pulse(elapsed_ms - second_pulse_start)
        } else {
            0.0
        };
        opacity.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ends_at_zero() {
        let a = DoubleEdgePulse::default();
        assert_eq!(a.opacity_at(a.total_duration_ms()), 0.0);
        assert!(a.opacity_at(a.fade_in_ms) > 0.9);
    }
}
