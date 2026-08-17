//! Per-screen eye-break banner. Click the banner after a 3s grace to dismiss.

use std::time::Instant;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{sel, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSButton, NSColor, NSFont, NSPanel, NSScreen, NSStatusWindowLevel,
    NSTextAlignment, NSTextField, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use crate::scheduler::break_timer;

const BANNER_W: f64 = 420.0;
const BANNER_H: f64 = 96.0;
const TOP_MARGIN: f64 = 48.0;
const GRACE_MS: u128 = 3000;

pub struct BreakBanner {
    windows: Vec<Retained<NSPanel>>,
    started: Option<Instant>,
    duration_secs: u32,
    countdown: bool,
}

impl BreakBanner {
    pub fn new(_mtm: MainThreadMarker) -> Self {
        Self {
            windows: Vec::new(),
            started: None,
            duration_secs: break_timer::DEFAULT_DURATION_SECS,
            countdown: true,
        }
    }

    pub fn is_showing(&self) -> bool {
        self.started.is_some()
    }

    pub fn show(
        &mut self,
        mtm: MainThreadMarker,
        duration_secs: u32,
        countdown: bool,
        target: &AnyObject,
    ) {
        self.hide();
        self.duration_secs = duration_secs;
        self.countdown = countdown;
        self.started = Some(Instant::now());

        let screens = NSScreen::screens(mtm);
        for screen in screens.iter() {
            let frame = screen.frame();
            let x = frame.origin.x + (frame.size.width - BANNER_W) / 2.0;
            let y = frame.origin.y + frame.size.height - TOP_MARGIN - BANNER_H;
            let rect = NSRect::new(NSPoint::new(x, y), NSSize::new(BANNER_W, BANNER_H));

            let window = NSPanel::initWithContentRect_styleMask_backing_defer(
                NSPanel::alloc(mtm),
                rect,
                NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
                NSBackingStoreType::Buffered,
                false,
            );
            let bg = NSColor::colorWithSRGBRed_green_blue_alpha(0.12, 0.12, 0.14, 0.92);
            window.setOpaque(false);
            window.setBackgroundColor(Some(&bg));
            window.setLevel(NSStatusWindowLevel + 1);
            window.setCollectionBehavior(
                NSWindowCollectionBehavior::CanJoinAllSpaces
                    | NSWindowCollectionBehavior::Stationary
                    | NSWindowCollectionBehavior::FullScreenAuxiliary
                    | NSWindowCollectionBehavior::FullScreenDisallowsTiling,
            );
            window.setHasShadow(true);
            window.setIgnoresMouseEvents(false);
            window.setHidesOnDeactivate(false);
            window.setBecomesKeyOnlyIfNeeded(true);

            let title = label(
                mtm,
                break_timer::MESSAGE,
                NSRect::new(NSPoint::new(16.0, 48.0), NSSize::new(BANNER_W - 32.0, 28.0)),
                17.0,
                true,
            );
            let subtitle = label(
                mtm,
                break_timer::SECONDARY_MESSAGE,
                NSRect::new(NSPoint::new(16.0, 28.0), NSSize::new(BANNER_W - 32.0, 20.0)),
                12.0,
                false,
            );

            if let Some(content) = window.contentView() {
                content.addSubview(&title);
                content.addSubview(&subtitle);
            }

            if countdown {
                let countdown_field = label(
                    mtm,
                    &format!("{duration_secs}s"),
                    NSRect::new(NSPoint::new(16.0, 8.0), NSSize::new(BANNER_W - 32.0, 18.0)),
                    12.0,
                    false,
                );
                countdown_field.setTag(1);
                if let Some(content) = window.contentView() {
                    content.addSubview(&countdown_field);
                }
            }

            // Transparent full-size button added last so it sits on top and receives clicks.
            let click_button = NSButton::new(mtm);
            let local_rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(BANNER_W, BANNER_H));
            click_button.setFrame(local_rect);
            click_button.setTitle(&NSString::from_str(""));
            click_button.setBordered(false);
            click_button.setTransparent(true);
            unsafe {
                click_button.setTarget(Some(target));
                click_button.setAction(Some(sel!(bannerClicked:)));
            }
            if let Some(content) = window.contentView() {
                content.addSubview(&click_button);
            }

            window.orderFrontRegardless();
            self.windows.push(window);
        }
    }

    /// Returns true when the break has ended because of timeout.
    /// Click dismissal is handled by [`BreakBanner::click`].
    pub fn tick(&mut self) -> bool {
        let Some(start) = self.started else {
            return false;
        };
        let elapsed_s = start.elapsed().as_secs_f64();
        let remaining = (self.duration_secs as f64 - elapsed_s).ceil().max(0.0) as u32;

        if self.countdown {
            for w in &self.windows {
                if let Some(content) = w.contentView() {
                    if let Some(field) = content.viewWithTag(1) {
                        let text = if remaining == 0 {
                            "Done".to_string()
                        } else {
                            format!("{remaining}s")
                        };
                        let ns = NSString::from_str(&text);
                        let _: () = unsafe { objc2::msg_send![&*field, setStringValue: &*ns] };
                    }
                }
            }
        }

        if elapsed_s >= self.duration_secs as f64 {
            self.hide();
            return true;
        }
        false
    }

    /// Pure grace-period check: returns true only if the 3s grace has passed.
    pub fn grace_elapsed(&self) -> bool {
        self.started
            .is_some_and(|s| s.elapsed().as_millis() >= GRACE_MS)
    }

    /// Attempt to dismiss by click. Enforces the 3s grace period.
    /// Returns true if the banner was dismissed.
    pub fn click(&mut self) -> bool {
        if !self.grace_elapsed() {
            return false;
        }
        self.hide();
        true
    }

    pub fn hide(&mut self) {
        for w in self.windows.drain(..) {
            w.orderOut(None);
            w.close();
        }
        self.started = None;
    }
}

fn label(
    mtm: MainThreadMarker,
    text: &str,
    frame: NSRect,
    size: f64,
    bold: bool,
) -> Retained<NSTextField> {
    let field = NSTextField::initWithFrame(NSTextField::alloc(mtm), frame);
    field.setStringValue(&NSString::from_str(text));
    field.setBezeled(false);
    field.setDrawsBackground(false);
    field.setEditable(false);
    field.setSelectable(false);
    field.setRefusesFirstResponder(true);
    field.setAlignment(NSTextAlignment::Left);
    field.setTextColor(Some(&NSColor::whiteColor()));
    let font = if bold {
        NSFont::boldSystemFontOfSize(size)
    } else {
        NSFont::systemFontOfSize(size)
    };
    field.setFont(Some(&font));
    field
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn grace_blocks_early_click() {
        let mut b = BreakBanner {
            windows: Vec::new(),
            started: Some(Instant::now()),
            duration_secs: 20,
            countdown: true,
        };
        assert!(!b.click());
        assert!(!b.grace_elapsed());
    }

    #[test]
    fn grace_allows_after_three_seconds() {
        let mut b = BreakBanner {
            windows: Vec::new(),
            started: Some(Instant::now() - Duration::from_millis(3100)),
            duration_secs: 20,
            countdown: true,
        };
        assert!(b.grace_elapsed());
        assert!(b.click());
    }

    #[test]
    fn tick_times_out() {
        let mut b = BreakBanner {
            windows: Vec::new(),
            started: Some(Instant::now() - Duration::from_secs(30)),
            duration_secs: 1,
            countdown: false,
        };
        assert!(b.tick());
    }
}
