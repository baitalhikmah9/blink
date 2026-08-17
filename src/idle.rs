//! Seconds since last keyboard/mouse input (CoreGraphics).

use objc2_core_graphics::{CGEventSource, CGEventSourceStateID, CGEventType};

/// `kCGAnyInputEventType` is `(CGEventType)(~0)`.
const ANY_INPUT_EVENT_TYPE: CGEventType = CGEventType(u32::MAX);

pub fn seconds_since_input() -> f64 {
    CGEventSource::seconds_since_last_event_type(
        CGEventSourceStateID::CombinedSessionState,
        ANY_INPUT_EVENT_TYPE,
    )
}
